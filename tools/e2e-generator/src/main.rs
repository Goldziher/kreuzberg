mod fixtures;
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
            };
        }
        Commands::List { fixtures } => {
            let fixtures = load_fixtures(fixtures.as_path())?;
            for fixture in fixtures {
                println!(
                    "{:<24} {:<12} {}",
                    fixture.id,
                    fixture.category(),
                    fixture.document.path
                );
            }
        }
    }

    Ok(())
}
