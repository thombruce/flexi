mod config;
mod storage;
mod time;

use anyhow::{Context, Result};
use chrono::Datelike;
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::{OwoColorize, Stream::Stdout};

#[derive(Parser)]
#[command(name = "clocking", version, about = "Log your working hours")]
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
    /// Clock in — start a work session
    In {
        /// Attach a note to this session
        #[arg(long, short = 'm', value_parser = parse_note)]
        note: Option<String>,
    },
    /// Clock out — close the open session and record the hours worked
    Out {
        /// Attach a note to this log entry
        #[arg(long, short = 'm', value_parser = parse_note)]
        note: Option<String>,
        /// Record the session even if it exceeds `max_session`
        #[arg(long)]
        force: bool,
    },
    /// Show worked sessions
    Log {
        #[command(flatten)]
        filter: LogFilter,
        /// Show the total worked instead of individual sessions
        #[arg(long)]
        summary: bool,
    },
    /// Show total hours worked for a period (shortcut for `log --summary`)
    Summary {
        #[command(flatten)]
        filter: LogFilter,
    },
    /// Open the log file in $EDITOR
    Edit,
    /// Undo the last log entry
    Undo,
    /// Print shell completion script
    Completions {
        shell: Shell,
    },
    /// Print the man page (roff) to stdout
    Man,
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

/// Returns the open `@in` marker if a session is currently running.
fn open_session(cfg: &config::ResolvedConfig) -> Result<Option<storage::LogEntry>> {
    Ok(storage::last_entry(&cfg.path)?.filter(|e| e.is_open()))
}

/// Parses a log timestamp (simple or full format) into a local datetime.
fn parse_entry_time(ts: &str) -> Result<chrono::DateTime<chrono::Local>> {
    if ts.len() > 10 && ts.as_bytes()[10] == b'T' {
        Ok(chrono::DateTime::parse_from_rfc3339(ts)
            .with_context(|| format!("parsing timestamp {:?}", ts))?
            .with_timezone(&chrono::Local))
    } else {
        let naive = chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M")
            .with_context(|| format!("parsing timestamp {:?}", ts))?;
        naive.and_local_timezone(chrono::Local)
            .single()
            .with_context(|| format!("ambiguous local time {:?}", ts))
    }
}

fn worked(mins: i32) -> String {
    time::format_duration(mins)
        .if_supports_color(Stdout, |t| t.green())
        .to_string()
}

fn print_log_entry(entry: &storage::LogEntry) {
    let ts = entry.timestamp.get(..16).unwrap_or(&entry.timestamp).replace('T', " ");
    let (body, note) = match entry.description.split_once(" # ") {
        Some((b, n)) => (b.to_string(), Some(n.to_string())),
        None => (entry.description.clone(), None),
    };
    let rendered = if entry.is_open() {
        format!("clocked in {} (open)", ts.get(11..).unwrap_or(""))
            .if_supports_color(Stdout, |t| t.yellow())
            .to_string()
    } else {
        body.if_supports_color(Stdout, |t| t.green()).to_string()
    };
    if let Some(n) = note {
        let note_str = format!("  # {}", n);
        println!("{}  {}{}", ts, rendered, note_str.if_supports_color(Stdout, |t| t.dimmed()));
    } else {
        println!("{}  {}", ts, rendered);
    }
}

/// Resolves the [since, until] date window a `LogFilter` selects.
fn filter_window(cfg: &config::ResolvedConfig, filter: &LogFilter)
    -> Result<(Option<chrono::NaiveDate>, Option<chrono::NaiveDate>)>
{
    let now = chrono::Local::now().date_naive();
    if filter.today {
        Ok((Some(now), Some(now)))
    } else if filter.yesterday {
        let y = now - chrono::Duration::days(1);
        Ok((Some(y), Some(y)))
    } else if filter.week {
        let days_from_start = match cfg.week_start {
            config::WeekStart::Monday => now.weekday().num_days_from_monday(),
            config::WeekStart::Sunday => now.weekday().num_days_from_sunday(),
        };
        Ok((Some(now - chrono::Duration::days(days_from_start as i64)), Some(now)))
    } else if filter.month {
        Ok((chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1), Some(now)))
    } else {
        let since = filter.since.as_deref()
            .map(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .context("invalid --since date, expected YYYY-MM-DD"))
            .transpose()?;
        let until = filter.until.as_deref()
            .map(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .context("invalid --until date, expected YYYY-MM-DD"))
            .transpose()?;
        Ok((since, until))
    }
}

