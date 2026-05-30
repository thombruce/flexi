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
        #[arg(required = true)]
        time: Vec<String>,
        /// Attach a note to this log entry
        #[arg(long, short = 'm')]
        note: Option<String>,
    },
    /// Remove time from your flexi balance
    #[command(alias = "rm")]
    Remove {
        #[arg(required = true)]
        time: Vec<String>,
        /// Attach a note to this log entry
        #[arg(long, short = 'm')]
        note: Option<String>,
    },
    /// Set your flexi balance to an exact value
    Set {
        #[arg(required = true)]
        time: Vec<String>,
        /// Attach a note to this log entry
        #[arg(long, short = 'm')]
        note: Option<String>,
    },
    /// Reset your flexi balance to zero
    Reset {
        /// Attach a note to this log entry
        #[arg(long, short = 'm')]
        note: Option<String>,
    },
    /// Show balance change history
    Log {
        /// Show only the last N entries
        #[arg(long, short = 'n')]
        last: Option<usize>,
        /// Show entries from today only
        #[arg(long, alias = "day", conflicts_with_all = ["yesterday", "week", "month", "since", "until"])]
        today: bool,
        /// Show entries from yesterday only
        #[arg(long, conflicts_with_all = ["today", "week", "month", "since", "until"])]
        yesterday: bool,
        /// Show entries from the current calendar week
        #[arg(long, conflicts_with_all = ["today", "yesterday", "month", "since", "until"])]
        week: bool,
        /// Show entries from the current calendar month
        #[arg(long, conflicts_with_all = ["today", "yesterday", "week", "since", "until"])]
        month: bool,
        /// Show entries on or after this date (YYYY-MM-DD)
        #[arg(long, conflicts_with_all = ["today", "yesterday", "week", "month"])]
        since: Option<String>,
        /// Show entries on or before this date (YYYY-MM-DD)
        #[arg(long, conflicts_with_all = ["today", "yesterday", "week", "month"])]
        until: Option<String>,
        /// Show totals instead of individual entries
        #[arg(long)]
        summary: bool,
    },
    /// Open the log file in $EDITOR
    Edit,
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

fn record_change(cfg: &config::ResolvedConfig, current: i32, new: i32, note: Option<&str>) -> anyhow::Result<()> {
    let change = new - current;
    let sign = if change >= 0 { "+" } else { "" };
    let mut desc = format!("{}{} > {}", sign, time::format_duration(change), time::format_duration(new));
    if let Some(n) = note {
        desc.push_str(&format!(" # {}", n));
    }
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
    let (body, note) = match entry.description.split_once(" # ") {
        Some((b, n)) => (b.to_string(), Some(n.to_string())),
        None => (entry.description.clone(), None),
    };
    let body = body
        .replace(" > ", " → ")
        .replace(" -> ", " → ");
    let colored = if body.starts_with('+') {
        body.if_supports_color(Stdout, |t| t.green()).to_string()
    } else if body.starts_with('-') {
        body.if_supports_color(Stdout, |t| t.red()).to_string()
    } else {
        body
    };
    if let Some(n) = note {
        let note_str = format!("  # {}", n);
        println!("{}  {}{}", ts, colored, note_str.if_supports_color(Stdout, |t| t.dimmed()));
    } else {
        println!("{}  {}", ts, colored);
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
        Some(Commands::Add { time, note }) => {
            let delta = time::parse_duration(&time.join(" "))?;
            let current = storage::read_minutes(&cfg.path)?;
            record_change(&cfg, current, current + delta, note.as_deref())?;
        }
        Some(Commands::Remove { time, note }) => {
            let delta = time::parse_duration(&time.join(" "))?;
            let current = storage::read_minutes(&cfg.path)?;
            record_change(&cfg, current, current - delta, note.as_deref())?;
        }
        Some(Commands::Set { time, note }) => {
            let mins = time::parse_duration(&time.join(" "))?;
            let mut desc = format!("= {}", time::format_duration(mins));
            if let Some(n) = note {
                desc.push_str(&format!(" # {}", n));
            }
            storage::append_log(&cfg.path, &desc, cfg.timestamp_format)?;
            print_balance(mins);
        }
        Some(Commands::Reset { note }) => {
            let mut desc = "= 0 min".to_string();
            if let Some(n) = note {
                desc.push_str(&format!(" # {}", n));
            }
            storage::append_log(&cfg.path, &desc, cfg.timestamp_format)?;
            print_balance(0);
        }
        Some(Commands::Log { last, today, yesterday, week, month, since, until, summary }) => {
            let entries = storage::read_log(&cfg.path)?;

            let now = chrono::Local::now().date_naive();
            let since_date: Option<chrono::NaiveDate>;
            let until_date: Option<chrono::NaiveDate>;

            if today {
                since_date = Some(now);
                until_date = Some(now);
            } else if yesterday {
                let y = now - chrono::Duration::days(1);
                since_date = Some(y);
                until_date = Some(y);
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

            if summary {
                let added: i32 = filtered.iter()
                    .filter_map(|e| e.delta_minutes())
                    .filter(|&d| d > 0)
                    .sum();
                let removed: i32 = filtered.iter()
                    .filter_map(|e| e.delta_minutes())
                    .filter(|&d| d < 0)
                    .sum();
                let net = added + removed;
                let added_str = time::format_duration(added);
                let removed_str = time::format_duration(removed);
                let net_sign = if net >= 0 { "+" } else { "" };
                let net_str = format!("{}{}", net_sign, time::format_duration(net));
                println!("Added:   {}", added_str.if_supports_color(Stdout, |t| t.green()));
                println!("Removed: {}", removed_str.if_supports_color(Stdout, |t| t.red()));
                if net > 0 {
                    println!("Net:     {}", net_str.if_supports_color(Stdout, |t| t.green()));
                } else if net < 0 {
                    println!("Net:     {}", net_str.if_supports_color(Stdout, |t| t.red()));
                } else {
                    println!("Net:     {}", net_str);
                }
            } else {
                for entry in &filtered {
                    print_log_entry(entry);
                }
            }
        }
        Some(Commands::Edit) => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            std::process::Command::new(&editor)
                .arg(&cfg.path)
                .status()
                .with_context(|| format!("failed to launch editor {:?}", editor))?;
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
