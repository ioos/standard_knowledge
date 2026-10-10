use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use indexmap::IndexMap;
use serde_yaml_ng::{Mapping, Value};

use super::config::ConfigStream;
use super::types::{QartodTestTypes, TestSuiteInfo, QARTOD_TESTS};
use crate::fx;

#[derive(Debug, Clone, PartialEq)]
pub struct QcTestSuite {
    pub slug: String,
    pub qc: StaticQc,
}

impl QcTestSuite {
    pub fn info(&self) -> TestSuiteInfo {
        TestSuiteInfo {
            slug: self.slug.clone(),
            name: self.qc.name.clone(),
            summary: self.qc.summary.clone(),
            description: self.qc.description.clone(),
            arguments: self.qc.arguments.clone(),
            test_types: self.qc.test_types(),
        }
    }

    /// The ioos_qc names of the suite's tests, in QARTOD manual order
    pub fn test_names(&self) -> Vec<&'static str> {
        self.qc.test_names()
    }

    /// This should return a Config that represents an `ioos_qc.Config`
    /// https://ioos.github.io/ioos_qc/usage.html#config
    pub fn scaffold(&self, arguments: &HashMap<String, f64>) -> Result<ConfigStream, String> {
        self.qc.scaffold(arguments)
    }
}

/// QARTOD test suite configuration defined in a knowledge YAML file.
///
/// `tests` has the shape of an ioos_qc stream config (`qartod:` and the
/// tests under it). Any number in it can instead be an expression,
/// written `{expr: "mhhw + 6 * ft"}`, that uses the suite's `arguments`
/// and `let` constants. See `core/src/fx/GRAMMAR.md` for the language.
///
/// A suite is checked when it is deserialized, so every `StaticQc` is valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UncheckedStaticQc")]
pub struct StaticQc {
    pub name: String,
    pub summary: String,
    pub description: String,

    /// Values the user supplies when scaffolding, by name
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub arguments: BTreeMap<String, SuiteArgument>,

    /// Named constants, evaluated in order. Each is a number or an
    /// expression, which can use the arguments and earlier constants.
    #[serde(default, rename = "let", skip_serializing_if = "IndexMap::is_empty")]
    pub constants: IndexMap<String, Num>,

    /// The ioos_qc stream config, with optional expressions
    pub tests: SuiteTests,
}

/// A `StaticQc` as written, before it is checked
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckedStaticQc {
    name: String,
    summary: String,
    description: String,
    #[serde(default)]
    arguments: BTreeMap<String, SuiteArgument>,
    #[serde(default, rename = "let")]
    constants: IndexMap<String, Num>,
    tests: SuiteTests,
}

impl TryFrom<UncheckedStaticQc> for StaticQc {
    type Error = String;

    fn try_from(qc: UncheckedStaticQc) -> Result<Self, String> {
        let qc = StaticQc {
            name: qc.name,
            summary: qc.summary,
            description: qc.description,
            arguments: qc.arguments,
            constants: qc.constants,
            tests: qc.tests,
        };
        qc.validate()?;
        Ok(qc)
    }
}

/// An argument to a YAML test suite. It is required unless it has a default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteArgument {
    #[serde(rename = "type")]
    pub argument_type: SuiteArgumentType,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<f64>,
}

impl SuiteArgument {
    pub fn is_required(&self) -> bool {
        self.default.is_none()
    }
}

/// Arguments are used in expressions, so they are numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuiteArgumentType {
    Float,
    Int,
}

impl std::fmt::Display for SuiteArgumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuiteArgumentType::Float => write!(f, "float"),
            SuiteArgumentType::Int => write!(f, "int"),
        }
    }
}

/// The tests of a suite: each ioos_qc test's config, by test and field name
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteTests {
    pub qartod: IndexMap<String, IndexMap<String, Template>>,
}

/// The value of a test config field
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Template {
    Num(Num),
    List(Vec<Template>),
    Text(String),
}

/// A number in a YAML test suite: a literal, or an fx expression written
/// `{expr: "mhhw + 6 * ft"}`, parsed when the suite loads
#[derive(Clone, PartialEq, Debug)]
pub enum Num {
    Value(f64),
    Expr {
        source: String,
        expression: fx::Expression,
    },
}

impl Serialize for Num {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            Num::Value(value) => serializer.serialize_f64(*value),
            Num::Expr { source, .. } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("expr", source)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Num {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        match Template::deserialize(deserializer)? {
            Template::Num(num) => Ok(num),
            _ => Err(D::Error::custom("expected a number or {expr: \"...\"}")),
        }
    }
}