fn run_log(cfg: &config::ResolvedConfig, filter: &LogFilter, summary: bool) -> Result<()> {
    let entries = storage::read_log(&cfg.path)?;
    let (since_date, until_date) = filter_window(cfg, filter)?;

    let mut filtered: Vec<_> = entries.into_iter().filter(|e| {
        let date_str = e.timestamp.get(..10).unwrap_or("");
        match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Err(_) => true,
            Ok(d) => since_date.is_none_or(|s| d >= s) && until_date.is_none_or(|u| d <= u),
        }
    }).collect();

    if let Some(n) = filter.last {
        let len = filtered.len();
        if n < len {
            filtered = filtered.into_iter().skip(len - n).collect();
        }
    }

    if summary {
        let total: i32 = filtered.iter().filter_map(|e| e.session_minutes()).sum();
        let count = filtered.iter().filter(|e| e.session_minutes().is_some()).count();
        let sessions = if count == 1 { "session" } else { "sessions" };
        println!("Worked: {} ({} {})", worked(total), count, sessions);
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
        generate(shell, &mut Cli::command(), "clocking", &mut std::io::stdout());
        return Ok(());
    }

    if let Some(Commands::Man) = cli.command {
        clap_mangen::Man::new(Cli::command())
            .render(&mut std::io::stdout())
            .context("rendering man page")?;
        return Ok(());
    }

    let cfg = config::resolve()?;

    match cli.command {
        None => {
            if let Some(entry) = open_session(&cfg)? {
                let start = parse_entry_time(&entry.timestamp)?;
                let elapsed_raw = (chrono::Local::now() - start).num_minutes().max(0) as i32;
                let elapsed = time::round_to_increment(elapsed_raw, cfg.increment);
                println!(
                    "clocked in since {} ({} so far)",
                    start.format("%H:%M"),
                    time::format_duration(elapsed)
                );
                if cfg.max_session.is_some_and(|cap| elapsed_raw > cap) {
                    let warn = "⚠ over max_session — close with `clocking out --force`";
                    println!("{}", warn.if_supports_color(Stdout, |t| t.yellow()));
                }
            } else {
                let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
                let total: i32 = storage::read_log(&cfg.path)?
                    .iter()
                    .filter(|e| e.timestamp.starts_with(&today))
                    .filter_map(|e| e.session_minutes())
                    .sum();
                if total > 0 {
                    println!("not clocked in — {} worked today", worked(total));
                } else {
                    println!("not clocked in");
                }
            }
        }
        Some(Commands::In { note }) => {
            if let Some(entry) = open_session(&cfg)? {
                let start = parse_entry_time(&entry.timestamp)?;
                anyhow::bail!("already clocked in since {} — run `clocking out` first", start.format("%H:%M"));
            }
            let mut desc = "@in".to_string();
            if let Some(n) = note {
                desc.push_str(&format!(" # {}", n));
            }
            storage::append_log(&cfg.path, &desc, cfg.timestamp_format)?;
            println!("clocked in at {}", chrono::Local::now().format("%H:%M"));
        }
        Some(Commands::Out { note, force }) => {
            let entry = open_session(&cfg)?
                .context("not clocked in — run `clocking in` first")?;
            let start = parse_entry_time(&entry.timestamp)?;
            let now = chrono::Local::now();
            let elapsed = (now - start).num_minutes();
            if elapsed < 0 {
                anyhow::bail!("session start time is in the future; not recording");
            }
            let elapsed = elapsed as i32;
            if let Some(cap) = cfg.max_session {
                if elapsed > cap && !force {
                    anyhow::bail!(
                        "session is {} (over max_session of {}) — did you forget to clock out? \
                         Run `clocking out --force` to record it anyway.",
                        time::format_duration(elapsed),
                        time::format_duration(cap),
                    );
                }
            }
            let elapsed = time::round_to_increment(elapsed, cfg.increment);
            let span = format!("{}–{}", start.format("%H:%M"), now.format("%H:%M"));
            let mut desc = format!("{} ({})", time::format_duration(elapsed), span);
            // Keep both the clock-in note and any clock-out `-m` note; don't drop either.
            let open_note = entry.description.split_once(" # ").map(|(_, n)| n.to_string());
            let extra: Vec<String> = [open_note, note].into_iter().flatten().collect();
            if !extra.is_empty() {
                desc.push_str(&format!(" # {}", extra.join("; ")));
            }
            storage::pop_log(&cfg.path)?;
            storage::append_log(&cfg.path, &desc, cfg.timestamp_format)?;
            println!("clocked out — worked {} ({})", worked(elapsed), span);
        }
        Some(Commands::Log { filter, summary }) => {
            run_log(&cfg, &filter, summary)?;
        }
        Some(Commands::Summary { filter }) => {
            run_log(&cfg, &filter, true)?;
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
                Some(entry) => println!("removed: {}", entry.description),
            }
        }
        Some(Commands::Completions { .. }) | Some(Commands::Man) => unreachable!(),
    }

    Ok(())
}
