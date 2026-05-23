mod config;
mod storage;
mod time;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "flexi", about = "Track your flexi-time balance")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add time to your flexi balance
    Add {
        #[arg(trailing_var_arg = true, required = true)]
        time: Vec<String>,
    },
    /// Remove time from your flexi balance
    Rm {
        #[arg(trailing_var_arg = true, required = true)]
        time: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let path = config::resolve_flexi_path()?;

    match cli.command {
        None => {
            let mins = storage::read_minutes(&path)?;
            println!("{}", time::format_duration(mins));
        }
        Some(Commands::Add { time }) => {
            let input = time.join(" ");
            let delta = time::parse_duration(&input)?;
            let current = storage::read_minutes(&path)?;
            storage::write_minutes(&path, current + delta)?;
        }
        Some(Commands::Rm { time }) => {
            let input = time.join(" ");
            let delta = time::parse_duration(&input)?;
            let current = storage::read_minutes(&path)?;
            storage::write_minutes(&path, current - delta)?;
        }
    }

    Ok(())
}
