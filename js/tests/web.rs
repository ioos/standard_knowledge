//! Test suite for the Web and headless browsers.
//!
//! These exercise the wasm-bindgen layer directly (the `*JS` wrappers in
//! `src/lib.rs`), catching serde-wasm-bindgen marshaling regressions before
//! any JavaScript is involved. Run with `wasm-pack test --headless --chrome`.

#![cfg(target_arch = "wasm32")]

use standard_knowledge_js::StandardsLibraryJS;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn loaded_library() -> StandardsLibraryJS {
    let mut library = StandardsLibraryJS::new();
    library.load_cf_standards();
    library.load_knowledge();
    library.load_test_suites();
    library
}

#[wasm_bindgen_test]
fn get_by_name() {
    let library = loaded_library();
    let standard = library.get("air_pressure_at_mean_sea_level").unwrap();
    assert_eq!(standard.name(), "air_pressure_at_mean_sea_level");
    assert_eq!(standard.unit(), "Pa");
}

#[wasm_bindgen_test]
fn get_by_alias() {
    let library = loaded_library();
    let standard = library.get("air_pressure_at_sea_level").unwrap();
    assert_eq!(standard.name(), "air_pressure_at_mean_sea_level");
}

#[wasm_bindgen_test]
fn get_unknown_is_err() {
    let library = loaded_library();
    assert!(library.get("not_a_real_standard").is_err());
}

#[wasm_bindgen_test]
fn knowledge_adds_metadata() {
    let library = loaded_library();
    let standard = library.get("air_pressure_at_mean_sea_level").unwrap();
    assert_eq!(standard.ioos_category(), Some("Meteorology".to_string()));
    assert!(standard
        .common_variable_names()
        .contains(&"pressure".to_string()));
}

#[wasm_bindgen_test]
fn filter_by_variable_name() {
    let library = loaded_library();
    let filtered = library.filter().by_variable_name("atmospheric_pressure");
    let names: Vec<String> = filtered.standards().iter().map(|s| s.name()).collect();
    assert!(names.contains(&"air_pressure_at_mean_sea_level".to_string()));
}

#[wasm_bindgen_test]
fn filter_has_qartod_tests() {
    let library = loaded_library();
    let filtered = library.filter().has_qartod_tests();
    let standards = filtered.standards();
    assert!(!standards.is_empty());
    assert!(standards.iter().all(|s| !s.qartod().is_empty()));
}

#[wasm_bindgen_test]
fn known_ioos_categories_present() {
    let library = loaded_library();
    let categories = library.known_ioos_categories();
    assert!(categories.contains(&"Meteorology".to_string()));
}
