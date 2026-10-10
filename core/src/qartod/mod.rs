use std::collections::HashMap;

// // Re-export config types for convenience
// pub use config::{ArgumentValue, Call, Config, ConfigBuilder, Context, ContextConfig, TimeWindow};

pub mod config;
pub mod static_qc;
pub mod types;

pub use static_qc::StaticQcTestSuite;
pub use types::TestSuite;

/// Test suites implemented in Rust, by standard name.
///
/// There are none at the moment: every suite, including the water level
/// suites that used to live here, is defined in the knowledge YAML and
/// loaded by `StandardsLibrary::load_knowledge`.
pub fn test_suites() -> HashMap<String, Vec<Box<dyn TestSuite>>> {
    HashMap::new()
}
