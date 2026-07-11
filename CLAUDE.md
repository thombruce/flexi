# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workspace layout

Cargo workspace (`Cargo.toml` at root, `members = ["crates/*"]`). Each tool is its own crate under `crates/`. Currently one member: `crates/flexi`. Sibling tools (e.g. `clocking`, a calendar) will be added as further crates; shared code is only extracted into a common crate once a second tool concretely repeats it — not speculatively.

The rest of this file describes the `flexi` crate (`crates/flexi/`).

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
- `config.rs` — reads `~/.config/flexi/flexi.toml` (optional `path` and `timestamp_format` keys). Falls back to `~/.local/share/flexi/flexi.txt`.
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

## Releases

Each crate versions and changelogs independently. For flexi, add notable changes to the `[Unreleased]` section of `crates/flexi/CHANGELOG.md` as they are made.

Before tagging, move `crates/flexi/CHANGELOG.md`'s `[Unreleased]` section to a new version heading with today's date, bump the version in `crates/flexi/Cargo.toml`, then build to update `Cargo.lock` (one lockfile at the workspace root). Commit all together.

**GitHub Releases** build automatically via `.github/workflows/release.yml` on `git tag vX.Y.Z && git push --tags`. (Tags are currently flexi-only; when a second crate ships, switch to per-crate tag prefixes.)

**crates.io:** published automatically on release via `.github/workflows/release.yml` (requires the `CARGO_REGISTRY_TOKEN` repo secret). To publish manually instead: `cargo publish -p flexi` (requires `cargo login` first).

**Homebrew tap:** updated automatically on release via the Git workflow.
