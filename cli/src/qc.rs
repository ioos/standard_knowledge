use clap::{Parser, Subcommand};
use standard_knowledge::StandardsLibrary;
use std::collections::HashMap;
use std::process;

use standard_knowledge::qartod::types::{ArgumentType, ArgumentValue};

#[derive(Parser)]
pub struct QcArgs {
    #[clap(subcommand)]
    pub command: QcCommands,
}

#[derive(Subcommand)]
pub enum QcCommands {
    /// List all QARTOD test suites
    List {
        /// Name of the test suite to get info about
        standard_name: String,
    },
    /// Get info about a specific QARTOD test suite
    Get {
        /// Name of the test suite to get info about
        standard_name: String,
        /// Name of test suite
        test_suite: String,
    },
    /// Generate a configuration for a specific QARTOD test suite
    Config {
        /// Name of the test suite to get info about
        standard_name: String,
        /// Name of test suite
        test_suite: String,
        /// Test configuration arguments
        args: Vec<String>,
        /// Only generate these tests, comma separated (like
        /// gross_range_test,flat_line_test), and only require the arguments
        /// they need
        #[clap(long, value_delimiter = ',')]
        tests: Vec<String>,
    },
}

pub fn execute(qc_args: &QcArgs, library: &StandardsLibrary) {
    match &qc_args.command {
        QcCommands::List { standard_name } => {
            if let Ok(standard) = library.get(standard_name.as_str()) {
                if standard.qartod.is_empty() {
                    eprintln!("No QARTOD test suites for standard: {standard_name}");
                    process::exit(2);
                } else {
                    println!(
                        "QARTOD Test Suites for {}:\n{}",
                        standard_name,
                        standard
                            .qartod
                            .iter()
                            .map(|suite| format!("- {}", suite.info()))
                            .collect::<Vec<String>>()
                            .join("\n")
                    );
                }
            } else {
                eprintln!("Didn't find a standard matching: {standard_name}");
                process::exit(2)
            }
        }
        QcCommands::Get {
            standard_name,
            test_suite,
        } => {
            if let Ok(standard) = library.get(standard_name.as_str()) {
                if let Some(suite) = standard
                    .qartod
                    .iter()
                    .find(|s| s.info().slug == *test_suite)
                {
                    println!("{}", suite.info().details());
                } else {
                    eprintln!(
                        "No QARTOD test suite named: {test_suite} for standard: {standard_name}"
                    );
                    process::exit(2);
                }
            } else {
                eprintln!("Didn't find a standard matching: {standard_name}");
                process::exit(2)
            }
        }
        QcCommands::Config {
            standard_name,
            test_suite,
            args,
            tests,
        } => {
            if let Ok(standard) = library.get(standard_name.as_str()) {
                if let Some(suite) = standard
                    .qartod
                    .iter()
                    .find(|s| s.info().slug == *test_suite)
                {
                    let info = suite.info();

                    let mut arguments: HashMap<String, ArgumentValue> = HashMap::new();

                    for arg in args {
                        let parts: Vec<&str> = arg.split('=').collect();
                        if parts.len() == 2 {
                            let key = parts[0];
                            let value = parts[1];

                            let arg_type = info
                                .arguments
                                .get(key)
                                .map(|arg| arg.argument_type.clone())
                                .unwrap_or(ArgumentType::String);
                            let arg_value = arg_type.value_type(value);

                            arguments.insert(key.to_string(), arg_value);
                        } else {
                            eprintln!("Invalid argument format: {arg}");
                            process::exit(2);
                        }
                    }

                    let config =
                        suite.scaffold(arguments, (!tests.is_empty()).then_some(tests.as_slice()));

                    if let Err(error) = config {
                        eprintln!("Error generating configuration: {error}");
                        process::exit(2);
                    } else {
                        let yaml = serde_yaml_ng::to_string(&config.unwrap())
                            .expect("Failed to serialize configuration to YAML");
                        println!("Generated configuration for {}:\n{yaml}", info.name);
                    }
                } else {
                    eprintln!(
                        "No QARTOD test suite named: {test_suite} for standard: {standard_name}"
                    );
                    process::exit(2);
                }
            } else {
                eprintln!("Didn't find a standard matching: {standard_name}");
                process::exit(2)
            }
        }
    }
}
