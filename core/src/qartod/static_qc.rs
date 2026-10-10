use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use indexmap::IndexMap;

use super::config::{ConfigStream, Num};
use super::types::{
    ArgumentType, ArgumentValue, QartodTestTypes, TestArgument, TestSuite, TestSuiteInfo,
};
use crate::fx;

include!("./static_qc_include.rs");

#[derive(Debug, Clone)]
pub struct StaticQcTestSuite {
    pub slug: String,
    pub qc: StaticQc,
}

impl TestSuite for StaticQcTestSuite {
    fn info(&self) -> TestSuiteInfo {
        let test_types = QartodTestTypes::tests_in_config(&self.qc.tests);
        let arguments = self
            .qc
            .arguments
            .iter()
            .map(|(name, argument)| {
                (
                    name.clone(),
                    TestArgument {
                        argument_type: match argument.argument_type {
                            SuiteArgumentType::Float => ArgumentType::Float,
                            SuiteArgumentType::Int => ArgumentType::Int,
                        },
                        description: argument.description.clone(),
                        required: argument.required,
                    },
                )
            })
            .collect();
        TestSuiteInfo {
            slug: self.slug.clone(),
            name: self.qc.name.clone(),
            summary: self.qc.summary.clone(),
            description: self.qc.description.clone(),
            arguments,
            test_types,
        }
    }

    fn scaffold(&self, arguments: HashMap<String, ArgumentValue>) -> Result<ConfigStream, String> {
        self.qc.scaffold(&arguments)
    }
}

impl StaticQc {
    /// Build the ioos_qc config, evaluating any expressions with `arguments`
    pub fn scaffold(
        &self,
        arguments: &HashMap<String, ArgumentValue>,
    ) -> Result<ConfigStream, String> {
        let scope = self.scope(arguments)?;
        self.tests
            .try_map(&mut |value| number(value, &scope), &mut |value| {
                let value = number(value, &scope)?;
                if value.fract() == 0.0 && value.abs() <= isize::MAX as f64 {
                    Ok(value as isize)
                } else {
                    Err(format!("Expected a whole number, found {value}"))
                }
            })
    }

    /// Scaffold with every argument set to its default, or 1 when it has
    /// none, to check that the suite is well formed
    pub fn check(&self) -> Result<ConfigStream, String> {
        let arguments = self
            .arguments
            .iter()
            .map(|(name, argument)| {
                let value = argument.default.unwrap_or(1.0);
                let value = match argument.argument_type {
                    SuiteArgumentType::Float => ArgumentValue::Float(value),
                    SuiteArgumentType::Int => ArgumentValue::Int(value as i64),
                };
                (name.clone(), value)
            })
            .collect();
        self.scaffold(&arguments)
    }

    /// The variables expressions can use: arguments, then `let` constants
    fn scope(
        &self,
        arguments: &HashMap<String, ArgumentValue>,
    ) -> Result<HashMap<String, f64>, String> {
        if arguments.is_empty() && self.arguments.values().any(|a| a.required) {
            return Err(format!("No arguments provided for {} tests", self.name));
        }

        let mut scope = HashMap::new();
        for (name, argument) in &self.arguments {
            check_name(name)?;
            let value = match arguments.get(name) {
                Some(ArgumentValue::Float(value)) => *value,
                Some(ArgumentValue::Int(value)) => *value as f64,
                Some(other) => {
                    return Err(format!("Argument {name} must be a number, not {other:?}"))
                }
                None => match argument.default {
                    Some(default) => default,
                    None if argument.required => {
                        return Err(format!("Missing required argument: {name}"))
                    }
                    None => continue,
                },
            };
            scope.insert(name.clone(), value);
        }

        for (name, value) in &self.constants {
            check_name(name)?;
            let value = number(value, &scope)?;
            scope.insert(name.clone(), value);
        }

        Ok(scope)
    }
}

fn check_name(name: &str) -> Result<(), String> {
    if fx::is_valid_variable_name(name) {
        Ok(())
    } else {
        Err(format!(
            "{name:?} can't be used as a variable name in expressions"
        ))
    }
}

