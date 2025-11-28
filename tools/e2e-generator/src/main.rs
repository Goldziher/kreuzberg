mod csharp;
mod fixtures;
mod go;
mod java;
mod python;
mod ruby;
mod rust;
mod typescript;

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};
use fixtures::load_fixtures;

#[derive(Parser)]
#[command(author, version, about = "Generate language-specific E2E suites from fixtures")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate test assets for a language.
    Generate {
        /// Target language.
        #[arg(long, value_enum)]
        lang: Language,
        /// Fixture directory (defaults to workspace fixtures/).
        #[arg(long, default_value = "fixtures")]
        fixtures: Utf8PathBuf,
        /// Output directory (defaults to workspace e2e/).
        #[arg(long, default_value = "e2e")]
        output: Utf8PathBuf,
    },
    /// List fixtures (for quick inspection).
    List {
        /// Fixture directory (defaults to workspace fixtures/).
        #[arg(long, default_value = "fixtures")]
        fixtures: Utf8PathBuf,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Language {
    Rust,
    Python,
    Typescript,
    Ruby,
    Java,
    Go,
    Csharp,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { lang, fixtures, output } => {
            let fixtures = load_fixtures(fixtures.as_path())?;
            match lang {
                Language::Rust => rust::generate(&fixtures, output.as_path())?,
                Language::Python => python::generate(&fixtures, output.as_path())?,
                Language::Typescript => typescript::generate(&fixtures, output.as_path())?,
                Language::Ruby => ruby::generate(&fixtures, output.as_path())?,
                Language::Java => java::generate(&fixtures, output.as_path())?,
                Language::Go => go::generate(&fixtures, output.as_path())?,
                Language::Csharp => csharp::generate(&fixtures, output.as_path())?,
            };
        }
        Commands::List { fixtures } => {
            let fixtures = load_fixtures(fixtures.as_path())?;
            for fixture in fixtures {
                if fixture.is_document_extraction() {
                    println!(
                        "{:<24} {:<12} [doc] {}",
                        fixture.id,
                        fixture.category(),
                        fixture.document().path
                    );
                } else if fixture.is_plugin_api() {
                    println!(
                        "{:<24} {:<12} [api] {} -> {}",
                        fixture.id,
                        fixture.category(),
                        fixture.api_category.as_deref().unwrap_or("N/A"),
                        fixture.api_function.as_deref().unwrap_or("N/A")
                    );
                }
            }
        }
    }

    Ok(())
}
