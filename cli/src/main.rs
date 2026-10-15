use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use standard_knowledge::StandardsLibrary;

pub mod filter;
pub mod knowledge_loader;
pub mod qc;

#[derive(Parser)]
struct Cli {
    /// Knowledge sources to load. Use 'lib' for built-in knowledge, path for local files/directories, or URL for remote sources.
    /// Can be specified multiple times to combine sources.
    #[arg(short = 'k', long = "knowledge", value_name = "SOURCE")]
    knowledge_sources: Vec<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get standard by name or alias
    Get {
        /// Standard name or alias
        name: String,

        /// Format to display in
        #[arg(short, long, value_enum, default_value_t = GetFormat::Full)]
        format: GetFormat,
    },

    /// Filter standards
    Filter(filter::FilterArgs),

    /// QARTOD test suites
    Qc(qc::QcArgs),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum GetFormat {
    /// Shorthand display
    Short,
    /// All info for standard
    Full,
    /// Xarray attributes
    Xarray,
    // /// ERDDAP datasets.xml <addAttributes>
    // Erddap,
}

fn main() {
    let cli = Cli::parse();

    let mut library = StandardsLibrary::default();
    library.load_cf_standards();

    // Handle knowledge loading based on CLI arguments
    if cli.knowledge_sources.is_empty() {
        // Default behavior: load built-in knowledge
        library.load_knowledge();
    } else {
        // Load knowledge from specified sources
        for source in &cli.knowledge_sources {
            if source == "lib" {
                // Load built-in knowledge
                library.load_knowledge();
            } else if source.starts_with("http://")
                || source.starts_with("https://")
                || source.starts_with("file://")
            {
                // Load from URL
                if let Err(e) = knowledge_loader::load_knowledge_from_url(&mut library, source) {
                    eprintln!("Error loading knowledge from URL '{source}': {e}");
                    process::exit(1);
                }
            } else {
                // Load from file path
                if let Err(e) = knowledge_loader::load_knowledge_from_path(&mut library, source) {
                    eprintln!("Error loading knowledge from path '{source}': {e}");
                    process::exit(1);
                }
            }
        }
    }

    match &cli.command {
        Commands::Get { name, format } => {
            if let Ok(standard) = library.get(name) {
                match format {
                    GetFormat::Short => {
                        println!("{}", standard.display_short())
                    }
                    GetFormat::Full => {
                        println!("{}", standard.display_all())
                    }
                    GetFormat::Xarray => {
                        println!("{}", standard.display_xarray_attrs());
                    }
                }
            } else {
                eprintln!("Didn't find a standard matching: {name}");
                process::exit(2)
            }
        }
        Commands::Filter(filter_args) => {
            filter::execute(filter_args, &library);
        }
        Commands::Qc(qc_args) => {
            qc::execute(qc_args, &library);
        }
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