/// A number, or the result of an expression
fn number(value: &Num, scope: &HashMap<String, f64>) -> Result<f64, String> {
    match value {
        Num::Value(value) => Ok(*value),
        Num::Expr(source) => {
            let result = fx::eval_fx(source, scope).map_err(|e| format!("{source:?}: {e}"))?;
            if result.is_finite() {
                Ok(result)
            } else {
                Err(format!("{source:?} evaluated to {result}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StandardsLibrary;

    fn library() -> StandardsLibrary {
        let mut library = StandardsLibrary::default();
        library.load_cf_standards();
        library.load_knowledge();
        library
    }

    fn suite(library: &StandardsLibrary, standard: &str, slug: &str) -> Box<dyn TestSuite> {
        let standard = library.get(standard).unwrap();
        dyn_clone::clone_box(
            &**standard
                .qartod
                .iter()
                .find(|s| s.info().slug == slug)
                .unwrap_or_else(|| panic!("no {slug} suite for {}", standard.name)),
        )
    }

    #[test]
    fn every_bundled_suite_scaffolds() {
        for standard in library().standards.values() {
            for suite in &standard.qartod {
                let info = suite.info();
                let arguments = info
                    .arguments
                    .iter()
                    .map(|(name, argument)| {
                        let value = match argument.argument_type {
                            ArgumentType::Int => ArgumentValue::Int(1),
                            _ => ArgumentValue::Float(1.0),
                        };
                        (name.clone(), value)
                    })
                    .collect();
                suite.scaffold(arguments).unwrap_or_else(|e| {
                    panic!("{} {} failed to scaffold: {e}", standard.name, info.slug)
                });
            }
        }
    }

    // The Gulf of Maine checks moved from the Rust implementation that this
    // YAML suite replaced. The expected values are unchanged.

    #[test]
    fn gulf_of_maine_info() {
        let library = library();
        let gulf = suite(
            &library,
            "sea_surface_height_above_geopotential_datum",
            "gulf_of_maine",
        );
        let info = gulf.info();
        assert_eq!(info.name, "Gulf of Maine");
        assert_eq!(
            info.test_types,
            vec![
                QartodTestTypes::GrossRange,
                QartodTestTypes::Spike,
                QartodTestTypes::RateOfChange,
                QartodTestTypes::FlatLine,
            ]
        );
        assert!(info.arguments["mllw"].required);
        assert!(info.arguments["mhhw"].required);
    }

    #[test]
    fn gulf_of_maine_scaffold_no_args() {
        let library = library();
        let gulf = suite(
            &library,
            "sea_surface_height_above_geopotential_datum",
            "gulf_of_maine",
        );
        assert_eq!(
            gulf.scaffold(HashMap::new()).unwrap_err(),
            "No arguments provided for Gulf of Maine tests"
        );
    }

    #[test]
    fn gulf_of_maine_missing_arg() {
        let library = library();
        let gulf = suite(
            &library,
            "sea_surface_height_above_geopotential_datum",
            "gulf_of_maine",
        );
        let args = HashMap::from([("mhhw".to_string(), ArgumentValue::Float(1.0))]);
        assert_eq!(
            gulf.scaffold(args).unwrap_err(),
            "Missing required argument: mllw"
        );
    }

    #[test]
    fn gulf_of_maine_scaffold_success() {
        use crate::qartod::config::{FlatLine, GrossRangeTest, RateOfChange, Spike};

        let library = library();
        let gulf = suite(
            &library,
            "sea_surface_height_above_geopotential_datum",
            "gulf_of_maine",
        );
        let args = HashMap::from([
            ("mllw".to_string(), ArgumentValue::Float(0.0)),
            ("mhhw".to_string(), ArgumentValue::Float(1.0)),
        ]);
        let config = gulf.scaffold(args).unwrap();

        assert_eq!(
            config.qartod.gross_range_test,
            Some(GrossRangeTest {
                suspect_span: (-1.3716000000000002, 2.8288),
                fail_span: (-1.3716000000000002, 2.8288),
            })
        );
        assert_eq!(
            config.qartod.rate_of_change_test,
            Some(RateOfChange {
                threshold: 0.22860000000000003,
            })
        );
        assert_eq!(
            config.qartod.spike_test,
            Some(Spike {
                suspect_threshold: 0.22860000000000003,
                fail_threshold: 0.45720000000000005,
            })
        );
        assert_eq!(
            config.qartod.flat_line_test,
            Some(FlatLine {
                tolerance: 0.030480000000000004,
                suspect_threshold: 7200,
                fail_threshold: 10800,
            })
        );
    }

    fn template(yaml: &str) -> StaticQc {
        serde_yaml_ng::from_str(yaml).unwrap()
    }

    #[test]
    fn defaults_and_constants() {
        let qc = template(
            r#"
name: Example
summary: s
description: d
arguments:
  k: {type: float, description: multiplier, default: 3}
let:
  sd: 0.5
  span: {expr: "k * sd"}
tests:
  qartod:
    spike_test:
      suspect_threshold: {expr: "span"}
      fail_threshold: {expr: "2 * span"}
"#,
        );
        let config = qc.scaffold(&HashMap::new()).unwrap();
        let spike = config.qartod.spike_test.unwrap();
        assert_eq!(spike.suspect_threshold, 1.5);
        assert_eq!(spike.fail_threshold, 3.0);

        let args = HashMap::from([("k".to_string(), ArgumentValue::Int(2))]);
        let spike = qc.scaffold(&args).unwrap().qartod.spike_test.unwrap();
        assert_eq!(spike.suspect_threshold, 1.0);
    }

    #[test]
    fn flat_line_thresholds_must_be_whole() {
        let qc = template(
            r#"
name: Example
summary: s
description: d
tests:
  qartod:
    flat_line_test:
      tolerance: 0.1
      suspect_threshold: {expr: "1.5 * 3600"}
      fail_threshold: 7200.5
"#,
        );
        assert!(qc.check().unwrap_err().contains("whole number"));
    }

    #[test]
    fn expression_errors_name_the_expression() {
        let qc = template(
            r#"
name: Example
summary: s
description: d
tests:
  qartod:
    rate_of_change_test:
      threshold: {expr: "1 / 0"}
"#,
        );
        let error = qc.check().unwrap_err();
        assert!(error.contains("\"1 / 0\""), "{error}");

        let qc = template(
            r#"
name: Example
summary: s
description: d
let:
  pi: 3
tests: {qartod: {}}
"#,
        );
        assert!(qc.check().unwrap_err().contains("\"pi\""));
    }

    #[test]
    fn invalid_test_shape_is_reported() {
        let error = serde_yaml_ng::from_str::<StaticQc>(
            r#"
name: Example
summary: s
description: d
tests:
  qartod:
    spike_test:
      suspect_threshold: 1
"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("fail_threshold"), "{error}");
    }
}
