use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use indexmap::IndexMap;

use super::config::{ConfigStream, Num};
use super::types::{ArgumentType, ArgumentValue, QartodTestTypes, TestArgument, TestSuiteInfo};
use crate::fx;

include!("./static_qc_include.rs");

#[derive(Debug, Clone, PartialEq)]
pub struct QcTestSuite {
    pub slug: String,
    pub qc: StaticQc,
}

impl QcTestSuite {
    pub fn info(&self) -> TestSuiteInfo {
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

    /// This should return a Config that represents an `ioos_qc.Config`
    /// https://ioos.github.io/ioos_qc/usage.html#config
    ///
    /// With `tests`, only those tests (by field name, like
    /// `gross_range_test`) are generated, and only the arguments they need
    /// are required.
    pub fn scaffold(
        &self,
        arguments: HashMap<String, ArgumentValue>,
        tests: Option<&[String]>,
    ) -> Result<ConfigStream, String> {
        self.qc.scaffold(&arguments, tests)
    }
}

/// Names of the tests in a stream, in a fixed order
fn present_tests<N, I>(stream: &ConfigStream<N, I>) -> Vec<&'static str> {
    let q = &stream.qartod;
    [
        ("gross_range_test", q.gross_range_test.is_some()),
        ("location_test", q.location_test.is_some()),
        ("rate_of_change_test", q.rate_of_change_test.is_some()),
        ("spike_test", q.spike_test.is_some()),
        ("flat_line_test", q.flat_line_test.is_some()),
        ("attenuated_signal_test", q.attenuated_signal_test.is_some()),
    ]
    .into_iter()
    .filter_map(|(name, present)| present.then_some(name))
    .collect()
}

/// A copy of `stream` with only the tests named in `keep`
fn keep_tests(stream: &ConfigStream<Num, Num>, keep: &[&str]) -> ConfigStream<Num, Num> {
    let mut stream = stream.clone();
    let q = &mut stream.qartod;
    if !keep.contains(&"gross_range_test") {
        q.gross_range_test = None;
    }
    if !keep.contains(&"location_test") {
        q.location_test = None;
    }
    if !keep.contains(&"rate_of_change_test") {
        q.rate_of_change_test = None;
    }
    if !keep.contains(&"spike_test") {
        q.spike_test = None;
    }
    if !keep.contains(&"flat_line_test") {
        q.flat_line_test = None;
    }
    if !keep.contains(&"attenuated_signal_test") {
        q.attenuated_signal_test = None;
    }
    stream
}

/// The variables read by the expressions in `stream`
fn stream_variables(stream: &ConfigStream<Num, Num>) -> Result<BTreeSet<String>, String> {
    let found = RefCell::new(BTreeSet::new());
    let collect = |value: &Num| -> Result<(), String> {
        if let Num::Expr(source) = value {
            let expression = fx::parse(source).map_err(|e| format!("{source:?}: {e}"))?;
            found.borrow_mut().extend(expression.variables());
        }
        Ok(())
    };
    stream.try_map(&mut |v| collect(v), &mut |v| collect(v))?;
    Ok(found.into_inner())
}

impl StaticQc {
    /// Build the ioos_qc config, evaluating any expressions with `arguments`.
    ///
    /// Only the tests named in `tests` are built (all of them when it is
    /// `None` or empty), and only the arguments and constants they need are
    /// resolved.
    pub fn scaffold(
        &self,
        arguments: &HashMap<String, ArgumentValue>,
        tests: Option<&[String]>,
    ) -> Result<ConfigStream, String> {
        let available = present_tests(&self.tests);
        let selected: Vec<&str> = match tests {
            Some(tests) if !tests.is_empty() => {
                if let Some(unknown) = tests.iter().find(|t| !available.contains(&t.as_str())) {
                    return Err(format!(
                        "{} has no test named {unknown}. Its tests are: {}",
                        self.name,
                        available.join(", ")
                    ));
                }
                available
                    .iter()
                    .copied()
                    .filter(|t| tests.iter().any(|s| s == t))
                    .collect()
            }
            _ => available,
        };

        // Which tests need each argument, and every name they need
        let mut needed = BTreeSet::new();
        let mut argument_tests: BTreeMap<String, Vec<&str>> = BTreeMap::new();
        for test in &selected {
            let names = self.names_needed(&keep_tests(&self.tests, &[test]))?;
            for name in &names {
                if self.arguments.contains_key(name) && !self.constants.contains_key(name) {
                    argument_tests.entry(name.clone()).or_default().push(test);
                }
            }
            needed.extend(names);
        }

        let scope = self.scope(arguments, &needed, &argument_tests)?;
        keep_tests(&self.tests, &selected).try_map(
            &mut |value| number(value, &scope),
            &mut |value| {
                let value = number(value, &scope)?;
                if value.fract() == 0.0 && value.abs() <= isize::MAX as f64 {
                    Ok(value as isize)
                } else {
                    Err(format!("Expected a whole number, found {value}"))
                }
            },
        )
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
        self.scaffold(&arguments, None)
    }

