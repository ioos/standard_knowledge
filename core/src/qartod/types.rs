use std::collections::HashMap;
use std::fmt::Display;

use dyn_clone::DynClone;

use super::config::ConfigStream;

#[derive(Clone, Debug, PartialEq)]
pub struct TestSuiteInfo {
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub description: String,
    pub arguments: HashMap<String, TestArgument>,
    pub test_types: Vec<QartodTestTypes>,
}

impl Display for TestSuiteInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}): {}", self.name, self.slug, self.summary)
    }
}

impl TestSuiteInfo {
    pub fn details(&self) -> String {
        let mut output = format!(
            "{} ({})\n\n{}\n\nTest types:",
            self.name, self.slug, self.summary
        );

        for test_type in &self.test_types {
            output.push_str(&format!("\n- {test_type}"));
        }

        if !self.arguments.is_empty() {
            output.push_str("\n\nArguments:");
            let mut sorted_args: Vec<_> = self.arguments.iter().collect();
            sorted_args.sort_by_key(|(name, _)| *name);
            for (name, arg) in sorted_args {
                output.push_str(&format!("\n- {}: {}", name, arg.description));
                if arg.required {
                    output.push_str(" (required)");
                }
            }
        }

        output.push_str(&format!("\n\n{}", self.description));

        output
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TestArgument {
    pub argument_type: ArgumentType,
    pub description: String,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArgumentType {
    String,
    Bool,
    Int,
    Float,
}

impl ArgumentType {
    pub fn value_type(&self, value: &str) -> ArgumentValue {
        match self {
            ArgumentType::String => ArgumentValue::String(value.to_string()),
            ArgumentType::Bool => ArgumentValue::Bool(value.parse().unwrap_or_default()),
            ArgumentType::Int => ArgumentValue::Int(value.parse().unwrap_or_default()),
            ArgumentType::Float => ArgumentValue::Float(value.parse().unwrap_or_default()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArgumentValue {
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum QartodTestTypes {
    Location,
    GrossRange,
    Climatology,
    Spike,
    RateOfChange,
    FlatLine,
    AttenuatedSignal,
    DensityInversion,
    NearestNeighbor,
}

impl QartodTestTypes {
    /// The tests a config runs, in QARTOD manual order
    pub fn tests_in_config<N, I>(config: &ConfigStream<N, I>) -> Vec<QartodTestTypes> {
        let q = &config.qartod;
        [
            (q.location_test.is_some(), QartodTestTypes::Location),
            (q.gross_range_test.is_some(), QartodTestTypes::GrossRange),
            (q.spike_test.is_some(), QartodTestTypes::Spike),
            (
                q.rate_of_change_test.is_some(),
                QartodTestTypes::RateOfChange,
            ),
            (q.flat_line_test.is_some(), QartodTestTypes::FlatLine),
        ]
        .into_iter()
        .filter_map(|(present, test_type)| present.then_some(test_type))
        .collect()
    }
}

impl Display for QartodTestTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QartodTestTypes::Location => write!(f, "Location"),
            QartodTestTypes::GrossRange => write!(f, "Gross Range"),
            QartodTestTypes::Climatology => write!(f, "Climatology"),
            QartodTestTypes::Spike => write!(f, "Spike"),
            QartodTestTypes::RateOfChange => write!(f, "Rate of Change"),
            QartodTestTypes::FlatLine => write!(f, "Flat Line"),
            QartodTestTypes::AttenuatedSignal => write!(f, "Attenuated Signal"),
            QartodTestTypes::DensityInversion => write!(f, "Density Inversion"),
            QartodTestTypes::NearestNeighbor => write!(f, "Nearest Neighbor"),
        }
    }
}

pub trait TestSuite: std::fmt::Debug + Send + Sync + DynClone {
    fn info(&self) -> TestSuiteInfo;

    /// This should return a Config that represents an `ioos_qc.Config`
    /// https://ioos.github.io/ioos_qc/usage.html#config
    fn scaffold(&self, arguments: HashMap<String, ArgumentValue>) -> Result<ConfigStream, String>;
}

dyn_clone::clone_trait_object!(TestSuite);
