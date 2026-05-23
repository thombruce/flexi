mod config;
mod storage;
mod time;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::{OwoColorize, Stream::Stdout};

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
    /// Set your flexi balance to an exact value
    Set {
        #[arg(trailing_var_arg = true, required = true)]
        time: Vec<String>,
    },
    /// Reset your flexi balance to zero
    Reset,
    /// Print shell completion script
    Completions {
        shell: Shell,
    },
}

fn print_balance(mins: i32) {
    let formatted = time::format_duration(mins);
    if mins > 0 {
        println!("{}", formatted.if_supports_color(Stdout, |t| t.green()));
    } else if mins < 0 {
        println!("{}", formatted.if_supports_color(Stdout, |t| t.red()));
    } else {
        println!("{}", formatted);
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Completions { shell }) = cli.command {
        generate(shell, &mut Cli::command(), "flexi", &mut std::io::stdout());
        return Ok(());
    }

    let path = config::resolve_flexi_path()?;

    match cli.command {
        None => {
            let mins = storage::read_minutes(&path)?;
            print_balance(mins);
        }
        Some(Commands::Add { time }) => {
            let delta = time::parse_duration(&time.join(" "))?;
            let current = storage::read_minutes(&path)?;
            let new = current + delta;
            storage::write_minutes(&path, new)?;
            print_balance(new);
        }
        Some(Commands::Rm { time }) => {
            let delta = time::parse_duration(&time.join(" "))?;
            let current = storage::read_minutes(&path)?;
            let new = current - delta;
            storage::write_minutes(&path, new)?;
            print_balance(new);
        }
        Some(Commands::Set { time }) => {
            let mins = time::parse_duration(&time.join(" "))?;
            storage::write_minutes(&path, mins)?;
            print_balance(mins);
        }
        Some(Commands::Reset) => {
            storage::write_minutes(&path, 0)?;
            print_balance(0);
        }
        Some(Commands::Completions { .. }) => unreachable!(),
    }

    Ok(())
}
