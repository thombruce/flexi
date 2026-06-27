# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build          # compile
cargo run            # run (display balance)
cargo run -- add 1 hr 30 min
cargo run -- add 1 hr 30 min --note "reason"
cargo run -- remove 30 min
cargo test           # run all tests
cargo test time      # run tests in a specific module
cargo clippy         # lint (CI runs with -D warnings — fix all warnings before committing)
```

## Architecture

Single binary crate. All state is `i32` minutes internally; `time.rs` owns the boundary between minutes and human-readable strings.

**Data flow for `add`/`remove`:**
`main.rs` joins `Vec<String>` args → `time::parse_duration` → arithmetic → `storage::append_log` → print delta.

**Modules:**
- `time.rs` — `parse_duration(s) -> i32`, `format_duration(i32) -> String`. All format rules live here. Negative balance renders as `-X hr Y min`.
- `config.rs` — reads `~/.config/flexi/flexi.toml` (optional `path` and `timestamp_format` keys). Falls back to `~/.local/share/flexi/flexi.txt`.
- `storage.rs` — `flexi.txt` is the log (single file). `append_log`, `read_log`, `pop_log`, `last_entry` all operate on this path. `read_minutes` derives current balance by parsing the last entry's description (`new_minutes()`). Log format: `timestamp description` — timestamp is fixed-width (16 chars simple, 25 chars full), parsed by position so any whitespace separator is accepted. Writes are atomic via `.tmp`. Notes are stored as ` # text` suffix in the description (e.g. `+30 min > 2 hr # stayed late`); both `delta_minutes()` and `new_minutes()` strip everything from ` # ` onward before parsing. Description forms: `+X > Y` / `-X > Y` (delta + new balance), `= Y` (set/reset), and `@in Y` (open clock-in marker; balance-neutral, `new_minutes()` reads `Y`, `delta_minutes()` returns `None`, `is_clock_in()` detects it).
- `main.rs` — clap CLI plus the command handlers and the `run_log` rendering helper. Clock-in state lives entirely in the log: `open_session()` returns the tail entry iff it is an `@in` marker; `ensure_not_clocked_in()` guards balance mutations while a session is open. `flexi out` pops the `@in` tail and appends a normal `+elapsed > balance` entry (so the append-only running-balance chain is never rewritten mid-log).

**Time string format:** `N hr M min`, `N hr`, `M min`, `0 min`. Accepts plural/abbreviated unit words (`hour`, `hours`, `hrs`, `minute`, `minutes`, `mins`). Units are summed in any order (`30 min 1 hr` parses the same as `1 hr 30 min`); canonical output is always hours before minutes.

**Config file** (`~/.config/flexi/flexi.toml`):
```toml
path = "/custom/path/to/flexi.txt"
timestamp_format = "simple"   # "simple" (default): "2026-05-24 10:20"  |  "full": "2026-05-24T10:20:16+01:00"
week_start = "monday"         # "monday" (default) | "sunday"
```

## Documentation

Update `README.md` whenever user-facing behaviour changes (new commands, flags, config keys, output format). Update `CLAUDE.md` when architecture or conventions change.

## Releases

Add notable changes to the `[Unreleased]` section of `CHANGELOG.md` as they are made.

Before tagging, move `CHANGELOG.md`'s `[Unreleased]` section to a new version heading with today's date, bump the version in `Cargo.toml`, then build to update `Cargo.lock`. Commit all together.

**GitHub Releases** build automatically via `.github/workflows/release.yml` on `git tag vX.Y.Z && git push --tags`.

**crates.io:** published automatically on release via `.github/workflows/release.yml` (requires the `CARGO_REGISTRY_TOKEN` repo secret). To publish manually instead: `cargo publish` (requires `cargo login` first).

**Homebrew tap:** updated automatically on release via the Git workflow.