    /// Every argument and constant that the expressions in `stream` read,
    /// directly or through the constants they use
    fn names_needed(&self, stream: &ConfigStream<Num, Num>) -> Result<BTreeSet<String>, String> {
        let mut needed = BTreeSet::new();
        let mut pending: Vec<String> = stream_variables(stream)?.into_iter().collect();
        while let Some(name) = pending.pop() {
            if !needed.insert(name.clone()) {
                continue;
            }
            if let Some(Num::Expr(source)) = self.constants.get(&name) {
                let expression = fx::parse(source).map_err(|e| format!("{source:?}: {e}"))?;
                pending.extend(expression.variables());
            }
        }
        Ok(needed)
    }

    /// The variables expressions can use: arguments, then `let` constants,
    /// limited to those in `needed`. `argument_tests` says which tests need
    /// each argument, for the error when required ones are missing.
    fn scope(
        &self,
        arguments: &HashMap<String, ArgumentValue>,
        needed: &BTreeSet<String>,
        argument_tests: &BTreeMap<String, Vec<&str>>,
    ) -> Result<HashMap<String, f64>, String> {
        for name in self.arguments.keys().chain(self.constants.keys()) {
            check_name(name)?;
        }

        let mut scope = HashMap::new();
        let mut missing = Vec::new();
        for (name, argument) in &self.arguments {
            if !needed.contains(name) {
                continue;
            }
            let value = match arguments.get(name) {
                Some(ArgumentValue::Float(value)) => *value,
                Some(ArgumentValue::Int(value)) => *value as f64,
                Some(other) => {
                    return Err(format!("Argument {name} must be a number, not {other:?}"))
                }
                None => match argument.default {
                    Some(default) => default,
                    None if argument.required => {
                        let tests = argument_tests.get(name).map(|t| t.join(", "));
                        missing.push(match tests {
                            Some(tests) => format!("  {name} ({tests})"),
                            None => format!("  {name}"),
                        });
                        continue;
                    }
                    None => continue,
                },
            };
            scope.insert(name.clone(), value);
        }
        if !missing.is_empty() {
            return Err(format!(
                "Missing required argument{} for {}:\n{}",
                if missing.len() == 1 { "" } else { "s" },
                self.name,
                missing.join("\n")
            ));
        }

        for (name, value) in &self.constants {
            if !needed.contains(name) {
                continue;
            }
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

    fn suite(library: &StandardsLibrary, standard: &str, slug: &str) -> QcTestSuite {
        let standard = library.get(standard).unwrap();
        standard
            .qartod
            .iter()
            .find(|s| s.info().slug == slug)
            .unwrap_or_else(|| panic!("no {slug} suite for {}", standard.name))
            .clone()
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
                suite.scaffold(arguments, None).unwrap_or_else(|e| {
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
        let error = gulf.scaffold(HashMap::new(), None).unwrap_err();
        assert!(
            error.starts_with("Missing required arguments for Gulf of Maine:"),
            "{error}"
        );
        assert!(error.contains("  mhhw ("), "{error}");
        assert!(error.contains("  mllw ("), "{error}");
    }

    fn names(tests: &[&str]) -> Vec<String> {
        tests.iter().map(|t| t.to_string()).collect()
    }

    fn reviewer_arguments() -> HashMap<String, ArgumentValue> {
        HashMap::from([
            ("mean".to_string(), ArgumentValue::Float(5.0)),
            ("std".to_string(), ArgumentValue::Float(3.0)),
            ("sensor_min".to_string(), ArgumentValue::Float(1.0)),
            ("sensor_max".to_string(), ArgumentValue::Float(15.0)),
        ])
    }

    #[test]
    fn scaffold_only_selected_tests() {
        let library = library();
        let manual = suite(&library, "sea_surface_temperature", "qartod_manual");
        let config = manual
            .scaffold(reviewer_arguments(), Some(&names(&["gross_range_test"])))
            .unwrap()
            .qartod;
        assert!(config.gross_range_test.is_some());
        assert!(config.rate_of_change_test.is_none());
        assert!(config.flat_line_test.is_none());
        assert!(config.attenuated_signal_test.is_none());
    }

    #[test]
    fn scaffold_without_filter_lists_every_missing_argument() {
        let library = library();
        let manual = suite(&library, "sea_surface_temperature", "qartod_manual");
        let error = manual.scaffold(reviewer_arguments(), None).unwrap_err();
        assert_eq!(
            error,
            "Missing required arguments for QARTOD Temperature and Salinity manual:\n  \
             min_var_fail (attenuated_signal_test)\n  \
             min_var_warn (attenuated_signal_test)\n  \
             sample_interval (rate_of_change_test, flat_line_test)\n  \
             sd (rate_of_change_test)"
        );
    }

    #[test]
    fn scaffold_missing_arguments_only_for_selected_tests() {
        let library = library();
        let manual = suite(&library, "sea_surface_temperature", "qartod_manual");
        let error = manual
            .scaffold(
                reviewer_arguments(),
                Some(&names(&["flat_line_test", "gross_range_test"])),
            )
            .unwrap_err();
        assert_eq!(
            error,
            "Missing required argument for QARTOD Temperature and Salinity manual:\n  \
             sample_interval (flat_line_test)"
        );
    }

    #[test]
    fn scaffold_unknown_or_absent_test_is_an_error() {
        let library = library();
        let manual = suite(&library, "sea_surface_temperature", "qartod_manual");
        let error = manual
            .scaffold(reviewer_arguments(), Some(&names(&["bogus_test"])))
            .unwrap_err();
        assert!(error.contains("bogus_test"), "{error}");
        assert!(error.contains("gross_range_test"), "{error}");

        // A real test that this suite doesn't have
        let error = manual
            .scaffold(reviewer_arguments(), Some(&names(&["spike_test"])))
            .unwrap_err();
        assert!(error.contains("spike_test"), "{error}");
        assert!(error.contains("flat_line_test"), "{error}");
    }

    #[test]
    fn scaffold_follows_constants_to_arguments() {
        let library = library();
        let manual = suite(&library, "sea_surface_temperature", "qartod_manual");
        // test_period uses the `hour` constant and the tst_tim default
        let args = HashMap::from([
            ("min_var_warn".to_string(), ArgumentValue::Float(0.02)),
            ("min_var_fail".to_string(), ArgumentValue::Float(0.01)),
        ]);
        let config = manual
            .scaffold(args, Some(&names(&["attenuated_signal_test"])))
            .unwrap()
            .qartod;
        assert_eq!(
            config.attenuated_signal_test.unwrap().test_period,
            Some(43200)
        );

        let qc = template(
            r#"
name: Example
summary: s
description: d
arguments:
  a: {type: float, description: a, required: true}
  b: {type: float, description: b, required: true}
let:
  first: {expr: "a * 2"}
  second: {expr: "first + 1"}
tests:
  qartod:
    rate_of_change_test: {threshold: {expr: "second"}}
    spike_test: {suspect_threshold: {expr: "b"}, fail_threshold: 2}
"#,
        );
        let error = qc
            .scaffold(&HashMap::new(), Some(&names(&["rate_of_change_test"])))
            .unwrap_err();
        assert!(error.contains("  a (rate_of_change_test)"), "{error}");
        assert!(!error.contains("  b "), "{error}");
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
        let error = gulf.scaffold(args, None).unwrap_err();
        assert!(
            error.starts_with("Missing required argument for Gulf of Maine:\n  mllw ("),
            "{error}"
        );
        assert!(!error.contains("mhhw"), "{error}");
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
        let config = gulf.scaffold(args, None).unwrap();

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

    #[test]
    fn qartod_manual_temperature() {
        use crate::qartod::config::{AttenuatedSignal, FlatLine, GrossRangeTest, RateOfChange};

        let library = library();
        let manual = suite(&library, "sea_water_temperature", "qartod_manual");
        let info = manual.info();
        assert_eq!(
            info.test_types,
            vec![
                QartodTestTypes::GrossRange,
                QartodTestTypes::RateOfChange,
                QartodTestTypes::FlatLine,
                QartodTestTypes::AttenuatedSignal,
            ]
        );
        assert!(!info.arguments["k"].required);

        let args = HashMap::from([
            ("sensor_min".to_string(), ArgumentValue::Float(-5.0)),
            ("sensor_max".to_string(), ArgumentValue::Float(35.0)),
            ("mean".to_string(), ArgumentValue::Float(12.0)),
            ("std".to_string(), ArgumentValue::Float(4.0)),
            ("sd".to_string(), ArgumentValue::Float(0.6)),
            ("sample_interval".to_string(), ArgumentValue::Int(600)),
            ("min_var_warn".to_string(), ArgumentValue::Float(0.02)),
            ("min_var_fail".to_string(), ArgumentValue::Float(0.01)),
        ]);
        let config = manual.scaffold(args, None).unwrap().qartod;
        assert_eq!(
            config.gross_range_test,
            Some(GrossRangeTest {
                suspect_span: (0.0, 24.0),
                fail_span: (-5.0, 35.0),
            })
        );
        assert_eq!(
            config.rate_of_change_test,
            Some(RateOfChange {
                threshold: 3.0 * 0.6 / 600.0,
            })
        );
        assert_eq!(
            config.flat_line_test,
            Some(FlatLine {
                tolerance: 0.05,
                suspect_threshold: 1800,
                fail_threshold: 3000,
            })
        );
        assert_eq!(
            config.attenuated_signal_test,
            Some(AttenuatedSignal {
                suspect_threshold: 0.02,
                fail_threshold: 0.01,
                test_period: Some(43200),
                min_obs: None,
                min_period: None,
                check_type: Some("std".to_string()),
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
        let config = qc.scaffold(&HashMap::new(), None).unwrap();
        let spike = config.qartod.spike_test.unwrap();
        assert_eq!(spike.suspect_threshold, 1.5);
        assert_eq!(spike.fail_threshold, 3.0);

        let args = HashMap::from([("k".to_string(), ArgumentValue::Int(2))]);
        let spike = qc.scaffold(&args, None).unwrap().qartod.spike_test.unwrap();
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
