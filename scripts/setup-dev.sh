#!/usr/bin/env bash
# Bootstrap the dev environment. Used by .devcontainer onCreateCommand
# and .github/workflows/copilot-setup-steps.yml. Idempotent.
set -euxo pipefail
cd "$(git rev-parse --show-toplevel)"

mise trust 2>/dev/null || true
mise install
# Make mise-managed tools available to the rest of this script
eval "$(mise activate bash --shims)"

# Keep in sync with [tool.cibuildwheel.pyodide] RUSTUP_TOOLCHAIN in py/pyproject.toml.
# Redundant while mise.toml pins rust 1.93.0; keeps pyodide wheel builds working
# if the default toolchain is bumped past it.
rustup toolchain install 1.93.0 --profile minimal --target wasm32-unknown-emscripten

cargo fetch

(cd py && uv sync --no-install-project)

(cd js && npm ci && npx playwright install --with-deps chromium)

prek install-hooks
# Register the git hook only where there's a real checkout to commit from
if [ "${CI:-}" != "true" ]; then prek install; fi
