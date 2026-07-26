# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Locale-aware price formatting: new `decimal_separator` (default `.`) and `decimal_places` (default `2`, must be `<= 9`) config keys let PRICE match your own convention (e.g. `12,99`, or a whole-unit `1500` for JPY-style currencies with `decimal_places = 0`). Invalid separators (alphanumeric, whitespace, or `( ) ?`) and out-of-range decimal places are rejected at config load.
- A `?` placeholder for "price deliberately unspecified" — occupies the PRICE slot without being a real value. Only relevant with `decimal_places = 0`, the one config where a priceless item whose name opens with a bare number (`4 slice toaster`) would otherwise misparse as a priced item; `to_line` now auto-inserts `?` whenever omitting it would make the name misparse on the next read, and leaves the common case (`Eggs`) untouched otherwise. `--price ?` is also accepted on `add`. Under any other `decimal_places`, `?` is never recognized or written — a name genuinely starting with `?` is left untouched rather than silently eaten.
- `always_show_unspecified_price` config key (default `false`) to always show `?` on priceless items rather than only when needed to disambiguate; only takes effect under `decimal_places = 0`, same as the placeholder itself.

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
