mod config;
mod storage;
mod time;

use anyhow::{Context, Result};
use arboard::Clipboard;
use chrono::Datelike;
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::{OwoColorize, Stream::Stdout};

#[derive(Parser)]
#[command(name = "flexi", version, about = "Track your flexi-time balance")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

fn parse_note(s: &str) -> Result<String, String> {
    if s.contains('\n') || s.contains('\r') {
        return Err("note must not contain newlines".to_string());
    }
    if s.trim().is_empty() {
        return Err("note must not be empty".to_string());
    }
    Ok(s.to_string())
}

#[derive(Subcommand)]
enum Commands {
    /// Add time to your flexi balance
    Add {
        #[arg(required = true)]
        time: Vec<String>,
        /// Attach a note to this log entry
        #[arg(long, short = 'm', value_parser = parse_note)]
        note: Option<String>,
    },
    /// Remove time from your flexi balance
    #[command(alias = "rm")]
    Remove {
        #[arg(required = true)]
        time: Vec<String>,
        /// Attach a note to this log entry
        #[arg(long, short = 'm', value_parser = parse_note)]
        note: Option<String>,
    },
    /// Set your flexi balance to an exact value
    Set {
        #[arg(required = true)]
        time: Vec<String>,
        /// Attach a note to this log entry
        #[arg(long, short = 'm', value_parser = parse_note)]
        note: Option<String>,
    },
    /// Record a note without changing your balance
    Note {
        /// The note text
        #[arg(required = true, value_parser = parse_note)]
        text: String,
    },
    /// Reset your flexi balance to zero
    Reset {
        /// Attach a note to this log entry
        #[arg(long, short = 'm', value_parser = parse_note)]
        note: Option<String>,
    },
    /// Show balance change history
    Log {
        #[command(flatten)]
        filter: LogFilter,
        /// Show totals instead of individual entries
        #[arg(long)]
        summary: bool,
        /// Describe the change in plain sentences
        #[arg(long, conflicts_with = "summary")]
        prose: bool,
        /// Output as JSON (machine-readable; combine with --summary for totals)
        #[arg(long, conflicts_with = "prose")]
        json: bool,
    },
    /// Show totals for a period (shortcut for `log --summary`)
    Summary {
        #[command(flatten)]
        filter: LogFilter,
    },
    /// Describe a period's change in plain sentences (shortcut for `log --prose`)
    Prose {
        #[command(flatten)]
        filter: LogFilter,
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

#[derive(Args)]
struct LogFilter {
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
    } else if change < 0 {
        delta.if_supports_color(Stdout, |t| t.red()).to_string()
    } else {
        delta
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

fn print_prose(label: &str, added: i32, removed: i32, balance: i32) {
    let net = added + removed;
    let bal = {
        let f = time::format_duration(balance);
        if balance > 0 {
            f.if_supports_color(Stdout, |t| t.green()).to_string()
        } else if balance < 0 {
            f.if_supports_color(Stdout, |t| t.red()).to_string()
        } else {
            f
        }
    };

    if added == 0 && removed == 0 {
        println!("{}: no change. Balance now {}.", label, bal);
        return;
    }

    let breakdown = format!(
        "(added {}, removed {})",
        time::format_duration(added),
        time::format_duration(removed.abs())
    );

    if net > 0 {
        let mag = time::format_duration(net)
            .if_supports_color(Stdout, |t| t.green())
            .to_string();
        if added != 0 && removed != 0 {
            println!("{}: banked {} {}. Balance now {}.", label, mag, breakdown, bal);
        } else {
            println!("{}: banked {}. Balance now {}.", label, mag, bal);
        }
    } else if net < 0 {
        let mag = time::format_duration(net.abs())
            .if_supports_color(Stdout, |t| t.red())
            .to_string();
        if added != 0 && removed != 0 {
            println!("{}: used {} {}. Balance now {}.", label, mag, breakdown, bal);
        } else {
            println!("{}: used {}. Balance now {}.", label, mag, bal);
        }
    } else {
        println!("{}: net zero {}. Balance now {}.", label, breakdown, bal);
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

fn run_log(cfg: &config::ResolvedConfig, filter: &LogFilter, summary: bool, prose: bool, json: bool) -> Result<()> {
    let entries = storage::read_log(&cfg.path)?;

    let now = chrono::Local::now().date_naive();
    let since_date: Option<chrono::NaiveDate>;
    let until_date: Option<chrono::NaiveDate>;

    if filter.today {
        since_date = Some(now);
        until_date = Some(now);
    } else if filter.yesterday {
        let y = now - chrono::Duration::days(1);
        since_date = Some(y);
        until_date = Some(y);
    } else if filter.week {
        let days_from_start = match cfg.week_start {
            config::WeekStart::Monday => now.weekday().num_days_from_monday(),
            config::WeekStart::Sunday => now.weekday().num_days_from_sunday(),
        };
        since_date = Some(now - chrono::Duration::days(days_from_start as i64));
        until_date = Some(now);
    } else if filter.month {
        since_date = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1);
        until_date = Some(now);
    } else {
        since_date = filter.since.as_deref()
            .map(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .context("invalid --since date, expected YYYY-MM-DD"))
            .transpose()?;
        until_date = filter.until.as_deref()
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

    if let Some(n) = filter.last {
        let len = filtered.len();
        if n < len {
            filtered = filtered.into_iter().skip(len - n).collect();
        }
    }

    if json {
        if summary {
            let added: i32 = filtered.iter()
                .filter_map(|e| e.delta_minutes())
                .filter(|&d| d > 0)
                .sum();
            let removed: i32 = filtered.iter()
                .filter_map(|e| e.delta_minutes())
                .filter(|&d| d < 0)
                .sum();
            let obj = serde_json::json!({
                "added_minutes": added,
                "removed_minutes": removed,
                "net_minutes": added + removed,
            });
            println!("{}", serde_json::to_string_pretty(&obj)?);
        } else {
            let arr: Vec<_> = filtered.iter().map(|e| {
                let note = e.description.split_once(" # ").map(|(_, n)| n);
                serde_json::json!({
                    "timestamp": e.timestamp,
                    "delta_minutes": e.delta_minutes(),
                    "balance_minutes": e.new_minutes().ok(),
                    "note": note,
                })
            }).collect();
            println!("{}", serde_json::to_string_pretty(&serde_json::Value::Array(arr))?);
        }
        return Ok(());
    }

    if summary || prose {
        let added: i32 = filtered.iter()
            .filter_map(|e| e.delta_minutes())
            .filter(|&d| d > 0)
            .sum();
        let removed: i32 = filtered.iter()
            .filter_map(|e| e.delta_minutes())
            .filter(|&d| d < 0)
            .sum();
        let net = added + removed;
        if prose {
            let label = if filter.today {
                "Today".to_string()
            } else if filter.yesterday {
                "Yesterday".to_string()
            } else if filter.week {
                "This week".to_string()
            } else if filter.month {
                "This month".to_string()
            } else {
                match (filter.since.as_deref(), filter.until.as_deref()) {
                    (Some(s), Some(u)) => format!("Between {} and {}", s, u),
                    (Some(s), None) => format!("Since {}", s),
                    (None, Some(u)) => format!("Up to {}", u),
                    (None, None) => "Overall".to_string(),
                }
            };
            let balance = storage::read_minutes(&cfg.path)?;
            print_prose(&label, added, removed, balance);
        } else {
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
        }
    } else {
        for entry in &filtered {
            print_log_entry(entry);
        }
    }

    Ok(())
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
        Some(Commands::Note { text }) => {
            let current = storage::read_minutes(&cfg.path)?;
            record_change(&cfg, current, current, Some(&text))?;
        }
        Some(Commands::Reset { note }) => {
            let mut desc = "= 0 min".to_string();
            if let Some(n) = note {
                desc.push_str(&format!(" # {}", n));
            }
            storage::append_log(&cfg.path, &desc, cfg.timestamp_format)?;
            print_balance(0);
        }
        Some(Commands::Log { filter, summary, prose, json }) => {
            run_log(&cfg, &filter, summary, prose, json)?;
        }
        Some(Commands::Summary { filter }) => {
            run_log(&cfg, &filter, true, false, false)?;
        }
        Some(Commands::Prose { filter }) => {
            run_log(&cfg, &filter, false, true, false)?;
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
