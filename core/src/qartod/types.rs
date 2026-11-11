use std::collections::BTreeMap;
use std::fmt::Display;

use super::static_qc::SuiteArgument;

#[derive(Clone, Debug, PartialEq)]
pub struct TestSuiteInfo {
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub description: String,
    pub arguments: BTreeMap<String, SuiteArgument>,
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
            for (name, arg) in &self.arguments {
                output.push_str(&format!("\n- {name} ({}", arg.argument_type));
                match arg.default {
                    Some(default) => output.push_str(&format!(", default {default}")),
                    None => output.push_str(", required"),
                }
                output.push_str(&format!("): {}", arg.description));
            }
        }

        output.push_str(&format!("\n\n{}", self.description));

        output
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

/// The tests a suite can configure, by ioos_qc name, in QARTOD manual order
pub static QARTOD_TESTS: [(&str, QartodTestTypes); 6] = [
    ("location_test", QartodTestTypes::Location),
    ("gross_range_test", QartodTestTypes::GrossRange),
    ("spike_test", QartodTestTypes::Spike),
    ("rate_of_change_test", QartodTestTypes::RateOfChange),
    ("flat_line_test", QartodTestTypes::FlatLine),
    ("attenuated_signal_test", QartodTestTypes::AttenuatedSignal),
];

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
