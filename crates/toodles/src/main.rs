mod config;
mod storage;
mod tags;

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::{OwoColorize, Stream::Stdout};
use storage::Todo;

#[derive(Parser)]
#[command(name = "toodles", version, about = "A plaintext todo list, with due dates")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a todo: `add [--priority A-Z] [--due YYYY-MM-DD] <title...>`
    Add {
        /// Priority, a single letter A-Z
        #[arg(long, short = 'p', value_parser = parse_priority_arg)]
        priority: Option<char>,
        /// Due date, YYYY-MM-DD
        #[arg(long, short = 'd', value_parser = parse_date_arg)]
        due: Option<NaiveDate>,
        #[arg(required = true)]
        title: Vec<String>,
    },
    /// List todos (everything, sorted; `--open`/`--done` to filter)
    List {
        #[command(flatten)]
        filter: TodoFilter,
        /// Only todos carrying any of these `+project`/`@context`/`key:value`
        /// tags (case-insensitive)
        tags: Vec<String>,
    },
    /// Toggle done/open for the todo at this position in `list`
    Done {
        /// The todo's index as shown by `toodles list`
        index: usize,
    },
    /// Remove a todo by its number in `list`
    #[command(alias = "remove")]
    Rm {
        /// The todo's index as shown by `toodles list`
        index: usize,
    },
    /// Open the todo file in $EDITOR
    Edit,
    /// Print shell completion script
    Completions {
        shell: Shell,
    },
    /// Print the man page (roff) to stdout
    Man,
}

#[derive(Args)]
struct TodoFilter {
    /// Only open todos
    #[arg(long, conflicts_with = "done")]
    open: bool,
    /// Only done todos
    #[arg(long, conflicts_with = "open")]
    done: bool,
}

fn parse_priority_arg(s: &str) -> Result<char, String> {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_uppercase() => Ok(c),
        _ => Err(format!("invalid priority {:?}, expected a single letter A-Z", s)),
    }
}

fn parse_date_arg(s: &str) -> Result<NaiveDate, String> {
    storage::parse_date(s).ok_or_else(|| format!("invalid date {:?}, expected YYYY-MM-DD", s))
}

/// All todos, sorted by status/priority/due. Indices used by `done`/`rm`
/// are positions in this list, so filtered `list` views keep their
/// original index rather than renumbering the visible subset.
fn all_sorted(cfg: &config::ResolvedConfig) -> Result<Vec<Todo>> {
    let mut todos = storage::read_todos(&cfg.path)?;
    todos.sort_by_key(|t| t.sort_key());
    Ok(todos)
}

fn find_todo(cfg: &config::ResolvedConfig, index: usize) -> Result<Todo> {
    let todos = all_sorted(cfg)?;
    index
        .checked_sub(1)
        .and_then(|i| todos.get(i).cloned())
        .with_context(|| format!("no todo {} — run `toodles list`", index))
}

fn run_list(cfg: &config::ResolvedConfig, filter: &TodoFilter, tag_queries: &[String]) -> Result<()> {
    let todos = all_sorted(cfg)?;
    if todos.is_empty() {
        println!("list is empty");
        return Ok(());
    }
    let today = Local::now().date_naive();
    let width = todos.len().to_string().len();
    for (i, todo) in todos.iter().enumerate() {
        if filter.open && todo.done {
            continue;
        }
        if filter.done && !todo.done {
            continue;
        }
        if !tags::matches(&todo.title, tag_queries) {
            continue;
        }
        let n = format!("{:>width$}", i + 1, width = width);
        let line = todo.to_line();
        let line = if todo.done {
            line.if_supports_color(Stdout, |t| t.dimmed()).to_string()
        } else if todo.due.is_some_and(|d| d < today) {
            line.if_supports_color(Stdout, |t| t.red()).to_string()
        } else {
            line
        };
        println!("{}  {}", n.if_supports_color(Stdout, |t| t.dimmed()), line);
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Completions { shell }) = cli.command {
        generate(shell, &mut Cli::command(), "toodles", &mut std::io::stdout());
        return Ok(());
    }
    if let Some(Commands::Man) = cli.command {
        clap_mangen::Man::new(Cli::command())
            .render(&mut std::io::stdout())
            .context("rendering man page")?;
        return Ok(());
    }

    let cfg = config::resolve()?;
    let today = Local::now().date_naive();

    match cli.command {
        None => run_list(&cfg, &TodoFilter { open: false, done: false }, &[])?,
        Some(Commands::List { filter, tags }) => run_list(&cfg, &filter, &tags)?,
        Some(Commands::Add { priority, due, title }) => {
            let title = title.join(" ");
            anyhow::ensure!(!title.trim().is_empty(), "a todo needs a title");
            anyhow::ensure!(!title.contains(['\n', '\r']), "title must not contain newlines");
            // Without --due, a date-leading title would land in the due slot on re-read.
            anyhow::ensure!(
                due.is_some() || title.split_whitespace().next().and_then(storage::parse_date).is_none(),
                "title starts with a date, which would be read as the due date — pass --due, or add it with `toodles edit`"
            );
            // Created is always stamped: a due date is only expressible after one.
            let todo = Todo { done: false, priority, closed: None, created: Some(today), due, title, raw: String::new() };
            let line = todo.to_line();
            storage::append_todo(&cfg.path, &line)?;
            println!("added: {}", line);
        }
        Some(Commands::Done { index }) => {
            let mut todo = find_todo(&cfg, index)?;
            let old_raw = todo.raw.clone();
            todo.done = !todo.done;
            todo.closed = todo.done.then_some(today);
            let new_line = todo.to_line();
            storage::replace_line(&cfg.path, &old_raw, &new_line)?;
            println!("{}: {}", if todo.done { "done" } else { "reopened" }, new_line);
        }
        Some(Commands::Rm { index }) => {
            let todo = find_todo(&cfg, index)?;
            storage::remove_line(&cfg.path, &todo.raw)?;
            println!("removed: {}", todo.to_line());
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
