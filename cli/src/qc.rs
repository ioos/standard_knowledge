use clap::{Parser, Subcommand};
use standard_knowledge::StandardsLibrary;
use std::collections::HashMap;
use std::process;

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
        } => {
            if let Ok(standard) = library.get(standard_name.as_str()) {
                if let Some(suite) = standard
                    .qartod
                    .iter()
                    .find(|s| s.info().slug == *test_suite)
                {
                    let info = suite.info();

                    let mut arguments = HashMap::new();

                    for arg in args {
                        let Some((key, value)) = arg.split_once('=') else {
                            eprintln!("Invalid argument format: {arg}, expected name=value");
                            process::exit(2);
                        };
                        let Ok(value) = value.parse::<f64>() else {
                            eprintln!("Invalid value for {key}: {value:?} is not a number");
                            process::exit(2);
                        };
                        arguments.insert(key.to_string(), value);
                    }

                    let config = suite.scaffold(&arguments);

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
