# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workspace layout

Cargo workspace (`Cargo.toml` at root, `members = ["crates/*"]`). Each tool is its own crate under `crates/`:

- `crates/flexi` — track a running flexi-time balance.
- `crates/clocking` — log working hours as discrete clock-in/clock-out sessions.
- `crates/calchemy` — a plaintext calendar of dated appointments.
- `crates/bagg` — a plaintext shopping list / wishlist.
- `crates/holler` — a plaintext contacts / rolodex.
- `crates/toodles` — a plaintext todo list (todo.txt + due date).
- `crates/foolscap` — library crate (no binary): the plumbing all six tools share, growing into a set of opt-in building blocks (see **The `foolscap` crate** below).

All six share the same plaintext-file + XDG-config approach. What's copied vs. divergent:

- `config.rs` — each crate keeps its own `RawConfig`/`ResolvedConfig` structs and `resolve()` validation, but loads via `foolscap::load_config::<RawConfig>("<tool>")` and defaults the data path via `foolscap::default_data_path("<tool>")`.
- `time.rs` (duration parse/format/round) is shared by flexi and clocking only; **calchemy, bagg, holler, and toodles have no `time.rs`** — calchemy and toodles parse dates via chrono rather than durations, and neither bagg nor holler have a time dimension at all.
- `storage.rs` is **not** shared — each crate's storage semantics differ: flexi keeps a running-balance chain (tail = state via `new_minutes()`); clocking records session durations summed over a period (`session_minutes()`); calchemy holds an unordered set of `Appt` records parsed and sorted by datetime, with `add`/`rm`/`edit` rewriting arbitrary lines rather than appending to a derived chain; bagg holds an unordered set of `Item` records parsed and sorted by status/priority/price (same shape as calchemy's set-of-records model, different sort key and no dates); holler holds an unordered set of `Contact` records with no positional grammar at all beyond extraction — sorted alphabetically by name, the only universal field; toodles holds an unordered set of `Todo` records — bagg's model with dates in place of price/qty.

**`foolscap`** holds only the plumbing that was byte-identical across all six crates: `xdg_config_dir`/`xdg_data_dir`, `default_data_path`, `load_config::<T>`, `read_lines` (non-blank lines), `append_line` (repairs a missing final newline before appending), atomic `write_lines` (writes `<file>.tmp`, appended rather than replacing the extension, then renames), `remove_line`/`replace_line`, and `open_in_editor` (runs `$EDITOR` via `sh -c '<editor> "$@"'` like git, so args and quoted paths work; exit 126/127 — editor couldn't be run — is an error, any other non-zero exit only warns, since the file is already saved). It was extracted once toodles became the sixth tool, and a review found the same append bug in all six copies. Before that it was deliberately deferred: the shared surface is small, and the shared crate must be **published to crates.io before any dependent bin can publish**. Keep it minimal. Per-crate config structs, validation, and storage semantics stay in each crate. Set-of-records crates (calchemy, bagg, holler, toodles) re-export the line primitives they use from `storage.rs` (`pub use foolscap::{append_line, ...}`), so `main.rs` still calls `storage::*`. `open_in_editor` is called directly from each `main.rs` (it isn't storage). flexi/clocking wrap the line primitives in `append_log`/`read_log`/`pop_log`/`last_entry`, so every log read goes through `read_lines`. `time.rs` is still copied (flexi/clocking only). Building blocks beyond plumbing (so far `foolscap::tags`) are covered in the next section.

## The `foolscap` crate: direction

