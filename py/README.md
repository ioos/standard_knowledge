# Standard Knowledge

Programmatically augmenting CF Standards with IOOS operational knowledge.

```py
# uv run python

import standard_knowledge

library = standard_knowledge.StandardsLibrary()

# Load all CF standards
library.load_cf_standards()

# Apply community knowledge to the standards
library.load_knowledge()

# Get a standard by name or alias
standard = library.get("air_pressure_at_mean_sea_level")

# Xarray compatible attributes for a standard
attrs = standard.attrs()

# find standards by variable names
standards = library.filter().by_variable_name("pressure")
# Notice the `.filter()`? It returns a StandardsFilter object,
# so you can chain multiple filters together.
# by_ioos_category, by_unit, has_qartod_tests

# Search for standards across multiple fields (name, aliases, common variable names, related standards)
under_pressure = library.filter().search("pressure")
```

## Testing

Test with `uv run pytest`, or `../noxfile.py -s test_python` to test against multiple versions.

## Building

Run `uvx cibuildwheel --platform linux py` from the top of the repo to build.

Note: [cibuildwheel only uses official builds](https://github.com/pypa/cibuildwheel/issues/2502), so it'll get ornery with Python from other sources (uv, Pixi, Brew).

`../noxfile.py -s wheel` should run that as well.

### Building Pyodide/WASM wheels locally

Pyodide wheels require specific Rust and Pyodide versions due to Emscripten compatibility:

```bash
# Install the compatible Rust version
rustup install nightly-2025-01-20
rustup target add wasm32-unknown-emscripten --toolchain nightly-2025-01-20

# Build pyodide wheels with pinned Rust toolchain and Pyodide 0.27.7
RUSTUP_TOOLCHAIN=nightly-2025-01-20 CIBW_BUILD=cp314-pyodide_wasm32 CIBW_PYODIDE_VERSION=314.0.0 uvx cibuildwheel --platform pyodide py
```

Try using `../noxfile.py -s wheel_wasm` which should encapsulate those commands.
