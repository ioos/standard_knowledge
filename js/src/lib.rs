use serde_wasm_bindgen::to_value;
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

    #[wasm_bindgen(js_name = loadTestSuites)]
    pub fn load_test_suites(&mut self) {
        self.inner.load_test_suites();
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
            .clone()
            .iter()
            .map(|q| QartodJS {
                name: q.info().name,
                slug: q.info().slug,
                description: q.info().description,
            })
            .collect()
    }
}

#[wasm_bindgen]
pub struct QartodJS {
    name: String,
    slug: String,
    description: String,
}

#[wasm_bindgen]
impl QartodJS {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn slug(&self) -> String {
        self.slug.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.description.clone()
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
