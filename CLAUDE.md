# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workspace layout

Cargo workspace (`Cargo.toml` at root, `members = ["crates/*"]`). Each tool is its own crate under `crates/`:

- `crates/flexi` — track a running flexi-time balance.
- `crates/clocking` — log working hours as discrete clock-in/clock-out sessions.
- `crates/calchemy` — a plaintext calendar of dated appointments.
- `crates/bagg` — a plaintext shopping list / wishlist.

All four share the same plaintext-file + XDG-config approach. What's copied vs. divergent:

- `config.rs` — the XDG path-discovery + toml pattern is copied into each crate (with the crate name and the relevant keys). This is the code most repeated across all four.
- `time.rs` (duration parse/format/round) is shared by flexi and clocking only; **calchemy and bagg have no `time.rs`** — calchemy parses dates/times via chrono rather than durations, and bagg has no time dimension at all.
- `storage.rs` is **not** shared — each crate's storage semantics differ: flexi keeps a running-balance chain (tail = state via `new_minutes()`); clocking records session durations summed over a period (`session_minutes()`); calchemy holds an unordered set of `Appt` records parsed and sorted by datetime, with `add`/`rm`/`edit` rewriting arbitrary lines rather than appending to a derived chain; bagg holds an unordered set of `Item` records parsed and sorted by status/priority/price (same shape as calchemy's set-of-records model, different sort key and no dates).

The duplication is **deliberate** (rule of three, now rule of four). The candidate for a shared crate is the config-discovery + atomic-write plumbing (repeated 4×), **not** duration parsing (only 2×). This was measured and **explicitly deferred**: the truly-identical surface is only ~25 lines (the two `xdg_*_dir` helpers, the `tmp`+`rename` core, and the toml-load skeleton) — the config structs, `resolve()` validation, and storage semantics all diverge per crate. Extracting would add a 5th crate that must itself be **published to crates.io** before the four bins can publish (a path dep needs a released version), coupling every shared-code change to a publish-then-bump-dependents dance. Not worth it for ~25 lines of inert primitives. **The fourth tool (bagg) has now landed** — this was the originally-named revisit trigger, and the call stands: still not worth extracting for ~25 lines. Next trigger: **the same `tmp`+`rename` bug gets fixed more than once**, or a fifth tool lands. If/when extracted, keep it minimal (`write_atomic`, the xdg helpers, `load_toml::<T>`); per-crate config structs and storage stay put.

## The `bagg` crate

`bagg` (`main.rs` + `config.rs` + `storage.rs` + `tags.rs`, clap CLI) is a plaintext shopping list / wishlist. A line is `[x ][(A) ][PRICE ]NAME[ xQTY]` (e.g. `x (A) 12.99 Widget thing x2`), parsed into `storage::Item { got: bool, priority: Option<char>, price: Option<i32>, qty: u32, name: String, raw: String }`. Unlike flexi/clocking, there is **no `#` delimiter** (calchemy has since dropped it too, see below) — the format is fully todo.txt-native (leading status/priority markers, a trailing inline tag), so `Item::parse` leans entirely on token shape rather than a split point: a leading `x` token is status (got), a leading `(A)`–`(Z)` token is priority, a leading `\d+\.\d{2}` token (the `.` is required — a bare integer is never a price) is price, and a trailing `xN` token is quantity (default `1`, and `to_line` omits the tag entirely when qty is `1` — so `Eggs` round-trips as a single plain word). Everything remaining is the name. This is a deliberate, accepted tradeoff: a name starting with a bare decimal or ending in an `xN`-shaped word will misparse, but it's rare for a shopping list and not worth an escaping scheme.

