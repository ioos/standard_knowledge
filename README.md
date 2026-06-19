# standard_knowledge
Programmatically augmenting CF Standards with operational knowledge.

```py
# uv run python
# in py

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

Or in Javascript ([preview here!](https://gulfofmaine.github.io/standard_knowledge/))

```js
import init, { StandardsLibrary } from "./pkg/standard_knowledge_js.js"

await init()

let library = new StandardsLibrary()

library.loadCfStandards()
library.loadKnowledge()
library.loadTestSuites()

let standard = library.get("air_pressure_at_mean_sea_level")

let attrs = standard.attrs()

let standards = library.filter().byVariableName("pressure")

let underPressure = library.filter().search("pressure")
```

A CLI can also be installed for interacting with the standards.

`cargo install --path cli` or `./noxfile.py -s install_cli` from the repo, or `cargo install standard_knowledge_cli` for the published version.

```sh
❯ standard_knowledge --help
Usage: standard_knowledge <COMMAND>

Commands:
  get     Get standard by name or alias
  filter  Filter standards
  qc      QARTOD test suites
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help

❯ standard_knowledge get -f xarray air_pressure_at_mean_sea_level
{
  "ioos_category": "Meteorology",
  "long_name": "Atmospheric Pressure at Sea Level",
  "standard_name": "air_pressure_at_mean_sea_level",
  "units": "Pa",
}

❯ standard_knowledge get air_pressure_at_mean_sea_level
air_pressure_at_mean_sea_level - Atmospheric Pressure at Sea Level - Pa
  Aliases: air_pressure_at_sea_level
  IOOS Category: Meteorology
  Related standards: air_pressure

Air pressure at sea level is the quantity often abbreviated as MSLP or PMSL. Air pressure is the force per unit area which would be exerted when the moving gas molecules of which the air is composed strike a theoretical surface of any orientation. "Mean sea level" means the time mean of sea surface elevation at a given location over an arbitrary period sufficient to eliminate the tidal signals.

❯ standard_knowledge filter --help
Filter standards

Usage: standard_knowledge filter [OPTIONS]

Options:
  -v, --var <VAR>
          Filter by common variable names

  -i, --ioos-category <IOOS_CATEGORY>
          Filter by IOOS category

  -u, --unit <UNIT>
          Filter by unit

  -s, --search <SEARCH>
          Search by string across multiple fields

  -f, --format <FORMAT>
          Format to display in

          [default: short]

          Possible values:
          - short:  Shorthand display,
          - xarray: Xarray attributes

  -h, --help
          Print help (see a summary with '-h')

❯ standard_knowledge filter --ioos-category Meteorology --search temp
- air_temperature - Air Temperature - K

❯ standard_knowledge qc config sea_surface_height_above_geopotential_datum gulf_of_maine mllw=0.2 mhhw=3
Generated configuration for Gulf of Maine:
qartod:
  gross_range_test:
    suspect_span:
    - -1.1716000000000002
    - 4.8288
    fail_span:
    - -1.1716000000000002
    - 4.8288
  location_test: null
  rate_of_change_test:
    rate_threshold: 0.22860000000000003
  spike_test:
    suspect_threshold: 0.22860000000000003
    fail_threshold: 0.45720000000000005
  flat_line_test:
    tolerance: 0.030480000000000004
    suspect_threshold: 7200
    fail_threshold: 10800
```

The CLI can also load other sources of knowledge with `--knowledge`/`-k`. Knowledge can be loaded from local paths, including directories, as well as URLs.

Once `-k` is specified the default knowledge loading is skipped, unless `-k lib` is included.

```sh
standard_knowledge -k lib -k https://gist.githubusercontent.com/abkfenris/ea3cd2eadff0d0ad35fee20d13fb51ab/raw/fce404c8ed3263512281f58d8d1fb629a828323e/multiple.yaml -k ./tests/load_knowledge/odd-filename.yaml get air_temperature
```

## Goals

Provide a cross language way (by packaging Rust into Python, Javascript, and other languages) of sharing learnings from users of CF Standards.

- Translations from CF-ese
- Common column/variable names

## Contributing Knowledge

The core of the library isn't the code, but the knowledge that we have gained as a community in implementing the CF Standards in our work.

The knowledge is stored as YAML files in [core/standards/](./core/standards/) by `<standard_name>.yaml`.

```yaml
# core/standards/air_pressure_at_mean_sea_level.yaml
ioos_category: Meteorology
long_name: Atmospheric Pressure at Sea Level
common_variable_names:
- pressure
- atmospheric_pressure
- sea_level_pressure
related_standards:
- air_pressure
sibling_standards:
- air_temperature
extra_attrs:
  coverage_content_type: physicalMeasurement
  ncei_name: PRESSURE - BAROMETRIC
  standard_name_url: https://vocab.nerc.ac.uk/collection/P07/current/CFSN0015
other_units:
- kPa
- bar
comments: |
  Raw pressure sensor values on buoys may need to be adjusted based on sensor tower height.
```

Knowledge keys:

- `ioos_category` - Category of measurement for the Integrated Ocean Observing System
- `long_name` - A more human readable name for the standard
- `common_variable_names` - When the standard name isn't used for a column or variable, what might commonly get used instead.
- `related_standards` - Standards that measure generally similar things, but differ in specifics that are worth investigating.
- `sibling_standards` - Standards that are usually used together.
- `extra_attrs` - Dictionary of extra attributes to be applied to Xarray or ERDDAP.
- `other_units` - Other units that may be used rather than the one defined in the standard.
- `comments` - What others may need to know about a standard. How is the standard used, rather than the CF description of how it is defined. Notes about implementation.

> [!NOTE]
>
> - IOOS categories are not (_currently_) validated, but the set of known values (derived from ERDDAP's internal list) is in [core/src/ioos_categories.rs](./core/src/ioos_categories.rs).

## Contributing Code

Cargo as manages the Rust components of the project, while Maturin and uv help keep things inline when working from the Python side of things.

### Rust Testing

`cargo test` will run tests in all the workspaces.

As the CLI changes, it's tests should be updated with `TRYCMD=overwrite cargo test`.

For new CLI tests, it's easiest to copy one of the files in `cli/tests/cmd`, and tweak the `args` to match the new command, then run `TRYCMD=overwrite cargo test` to replace the status code, stdout and stderr.

Can be run with `./noxfile.py -s test_rust`

### Python testing

From `py`, `uv run pytest` will run tests.
It will also pick up changes in Rust, both for the Python bindings and changes in the core library as well.
`uv run python` will open a shell with the library rebuilt for interactive tinkering.

### Utils

- `utils/update_standards.py` - Run with `uv run --script utils/update_standards.py` to update the standard names and alias files from CF Conventions that are imported into the Rust library.

### Nox

`./noxfile.py` has helpers for testing, bumping versions, updating standards, and other things that are easy to forget how to do.

Unless a session is specified, Nox will run all but `release` sessions.

Sessions (use `-s`):
- `update-standards` - Updates the CF standards.
- `release` -- <patch|minor|major> - Bumps the version of the all the packages for a release.
- `test_ptyhon-<version>` - Test a Python version as specified in the Github actions matrix.
- `wheel` - Build Linux wheels for currently supported Python versions.
- `wheel_wasm` - Build 3.14 WASM wheel.
-
