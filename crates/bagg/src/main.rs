mod config;
mod storage;
mod tags;

use anyhow::{Context, Result};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::{OwoColorize, Stream::Stdout};
use storage::Item;

#[derive(Parser)]
#[command(name = "bagg", version, about = "A plaintext shopping list / wishlist")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add an item: `add [--price 12.99] [--qty N] [--priority A-Z] <name...>`
    Add {
        /// Price, e.g. `12.99`
        #[arg(long, short = 'p', value_parser = parse_price_arg)]
        price: Option<i32>,
        /// Quantity desired (default 1)
        #[arg(long, short = 'q')]
        qty: Option<u32>,
        /// Priority, a single letter A-Z
        #[arg(long, value_parser = parse_priority_arg)]
        priority: Option<char>,
        #[arg(required = true)]
        name: Vec<String>,
    },
    /// List items (everything, sorted; `--pending`/`--got` to filter)
    List {
        #[command(flatten)]
        filter: ItemFilter,
        /// Only items carrying any of these `+project`/`@context`/`key:value`
        /// tags (case-insensitive)
        tags: Vec<String>,
    },
    /// Toggle got/not-got status for the item at this position in `list`
    Got {
        /// The item's index as shown by `bagg list`
        index: usize,
    },
    /// Remove an item by its number in `list`
    #[command(alias = "remove")]
    Rm {
        /// The item's index as shown by `bagg list`
        index: usize,
    },
    /// Open the list file in $EDITOR
    Edit,
    /// Print shell completion script
    Completions {
        shell: Shell,
    },
    /// Print the man page (roff) to stdout
    Man,
}

#[derive(Args)]
struct ItemFilter {
    /// Only not-got items
    #[arg(long, conflicts_with = "got")]
    pending: bool,
    /// Only got items
    #[arg(long, conflicts_with = "pending")]
    got: bool,
}

fn parse_price_arg(s: &str) -> Result<i32, String> {
    storage::parse_price(s).ok_or_else(|| format!("invalid price {:?}, expected e.g. 12.99", s))
}

fn parse_priority_arg(s: &str) -> Result<char, String> {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_uppercase() => Ok(c),
        _ => Err(format!("invalid priority {:?}, expected a single letter A-Z", s)),
    }
}

/// All items, sorted by status/priority/price. Indices used by `got`/`rm`
/// are positions in this list, so filtered `list` views keep their
/// original index rather than renumbering the visible subset.
fn all_sorted(cfg: &config::ResolvedConfig) -> Result<Vec<Item>> {
    let mut items = storage::read_items(&cfg.path)?;
    items.sort_by_key(|i| i.sort_key());
    Ok(items)
}

fn find_item(cfg: &config::ResolvedConfig, index: usize) -> Result<Item> {
    let items = all_sorted(cfg)?;
    index
        .checked_sub(1)
        .and_then(|i| items.get(i).cloned())
        .with_context(|| format!("no item {} — run `bagg list`", index))
}

fn run_list(cfg: &config::ResolvedConfig, filter: &ItemFilter, tag_queries: &[String]) -> Result<()> {
    let items = all_sorted(cfg)?;
    if items.is_empty() {
        println!("list is empty");
        return Ok(());
    }
    let width = items.len().to_string().len();
    for (i, item) in items.iter().enumerate() {
        if filter.pending && item.got {
            continue;
        }
        if filter.got && !item.got {
            continue;
        }
        if !tags::matches(&item.name, tag_queries) {
            continue;
        }
        let n = format!("{:>width$}", i + 1, width = width);
        let line = item.to_line();
        let line = if item.got { line.if_supports_color(Stdout, |t| t.dimmed()).to_string() } else { line };
        println!("{}  {}", n.if_supports_color(Stdout, |t| t.dimmed()), line);
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Completions { shell }) = cli.command {
        generate(shell, &mut Cli::command(), "bagg", &mut std::io::stdout());
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
        None => run_list(&cfg, &ItemFilter { pending: false, got: false }, &[])?,
        Some(Commands::List { filter, tags }) => run_list(&cfg, &filter, &tags)?,
        Some(Commands::Add { price, qty, priority, name }) => {
            let name = name.join(" ");
            anyhow::ensure!(!name.trim().is_empty(), "an item needs a name");
            anyhow::ensure!(!name.contains(['\n', '\r']), "name must not contain newlines");
            let item = Item { got: false, priority, price, qty: qty.unwrap_or(1), name, raw: String::new() };
            storage::append_item(&cfg.path, &item)?;
            println!("added: {}", item.to_line());
        }
        Some(Commands::Got { index }) => {
            let mut item = find_item(&cfg, index)?;
            let old_raw = item.raw.clone();
            item.got = !item.got;
            let new_line = item.to_line();
            storage::replace_line(&cfg.path, &old_raw, &new_line)?;
            println!("{}: {}", if item.got { "got" } else { "want" }, new_line);
        }
        Some(Commands::Rm { index }) => {
            let item = find_item(&cfg, index)?;
            storage::remove_line(&cfg.path, &item.raw)?;
            println!("removed: {}", item.to_line());
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
