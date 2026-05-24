mod config;
mod storage;
mod time;

use anyhow::{Context, Result};
use arboard::Clipboard;
use chrono::Datelike;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::{OwoColorize, Stream::Stdout};

#[derive(Parser)]
#[command(name = "flexi", version, about = "Track your flexi-time balance")]
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
    Log {
        /// Show only the last N entries
        #[arg(long, short = 'n')]
        last: Option<usize>,
        /// Show entries from today only
        #[arg(long, alias = "day", conflicts_with_all = ["week", "month", "since", "until"])]
        today: bool,
        /// Show entries from the current calendar week
        #[arg(long, conflicts_with_all = ["today", "month", "since", "until"])]
        week: bool,
        /// Show entries from the current calendar month
        #[arg(long, conflicts_with_all = ["today", "week", "since", "until"])]
        month: bool,
        /// Show entries on or after this date (YYYY-MM-DD)
        #[arg(long, conflicts_with_all = ["today", "week", "month"])]
        since: Option<String>,
        /// Show entries on or before this date (YYYY-MM-DD)
        #[arg(long, conflicts_with_all = ["today", "week", "month"])]
        until: Option<String>,
    },
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

fn print_log_entry(entry: &storage::LogEntry) {
    let ts = entry.timestamp.get(..16).unwrap_or(&entry.timestamp).replace('T', " ");
    let description = entry.description.replace(" -> ", " → ");
    let desc = if description.starts_with('+') {
        description.if_supports_color(Stdout, |t| t.green()).to_string()
    } else if description.starts_with('-') {
        description.if_supports_color(Stdout, |t| t.red()).to_string()
    } else {
        description
    };
    println!("{}  {}", ts, desc);
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
        Some(Commands::Log { last, today, week, month, since, until }) => {
            let entries = storage::read_log(&cfg.path)?;

            let now = chrono::Local::now().date_naive();
            let since_date: Option<chrono::NaiveDate>;
            let until_date: Option<chrono::NaiveDate>;

            if today {
                since_date = Some(now);
                until_date = Some(now);
            } else if week {
                let days_from_start = match cfg.week_start {
                    config::WeekStart::Monday => now.weekday().num_days_from_monday(),
                    config::WeekStart::Sunday => now.weekday().num_days_from_sunday(),
                };
                since_date = Some(now - chrono::Duration::days(days_from_start as i64));
                until_date = Some(now);
            } else if month {
                since_date = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1);
                until_date = Some(now);
            } else {
                since_date = since.as_deref()
                    .map(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .context("invalid --since date, expected YYYY-MM-DD"))
                    .transpose()?;
                until_date = until.as_deref()
                    .map(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .context("invalid --until date, expected YYYY-MM-DD"))
                    .transpose()?;
            }

            let mut filtered: Vec<_> = entries.into_iter().filter(|e| {
                let date_str = e.timestamp.get(..10).unwrap_or("");
                match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    Err(_) => true,
                    Ok(d) => {
                        since_date.is_none_or(|s| d >= s)
                            && until_date.is_none_or(|u| d <= u)
                    }
                }
            }).collect();

            if let Some(n) = last {
                let len = filtered.len();
                if n < len {
                    filtered = filtered.into_iter().skip(len - n).collect();
                }
            }

            for entry in &filtered {
                print_log_entry(entry);
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
