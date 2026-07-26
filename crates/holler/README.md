# holler

A minimal plaintext contacts / rolodex for the command line.

Part of the [flexi](../../) family of plaintext-storage tools. Named as a verb rather than a noun — `holler email John Smith` reads as an imperative sentence, which is the point: alongside the usual `add`/`list`/`rm`, `holler` has a lookup-and-surface command shape none of the other tools need.

## Quickstart

```sh
cargo install holler
holler add John Smith phone:555-123-4567 email:john@example.com
holler add Jane Doe +family phone:555-987-6543
holler              # everyone, sorted
holler email John   # john@example.com
holler phone Jane   # 555-987-6543
```

## Installation

**Homebrew** (macOS/Linux):
```sh
brew tap thombruce/tap
brew install thombruce/tap/holler
```

**crates.io** (requires Rust):
```sh
cargo install holler
```

**From source** (from the repo root):
```sh
cargo install --path crates/holler
```

## Usage

```sh
holler                                  # everyone, sorted (default: holler list)
holler add Jane Doe                     # bare name, no details yet
holler add John Smith phone:555-123-4567 email:john@example.com
holler add Alex Kim +work @conference   # tags live inline in the name
holler list                             # everyone, sorted (alias for bare invocation)
holler list +family                     # only contacts tagged +family
holler phone John                       # phone number(s) for contacts matching "John"
holler email John                       # email address(es), same matching rule
holler rm 2                             # remove contact #2 as shown by `list`
holler edit                             # open the contacts file in $EDITOR
holler completions <shell>              # print a shell completion script
holler man                              # print the man page (roff) to stdout
```

`phone`/`email` match case-insensitively against a substring of the contact's name. A single match prints the bare value(s) — one per line if there's more than one — so it's pipe-friendly (`holler email John | xargs open`, say). Multiple matches print `Name: value` per line so you can tell them apart; a contact with nothing on file for that field shows as `Name: (no phone)` in a multi-match list, or a plain message if it's the only match. No match at all is an error.

Contacts are always listed alphabetically by name, regardless of file order — the only universal field, unlike the other tools' domain-specific sort keys (chronological, status/priority/price). `list` also takes `+project`/`@context`/`key:value` tag queries the same way bagg/calchemy's `list` does, matched against whatever's embedded in the name.

## Storage format

The contacts file is plaintext, one contact per line, hand-editable:

```
Alex Kim +work @conference
Jane Doe +family phone:555-987-6543
John Smith phone:555-123-4567 email:john@example.com
```

Each line is `NAME [phone:VALUE]... [email:VALUE]...` — no delimiter, todo.txt-native like calchemy/bagg:

- `phone:VALUE` / `email:VALUE` — reserved `key:value` tags (key matched case-insensitively), extracted from anywhere in the line into the contact's phone/email lists. Multiple of each are allowed (mobile + home, work + personal) and are kept in the order they're written.
- `NAME` is everything else, joined back together — plain words and any *unreserved* tag alike (`+project`, `@context`, other `key:value` pairs). Unreserved tags get no special handling: they're indistinguishable from plain name text to the parser, so they're never repositioned — a tag stays exactly where you typed it, the same as bagg/calchemy never reposition their own tags.

Because `phone:`/`email:` are found by shape rather than position, a phone number must be written as a single token — `phone:555-123-4567`, not `phone:555 123 4567` — since this format (like the rest of the family) splits on whitespace and has no quoting/escaping scheme. A `key:value` token only counts as a tag at all if its key starts with a letter, which is what keeps stray colon-containing text from being misread.

Accepted tradeoff, lowest-risk in this family so far: a name that itself contains a literal `phone:`/`email:`-shaped substring would misparse — rare in practice, since ordinary names essentially never contain colons.

## Configuration

Create `~/.config/holler/holler.toml`:

```toml
path = "/path/to/holler.txt"
```

Without config, contacts live at `~/.local/share/holler/holler.txt` (or the platform equivalent).

## Not yet supported

Filtering `list`/`phone`/`email` lookups *by* phone or email content (they're extracted out of the name specifically for retrieval, not indexed for search), a detail view beyond the raw line, and JSON output are deliberately left out of this first version.

## License

MIT