impl<'de> Deserialize<'de> for Template {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TemplateVisitor;

        impl<'de> serde::de::Visitor<'de> for TemplateVisitor {
            type Value = Template;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a number, {expr: \"...\"}, a list or text")
            }

            fn visit_f64<E>(self, value: f64) -> Result<Template, E> {
                Ok(Template::Num(Num::Value(value)))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Template, E> {
                Ok(Template::Num(Num::Value(value as f64)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Template, E> {
                Ok(Template::Num(Num::Value(value as f64)))
            }

            fn visit_str<E>(self, value: &str) -> Result<Template, E> {
                Ok(Template::Text(value.to_string()))
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Template, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Template::List(items))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Template, A::Error> {
                use serde::de::Error;
                let source = match map.next_key::<String>()? {
                    Some(key) if key == "expr" => map.next_value::<String>()?,
                    _ => return Err(A::Error::custom("expected {expr: \"...\"}")),
                };
                if map.next_key::<String>()?.is_some() {
                    return Err(A::Error::custom("expected only an expr key"));
                }
                let expression = fx::parse(&source)
                    .map_err(|e| A::Error::custom(format!("invalid expression {source:?}: {e}")))?;
                Ok(Template::Num(Num::Expr { source, expression }))
            }
        }

        deserializer.deserialize_any(TemplateVisitor)
    }
}

impl Template {
    /// The numbers in the value
    fn nums(&self) -> Vec<&Num> {
        match self {
            Template::Num(num) => vec![num],
            Template::List(items) => items.iter().flat_map(Template::nums).collect(),
            Template::Text(_) => Vec::new(),
        }
    }

    /// The value with each number replaced by `number(num)`
    fn fill(&self, number: &impl Fn(&Num) -> Result<f64, String>) -> Result<Value, String> {
        match self {
            Template::Num(num) => number(num).map(Value::from),
            Template::List(items) => items
                .iter()
                .map(|item| item.fill(number))
                .collect::<Result<_, _>>()
                .map(Value::Sequence),
            Template::Text(text) => Ok(Value::from(text.as_str())),
        }
    }
}

impl StaticQc {
    /// Build the ioos_qc config, evaluating any expressions with `arguments`
    pub fn scaffold(&self, arguments: &HashMap<String, f64>) -> Result<ConfigStream, String> {
        let unknown: BTreeSet<&str> = arguments
            .keys()
            .filter(|name| !self.arguments.contains_key(*name))
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            let unknown = unknown.into_iter().collect::<Vec<_>>().join(", ");
            if self.arguments.is_empty() {
                return Err(format!(
                    "Unknown argument(s) for {}: {unknown}. It takes no arguments",
                    self.name
                ));
            }
            let known = self
                .arguments
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "Unknown argument(s) for {}: {unknown}. Its arguments are: {known}",
                self.name
            ));
        }

        // The variables the tests read, and the variables of the constants
        // they read. Constants only read earlier constants, so walking them
        // backwards finds everything.
        let mut needed: BTreeSet<String> = self
            .fields()
            .flat_map(|(_, template)| template.nums())
            .flat_map(variables)
            .collect();
        for (name, value) in self.constants.iter().rev() {
            if needed.contains(name) {
                needed.extend(variables(value));
            }
        }

        let mut scope = HashMap::new();
        let mut missing = Vec::new();
        for (name, argument) in &self.arguments {
            match arguments.get(name).copied().or(argument.default) {
                Some(value) => {
                    if argument.argument_type == SuiteArgumentType::Int && value.fract() != 0.0 {
                        return Err(format!("{name} must be a whole number, found {value}"));
                    }
                    scope.insert(name.clone(), value);
                }
                None if needed.contains(name) => missing.push(name.as_str()),
                None => {}
            }
        }
        if !missing.is_empty() {
            return Err(format!(
                "Missing required argument(s) for {}:\n  {}",
                self.name,
                missing.join("\n  ")
            ));
        }

        for (name, value) in &self.constants {
            if needed.contains(name) {
                let value = evaluate(value, &scope).map_err(|e| format!("let.{name}: {e}"))?;
                scope.insert(name.clone(), value);
            }
        }

