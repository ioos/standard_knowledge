# Standard Knowledge

Programmatically augmenting CF Standards with IOOS operational knowledge.

```js
import init, { StandardsLibrary } from "./pkg/standard_knowledge_js.js"

await init()

let library = new StandardsLibrary()

library.loadCfStandards()
library.loadKnowledge()

let standard = library.get("air_pressure_at_mean_sea_level")

let attrs = standard.attrs()

let standards = library.filter().byVariableName("pressure")

let underPressure = library.filter().search("pressure")
```

## QARTOD test suites

Each standard's `qartod` lists its test suites. A suite describes its
`arguments` and `tests`, and `scaffold` builds an
[ioos_qc config](https://ioos.github.io/ioos_qc/usage.html#config) from
argument values:

```js
let suite = library
  .get("sea_surface_temperature")
  .qartod.find((q) => q.slug === "qartod_manual")

suite.tests // ["gross_range_test", "rate_of_change_test", ...]
suite.arguments.mean // { type: "float", description: "...", required: true }

// Only build some tests, and only require the arguments they use
let config = suite.scaffold(
  { mean: 5, std: 3, sensor_min: 1, sensor_max: 15 },
  ["gross_range_test"],
)
// { qartod: { gross_range_test: { suspect_span: [-4, 14], fail_span: [1, 15] } } }
```

Without a list of tests, every test is built, and `scaffold` throws an
`Error` listing each missing argument and the tests that need it.

## Testing

The Javascript tests for both library interfaces, and the demo site can be run via `../noxfile.py -s test_js`.

- `npm run test:rust` - Tests at the WASM bindgen layer (`js/tests/pack-smoke.mjs`).
- `npm run test` - Tests the library interfaces with vitest (`js/tests/api.test.ts`).
- `npm run test:e2e` - Runs end to end tests on the demo site using Playwright (`js/tests/e2e/demo.spec.ts`).
- `npm run test:pack` - Makes sure the installable package won't blow up and exposes the correct interfaces (`js/tests/pack-smoke.mjs`).
