#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["nox", "pyyaml"]
# ///
"""Test against the same matrix as Github Actions, build wheels, and
bump versions for release."""

import argparse

import nox
import yaml

nox.needs_version = ">= 2025.10.14"
nox.options.default_venv_backend = "uv|virtualenv"

with open("./.github/workflows/python.yml") as f:
    workflow = yaml.safe_load(f)

python_versions = workflow["jobs"]["test"]["strategy"]["matrix"]["python-version"]


@nox.session(default=False)
def release(session: nox.Session) -> None:
    """
    Kicks off an automated release process by bumping the package version.

    Invokes cargo-edit set-version with the posarg setting the version.

    Usage:
    $ nox -s release -- [major|minor|patch]
    """
    parser = argparse.ArgumentParser(description="Release a semver version.")
    parser.add_argument(
        "version",
        type=str,
        nargs=1,
        help="The type of semver release to make.",
        choices={"major", "minor", "patch"},
    )
    args: argparse.Namespace = parser.parse_args(args=session.posargs)
    version: str = args.version.pop()

    session.log(f"Dry run bumping the {version!r} version to check for errors")
    session.run("cargo", "set-version", "--dry-run", "--bump", version, external=True)

    # If we get here, we should be good to go
    # Let's do a final check for safety
    confirm = input(
        f"You are about to bump the {version!r} version. Are you sure? [y/n]: "
    )

    # Abort on anything other than 'y'
    if confirm.lower().strip() != "y":
        session.error(f"You said no when prompted to bump the {version!r} version.")

    session.log(f"Bumping the {version!r} version")
    session.run("cargo", "set-version", "--bump", version, external=True)


@nox.session(python=python_versions)
def test_python(session: nox.Session) -> None:
    """Run the Python tests."""
    session.cd("py")
    session.run("uv", "run", "pytest")


@nox.session
def wheel(session: nox.Session) -> None:
    """Build Linux wheels for multiple python versions"""
    session.run("uvx", "cibuildwheel", "--platform", "linux", "py")


@nox.session
def wheel_wasm(session: nox.Session) -> None:
    """Build a wheel for the pyodide target."""
    session.run(
        "uvx",
        "cibuildwheel",
        "--platform",
        "pyodide",
        "py",
        env={
            # Must match the toolchain Pyodide expects for this version:
            # `PYODIDE_ROOT=<xbuildenv> pyodide config get rust_toolchain`.
            # For Pyodide 314.0.0 that is stable 1.93.0, whose
            # wasm32-unknown-emscripten target defaults to native wasm
            # exception handling (matching Pyodide's runtime). An older
            # nightly defaults to the legacy Emscripten EH/SjLj ABI, which
            # emits `invoke_*` imports the Pyodide runtime can't resolve
            # ("Dynamic linking error: cannot resolve symbol invoke_i").
            "RUSTUP_TOOLCHAIN": "1.93.0",
            "CIBW_BUILD": "cp314-pyodide_wasm32",
            "CIBW_PYODIDE_VERSION": "314.0.0",
        },
    )


@nox.session(default=False, python=False)
def test_wasm(session: nox.Session) -> None:
    """Create a virtual environment for testing the pyodide wheel."""
    python_path = ".venv-pyodide/bin/python"

    session.run("pyodide", "venv", ".venv-pyodide")
    session.run(
        python_path,
        "-m",
        "pip",
        "install",
        "wheelhouse/standard_knowledge-0.1.0-cp314-cp314-pyemscripten_2026_0_wasm32.whl",
    )

    pyproject = nox.project.load_toml("py/pyproject.toml")
    session.run(
        python_path,
        "-m",
        "pip",
        "install",
        *nox.project.dependency_groups(pyproject, "dev"),
    )

    session.run(python_path, "-m", "pytest")


if __name__ == "__main__":
    nox.main()
