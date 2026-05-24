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
    /// Show balance change history
    Log,
    /// Undo the last change
    Undo,
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

fn record_change(cfg: &config::ResolvedConfig, current: i32, new: i32) -> anyhow::Result<()> {
    let change = new - current;
    let sign = if change >= 0 { "+" } else { "" };
    let desc = format!("{}{} → {}", sign, time::format_duration(change), time::format_duration(new));
    storage::append_log(&cfg.path, &desc, cfg.timestamp_format)?;
    print_change(change, new);
    Ok(())
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

    let cfg = config::resolve()?;

    match cli.command {
        None => {
            let mins = storage::read_minutes(&cfg.path)?;
            print_balance(mins);
        }
        Some(Commands::Add { time }) => {
            let delta = time::parse_duration(&time.join(" "))?;
            let current = storage::read_minutes(&cfg.path)?;
            record_change(&cfg, current, current + delta)?;
        }
        Some(Commands::Remove { time }) => {
            let delta = time::parse_duration(&time.join(" "))?;
            let current = storage::read_minutes(&cfg.path)?;
            record_change(&cfg, current, current - delta)?;
        }
        Some(Commands::Set { time }) => {
            let mins = time::parse_duration(&time.join(" "))?;
            let desc = format!("= {}", time::format_duration(mins));
            storage::append_log(&cfg.path, &desc, cfg.timestamp_format)?;
            print_balance(mins);
        }
        Some(Commands::Reset) => {
            storage::append_log(&cfg.path, "= 0 min", cfg.timestamp_format)?;
            print_balance(0);
        }
        Some(Commands::Log) => {
            for entry in storage::read_log(&cfg.path)? {
                let ts = entry.timestamp.get(..16).unwrap_or(&entry.timestamp).replace('T', " ");
                let desc = if entry.description.starts_with('+') {
                    entry.description.if_supports_color(Stdout, |t| t.green()).to_string()
                } else if entry.description.starts_with('-') {
                    entry.description.if_supports_color(Stdout, |t| t.red()).to_string()
                } else {
                    entry.description.clone()
                };
                println!("{}  {}", ts, desc);
            }
        }
        Some(Commands::Undo) => {
            match storage::pop_log(&cfg.path)? {
                None => println!("nothing to undo"),
                Some(entry) => {
                    let popped_new = entry.new_minutes()?;
                    let restored = storage::read_minutes(&cfg.path)?;
                    print_change(restored - popped_new, restored);
                }
            }
        }
        Some(Commands::Copy) => {
            let mins = storage::read_minutes(&cfg.path)?;
            let formatted = time::format_duration(mins);
            copy_to_clipboard(&formatted)?;
            print_balance(mins);
        }
        Some(Commands::Completions { .. }) => unreachable!(),
    }

    Ok(())
}
