mod config;
mod storage;

use anyhow::{Context, Result};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use foolscap::tags;
use owo_colors::{OwoColorize, Stream::Stdout};
use foolscap::prefix::Status;
use storage::{Item, PriceFormat};

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
        /// Price, e.g. `12.99`, or `?` for deliberately unspecified
        #[arg(long, short = 'p')]
        price: Option<String>,
        /// Quantity desired (default 1)
        #[arg(long, short = 'q')]
        qty: Option<u32>,
        /// Priority, a single letter A-Z
        #[arg(long, value_parser = foolscap::prefix::parse_priority_arg)]
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
    /// Toggle "buying this trip" (`!`) for the item at this position in `list`
    Focus {
        /// The item's index as shown by `bagg list`
        index: usize,
    },
    /// Toggle "won't buy" (`~`, kept for the record) for the item at this position in `list`
    Wont {
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
    /// Only not-got items (including `!` this trip)
    #[arg(long, conflicts_with_all = ["got", "wont"])]
    pending: bool,
    /// Only got items
    #[arg(long, conflicts_with_all = ["pending", "wont"])]
    got: bool,
    /// Only won't-buy items
    #[arg(long, conflicts_with_all = ["pending", "got"])]
    wont: bool,
}

/// All items, sorted by status/priority/price. Indices used by `got`/`rm`
/// are positions in this list, so filtered `list` views keep their
/// original index rather than renumbering the visible subset.
fn all_sorted(cfg: &config::ResolvedConfig, fmt: PriceFormat) -> Result<Vec<Item>> {
    let mut items = storage::read_items(&cfg.path, fmt)?;
    items.sort_by_key(|i| i.sort_key());
    Ok(items)
}

fn find_item(cfg: &config::ResolvedConfig, fmt: PriceFormat, index: usize) -> Result<Item> {
    let items = all_sorted(cfg, fmt)?;
    index
        .checked_sub(1)
        .and_then(|i| items.get(i).cloned())
        .with_context(|| format!("no item {} — run `bagg list`", index))
}

/// Toggles the item at `index` towards `to`, or back to wanted.
fn toggle_status(cfg: &config::ResolvedConfig, fmt: PriceFormat, index: usize, to: Status) -> Result<()> {
    let mut item = find_item(cfg, fmt, index)?;
    let old_raw = item.raw.clone();
    item.status = item.status.toggled(to);
    let new_line = item.to_line(fmt, cfg.always_show_unspecified_price);
    storage::replace_line(&cfg.path, &old_raw, &new_line)?;
    let label = match item.status {
        Status::Focus => "this trip",
        Status::Open => "want",
        Status::Done => "got",
        Status::Obsolete => "won't buy",
    };
    println!("{}: {}", label, new_line);
    Ok(())
}

fn run_list(cfg: &config::ResolvedConfig, fmt: PriceFormat, filter: &ItemFilter, tag_queries: &[String]) -> Result<()> {
    let items = all_sorted(cfg, fmt)?;
    if items.is_empty() {
        println!("list is empty");
        return Ok(());
    }
    let width = items.len().to_string().len();
    for (i, item) in items.iter().enumerate() {
        if filter.pending && item.status.is_closed() {
            continue;
        }
        if filter.got && item.status != Status::Done {
            continue;
        }
        if filter.wont && item.status != Status::Obsolete {
            continue;
        }
        if !tags::matches(&item.name, tag_queries) {
            continue;
        }
        let n = format!("{:>width$}", i + 1, width = width);
        let line = item.to_line(fmt, cfg.always_show_unspecified_price);
        let line = if item.status.is_closed() { line.if_supports_color(Stdout, |t| t.dimmed()).to_string() } else { line };
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
    let fmt = PriceFormat { separator: cfg.decimal_separator, places: cfg.decimal_places };

    match cli.command {
        None => run_list(&cfg, fmt, &ItemFilter { pending: false, got: false, wont: false }, &[])?,
        Some(Commands::List { filter, tags }) => run_list(&cfg, fmt, &filter, &tags)?,
        Some(Commands::Add { price, qty, priority, name }) => {
            let name = name.join(" ");
            anyhow::ensure!(!name.trim().is_empty(), "an item needs a name");
            anyhow::ensure!(!name.contains(['\n', '\r']), "name must not contain newlines");
            let price = match price.as_deref() {
                None | Some(storage::UNSPECIFIED_PRICE) => None,
                Some(s) => Some(storage::parse_price(s, fmt).with_context(|| {
                    format!("invalid price {:?}, expected e.g. {}", s, storage::format_price(1234, fmt))
                })?),
            };
            let item = Item { status: Status::Open, priority, price, qty: qty.unwrap_or(1), name, raw: String::new() };
            let line = item.to_line(fmt, cfg.always_show_unspecified_price);
            storage::append_line(&cfg.path, &line)?;
            println!("added: {}", line);
        }
        Some(Commands::Got { index }) => toggle_status(&cfg, fmt, index, Status::Done)?,
        Some(Commands::Focus { index }) => toggle_status(&cfg, fmt, index, Status::Focus)?,
        Some(Commands::Wont { index }) => toggle_status(&cfg, fmt, index, Status::Obsolete)?,
        Some(Commands::Rm { index }) => {
            let item = find_item(&cfg, fmt, index)?;
            storage::remove_line(&cfg.path, &item.raw)?;
            println!("removed: {}", item.to_line(fmt, cfg.always_show_unspecified_price));
        }
        Some(Commands::Edit) => {
            foolscap::open_in_editor(&cfg.path)?;
        }
        Some(Commands::Completions { .. }) | Some(Commands::Man) => unreachable!(),
    }

    Ok(())
}