**Committed direction (#53):** foolscap grows from shared plumbing into a library of **opt-in building blocks** for plaintext, one-record-per-line CLI tools. It is **never a line grammar.** Each tool keeps its own line shape (DATE first in calchemy, PRICE in bagg, no fixed positions in holler) and calls only the blocks it needs, then parses its own remainder.

Planned blocks, each tracked as a sub-issue of #53:

- **Tags** (#54, **done**): `foolscap::tags`, `+project`/`@context`/`key:value` in free text, used by bagg, calchemy, holler, toodles.
- **`#hashtag`** (#55): a new tag kind in the tag family.
- **Prefix markers** (#56): leading `x` done and `(A)` priority (bagg, toodles), later `~`/`!` (#41).
- **Dates** (#57): strict `YYYY-MM-DD`, behind a `dates` cargo feature; the family rule is decided in #52.
- **Note** (#58): the ` # note` suffix (flexi, clocking). It's opt-in and separate from the description/tags block.

`# note` and `#hashtag` coexist by shape: a note starts at a **bare** `#` token, while a hashtag is `#` immediately followed by a word. Never make a bare `#` a tag, or `#word` a note.

Rules for adding to foolscap:

- **Extract only once real tools share it.** Two or more real users, no speculative blocks. Each block lands in foolscap and replaces the copies in the same change.
- **Opt-in, and pay only for what you use.** A tool calls a block or doesn't. Blocks with heavy dependencies sit behind a cargo feature, so bagg/holler never pull in chrono.
- **Tool-agnostic API.** Blocks take tokens or strings and return plain values, with no knowledge of which tool is calling and no tool-specific types or config.
- **Still internal.** The README says foolscap isn't for general use, and that stays true until #59 (whether it *can* and *should* become a public library) is answered. Design blocks as if they might be public, but don't promise stability yet.
- **Release order:** a foolscap release must be on crates.io before any tool that needs it (see Releases).

## The `toodles` crate

`toodles` (`main.rs` + `config.rs` + `storage.rs`, clap CLI, tags via `foolscap::tags`) is a plaintext todo list — the family's own todo.txt, reduced from the earlier standalone Toodles project (issue #40). A line is `[x ][(A) ][CLOSED ][CREATED ][DUE ]TITLE`, parsed into `storage::Todo { done, priority, closed, created, due: Option<NaiveDate>, title, raw }`. todo.txt's two dates plus a third, **positional** due date: open lines take up to two leading dates (created, then due), done lines up to three (closed, created, due) — the date *count* is what disambiguates, so due requires created. `add` always stamps created (today), and `done <n>` toggles status in place via `replace_line`, stamping `closed` on completion and clearing it on reopen, so the CLI can never produce an unrepresentable line — `add` also refuses a date-leading title without `--due` (it would otherwise land in the due slot on re-read). `storage::parse_date` requires length 10, since chrono alone accepts unpadded `2024-9-1`. `to_line` writes dates in order and stops at the first absent one (`map_while`). Positional due was chosen over the community `due:` tag because core sort fields in this family are positional/shape-parsed; other todo.txt tools just see the due date as description text. Priority is **kept** on completion (`x (A) ...`), matching bagg rather than todo.txt.

Structurally this is bagg with dates in place of price/qty: same leading `x`/`(A)` markers, same full-sorted-list indexing for `done`/`rm` (filtered `--open`/`--done`/tag views never renumber), same `remove_line`/`replace_line` primitives (from `foolscap`, re-exported by `storage.rs`). Sort key (`Todo::sort_key()`) is **status → priority → due** (absent priority/due last), stable on file order. `list` shows open todos past due in red. Accepted tradeoff (calchemy's): a title opening with a date-shaped word, on a line with fewer dates than its max, has it claimed as a date. Config is `path` only. Deliberately out of scope vs. the old Toodles: nesting (breaks one-line-per-record; `+project` groups instead), price/multiplier (bagg), `#hashtags` (clash with ` # ` notes), Markdown. Deferred: `!`/`~` focus/obsolete markers (#41), `every:` recurrence (#42), `time:`/`count:`, natural-language dates, JSON.

## The `holler` crate

`holler` (`main.rs` + `config.rs` + `storage.rs`, clap CLI, tags via `foolscap::tags`) is a plaintext contacts / rolodex. A line is `NAME [phone:VALUE]... [email:VALUE]...` (e.g. `John Smith phone:555-123-4567 email:john@example.com`), parsed into `storage::Contact { name: String, phone: Vec<String>, email: Vec<String>, raw: String }`. This is the first crate in the family with **no positional grammar at all** beyond extraction — there's no DATE-always-first (calchemy) or PRICE-then-name-then-qty ordering (bagg), because `phone:`/`email:` (key matched case-insensitively) are found anywhere in the line via a small local `reserved_tag` helper (not `foolscap::tags`'s private `extract_tag`, which returns a combined lowercased string rather than the split key/value this needs), extracted into the `Vec`s in encounter order, and everything else — plain words and any *unreserved* tag (`+project`, `@context`, other `key:value` pairs) alike — is rejoined into `name` untouched. Unreserved tags get zero special handling: the parser can't distinguish one from a plain word, so unlike `phone`/`email` they're never repositioned, mirroring how bagg/calchemy never reposition their own tags either.

This design fell out of a real constraint: a phone number's natural human formatting (`+1 555 123 4567`) spans multiple whitespace tokens, which nothing else in this family's shape-based parsing has had to handle (DATE/PRICE/status are always single tokens). Resolved by convention — phone must be written as one token (`555-123-4567`) — rather than inventing a delimiter/quoting scheme, consistent with calchemy/bagg both having moved *away* from delimiters. Once phone is single-token, it (and email, already single-token) stop needing positional anchoring at all.

Extraction, not filter-only: this is a deliberate departure from bagg/calchemy's `foolscap::tags` model, where a tag stays embedded in the free text forever since nobody needs to *retrieve* its value, only filter on it. holler's whole point is retrieval (look up a phone number), so `phone:`/`email:` get pulled into first-class `Vec<String>` fields (cardinality: multiple allowed per contact — mobile + home, work + personal) while everything else stays inline, exactly like bagg/calchemy's tags. `to_line()` renders `name` (tags and all, wherever the user put them) first, then every phone, then every email — the *only* thing ever canonicalized to a fixed position is the extracted fields, chosen because name is the sort/lookup key and matches how any real contact list puts identity before details; nothing inside `name` itself is ever reordered.

Accepted tradeoff, lowest-risk collision in the family so far: a name containing a literal `phone:`/`email:`-shaped substring would misparse — contact names essentially never contain colons, rarer in practice than "an item starts with a bare number" (bagg) or "a title starts with a time" (calchemy). `Contact::sort_key()` is alphabetical by name (case-insensitive) — the only universal field, unlike the other crates' domain-specific keys. Commands: bare/`list` (+ tag filtering via `foolscap::tags`, same as bagg/calchemy) shows everyone sorted, `rm <n>` indexes the full sorted list (same index-stability principle as bagg over calchemy's filtered-view `rm`), `edit`, `completions`, `man` — plus the naming-driven addition, `phone <query>`/`email <query>`: case-insensitive substring match against `name`; a single match prints the bare value(s) (pipe-friendly), multiple matches print `Name: value` per line to disambiguate, no match errors. Config is `path` only — no format-specific keys needed. Deferred (not built): filtering `list`/lookups *by* phone/email content (extracted out of `name`, so the generic tag filter can't see them), a detail view beyond the raw line, JSON.

## The `bagg` crate

`bagg` (`main.rs` + `config.rs` + `storage.rs`, clap CLI, tags via `foolscap::tags`) is a plaintext shopping list / wishlist. A line is `[x ][(A) ][PRICE ]NAME[ xQTY]` (e.g. `x (A) 12.99 Widget thing x2`), parsed into `storage::Item { got: bool, priority: Option<char>, price: Option<i32>, qty: u32, name: String, raw: String }`. Unlike flexi/clocking, there is **no `#` delimiter** (calchemy has since dropped it too, see below) — the format is fully todo.txt-native (leading status/priority markers, a trailing inline tag), so `Item::parse` leans entirely on token shape rather than a split point: a leading `x` token is status (got), a leading `(A)`–`(Z)` token is priority, a leading price-shaped token is price, and a trailing `xN` token is quantity (default `1`, and `to_line` omits the tag entirely when qty is `1` — so `Eggs` round-trips as a single plain word). Everything remaining is the name.

Price shape is **configurable**, not hardcoded: `storage::PriceFormat { separator: char, places: u32 }` (defaults `.`/`2`, matching the original behavior) drives both `parse_price`/`format_price`, built once in `main()` from `cfg.decimal_separator`/`cfg.decimal_places` and threaded through every call site that touches a price (`Item::parse`, `Item::to_line`, `all_sorted`, `find_item`, `run_list`, and the `Add` handler's own price-argument parsing, which is deferred out of clap's `value_parser` into the handler body specifically because config isn't loaded yet at CLI-parse time). With `places > 0` a price requires the separator (`\d+<sep>\d{places}`); with `places == 0` a price is a bare integer. Either way, a NAME whose leading token happens to fully match that shape collides with PRICE — this is the *original* bagg tradeoff (a name starting with an exact `X.YY`-shaped word was always claimed as PRICE, from the very first version, well before locale config existed), not something specific to `places == 0`. `places == 0` just makes the collision shape a bare integer, which many more ordinary names start with in practice, so it's the config most likely to actually hit it — but the underlying mechanism (and the fix) is identical for any `places`.

The fix is `storage::UNSPECIFIED_PRICE` (`"?"`) — an explicit placeholder that occupies the PRICE slot without being a real value (chosen partly because ASCII `?` (63) sorts after all digits (48–57) under plain `sort`, matching the existing `price.unwrap_or(i32::MAX)` sort-last rule for absent prices), recognized and (potentially) written **for any `PriceFormat`**, not gated to `places == 0` — an earlier version of this feature gated it that way and was wrong to: gating broke the case a decimal-currency user still wants (deliberately marking a price unspecified via `?`/`always_show_unspecified_price` even though `places > 0`), while not actually eliminating the underlying collision, which was never places-specific to begin with. `Item` gains no new field for the placeholder — `to_line` is shape-aware instead: when `price` is `None`, it checks whether `name`'s leading token would itself parse as PRICE-shaped under the current config, and only then auto-emits `?` (or always, if `cfg.always_show_unspecified_price` is set) — otherwise it stays omitted exactly as before (`Eggs` stays `Eggs`). This is what keeps the file self-healing across edits: a hand-typed or CLI-supplied `?` need not be remembered anywhere, since the next `to_line()` call re-derives whether one is needed from the name's shape alone. A real price always claims exactly the first leading token regardless of config, so a collision is only ever live when no price is given at all — narrower than "names can't start with a number."

`?` itself is unconditionally the marker in leading position — the same way PRICE and QTY tokens are unconditional in their positions — so a NAME that itself genuinely opens with a literal `?` loses that character on parse, for any `PriceFormat`. This is accepted and documented as the same tier of tradeoff as the price/qty shape collisions, not engineered around; there is no signal in the plain-text format that could distinguish "the marker" from "a name that happens to start with `?`" (the digit-collision case is analogous, but scoped to when `places == 0` in particular, since that's what makes a leading bare-integer name collide).

`config::resolve()` bounds `decimal_places` to `<= 9` (so `10i32.pow(places)` can't overflow `i32`, which would otherwise panic in debug or divide-by-zero in `format_price` in release) and rejects whitespace as `decimal_separator` (tokens are already whitespace-split before `parse_price` ever runs, so a space separator would silently disable all price parsing rather than erroring).

Like calchemy, `bagg` is a mutable set rewritten via atomic `.tmp`+rename (`read_items` plus `foolscap`'s `append_line`/`remove_line`/`replace_line`, re-exported from `storage.rs`), not an append-only chain — items get their price edited, status toggled, and get removed. Sort key (`Item::sort_key()`) is **status → priority → price**, ascending, matching the field order in the format itself: not-got before got, then priority `A`..`Z` (absent sorts last), then price cheap-first (absent sorts last), stable on insertion order for ties. `list` (also the bare-invocation default) always shows the **full** sorted listing; `--pending`/`--got` filter that same listing for display without renumbering it, and `got <n>`/`rm <n>` always index against the identical full-sorted computation `list` uses — this is deliberately more conservative than calchemy's `rm`, which indexes into a filtered (`upcoming()`) view; bagg's status-toggling makes that kind of view-drift more likely, so mutating commands never index into a subset. `got <n>` toggles status in place via `replace_line` (from `foolscap`; calchemy doesn't re-export it, since calchemy never mutates a line in place — only appends or removes whole lines). No dates, no `time.rs`, no notes/links as structured fields (they live in the free-text `NAME`) — deliberately deferred pending real need. `list` does support `+project`/`@context`/`key:value` tag filtering (see Tag filtering below) — these aren't a stored struct field, just parsed on the fly out of `NAME`.

The rest of this file describes the `flexi` crate (`crates/flexi/`). The `clocking` crate mirrors flexi's module layout (`main.rs` + `config.rs` + `storage.rs` + `time.rs`, clap CLI, `@in` open-marker session model) with a leaner command set: `in`/`out`, `log`/`summary` (with the same date filters), `edit`, `undo`, `completions`, `man`. It has no balance, so no `add`/`remove`/`set`/`reset`, no clipboard, no json/prose.

clocking's closed-session line is `<clock-in> <end> = <duration>` (e.g. `2026-07-11 09:00 17:00 = 8 hr`), **keyed by the clock-in time** — the leading fixed-width timestamp is the start, so date filters bucket a shift by its start day (a shift past midnight counts on the day it began). The end field is a bare `HH:MM` for a same-day shift, or a full `YYYY-MM-DD HH:MM` when it crosses midnight; in `full` timestamp mode both endpoints are always full. `= <duration>` is the recorded (increment-rounded) worked time and is authoritative — the end field is never parsed back for logic, only displayed, so its shape is purely cosmetic. `out` computes the end field, pops the `@in` marker, and appends the closed line stamped with the marker's (clock-in) timestamp via `storage::append_log(path, timestamp, desc)` (which takes an explicit leading timestamp; `storage::now_timestamp` formats "now" for the `@in` marker). `session_minutes()` parses the duration after ` = `; `summary` sums it. There is no backward compatibility with the 0.1.0 `<clock-out> <duration> (<span>)` format.

The `calchemy` crate (`main.rs` + `config.rs` + `storage.rs`, clap CLI, tags via `foolscap::tags`) is a plaintext calendar. A line is `DATE [START [END]] TITLE` (e.g. `2026-07-14 09:00 10:00 Dentist`), parsed into `storage::Appt { date, start: Option<NaiveTime>, end: Option<NaiveDateTime>, title, raw }`. `START` absent = all-day; `END` is `HH:MM` (same day) or `YYYY-MM-DD HH:MM` (crosses to a later day) — the same cross-day rule clocking uses, and `add` rolls an end `<= start` to the next day. `add` also accepts an explicit timed later-day end (`HH:MM YYYY-MM-DD HH:MM`, claimed only when the full time-date-time triple parses; end must be after start). With no `START`, a bare `YYYY-MM-DD` END is a multi-day all-day event (inclusive last day, stored as that date at `NaiveTime::MIN`; must be after `DATE`, enforced in both `Appt::parse` and `add`). Day filters are span-aware via `Appt::last_date()` (`end`'s date, else `date`): an appointment appears on every day in `[date, last_date]`, so window checks are overlap tests (`last_date >= since && date <= until`), the default upcoming view keeps ongoing events (`last_date >= today`), and `--past` means `last_date < today`. **No `#` delimiter** (dropped in favor of bagg's todo.txt-native shape-parsing — see the bagg paragraph below): `Appt::parse` matches the longest shape-valid prefix of `tokens[1..]` (`TIME DATE TIME` → `TIME TIME` → `TIME` alone → `DATE` alone → none), consuming that many tokens as `START`/`END`, and joins whatever's left as `TITLE`. Accepted tradeoff: a title opening with well-formed date/time-shaped words (e.g. "09:00 sync") can have them misclaimed as `START`/`END`; old files written with ` # ` still parse without error, the `#` just becomes a stray leading character in the title until re-saved. Unlike the append-only logs, the file is an **unordered set**: `read_appts` parses every line and callers sort by `Appt::start_dt()` (all-day sorts first via `NaiveTime::MIN`); `add` appends, `rm` deletes an arbitrary line (`remove_line`, atomic rewrite preserving file order) indexed by position in the default upcoming view, `edit` hand-edits. Commands: bare/`today` = today's agenda, `next` = soonest upcoming, `list`/`agenda` (+ `--today`/`--week`/`--month`/`--since`/`--until`/`--all`/`--past`), `week`, `rm`, `edit`, `completions`, `man`. Note the divergence: `--week`/`--month` mean the **full** calendar week/month (forward-looking), not "up to today" as in flexi/clocking. Config is `path` + `week_start` only. `list` supports `+project`/`@context`/`key:value` tag filtering, same as bagg (see Tag filtering below). Deferred (not built): recurrence, natural-language dates, undo, JSON.

## Tag filtering (`foolscap::tags`, bagg + calchemy + holler + toodles)

bagg, calchemy, holler, and toodles' `list` all take trailing `+project`/`@context`/`key:value` tag queries (e.g. `bagg list +kitchen-reno`, `calchemy list @clinic`), matched case-insensitively, OR'd across multiple queries. This directly answers "how do I group related bagg items" (several parts for one project) — a shared `+project` tag is the grouping mechanism; nesting/indentation was considered and rejected since it breaks the one-line-per-record model these crates share.

`foolscap::tags` (`crates/foolscap/src/tags.rs`) is the single copy; each crate imports it with `use foolscap::tags;` and calls `tags::matches`. It's read-only and works entirely off the existing free-text field (`NAME`/`TITLE`, or the untouched portion of holler's `name`): `extract_tag` classifies a single whitespace token as a tag if it starts with `+`/`@` (len > 1), or if it's a `key:value` pair whose key starts with a letter — that last condition specifically excludes a bare numeric token (a clock time like `10:00` in a calchemy title) from being misread as a tag, a real collision risk raised and resolved before implementation. `matches(text, queries)` extracts all tags from `text` and checks whether any query is present (case-folded); an empty query list matches everything. No struct field, no format change, no round-trip risk — tags are computed at query time only, and don't affect sort order in any of them. (holler additionally reserves `phone`/`email` keys for its own extraction, via a separate local helper in `storage.rs`, not `tags.rs` — see the holler section above.)

## Commands

Cargo commands run from the repo root operate on the whole workspace; use `-p flexi` to scope to one crate.

```bash
cargo build          # compile all crates
cargo run -p flexi   # run flexi (display balance)
cargo run -p flexi -- add 1 hr 30 min
cargo run -p flexi -- add 1 hr 30 min --note "reason"
cargo run -p flexi -- remove 30 min
cargo test           # run all tests
cargo test time      # run tests in a specific module
cargo clippy         # lint (CI runs with -D warnings — fix all warnings before committing)
```

## Architecture

`flexi` is a single binary crate at `crates/flexi/` (`src/`, `tests/`). All state is `i32` minutes internally; `time.rs` owns the boundary between minutes and human-readable strings.

**Data flow for `add`/`remove`:**
`main.rs` joins `Vec<String>` args → `time::parse_duration` → arithmetic → `storage::append_log` → print delta.

**Modules:**
- `time.rs` — `parse_duration(s) -> i32`, `format_duration(i32) -> String`. All format rules live here. Negative balance renders as `-X hr Y min`.
- `config.rs` — reads `~/.config/flexi/flexi.toml` (optional keys: `path`, `timestamp_format`, `week_start`, `increment`, `max_session` — see the config block below). Falls back to `~/.local/share/flexi/flexi.txt`.
- `storage.rs` — `flexi.txt` is the log (single file). `append_log`, `read_log`, `pop_log`, `last_entry` all operate on this path. `read_minutes` derives current balance by parsing the last entry's description (`new_minutes()`). Log format: `timestamp description` — timestamp is fixed-width (16 chars simple, 25 chars full), parsed by position so any whitespace separator is accepted. Writes are atomic via `.tmp`. Notes are stored as ` # text` suffix in the description (e.g. `+30 min > 2 hr # stayed late`); both `delta_minutes()` and `new_minutes()` strip everything from ` # ` onward before parsing. Description forms: `+X > Y` / `-X > Y` (delta + new balance), `= Y` (set/reset), `@in Y` (open clock-in marker) and `@out Y` (open spending marker). Both markers are balance-neutral: `new_minutes()` reads `Y`, `delta_minutes()` returns `None`. `is_open_marker()` detects either; `is_spend()` distinguishes `@out`.
- `main.rs` — clap CLI plus the command handlers and the `run_log` rendering helper. Session state lives entirely in the log: `open_session()` returns the tail entry iff it is an `@in`/`@out` marker; `ensure_not_clocked_in()` guards balance mutations while a session is open. `flexi in`/`flexi away` open `@in`/`@out` markers; `flexi out` (alias `back`) pops the tail marker and appends a normal `+elapsed > balance` (work) or `-elapsed > balance` (spend) entry — sign chosen by `is_spend()`, not the closing verb (so the append-only running-balance chain is never rewritten mid-log). Only one session is open at a time, so either closing verb ends whichever is running.

**Time string format:** `N hr M min`, `N hr`, `M min`, `0 min`. Accepts plural/abbreviated unit words (`hour`, `hours`, `hrs`, `minute`, `minutes`, `mins`). Units are summed in any order (`30 min 1 hr` parses the same as `1 hr 30 min`); canonical output is always hours before minutes.

**Config file** (`~/.config/flexi/flexi.toml`):
```toml
path = "/custom/path/to/flexi.txt"
timestamp_format = "simple"   # "simple" (default): "2026-05-24 10:20"  |  "full": "2026-05-24T10:20:16+01:00"
week_start = "monday"         # "monday" (default) | "sunday"
increment = 1                 # round balance-changing durations to N minutes, nearest/half-up (default 1 = none); must be >= 1
max_session = 1440            # `out` refuses (without --force) if raw elapsed exceeds N minutes (default: unset/None); must be >= 1
```

Rounding (`time::round_to_increment`, nearest/half-up, sign-preserving) is applied in `main.rs` to every delta entering the balance — `add`/`remove`/`set` parsed input and `out` elapsed. `format_duration`/storage are unaffected; rounding happens before the value is written.

`max_session` (`Option<i32>`) is checked in the `out` handler against the **raw** elapsed (before rounding); over the cap it bails unless `flexi out --force` is passed. Bare `flexi` rounds the displayed "so far" elapsed and prints a warning line when over the cap.

## Documentation

Update `crates/flexi/README.md` whenever user-facing behaviour changes (new commands, flags, config keys, output format). The root `README.md` is a workspace overview — update it when crates are added or removed. Update `CLAUDE.md` when architecture or conventions change.

## Git workflow

Never commit directly to `main`. For every change, branch off `main`, commit there, push, and open a PR — even for small or solo work. Let CI (test + clippy) run before merging.

- Branch names: `type/short-description` (e.g. `feat/clocking-crate`, `chore/cargo-workspace`, `fix/rounding-sign`).
- Commit messages and PR titles follow Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`), matching the existing history.
- Keep the changelog current on the branch (see Releases) so PRs are self-contained.
- CI runs the latest stable clippy. A lint that passes locally can still fail CI if your toolchain is behind — run `rustup update stable` before relying on a green local `cargo clippy`.

## Releases

Each crate versions, changelogs, and releases independently. Add notable changes to the `[Unreleased]` section of that crate's `CHANGELOG.md` (e.g. `crates/clocking/CHANGELOG.md`) as they are made.

Before tagging a crate, move its `CHANGELOG.md` `[Unreleased]` section to a new version heading with today's date, bump the version in that crate's `Cargo.toml`, then build to update `Cargo.lock` (one lockfile at the workspace root). Commit all together.

**Changing foolscap's API:** a tool can only depend on a foolscap version that exists locally, so a change that adds API bumps foolscap's `version` (patch for additions, minor for breaking changes, per Cargo's 0.x rules) **in the same PR**, and raises the requirement only in the tools that use the new API (e.g. #54 moved to 0.1.1 and only the four tag users require it). Changelogs stay `[Unreleased]` until the release PR, as usual.

**`foolscap` ordering:** bins depend on it as `{ path = "../foolscap", version = "X.Y.Z" }`. `cargo publish` of a bin resolves that version from crates.io, so a `foolscap` change that a bin needs must be tagged and published **first** (`foolscap-vX.Y.Z`), then the bins bumped and released. A library tag only runs the crates.io job; `release.yml` skips the binary, GitHub Release, and Homebrew jobs when `crates/<crate>/src/main.rs` doesn't exist.

**Tag scheme:** `<crate>-vX.Y.Z` (e.g. `flexi-v0.15.0`, `clocking-v0.2.0`). `.github/workflows/release.yml` is a single generic workflow triggered by any `*-vX.Y.Z` tag; its `setup` job derives the crate name and version from the tag (`crate = ${TAG%-v*}`, `version = ${TAG##*-v}`) and every downstream job is parameterised on them. To release: `git tag <crate>-vX.Y.Z && git push origin <crate>-vX.Y.Z` (push the one tag, not `--tags`).

- **GitHub Release** — builds the four target archives (`<crate>-vX.Y.Z-<target>.tar.gz`) and attaches them.
- **crates.io** — `cargo publish -p <crate>` (requires the `CARGO_REGISTRY_TOKEN` secret). Manual fallback: same command locally after `cargo login`.
- **Homebrew tap** — writes `Formula/<crate>.rb` (class = capitalised crate, `desc` read from the crate's `Cargo.toml`, `test` runs `<crate> --version`) and pushes to `thombruce/homebrew-tap`.

Adding a new bin crate needs no workflow edits — it releases as soon as you push a `<crate>-vX.Y.Z` tag.

After tagging, confirm all three outputs landed: the GitHub Release assets, the crates.io version, and the homebrew formula. Note: the crates.io API needs a `User-Agent` header (`curl -s -H "User-Agent: check" https://crates.io/api/v1/crates/<crate>`) or it returns an error body instead of JSON.
