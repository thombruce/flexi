# bagg

A minimal plaintext shopping list / wishlist for the command line — todo.txt for your cart.

Part of the [flexi](../../) family of plaintext-storage tools; where calchemy keeps a sortable list of dated appointments, bagg keeps a sortable, hand-editable list of things to buy.

## Quickstart

```sh
cargo install bagg
bagg add --price 12.99 --priority A "Widget thing"
bagg add Eggs
bagg              # everything, sorted
bagg got 1        # mark item 1 as got (toggles back if run again)
bagg rm 2         # remove item 2
```

## Installation

**Homebrew** (macOS/Linux):
```sh
brew tap thombruce/tap
brew install thombruce/tap/bagg
```

**crates.io** (requires Rust):
```sh
cargo install bagg
```

**From source** (from the repo root):
```sh
cargo install --path crates/bagg
```

## Usage

```sh
bagg                                   # everything, sorted (default: bagg list)
bagg add Eggs                          # plain item, qty 1
bagg add --qty 3 Eggs                  # qty 3
bagg add --price 12.99 "Widget thing"  # priced item
bagg add --priority A --price 12.99 --qty 2 "Widget thing"
bagg list                              # everything, sorted (alias for bare invocation)
bagg list --pending                    # only not-got items
bagg list --got                        # only got items
bagg got 2                             # toggle got/not-got for item #2 as shown by `list`
bagg rm 2                              # remove item #2 as shown by `list`
bagg edit                              # open the list file in $EDITOR
bagg completions <shell>                # print a shell completion script
bagg man                               # print the man page (roff) to stdout
```

Items are always listed sorted, regardless of their order in the file: not-got before got, then priority `A` before `B` before none, then cheapest price before priciest before unpriced. `list --pending`/`--got` filter that same sorted listing without renumbering it — an item keeps the index it has in the full list even when a filter hides its neighbors, so `got`/`rm` always target what you expect.

## Storage format

The list is a plaintext file, one item per line, hand-editable:

```
x (A) 12.99 Widget thing x2
(B) Cheap thing, price unknown
5.00 Coffee — Tesco
Eggs
x Got already
```

Each line is `[x ][(A) ][PRICE ]NAME[ xQTY]`:

- A leading `x` marks the item as got (todo.txt convention); absent means not-got. No placeholder token is needed for "not got".
- `(A)`–`(Z)` is an optional priority. Unbounded — there's no fixed set of levels, and no priority sorts after any lettered one.
- `PRICE` is an optional decimal amount, always written with a `.` (e.g. `12.99`, `0.99`) — that's what lets it be told apart from a bare integer, which is never a price.
- `NAME` is required free text — a URL, a store name, a note, all just live in the name itself.
- A trailing `xN` tag (e.g. `x3`) is the desired quantity; omitted entirely when it's `1`, so the common case (`Eggs`) stays a single plain word.

Unlike calchemy's `DATE ... # TITLE`, there's no `#` delimiter — bagg's format is fully todo.txt-native: leading status/priority markers, a trailing inline tag. This means parsing leans on token shape at both ends of the line: a name that starts with a bare `12.99`-shaped word, or ends in a word shaped like `x3`, will be misread as a price or quantity tag rather than plain text. This is a deliberate, accepted tradeoff for a shopping list (rare in practice) rather than something worth an escaping scheme.

## Configuration

Create `~/.config/bagg/bagg.toml`:

```toml
path = "/path/to/bagg.txt"
```

Without config, the list lives at `~/.local/share/bagg/bagg.txt` (or the platform equivalent).

## Not yet supported

Notes/links as structured fields (they live in the free-text name instead), categories/tags, and JSON output are deliberately left out of this first version.

## License

MIT
