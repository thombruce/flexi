mod config;
mod storage;
mod time;

use anyhow::{Context, Result};
use arboard::Clipboard;
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
    #[command(alias = "rm")]
    Remove {
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
    /// Copy flexi balance to clipboard
    #[command(alias = "cp")]
    Copy,
    /// Print shell completion script
    Completions {
        shell: Shell,
    },
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("wl-copy not found — install wl-clipboard")?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        return Ok(());
    }
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

fn print_change(change: i32, new: i32) {
    let sign = if change >= 0 { "+" } else { "" };
    let delta = format!("{}{}", sign, time::format_duration(change));
    let delta_colored = if change > 0 {
        delta.if_supports_color(Stdout, |t| t.green()).to_string()
    } else {
        delta.if_supports_color(Stdout, |t| t.red()).to_string()
    };
    print!("{} → ", delta_colored);
    print_balance(new);
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
            print_change(new - current, new);
        }
        Some(Commands::Remove { time }) => {
            let delta = time::parse_duration(&time.join(" "))?;
            let current = storage::read_minutes(&path)?;
            let new = current - delta;
            storage::write_minutes(&path, new)?;
            print_change(new - current, new);
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
        Some(Commands::Copy) => {
            let mins = storage::read_minutes(&path)?;
            let formatted = time::format_duration(mins);
            copy_to_clipboard(&formatted)?;
            print_balance(mins);
        }
        Some(Commands::Completions { .. }) => unreachable!(),
    }

    Ok(())
}
