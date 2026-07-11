mod config;
mod storage;

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, NaiveTime};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::{OwoColorize, Stream::Stdout};
use storage::Appt;

#[derive(Parser)]
#[command(name = "calchemy", version, about = "A plaintext calendar / appointment book")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add an appointment: `add <date> [HH:MM[-HH:MM]] <title...>`
    Add {
        /// Appointment date, `YYYY-MM-DD`
        date: String,
        /// Optional `HH:MM` or `HH:MM-HH:MM`, then the title
        #[arg(required = true)]
        rest: Vec<String>,
    },
    /// Show the next upcoming appointment
    Next,
    /// List appointments (upcoming by default)
    #[command(alias = "agenda")]
    List {
        #[command(flatten)]
        filter: ApptFilter,
    },
    /// Show today's appointments (shortcut for `list --today`)
    Today,
    /// Show this week's appointments (shortcut for `list --week`)
    Week,
    /// Remove an appointment by its number in `list`
    #[command(alias = "remove")]
    Rm {
        /// The appointment's index as shown by `calchemy list`
        index: usize,
    },
    /// Open the calendar file in $EDITOR
    Edit,
    /// Print shell completion script
    Completions {
        shell: Shell,
    },
    /// Print the man page (roff) to stdout
    Man,
}

#[derive(Args)]
struct ApptFilter {
    /// Only today
    #[arg(long, conflicts_with_all = ["week", "month", "since", "until", "all", "past"])]
    today: bool,
    /// The current calendar week (full week)
    #[arg(long, conflicts_with_all = ["today", "month", "since", "until", "all", "past"])]
    week: bool,
    /// The current calendar month (full month)
    #[arg(long, conflicts_with_all = ["today", "week", "since", "until", "all", "past"])]
    month: bool,
    /// On or after this date (YYYY-MM-DD)
    #[arg(long, conflicts_with_all = ["today", "week", "month", "all", "past"])]
    since: Option<String>,
    /// On or before this date (YYYY-MM-DD)
    #[arg(long, conflicts_with_all = ["today", "week", "month", "all", "past"])]
    until: Option<String>,
    /// Include past appointments as well
    #[arg(long, conflicts_with_all = ["today", "week", "month", "since", "until", "past"])]
    all: bool,
    /// Only past appointments
    #[arg(long, conflicts_with_all = ["today", "week", "month", "since", "until", "all"])]
    past: bool,
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").with_context(|| format!("invalid date {:?}, expected YYYY-MM-DD", s))
}

/// Parses a CLI time argument: `HH:MM` or `HH:MM-HH:MM`. Returns None if `s`
/// isn't a time (so it belongs to the title).
fn parse_time_arg(s: &str) -> Option<(NaiveTime, Option<NaiveTime>)> {
    match s.split_once('-') {
        Some((a, b)) => {
            let start = NaiveTime::parse_from_str(a, "%H:%M").ok()?;
            let end = NaiveTime::parse_from_str(b, "%H:%M").ok()?;
            Some((start, Some(end)))
        }
        None => Some((NaiveTime::parse_from_str(s, "%H:%M").ok()?, None)),
    }
}

/// The time column for an appointment, or None for an all-day event.
fn time_col(a: &Appt) -> Option<String> {
    let start = a.start?;
    Some(match a.end {
        None => start.format("%H:%M").to_string(),
        Some(end) if end.date() == a.date => format!("{}-{}", start.format("%H:%M"), end.format("%H:%M")),
        Some(end) => format!("{}-{}", start.format("%H:%M"), end.format("%Y-%m-%d %H:%M")),
    })
}

/// One-line rendering: `DATE [TIME] TITLE`.
fn render_inline(a: &Appt) -> String {
    match time_col(a) {
        Some(t) => format!("{} {} {}", a.date.format("%Y-%m-%d"), t, a.title),
        None => format!("{} {}", a.date.format("%Y-%m-%d"), a.title),
    }
}

fn relative_day(date: NaiveDate, today: NaiveDate) -> String {
    match (date - today).num_days() {
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        -1 => "yesterday".to_string(),
        n if n > 0 => format!("in {} days", n),
        n => format!("{} days ago", -n),
    }
}

fn sorted(mut appts: Vec<Appt>) -> Vec<Appt> {
    appts.sort_by_key(|a| a.start_dt());
    appts
}

/// Resolves the [since, until] date window a filter selects. `None`/`None`
/// with neither `all` nor `past` means "today onward" (handled by the caller).
fn filter_window(cfg: &config::ResolvedConfig, f: &ApptFilter, today: NaiveDate)
    -> Result<(Option<NaiveDate>, Option<NaiveDate>)>
{
    if f.today {
        Ok((Some(today), Some(today)))
    } else if f.week {
        let from_start = match cfg.week_start {
            config::WeekStart::Monday => today.weekday().num_days_from_monday(),
            config::WeekStart::Sunday => today.weekday().num_days_from_sunday(),
        };
        let start = today - chrono::Duration::days(from_start as i64);
        Ok((Some(start), Some(start + chrono::Duration::days(6))))
    } else if f.month {
        let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1);
        let end = start.map(month_end);
        Ok((start, end))
    } else {
        let since = f.since.as_deref().map(parse_date).transpose()?;
        let until = f.until.as_deref().map(parse_date).transpose()?;
        Ok((since, until))
    }
}