        self.build(|num| evaluate(num, &scope))
    }

    /// The suite's tests, in QARTOD manual order
    fn tests(&self) -> impl Iterator<Item = &'static (&'static str, QartodTestTypes)> + '_ {
        QARTOD_TESTS
            .iter()
            .filter(|(name, _)| self.tests.qartod.contains_key(*name))
    }

    /// The ioos_qc names of the suite's tests, in QARTOD manual order
    pub fn test_names(&self) -> Vec<&'static str> {
        self.tests().map(|(name, _)| *name).collect()
    }

    /// The suite's test types, in QARTOD manual order
    pub fn test_types(&self) -> Vec<QartodTestTypes> {
        self.tests().map(|(_, test_type)| *test_type).collect()
    }

    /// Each test config field, by `test.field`
    fn fields(&self) -> impl Iterator<Item = (String, &Template)> {
        self.tests.qartod.iter().flat_map(|(test, fields)| {
            fields
                .iter()
                .map(move |(field, template)| (format!("{test}.{field}"), template))
        })
    }

    /// Build the ioos_qc config, with `number` giving the value of each number
    fn build(&self, number: impl Fn(&Num) -> Result<f64, String>) -> Result<ConfigStream, String> {
        let mut qartod = Mapping::new();
        for (test, fields) in &self.tests.qartod {
            let mut config = Mapping::new();
            for (field, template) in fields {
                let value = template
                    .fill(&number)
                    .map_err(|e| format!("{test}.{field}: {e}"))?;
                config.insert(Value::from(field.as_str()), value);
            }
            qartod.insert(Value::from(test.as_str()), Value::Mapping(config));
        }
        let stream = Mapping::from_iter([(Value::from("qartod"), Value::Mapping(qartod))]);

        serde_path_to_error::deserialize(Value::Mapping(stream)).map_err(|e| {
            let path = e.path().to_string();
            let path = path.strip_prefix("qartod.").unwrap_or(&path);
            format!("{path}: {}", e.inner())
        })
    }

    /// Check the suite without evaluating it: names, the variables each
    /// expression uses, and the shape of the tests
    fn validate(&self) -> Result<(), String> {
        let mut known = BTreeSet::new();
        for (name, argument) in &self.arguments {
            check_name(name)?;
            if let Some(default) = argument.default {
                if argument.argument_type == SuiteArgumentType::Int && default.fract() != 0.0 {
                    return Err(format!(
                        "arguments.{name}: the default must be a whole number, found {default}"
                    ));
                }
            }
            known.insert(name.as_str());
        }

        for (name, value) in &self.constants {
            check_name(name)?;
            if self.arguments.contains_key(name) {
                return Err(format!("{name:?} is both an argument and a `let` constant"));
            }
            check_variables(&format!("let.{name}"), value, &known)?;
            known.insert(name.as_str());
        }

        if self.tests.qartod.is_empty() {
            return Err(format!("{} needs at least one test", self.name));
        }
        for (path, template) in self.fields() {
            for num in template.nums() {
                check_variables(&path, num, &known)?;
            }
        }

        // Expressions stand in as 1 to check the shape of each test
        self.build(|num| match num {
            Num::Value(value) => Ok(*value),
            Num::Expr { .. } => Ok(1.0),
        })
        .map(|_| ())
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

/// Check that an expression only uses `known` variables
fn check_variables(path: &str, num: &Num, known: &BTreeSet<&str>) -> Result<(), String> {
    if let Num::Expr { source, expression } = num {
        if let Some(name) = expression
            .variables()
            .into_iter()
            .find(|name| !known.contains(name.as_str()))
        {
            return Err(format!("{path}: {source:?} uses unknown variable {name}"));
        }
    }
    Ok(())
}

/// The variables a number reads
fn variables(num: &Num) -> BTreeSet<String> {
    match num {
        Num::Value(_) => BTreeSet::new(),
        Num::Expr { expression, .. } => expression.variables(),
    }
}

/// A number, or the result of an expression
fn evaluate(num: &Num, scope: &HashMap<String, f64>) -> Result<f64, String> {
    match num {
        Num::Value(value) => Ok(*value),
        Num::Expr { source, expression } => {
            let result = expression
                .evaluate(scope)
                .map_err(|e| format!("{source:?}: {e}"))?;
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
    use crate::qartod::config::{FlatLine, GrossRangeTest, LocationTest, RateOfChange, Spike};
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

    fn gulf_of_maine() -> QcTestSuite {
        suite(
            &library(),
            "sea_surface_height_above_geopotential_datum",
            "gulf_of_maine",
        )
    }

    fn args(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), *value))
            .collect()
    }

    #[test]
    fn every_bundled_suite_scaffolds() {
        for standard in library().standards.values() {
            for suite in &standard.qartod {
                let info = suite.info();
                let arguments = info
                    .arguments
                    .keys()
                    .map(|name| (name.clone(), 1.0))
                    .collect();
                suite.scaffold(&arguments).unwrap_or_else(|e| {
                    panic!("{} {} failed to scaffold: {e}", standard.name, info.slug)
                });
            }
        }
    }

    #[test]
    fn bundled_suites_round_trip_through_yaml() {
        for standard in library().standards.values() {
            for suite in &standard.qartod {
                let yaml = serde_yaml_ng::to_string(&suite.qc).unwrap();
                let reloaded: StaticQc = serde_yaml_ng::from_str(&yaml).unwrap();
                assert_eq!(reloaded, suite.qc, "{yaml}");
            }
        }
    }

    // The Gulf of Maine checks moved from the Rust implementation that this
    // YAML suite replaced. The expected values are unchanged.

    #[test]
    fn gulf_of_maine_info() {
        let gulf = gulf_of_maine();
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
        assert_eq!(
            gulf.test_names(),
            [
                "gross_range_test",
                "spike_test",
                "rate_of_change_test",
                "flat_line_test"
            ]
        );
        assert!(info.arguments["mllw"].is_required());
        assert!(info.arguments["mhhw"].is_required());
        assert_eq!(
            info.arguments["mllw"].argument_type,
            SuiteArgumentType::Float
        );
    }

    #[test]
    fn gulf_of_maine_scaffold_no_args() {
        assert_eq!(
            gulf_of_maine().scaffold(&HashMap::new()).unwrap_err(),
            "Missing required argument(s) for Gulf of Maine:\n  mhhw\n  mllw"
        );
    }

    #[test]
    fn gulf_of_maine_missing_arg() {
        assert_eq!(
            gulf_of_maine()
                .scaffold(&args(&[("mhhw", 1.0)]))
                .unwrap_err(),
            "Missing required argument(s) for Gulf of Maine:\n  mllw"
        );
    }

    #[test]
    fn unknown_argument_is_an_error() {
        let error = gulf_of_maine()
            .scaffold(&args(&[("mllw", 0.0), ("mhhw", 1.0), ("mlw", 0.0)]))
            .unwrap_err();
        assert_eq!(
            error,
            "Unknown argument(s) for Gulf of Maine: mlw. Its arguments are: mhhw, mllw"
        );
    }

    #[test]
    fn gulf_of_maine_scaffold_success() {
        let config = gulf_of_maine()
            .scaffold(&args(&[("mllw", 0.0), ("mhhw", 1.0)]))
            .unwrap();

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

    /// A suite named Example with `body` after its descriptions
    fn yaml(body: &str) -> String {
        format!("name: Example\nsummary: s\ndescription: d\n{body}")
    }

    fn template(body: &str) -> StaticQc {
        serde_yaml_ng::from_str(&yaml(body)).unwrap_or_else(|e| panic!("{e}"))
    }

    fn load_error(body: &str) -> String {
        serde_yaml_ng::from_str::<StaticQc>(&yaml(body))
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn defaults_and_constants() {
        let qc = template(
            r#"
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
        assert!(!qc.arguments["k"].is_required());
        let config = qc.scaffold(&HashMap::new()).unwrap();
        let spike = config.qartod.spike_test.unwrap();
        assert_eq!(spike.suspect_threshold, 1.5);
        assert_eq!(spike.fail_threshold, 3.0);

        let spike = qc
            .scaffold(&args(&[("k", 2.0)]))
            .unwrap()
            .qartod
            .spike_test
            .unwrap();
        assert_eq!(spike.suspect_threshold, 1.0);
    }

    const DATUM_SUITE: &str = r#"
arguments:
  k: {type: float, description: multiplier, default: 3}
  mhhw: {type: float, description: high water}
  mllw: {type: float, description: low water}
  window: {type: int, description: seconds}
let:
  scale: {expr: "k / (mhhw - mllw)"}
tests:
  qartod:
    spike_test:
      suspect_threshold: {expr: "scale"}
      fail_threshold: {expr: "2 * scale"}
    flat_line_test:
      tolerance: 0.1
      suspect_threshold: {expr: "window / 2"}
      fail_threshold: {expr: "window"}
"#;

    #[test]
    fn expressions_that_would_divide_by_zero_with_placeholders_load() {
        let qc = template(DATUM_SUITE);
        let config = qc
            .scaffold(&args(&[("mhhw", 2.0), ("mllw", 1.0), ("window", 7200.0)]))
            .unwrap();
        assert_eq!(config.qartod.spike_test.unwrap().suspect_threshold, 3.0);
        let flat_line = config.qartod.flat_line_test.unwrap();
        assert_eq!(flat_line.suspect_threshold, 3600);
        assert_eq!(flat_line.fail_threshold, 7200);
    }

    #[test]
    fn fractional_integer_field_names_the_field() {
        let error = template(DATUM_SUITE)
            .scaffold(&args(&[("mhhw", 2.0), ("mllw", 1.0), ("window", 3603.0)]))
            .unwrap_err();
        assert_eq!(
            error,
            "flat_line_test.suspect_threshold: expected a whole number, found 1801.5"
        );
    }

    #[test]
    fn fractional_integer_argument_is_an_error() {
        let error = template(DATUM_SUITE)
            .scaffold(&args(&[("mhhw", 2.0), ("mllw", 1.0), ("window", 600.5)]))
            .unwrap_err();
        assert_eq!(error, "window must be a whole number, found 600.5");
    }

    #[test]
    fn missing_arguments_are_all_reported() {
        let error = template(DATUM_SUITE)
            .scaffold(&args(&[("mhhw", 2.0)]))
            .unwrap_err();
        assert_eq!(
            error,
            "Missing required argument(s) for Example:\n  mllw\n  window"
        );
    }

    #[test]
    fn literal_fraction_in_an_integer_field_is_rejected_at_load() {
        let error = load_error(
            r#"
tests:
  qartod:
    flat_line_test:
      tolerance: 0.1
      suspect_threshold: {expr: "1.5 * 3600"}
      fail_threshold: 7200.5
"#,
        );
        assert!(
            error.contains("flat_line_test.fail_threshold: expected a whole number, found 7200.5"),
            "{error}"
        );
    }

    #[test]
    fn unknown_variable_is_rejected_at_load() {
        let error = load_error(
            r#"
arguments:
  mhhw: {type: float, description: high water}
tests:
  qartod:
    rate_of_change_test:
      threshold: {expr: "mhw + 1"}
"#,
        );
        assert!(
            error.contains("rate_of_change_test.threshold: \"mhw + 1\" uses unknown variable mhw"),
            "{error}"
        );
    }

    #[test]
    fn constant_can_only_use_earlier_constants() {
        let error = load_error(
            r#"
let:
  a: {expr: "b * 2"}
  b: 1
tests:
  qartod:
    rate_of_change_test:
      threshold: {expr: "a"}
"#,
        );
        assert!(
            error.contains("let.a: \"b * 2\" uses unknown variable b"),
            "{error}"
        );
    }

    #[test]
    fn unknown_test_is_rejected_at_load() {
        let error = load_error(
            r#"
tests:
  qartod:
    spike_tset:
      suspect_threshold: 1
      fail_threshold: 2
"#,
        );
        assert!(error.contains("spike_tset"), "{error}");
    }

    #[test]
    fn misspelt_field_is_rejected_at_load() {
        let error = load_error(
            r#"
tests:
  qartod:
    spike_test:
      suspect_treshold: 1
      fail_threshold: 2
"#,
        );
        assert!(error.contains("suspect_treshold"), "{error}");
    }

    #[test]
    fn missing_field_is_rejected_at_load() {
        let error = load_error(
            r#"
tests:
  qartod:
    spike_test:
      suspect_threshold: 1
"#,
        );
        assert!(error.contains("fail_threshold"), "{error}");
    }

    #[test]
    fn bad_expression_syntax_is_rejected_at_load() {
        let error = load_error(
            r#"
tests:
  qartod:
    rate_of_change_test:
      threshold: {expr: "1 +"}
"#,
        );
        assert!(error.contains("invalid expression \"1 +\""), "{error}");
    }

    #[test]
    fn suite_without_tests_is_rejected_at_load() {
        let error = load_error("tests: {qartod: {}}\n");
        assert!(error.contains("Example needs at least one test"), "{error}");
    }

    #[test]
    fn invalid_names_are_rejected_at_load() {
        let error = load_error(
            r#"
let:
  pi: 3
tests:
  qartod:
    rate_of_change_test:
      threshold: 1
"#,
        );
        assert!(error.contains("\"pi\""), "{error}");

        let error = load_error(
            r#"
arguments:
  k: {type: float, description: multiplier}
let:
  k: 3
tests:
  qartod:
    rate_of_change_test:
      threshold: {expr: "k"}
"#,
        );
        assert!(
            error.contains("\"k\" is both an argument and a `let` constant"),
            "{error}"
        );
    }

    #[test]
    fn required_is_no_longer_an_argument_field() {
        let error = load_error(
            r#"
arguments:
  k: {type: float, description: multiplier, required: true}
tests:
  qartod:
    rate_of_change_test:
      threshold: {expr: "k"}
"#,
        );
        assert!(error.contains("required"), "{error}");
    }

    #[test]
    fn expression_errors_name_the_field_and_expression() {
        let qc = template(
            r#"
tests:
  qartod:
    rate_of_change_test:
      threshold: {expr: "1 / 0"}
"#,
        );
        let error = qc.scaffold(&HashMap::new()).unwrap_err();
        assert!(
            error.starts_with("rate_of_change_test.threshold: \"1 / 0\""),
            "{error}"
        );
    }

    #[test]
    fn location_test_scaffolds() {
        let qc = template(
            r#"
arguments:
  lon: {type: float, description: longitude}
tests:
  qartod:
    location_test:
      bbox: [{expr: "lon - 1"}, 40, {expr: "lon + 1"}, 45]
"#,
        );
        let config = qc.scaffold(&args(&[("lon", -70.0)])).unwrap();
        assert_eq!(
            config.qartod.location_test,
            Some(LocationTest {
                bbox: (-71.0, 40.0, -69.0, 45.0)
            })
        );
    }

    #[test]
    fn expressions_serialize_as_written() {
        let qc = template(
            r#"
tests:
  qartod:
    rate_of_change_test:
      threshold: {expr: "2 * PI"}
"#,
        );
        let yaml = serde_yaml_ng::to_string(&qc).unwrap();
        assert!(yaml.contains("threshold:\n        expr: 2 * PI"), "{yaml}");
    }

    #[test]
    fn test_names_and_types_follow_the_manual() {
        let qc = template(
            r#"
tests:
  qartod:
    flat_line_test:
      tolerance: 0.1
      suspect_threshold: 2
      fail_threshold: 3
    location_test:
      bbox: [0, 0, 1, 1]
"#,
        );
        assert_eq!(qc.test_names(), ["location_test", "flat_line_test"]);
        assert_eq!(
            qc.test_types(),
            [QartodTestTypes::Location, QartodTestTypes::FlatLine]
        );
    }

    #[test]
    fn qartod_manual_temperature() {
        use crate::qartod::config::AttenuatedSignal;

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
        assert!(info.arguments["k"].default.is_some());
        assert!(info.arguments["sd"].default.is_none());

        let config = manual
            .scaffold(&args(&[
                ("sensor_min", -5.0),
                ("sensor_max", 35.0),
                ("mean", 12.0),
                ("std", 4.0),
                ("sd", 0.6),
                ("sample_interval", 600.0),
                ("min_var_warn", 0.02),
                ("min_var_fail", 0.01),
            ]))
            .unwrap()
            .qartod;
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

    #[test]
    fn attenuated_signal_integers_must_be_whole() {
        let yaml = |period: &str| {
            format!(
                "qartod:\n  attenuated_signal_test:\n    suspect_threshold: 1\n    fail_threshold: 2\n    test_period: {period}\n"
            )
        };
        let whole: ConfigStream = serde_yaml_ng::from_str(&yaml("3600.0")).unwrap();
        assert_eq!(
            whole.qartod.attenuated_signal_test.unwrap().test_period,
            Some(3600)
        );
        assert!(serde_yaml_ng::from_str::<ConfigStream>(&yaml("1.5")).is_err());
    }

    #[test]
    fn every_config_test_is_in_the_table() {
        use crate::qartod::config::ConfigStreamQartod;

        let every_test = ConfigStreamQartod {
            gross_range_test: Some(Default::default()),
            location_test: Some(Default::default()),
            rate_of_change_test: Some(Default::default()),
            spike_test: Some(Default::default()),
            flat_line_test: Some(Default::default()),
            attenuated_signal_test: Some(Default::default()),
        };
        let value = serde_yaml_ng::to_value(every_test).unwrap();
        let config_names: BTreeSet<&str> = value
            .as_mapping()
            .unwrap()
            .keys()
            .map(|key| key.as_str().unwrap())
            .collect();
        let table_names: BTreeSet<&str> = QARTOD_TESTS.iter().map(|(name, _)| *name).collect();
        assert_eq!(config_names, table_names);
    }
}
