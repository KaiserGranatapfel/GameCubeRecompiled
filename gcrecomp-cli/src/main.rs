// CLI application
use clap::Parser;
use gcrecomp_cli::commands::{analyze_dol, build_dol, recompile_dol};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

pub mod build;
pub mod compression;

#[derive(Parser)]
#[command(name = "gcrecomp")]
#[command(about = "GameCube static recompiler")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Analyze a DOL file using Ghidra
    Analyze {
        /// Path to the DOL file
        #[arg(short, long)]
        dol_file: PathBuf,

        /// Use ReOxide backend (default: headless CLI)
        #[arg(long)]
        use_reoxide: bool,
    },
    /// Recompile a DOL file to Rust code
    Recompile {
        /// Path to the DOL file
        #[arg(short, long)]
        dol_file: PathBuf,

        /// Output directory for generated Rust code
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Use ReOxide backend (default: headless CLI)
        #[arg(long)]
        use_reoxide: bool,

        /// Path to linker script for hierarchical organization
        #[arg(long)]
        linker_script: Option<PathBuf>,

        /// Path to Function ID database
        #[arg(long)]
        fidb: Option<PathBuf>,

        /// Enable BSim fuzzy matching analysis
        #[arg(long)]
        enable_bsim: bool,

        /// Use hierarchical file structure (functions → modules → namespaces)
        #[arg(long, default_value = "true")]
        hierarchical: bool,
    },
    /// Full pipeline: analyze, recompile, and build
    Build {
        /// Path to the DOL file
        #[arg(short, long)]
        dol_file: PathBuf,

        /// Output directory for generated Rust code
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Use ReOxide backend (default: headless CLI)
        #[arg(long)]
        use_reoxide: bool,

        /// Path to linker script for hierarchical organization
        #[arg(long)]
        linker_script: Option<PathBuf>,

        /// Path to Function ID database
        #[arg(long)]
        fidb: Option<PathBuf>,

        /// Enable BSim fuzzy matching analysis
        #[arg(long)]
        enable_bsim: bool,

        /// Use hierarchical file structure (functions → modules → namespaces)
        #[arg(long, default_value = "true")]
        hierarchical: bool,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            dol_file,
            use_reoxide,
        } => {
            let pb = create_progress_bar("Analyzing DOL file...");
            analyze_dol(&dol_file, use_reoxide)?;
            pb.finish_with_message("Analysis complete");
        }
        Commands::Recompile {
            dol_file,
            output_dir,
            use_reoxide,
            linker_script,
            fidb,
            enable_bsim,
            hierarchical,
        } => {
            let pb = create_progress_bar("Recompiling DOL file...");
            recompile_dol(
                &dol_file,
                output_dir.as_deref(),
                use_reoxide,
                linker_script.as_deref(),
                fidb.as_deref(),
                enable_bsim,
                hierarchical,
            )?;
            pb.finish_with_message("Recompilation complete");
        }
        Commands::Build {
            dol_file,
            output_dir,
            use_reoxide,
            linker_script,
            fidb,
            enable_bsim,
            hierarchical,
        } => {
            let pb = create_progress_bar("Building recompiled game...");
            build_dol(
                &dol_file,
                output_dir.as_deref(),
                use_reoxide,
                linker_script.as_deref(),
                fidb.as_deref(),
                enable_bsim,
                hierarchical,
            )?;
            pb.finish_with_message("Build complete");
        }
    }

    Ok(())
}

fn create_progress_bar(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb
}