fn month_end(first: NaiveDate) -> NaiveDate {
    let (y, m) = (first.year(), first.month());
    let next = if m == 12 {
        NaiveDate::from_ymd_opt(y + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(y, m + 1, 1)
    };
    next.unwrap() - chrono::Duration::days(1)
}

fn run_list(cfg: &config::ResolvedConfig, f: &ApptFilter) -> Result<()> {
    let today = chrono::Local::now().date_naive();
    let appts = storage::read_appts(&cfg.path)?;
    let (since, until) = filter_window(cfg, f, today)?;

    let windowed = since.is_some() || until.is_some() || f.today || f.week || f.month;
    let view: Vec<Appt> = appts.into_iter().filter(|a| {
        if windowed {
            since.is_none_or(|s| a.date >= s) && until.is_none_or(|u| a.date <= u)
        } else if f.all {
            true
        } else if f.past {
            a.date < today
        } else {
            a.date >= today
        }
    }).collect();

    let view = sorted(view);
    if view.is_empty() {
        println!("no appointments");
        return Ok(());
    }
    let width = view.len().to_string().len();
    for (i, a) in view.iter().enumerate() {
        let n = format!("{:>width$}", i + 1, width = width);
        println!("{}  {}", n.if_supports_color(Stdout, |t| t.dimmed()), render_inline(a));
    }
    Ok(())
}

/// The default upcoming view (today onward, sorted) — also what `rm` indexes.
fn upcoming(cfg: &config::ResolvedConfig, today: NaiveDate) -> Result<Vec<Appt>> {
    let appts = storage::read_appts(&cfg.path)?;
    Ok(sorted(appts.into_iter().filter(|a| a.date >= today).collect()))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Completions { shell }) = cli.command {
        generate(shell, &mut Cli::command(), "calchemy", &mut std::io::stdout());
        return Ok(());
    }
    if let Some(Commands::Man) = cli.command {
        clap_mangen::Man::new(Cli::command())
            .render(&mut std::io::stdout())
            .context("rendering man page")?;
        return Ok(());
    }

    let cfg = config::resolve()?;
    let today = chrono::Local::now().date_naive();

    match cli.command {
        // Bare invocation: today's agenda.
        None | Some(Commands::Today) => {
            let todays: Vec<Appt> = sorted(
                storage::read_appts(&cfg.path)?.into_iter().filter(|a| a.date == today).collect(),
            );
            if todays.is_empty() {
                println!("no appointments today");
            } else {
                let header = format!("{} (today)", today.format("%Y-%m-%d"));
                println!("{}", header.if_supports_color(Stdout, |t| t.bold()));
                let width = todays.iter().filter_map(time_col).map(|t| t.len()).max().unwrap_or(7).max(7);
                for a in &todays {
                    let col = time_col(a).unwrap_or_else(|| "all day".to_string());
                    println!("  {:<width$}  {}", col, a.title, width = width);
                }
            }
        }
        Some(Commands::Week) => {
            run_list(&cfg, &ApptFilter { today: false, week: true, month: false, since: None, until: None, all: false, past: false })?;
        }
        Some(Commands::List { filter }) => run_list(&cfg, &filter)?,
        Some(Commands::Next) => {
            let now = chrono::Local::now().naive_local();
            let next = upcoming(&cfg, today)?
                .into_iter()
                .find(|a| a.start.is_none() || a.start_dt() >= now);
            match next {
                None => println!("no upcoming appointments"),
                Some(a) => {
                    let rel = relative_day(a.date, today);
                    println!("next: {} ({})", render_inline(&a), rel.if_supports_color(Stdout, |t| t.dimmed()));
                }
            }
        }
        Some(Commands::Add { date, rest }) => {
            let date = parse_date(&date)?;
            // The first arg may be a time; if so the rest is the title.
            let (start, end_time, title_parts): (Option<NaiveTime>, Option<NaiveTime>, &[String]) =
                match parse_time_arg(&rest[0]) {
                    Some((s, e)) => (Some(s), e, &rest[1..]),
                    None => (None, None, &rest[..]),
                };
            let title = title_parts.join(" ");
            anyhow::ensure!(!title.trim().is_empty(), "an appointment needs a title");
            anyhow::ensure!(!title.contains(['\n', '\r']), "title must not contain newlines");

            // Build the end datetime: same day, or next day if end <= start.
            let end = end_time.map(|e| {
                let end_date = if Some(e) > start { date } else { date + chrono::Duration::days(1) };
                end_date.and_time(e)
            });
            let appt = Appt { date, start, end, title: title.clone(), raw: String::new() };
            storage::append_appt(&cfg.path, &appt)?;
            println!("added: {}", render_inline(&appt));
        }
        Some(Commands::Rm { index }) => {
            let up = upcoming(&cfg, today)?;
            let target = index
                .checked_sub(1)
                .and_then(|i| up.get(i))
                .with_context(|| format!("no appointment {} — run `calchemy list`", index))?;
            let shown = render_inline(target);
            storage::remove_line(&cfg.path, &target.raw)?;
            println!("removed: {}", shown);
        }
        Some(Commands::Edit) => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            std::process::Command::new(&editor)
                .arg(&cfg.path)
                .status()
                .with_context(|| format!("failed to launch editor {:?}", editor))?;
        }
        Some(Commands::Completions { .. }) | Some(Commands::Man) => unreachable!(),
    }

    Ok(())
}
