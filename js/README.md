# Standard Knowledge

Programmatically augmenting CF Standards with IOOS operational knowledge.

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

## Testing

The Javascript tests for both library interfaces, and the demo site can be run via `../noxfile.py -s test_js`.

- `npm run test:rust` - Tests at the WASM bindgen layer (`js/tests/pack-smoke.mjs`).
- `npm run test` - Tests the library interfaces with vitest (`js/tests/api.test.ts`).
- `npm run test:e2e` - Runs end to end tests on the demo site using Playwright (`js/tests/e2e/demo.spec.ts`).
- `npm run test:pack` - Makes sure the installable package won't blow up and exposes the correct interfaces (`js/tests/pack-smoke.mjs`).
