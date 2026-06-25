#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["nox", "pyyaml"]
# ///
"""Test against the same matrix as Github Actions, build wheels, and
bump versions for release."""

import argparse
import os

import nox
import yaml

nox.needs_version = ">= 2025.10.14"
nox.options.default_venv_backend = "uv|virtualenv"

with open("./.github/workflows/python.yml") as f:
    workflow = yaml.safe_load(f)

python_versions = workflow["jobs"]["test"]["strategy"]["matrix"]["python-version"]


@nox.session(default=False, python=False)
def update_standards(session: nox.Session) -> None:
    """Update CF standards"""
    session.run("uv", "run", "--script", "utils/update_standards.py")


@nox.session(python=False, default=False)
def install_cli(session: nox.Session) -> None:
    """Install the CLI tool."""
    session.run("cargo", "install", "--path", "cli", external=True)


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
def py_test(session: nox.Session) -> None:
    """Run the Python tests."""
    session.cd("py")
    session.run("uv", "run", "pytest")


@nox.session
def py_wheel(session: nox.Session) -> None:
    """Build Linux wheels for multiple python versions"""
    session.run("uvx", "cibuildwheel", "--platform", "linux", "py")


@nox.session
def py_wheel_wasm(session: nox.Session) -> None:
    """Build a wheel for the pyodide target."""
    session.run(
        "uvx",
        "cibuildwheel",
        "--platform",
        "pyodide",
        "py",
    )


@nox.session(default=False, python=False)
def py_test_wasm_env(session: nox.Session) -> None:
    """Create a virtual environment for testing the pyodide wheel.

    Now pytest is automatically run after building WASM wheels.
    """
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


@nox.session(python=False)
def rust_test(session: nox.Session) -> None:
    """Run the Rust tests.

    Strips a lingering ``VIRTUAL_ENV`` before invoking cargo. The
    ``test_wasm_env`` session leaves ``.venv-pyodide`` activated, and PyO3's
    build script honors ``VIRTUAL_ENV`` (regardless of PATH) to locate the
    interpreter. That venv is a 32-bit wasm32/emscripten target, which makes
    native ``cargo test`` fail with "your Rust target architecture (64-bit)
    does not match your python interpreter (32-bit)".

    Excludes ``standard_knowledge_py``: it has no Rust tests (the bindings are
    tested with pytest in ``test_python``), and building its lib-test binary
    links against libpython. After a pyodide wheel build (``wheel_wasm`` /
    ``test_wasm_env``) the cached PyO3 config points at the emscripten
    ``/install`` prefix, so that binary fails to load ``libpython*.dylib``.
    Skipping the crate keeps ``cargo test`` independent of those builds.
    """
    venv = os.environ.pop("VIRTUAL_ENV", None)
    if venv:
        session.warn(
            f"Ignoring VIRTUAL_ENV={venv!r} so cargo/PyO3 builds against the "
            "host Python interpreter."
        )
    session.run(
        "cargo",
        "test",
        "--workspace",
        "--exclude",
        "standard_knowledge_py",
        external=True,
    )


@nox.session(python=False, default=False)
def rust_test_no_embedded(session: nox.Session) -> None:
    """Run core Rust tests without the embedded-data feature.

    Verifies the JS-build path: YAML-string and JSON-string ingestion work
    without the compressed blobs compiled in.
    """
    venv = os.environ.pop("VIRTUAL_ENV", None)
    if venv:
        session.warn(
            f"Ignoring VIRTUAL_ENV={venv!r} so cargo/PyO3 builds against the "
            "host Python interpreter."
        )
    session.run(
        "cargo",
        "test",
        "-p",
        "standard_knowledge",
        "--no-default-features",
        external=True,
    )


@nox.session(python=False, default=False)
def rust_update_tests(session: nox.Session) -> None:
    """Update the Rust test fixtures."""
    session.run(
        "cargo",
        "test",
        "--workspace",
        "--exclude",
        "standard_knowledge_py",
        external=True,
        env={"TRYCMD": "overwrite"},
    )


@nox.session(python=False, default=False)
def generate_partitions(session: nox.Session) -> None:
    """Generate per-IOOS-category, all-knowledge, and all-standards partition
    files (JSON + YAML) from core/standards/*.yaml → data/.

    These are the canonical release artifacts consumed by Phase 2e CI uploads
    and (eventually) the npm data package. The JS dev/test workflow generates
    its own copy via ``npm run wasm`` → gen-data.mjs → public/data/.

    Run: nox -s generate_partitions
    """
    session.run(
        "uv",
        "run",
        "--script",
        "utils/generate_partitions.py",
        external=True,
    )


@nox.session(python=False)
def js_test(session: nox.Session) -> None:
    """Run the JavaScript/WASM tests: Vitest API suite, Playwright demo E2E,
    and the packaged-tarball smoke test.

    Mirrors the ``test`` job in ``.github/workflows/javascript.yml``.
    """
    session.cd("js")
    session.run("npm", "ci", external=True)
    session.run("npm", "run", "wasm", external=True)
    session.run(
        "npx", "playwright", "install", "--with-deps", "chromium", external=True
    )
    session.run("npm", "test", external=True)
    session.run("npm", "run", "test:e2e", external=True)
    session.run("npm", "run", "test:pack", external=True)


@nox.session(python=False, default=False)
def js_test_wasm(session: nox.Session) -> None:
    """Run the Rust->WASM headless binding tests (wasm-pack test).

    Opt-in (not in the default sweep): ``wasm-pack test --chrome`` drives the
    locally installed Chrome with a chromedriver that wasm-pack downloads to
    match the *latest* Chrome, so it fails (HTTP 404 on session creation) when
    the installed Chrome is a major version behind. CI pins a matching pair on
    Linux and runs this step directly in ``javascript.yml``; locally the same
    bindings are also exercised by the Vitest browser suite in ``test_js``.
    Run explicitly with ``nox -s test_js_wasm`` when your Chrome matches.
    """
    session.cd("js")
    session.run("wasm-pack", "test", "--headless", "--chrome", external=True)


@nox.session(python=False)
def js_build(session: nox.Session) -> None:
    """Build the JavaScript/WASM tool."""
    session.cd("js")
    session.run("npm", "run", "build", external=True)


@nox.session(python=False, default=False)
def js_dev(session: nox.Session) -> None:
    """Run the JavaScript/WASM development environment."""
    session.cd("js")
    session.run("npm", "run", "dev", external=True)


@nox.session(python=False, default=False)
def find_standards(session: nox.Session) -> None:
    """Run the find_standards_app.py web service.

    Run: nox -s find_standards
    """
    session.run(
        "uv",
        "run",
        "--script",
        "utils/find_standards_app.py",
        external=True,
    )


@nox.session(python=False, default=False)
def data_pkg_build(session: nox.Session) -> None:
    """Build the standard_knowledge_data npm package.

    Reads core/standards/*.yaml and emits tree-shakable ESM modules to
    data-pkg/ (all-standards.js, all-knowledge.js, partitions/{slug}.js).
    Output is gitignored; CI regenerates it before publishing.

    Run: nox -s data_pkg_build
    """
    session.cd("js-data-pkg")
    session.run("npm", "ci", external=True)
    session.run("npm", "run", "build", external=True)


if __name__ == "__main__":
    nox.main()
