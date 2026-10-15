use serde_wasm_bindgen::to_value;
use standard_knowledge::qartod::static_qc::SuiteArgumentType;
use standard_knowledge::qartod::types::ArgumentValue;
use standard_knowledge::qartod::QcTestSuite;
use standard_knowledge::{Standard, StandardsLibrary};
use std::collections::HashMap;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[wasm_bindgen(js_name = StandardsLibrary)]
pub struct StandardsLibraryJS {
    inner: StandardsLibrary,
}

#[wasm_bindgen(js_class = StandardsLibrary)]
impl StandardsLibraryJS {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: StandardsLibrary::default(),
        }
    }

    #[wasm_bindgen(js_name = loadCfStandards)]
    pub fn load_cf_standards(&mut self) {
        self.inner.load_cf_standards();
    }

    #[wasm_bindgen(js_name = loadKnowledge)]
    pub fn load_knowledge(&mut self) {
        self.inner.load_knowledge();
    }

    #[wasm_bindgen]
    pub fn get(&self, name_or_alias: &str) -> Result<StandardJS, JsValue> {
        match self.inner.get(name_or_alias) {
            Ok(standard) => Ok(StandardJS { inner: standard }),
            Err(e) => Err(JsValue::from_str(e)),
        }
    }

    #[wasm_bindgen]
    pub fn filter(&self) -> StandardsFilterJS {
        StandardsFilterJS {
            inner: self.inner.filter(),
        }
    }

    #[wasm_bindgen(js_name = knownIoosCategories)]
    pub fn known_ioos_categories(&self) -> Vec<String> {
        self.inner.known_ioos_categories().into_iter().collect()
    }
}

#[derive(Clone)]
#[wasm_bindgen(js_name = Standard)]
pub struct StandardJS {
    inner: Standard,
}

#[wasm_bindgen(js_class = Standard)]
impl StandardJS {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[wasm_bindgen(getter, js_name = longName)]
    pub fn long_name(&self) -> Option<String> {
        self.inner.long_name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn unit(&self) -> String {
        self.inner.unit.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn aliases(&self) -> Vec<String> {
        self.inner.aliases.clone()
    }

    #[wasm_bindgen(getter, js_name = ioosCategory)]
    pub fn ioos_category(&self) -> Option<String> {
        self.inner.ioos_category.clone()
    }

    #[wasm_bindgen(getter, js_name = commonVariableNames)]
    pub fn common_variable_names(&self) -> Vec<String> {
        self.inner.common_variable_names.clone()
    }

    #[wasm_bindgen(getter, js_name = relatedStandards)]
    pub fn related_standards(&self) -> Vec<String> {
        self.inner.related_standards.clone()
    }

    #[wasm_bindgen(getter, js_name = otherUnits)]
    pub fn other_units(&self) -> Vec<String> {
        self.inner.other_units.clone()
    }

    #[wasm_bindgen(getter, js_name = siblingStandards)]
    pub fn sibling_standards(&self) -> Vec<String> {
        self.inner.sibling_standards.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn comments(&self) -> Option<String> {
        self.inner.comments.clone()
    }

    #[wasm_bindgen]
    pub fn attrs(&self) -> JsValue {
        let attrs_map = self.inner.xarray_attrs();
        let mut js_map = HashMap::new();

        for (key, value) in attrs_map {
            js_map.insert(key.to_string(), value.to_string());
        }

        to_value(&js_map).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn display_short(&self) -> String {
        self.inner.display_short()
    }

    #[wasm_bindgen]
    pub fn display_all(&self) -> String {
        self.inner.display_all()
    }

    #[wasm_bindgen(getter)]
    pub fn qartod(&self) -> Vec<QartodJS> {
        self.inner
            .qartod
            .iter()
            .map(|suite| QartodJS {
                inner: suite.clone(),
            })
            .collect()
    }
}

/// Convert to a plain JS value, with maps as objects
fn to_plain<T: serde::Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

/// A QARTOD test suite, which can scaffold an ioos_qc config
#[wasm_bindgen]
pub struct QartodJS {
    inner: QcTestSuite,
}

#[wasm_bindgen]
impl QartodJS {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.qc.name.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn slug(&self) -> String {
        self.inner.slug.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.inner.qc.description.clone()
    }

    /// The arguments `scaffold` takes, by name:
    /// `{type: "float" | "int", description, required, default?}`
    #[wasm_bindgen(getter)]
    pub fn arguments(&self) -> Result<JsValue, JsError> {
        to_plain(&self.inner.qc.arguments)
    }

    /// The names of the suite's tests, like `gross_range_test`
    #[wasm_bindgen(getter)]
    pub fn tests(&self) -> Vec<String> {
        self.inner
            .test_names()
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// Build an ioos_qc config from `args`, an object of argument values by
    /// name. With `tests`, only those tests are built, and only the arguments
    /// they need are required. Throws an `Error` listing every missing
    /// argument and the tests that need it.
    #[wasm_bindgen]
    pub fn scaffold(&self, args: JsValue, tests: Option<Vec<String>>) -> Result<JsValue, JsError> {
        let values: HashMap<String, f64> = serde_wasm_bindgen::from_value(args)
            .map_err(|e| JsError::new(&format!("arguments must map names to numbers: {e}")))?;

        let mut arguments = HashMap::new();
        for (name, value) in values {
            // Like the CLI, ignore values for arguments the suite doesn't take
            let Some(argument) = self.inner.qc.arguments.get(&name) else {
                continue;
            };
            let value = match argument.argument_type {
                SuiteArgumentType::Float => ArgumentValue::Float(value),
                SuiteArgumentType::Int if value.fract() == 0.0 => ArgumentValue::Int(value as i64),
                SuiteArgumentType::Int => {
                    return Err(JsError::new(&format!(
                        "{name} must be a whole number, found {value}"
                    )))
                }
            };
            arguments.insert(name, value);
        }

        let config = self
            .inner
            .scaffold(arguments, tests.as_deref())
            .map_err(|e| JsError::new(&e))?;
        to_plain(&config)
    }
}

#[wasm_bindgen]
pub struct StandardsFilterJS {
    inner: standard_knowledge::standards_filter::StandardsFilter,
}

#[wasm_bindgen]
impl StandardsFilterJS {
    #[wasm_bindgen(js_name = byVariableName)]
    pub fn by_variable_name(&self, variable_name: &str) -> Self {
        StandardsFilterJS {
            inner: self.inner.by_variable_name(variable_name),
        }
    }

    #[wasm_bindgen(js_name = byIoosCategory)]
    pub fn by_ioos_category(&self, category: &str) -> Self {
        StandardsFilterJS {
            inner: self.inner.by_ioos_category(category),
        }
    }

    #[wasm_bindgen(js_name = byUnit)]
    pub fn by_unit(&self, unit: &str) -> Self {
        StandardsFilterJS {
            inner: self.inner.by_unit(unit),
        }
    }

    #[wasm_bindgen(js_name = hasQartodTests)]
    pub fn has_qartod_tests(&self) -> Self {
        StandardsFilterJS {
            inner: self.inner.has_qartod_tests(),
        }
    }

    #[wasm_bindgen]
    pub fn search(&self, search_str: &str) -> Self {
        StandardsFilterJS {
            inner: self.inner.search(search_str),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn standards(&self) -> Vec<StandardJS> {
        self.inner
            .standards
            .iter()
            .map(|s| StandardJS { inner: s.clone() })
            .collect()
    }
}