Like calchemy, `bagg` is a mutable set rewritten via atomic `.tmp`+rename (`read_items`/`append_item`/`write_lines`/`remove_line`/`replace_line`), not an append-only chain — items get their price edited, status toggled, and get removed. Sort key (`Item::sort_key()`) is **status → priority → price**, ascending, matching the field order in the format itself: not-got before got, then priority `A`..`Z` (absent sorts last), then price cheap-first (absent sorts last), stable on insertion order for ties. `list` (also the bare-invocation default) always shows the **full** sorted listing; `--pending`/`--got` filter that same listing for display without renumbering it, and `got <n>`/`rm <n>` always index against the identical full-sorted computation `list` uses — this is deliberately more conservative than calchemy's `rm`, which indexes into a filtered (`upcoming()`) view; bagg's status-toggling makes that kind of view-drift more likely, so mutating commands never index into a subset. `got <n>` toggles status in place via `replace_line` (a new `storage.rs` primitive calchemy doesn't have, since calchemy never mutates a line in place — only appends or removes whole lines). No dates, no `time.rs`, no notes/links as structured fields (they live in the free-text `NAME`) — deliberately deferred pending real need. `list` does support `+project`/`@context`/`key:value` tag filtering (see `tags.rs` below) — these aren't a stored struct field, just parsed on the fly out of `NAME`.

The rest of this file describes the `flexi` crate (`crates/flexi/`). The `clocking` crate mirrors flexi's module layout (`main.rs` + `config.rs` + `storage.rs` + `time.rs`, clap CLI, `@in` open-marker session model) with a leaner command set: `in`/`out`, `log`/`summary` (with the same date filters), `edit`, `undo`, `completions`, `man`. It has no balance, so no `add`/`remove`/`set`/`reset`, no clipboard, no json/prose.

clocking's closed-session line is `<clock-in> <end> = <duration>` (e.g. `2026-07-11 09:00 17:00 = 8 hr`), **keyed by the clock-in time** — the leading fixed-width timestamp is the start, so date filters bucket a shift by its start day (a shift past midnight counts on the day it began). The end field is a bare `HH:MM` for a same-day shift, or a full `YYYY-MM-DD HH:MM` when it crosses midnight; in `full` timestamp mode both endpoints are always full. `= <duration>` is the recorded (increment-rounded) worked time and is authoritative — the end field is never parsed back for logic, only displayed, so its shape is purely cosmetic. `out` computes the end field, pops the `@in` marker, and appends the closed line stamped with the marker's (clock-in) timestamp via `storage::append_log(path, timestamp, desc)` (which takes an explicit leading timestamp; `storage::now_timestamp` formats "now" for the `@in` marker). `session_minutes()` parses the duration after ` = `; `summary` sums it. There is no backward compatibility with the 0.1.0 `<clock-out> <duration> (<span>)` format.

The `calchemy` crate (`main.rs` + `config.rs` + `storage.rs` + `tags.rs`, clap CLI) is a plaintext calendar. A line is `DATE [START [END]] TITLE` (e.g. `2026-07-14 09:00 10:00 Dentist`), parsed into `storage::Appt { date, start: Option<NaiveTime>, end: Option<NaiveDateTime>, title, raw }`. `START` absent = all-day; `END` is `HH:MM` (same day) or `YYYY-MM-DD HH:MM` (crosses to a later day) — the same cross-day rule clocking uses, and `add` rolls an end `<= start` to the next day. `add` also accepts an explicit timed later-day end (`HH:MM YYYY-MM-DD HH:MM`, claimed only when the full time-date-time triple parses; end must be after start). With no `START`, a bare `YYYY-MM-DD` END is a multi-day all-day event (inclusive last day, stored as that date at `NaiveTime::MIN`; must be after `DATE`, enforced in both `Appt::parse` and `add`). Day filters are span-aware via `Appt::last_date()` (`end`'s date, else `date`): an appointment appears on every day in `[date, last_date]`, so window checks are overlap tests (`last_date >= since && date <= until`), the default upcoming view keeps ongoing events (`last_date >= today`), and `--past` means `last_date < today`. **No `#` delimiter** (dropped in favor of bagg's todo.txt-native shape-parsing — see the bagg paragraph below): `Appt::parse` matches the longest shape-valid prefix of `tokens[1..]` (`TIME DATE TIME` → `TIME TIME` → `TIME` alone → `DATE` alone → none), consuming that many tokens as `START`/`END`, and joins whatever's left as `TITLE`. Accepted tradeoff: a title opening with well-formed date/time-shaped words (e.g. "09:00 sync") can have them misclaimed as `START`/`END`; old files written with ` # ` still parse without error, the `#` just becomes a stray leading character in the title until re-saved. Unlike the append-only logs, the file is an **unordered set**: `read_appts` parses every line and callers sort by `Appt::start_dt()` (all-day sorts first via `NaiveTime::MIN`); `add` appends, `rm` deletes an arbitrary line (`remove_line`, atomic rewrite preserving file order) indexed by position in the default upcoming view, `edit` hand-edits. Commands: bare/`today` = today's agenda, `next` = soonest upcoming, `list`/`agenda` (+ `--today`/`--week`/`--month`/`--since`/`--until`/`--all`/`--past`), `week`, `rm`, `edit`, `completions`, `man`. Note the divergence: `--week`/`--month` mean the **full** calendar week/month (forward-looking), not "up to today" as in flexi/clocking. Config is `path` + `week_start` only. `list` supports `+project`/`@context`/`key:value` tag filtering, same as bagg (see `tags.rs` below). Deferred (not built): recurrence, natural-language dates, undo, JSON.

## Tag filtering (`tags.rs`, bagg + calchemy)

Both bagg and calchemy's `list` take trailing `+project`/`@context`/`key:value` tag queries (e.g. `bagg list +kitchen-reno`, `calchemy list @clinic`), matched case-insensitively, OR'd across multiple queries. This directly answers "how do I group related bagg items" (several parts for one project) — a shared `+project` tag is the grouping mechanism; nesting/indentation was considered and rejected since it breaks the one-line-per-record model both crates share.

`tags.rs` is identical in both crates (same duplication treatment as `time.rs` — shared by exactly two, not extracted to a crate). It's read-only and works entirely off the existing free-text field (`NAME`/`TITLE`): `extract_tag` classifies a single whitespace token as a tag if it starts with `+`/`@` (len > 1), or if it's a `key:value` pair whose key starts with a letter — that last condition specifically excludes a bare numeric token (a clock time like `10:00` in a calchemy title) from being misread as a tag, a real collision risk raised and resolved before implementation. `matches(text, queries)` extracts all tags from `text` and checks whether any query is present (case-folded); an empty query list matches everything. No struct field, no format change, no round-trip risk — tags are computed at query time only, and don't affect sort order in either crate.

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

**Tag scheme:** `<crate>-vX.Y.Z` (e.g. `flexi-v0.15.0`, `clocking-v0.2.0`). `.github/workflows/release.yml` is a single generic workflow triggered by any `*-vX.Y.Z` tag; its `setup` job derives the crate name and version from the tag (`crate = ${TAG%-v*}`, `version = ${TAG##*-v}`) and every downstream job is parameterised on them. To release: `git tag <crate>-vX.Y.Z && git push origin <crate>-vX.Y.Z` (push the one tag, not `--tags`).

- **GitHub Release** — builds the four target archives (`<crate>-vX.Y.Z-<target>.tar.gz`) and attaches them.
- **crates.io** — `cargo publish -p <crate>` (requires the `CARGO_REGISTRY_TOKEN` secret). Manual fallback: same command locally after `cargo login`.
- **Homebrew tap** — writes `Formula/<crate>.rb` (class = capitalised crate, `desc` read from the crate's `Cargo.toml`, `test` runs `<crate> --version`) and pushes to `thombruce/homebrew-tap`.

Adding a new crate needs no workflow edits — it releases as soon as you push a `<crate>-vX.Y.Z` tag.

After tagging, confirm all three outputs landed: the GitHub Release assets, the crates.io version, and the homebrew formula. Note: the crates.io API needs a `User-Agent` header (`curl -s -H "User-Agent: check" https://crates.io/api/v1/crates/<crate>`) or it returns an error body instead of JSON.
