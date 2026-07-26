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
bagg list +kitchen-reno                # only items tagged +kitchen-reno
bagg got 2                             # toggle got/not-got for item #2 as shown by `list`
bagg rm 2                              # remove item #2 as shown by `list`
bagg edit                              # open the list file in $EDITOR
bagg completions <shell>                # print a shell completion script
bagg man                               # print the man page (roff) to stdout
```

Items are always listed sorted, regardless of their order in the file: not-got before got, then priority `A` before `B` before none, then cheapest price before priciest before unpriced. `list --pending`/`--got` filter that same sorted listing without renumbering it — an item keeps the index it has in the full list even when a filter hides its neighbors, so `got`/`rm` always target what you expect.

`list` also takes `+project`, `@context`, or `key:value` tag queries (e.g. `bagg list +kitchen-reno @tesco`), matched case-insensitively against tags embedded anywhere in an item's name — multiple queries match if the item carries any of them. Tags aren't a separate stored field; they're plain words in `NAME` (`Screws +kitchen-reno`), parsed on the fly when filtering, todo.txt's own convention. This is also the answer to "how do I group related items" (e.g. several parts for one project) — tag them all with the same `+project`, no nesting needed.

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
- `PRICE` is an optional decimal amount, or the literal `?` for "deliberately unspecified". By default it's written with a `.` and 2 decimal places (e.g. `12.99`) — that's what lets it be told apart from a bare integer, which is never a price. Both the separator and the decimal-place count are configurable (see Configuration) for other locales/currencies.
- `NAME` is required free text — a URL, a store name, a note, all just live in the name itself.
- A trailing `xN` tag (e.g. `x3`) is the desired quantity; omitted entirely when it's `1`, so the common case (`Eggs`) stays a single plain word.

Unlike calchemy's `DATE ... # TITLE`, there's no `#` delimiter — bagg's format is fully todo.txt-native: leading status/priority markers, a trailing inline tag. This means parsing leans on token shape at both ends of the line: a name that starts with a bare price-shaped word, or ends in a word shaped like `x3`, will be misread as a price or quantity tag rather than plain text. This is a deliberate, accepted tradeoff for a shopping list (rare in practice) rather than something worth an escaping scheme.

### Price placeholder and locale config

Currency isn't tracked at all (no symbol/code) — `PRICE` is just a bare number in your own convention. Two config keys adjust its shape:

```toml
decimal_separator = ","   # default "."
decimal_places = 0        # default 2 — set 0 for whole-unit currencies like JPY
```

A name whose leading word happens to fully match the current price shape collides with PRICE — this isn't limited to `decimal_places = 0`; the default config has the same risk for a name that genuinely opens with a decimal-shaped word (a book titled starting with "1.99...", say). `decimal_places = 0` just makes the shape a bare integer, so it collides with any plain number-leading name ("4 slice toaster") too, which is why it's the config most likely to actually hit this in practice.

This is where the literal `?` placeholder comes in: it occupies the PRICE slot to mean "unspecified" without being a real value, so `? 4 slice toaster` (or `? 1.99 and the Psychology of Pricing`) parses correctly under any config. You rarely need to type it yourself — `to_line` (what `add`/`got`/`rm` all write through) automatically inserts `?` whenever omitting it would make the name misparse as a price on the next read, and leaves priceless items exactly as clean as before (`Eggs` stays `Eggs`) when there's no risk of collision. Set `always_show_unspecified_price = true` if you'd simply prefer every priceless item to visibly carry `?`, not just the ones that need it — this works under any locale config, not just `decimal_places = 0`, so you can always deliberately mark a price unknown even with a decimal currency.

Note two things about scope: first, this only ever bites a **priceless** item — as soon as an actual price is present, it always claims exactly the first token, so `bagg add --price 1500 "4 slice toaster"` is unambiguous regardless of config. Second, `?` itself is unconditionally the marker in leading position, the same way PRICE and QTY are unconditional in their positions — so a name that itself genuinely opens with a literal `?` loses that character on parse, the same class of accepted tradeoff as the price/qty ones above, not something engineered around.

## Configuration

Create `~/.config/bagg/bagg.toml`:

```toml
path = "/path/to/bagg.txt"
decimal_separator = "."    # default "."
decimal_places = 2         # default 2
always_show_unspecified_price = false   # default false
```

Without config, the list lives at `~/.local/share/bagg/bagg.txt` (or the platform equivalent).

## Not yet supported

Notes/links as structured fields (they live in the free-text name instead, alongside `+project`/`@context`/`key:value` tags) and JSON output are deliberately left out of this first version.

## License

MIT
