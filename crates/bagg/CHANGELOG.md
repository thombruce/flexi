# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-10-18

### Added

- Lines may start with `!` (buying this trip) or `~` (decided not to buy) as well as `x`, at most one per line. Items sort `!`, wanted, got, then `~`. `--pending` includes `!` items.
- `bagg focus <n>` and `bagg wont <n>` toggle an item's "buying this trip" (`!`) or "won't buy" (`~`) status; running one again sets it back to wanted. `list --wont` shows only won't-buy items.
- `list` tag queries now also match `#hashtag` tags (a `#` followed by a word starting with a letter, e.g. `#urgent`; `#42` isn't one). Quote it on the command line (`list '#urgent'`), or the shell treats it as a comment.

### Changed

- A line that already starts with a bare `!` or `~` word now reads it as a status marker rather than part of the name. For example, `~ Gadget` becomes a won't-buy item named `Gadget`. Write `!`/`~` attached to a word to keep it in the name.

## [0.4.0] - 2026-10-18

### Changed

- **Breaking:** `edit` now runs `$EDITOR` through `sh`, as git does. It may now carry arguments (e.g. `code -w`), but an editor path containing spaces must be quoted inside `$EDITOR`; an unquoted one that used to work now fails. `edit` now needs `sh`, so it no longer works on Windows.
- `edit` reports an error when the editor can't be run (exit 126/127, e.g. a typo), and only warns for any other non-zero exit. It also creates the data directory first, so editing before the first entry works.

### Fixed

- Data-file rewrites are synced to disk before the atomic rename, so a crash can't leave the file empty or truncated.
- Adding an entry to a hand-edited file that lacks a final newline no longer glues the new line onto the last one.

## [0.3.0] - 2026-07-26

### Added

- Locale-aware price formatting: new `decimal_separator` (default `.`) and `decimal_places` (default `2`, must be `<= 9`) config keys let PRICE match your own convention (e.g. `12,99`, or a whole-unit `1500` for JPY-style currencies with `decimal_places = 0`). Invalid separators (alphanumeric, whitespace, or `( ) ?`) and out-of-range decimal places are rejected at config load.
- A `?` placeholder for "price deliberately unspecified" — occupies the PRICE slot without being a real value. Relevant whenever a priceless item's name opens with a PRICE-shaped word under the current locale config (not just `decimal_places = 0`'s bare-integer case — the default config has the same risk for a name that's genuinely decimal-shaped, e.g. "1.99 and the Psychology of Pricing"); `to_line` auto-inserts `?` whenever omitting it would make the name misparse on the next read, and leaves the common case (`Eggs`) untouched otherwise. `--price ?` is also accepted on `add`. `?` is unconditionally the marker in leading position, the same way PRICE/QTY are unconditional in theirs — a name that itself genuinely opens with a literal `?` loses that character on parse, the same accepted-tradeoff tier as the other shape collisions, not something engineered around.
- `always_show_unspecified_price` config key (default `false`) to always show `?` on priceless items rather than only when needed to disambiguate — works under any locale config, so a price can be deliberately marked unknown even with a decimal currency.

## [0.2.0] - 2026-07-26

### Added

- `bagg list` now takes `+project`/`@context`/`key:value` tag queries (e.g. `bagg list +kitchen-reno`), matched case-insensitively against tags embedded in item names; multiple queries match if an item carries ANY of them. Tags aren't a stored field — they're parsed on the fly from the existing free-text `NAME`, same convention todo.txt uses, and don't affect sort order.

## [0.1.0] - 2026-07-26

### Added

- Initial release. A plaintext shopping list / wishlist, one item per line: `[x ][(A) ][PRICE ]NAME[ xQTY]` (e.g. `x (A) 12.99 Widget thing x2`), sorted by got-status, then priority, then price, and hand-editable.
- `bagg add [--price 12.99] [--qty N] [--priority A-Z] <name...>` adds an item; quantity defaults to 1 and is omitted from the line when 1.
- Bare `bagg` and `bagg list` show every item, sorted; `--pending`/`--got` filter the view without renumbering.
- `bagg got <n>` toggles got/not-got status for the item at position `n` in `list`.
- `bagg rm <n>` (alias `remove`) removes the item at position `n` in `list`; `bagg edit` opens the file in `$EDITOR`.
- `bagg completions <shell>` and `bagg man`.
- Config at `~/.config/bagg/bagg.toml`: `path`.
