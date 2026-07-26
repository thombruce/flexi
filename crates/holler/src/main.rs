mod config;
mod storage;
mod tags;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::{OwoColorize, Stream::Stdout};
use storage::Contact;

#[derive(Parser)]
#[command(name = "holler", version, about = "A plaintext contacts / rolodex")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a contact: `add <name...> [phone:N] [email:E] [+tag]...`
    Add {
        #[arg(required = true)]
        args: Vec<String>,
    },
    /// List contacts (alphabetical by name; `+project`/`@context`/`key:value` tags to filter)
    List { tags: Vec<String> },
    /// Remove a contact by its number in `list`
    #[command(alias = "remove")]
    Rm { index: usize },
    /// Open the contacts file in $EDITOR
    Edit,
    /// Look up phone number(s) for contacts matching a name
    Phone {
        #[arg(required = true)]
        query: Vec<String>,
    },
    /// Look up email address(es) for contacts matching a name
    Email {
        #[arg(required = true)]
        query: Vec<String>,
    },
    /// Print shell completion script
    Completions { shell: Shell },
    /// Print the man page (roff) to stdout
    Man,
}

#[derive(Clone, Copy)]
enum Field {
    Phone,
    Email,
}

impl Field {
    fn values<'a>(&self, c: &'a Contact) -> &'a [String] {
        match self {
            Field::Phone => &c.phone,
            Field::Email => &c.email,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Field::Phone => "phone",
            Field::Email => "email",
        }
    }
}

/// All contacts, sorted alphabetically by name. Indices used by `rm` are
/// positions in this list, so filtered `list` views keep their original
/// index rather than renumbering the visible subset.
fn all_sorted(cfg: &config::ResolvedConfig) -> Result<Vec<Contact>> {
    let mut contacts = storage::read_contacts(&cfg.path)?;
    contacts.sort_by_key(|c| c.sort_key());
    Ok(contacts)
}

fn run_list(cfg: &config::ResolvedConfig, tag_queries: &[String]) -> Result<()> {
    let contacts = all_sorted(cfg)?;
    if contacts.is_empty() {
        println!("no contacts");
        return Ok(());
    }
    let width = contacts.len().to_string().len();
    for (i, c) in contacts.iter().enumerate() {
        if !tags::matches(&c.name, tag_queries) {
            continue;
        }
        let n = format!("{:>width$}", i + 1, width = width);
        println!("{}  {}", n.if_supports_color(Stdout, |t| t.dimmed()), c.to_line());
    }
    Ok(())
}

fn run_lookup(cfg: &config::ResolvedConfig, field: Field, query: &str) -> Result<()> {
    let query_lower = query.to_lowercase();
    let contacts = all_sorted(cfg)?;
    let matches: Vec<&Contact> = contacts.iter().filter(|c| c.name.to_lowercase().contains(&query_lower)).collect();

    match matches.as_slice() {
        [] => anyhow::bail!("no contact matching {:?}", query),
        [only] => {
            let values = field.values(only);
            if values.is_empty() {
                println!("no {} on file for {}", field.label(), only.name);
            } else {
                for v in values {
                    println!("{v}");
                }
            }
        }
        many => {
            for c in many {
                let values = field.values(c);
                if values.is_empty() {
                    println!("{}: (no {})", c.name, field.label());
                } else {
                    for v in values {
                        println!("{}: {}", c.name, v);
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Completions { shell }) = cli.command {
        generate(shell, &mut Cli::command(), "holler", &mut std::io::stdout());
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
        None => run_list(&cfg, &[])?,
        Some(Commands::List { tags }) => run_list(&cfg, &tags)?,
        Some(Commands::Add { args }) => {
            let line = args.join(" ");
            let contact = Contact::parse(&line)?;
            let rendered = contact.to_line();
            storage::append_contact(&cfg.path, &rendered)?;
            println!("added: {rendered}");
        }
        Some(Commands::Rm { index }) => {
            let contacts = all_sorted(&cfg)?;
            let target = index
                .checked_sub(1)
                .and_then(|i| contacts.get(i))
                .with_context(|| format!("no contact {} — run `holler list`", index))?;
            storage::remove_line(&cfg.path, &target.raw)?;
            println!("removed: {}", target.to_line());
        }
        Some(Commands::Edit) => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            std::process::Command::new(&editor)
                .arg(&cfg.path)
                .status()
                .with_context(|| format!("failed to launch editor {:?}", editor))?;
        }
        Some(Commands::Phone { query }) => run_lookup(&cfg, Field::Phone, &query.join(" "))?,
        Some(Commands::Email { query }) => run_lookup(&cfg, Field::Email, &query.join(" "))?,
        Some(Commands::Completions { .. }) | Some(Commands::Man) => unreachable!(),
    }

    Ok(())
}
