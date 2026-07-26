# flexi workspace

A Cargo workspace for a family of minimal, plaintext-storage CLI tools for
tracking time, inspired by [todo.txt](https://github.com/todotxt/todo.txt).
Each keeps its data in a single human-readable text file you can grep, diff,
and hand-edit — one entry per line — and reads optional config from
`~/.config/<tool>/<tool>.toml`.

## The tools

| Tool | What it does | Install |
|------|--------------|---------|
| [`flexi`](crates/flexi/) | Track a running flexi-time balance — bank overtime, spend it back, see where you stand. | `cargo install flexi` |
| [`clocking`](crates/clocking/) | Log working hours as clock-in/clock-out sessions and total them by day, week, or month. | `cargo install clocking` |
| [`calchemy`](crates/calchemy/) | Keep a plaintext calendar of dated appointments — add them, list them, see what's next. | `cargo install calchemy` |
| [`bagg`](crates/bagg/) | Keep a plaintext shopping list / wishlist — name, price, quantity, got/not-got status. | `cargo install bagg` |

Each is also on the Homebrew tap: `brew install thombruce/tap/<tool>`.

## Shared design

- **Plaintext, one entry per line** — greppable, diffable, and editable by hand or with `<tool> edit`.
- **A ` # ` convention** separates the machine-readable fields at the start of a line from free text (a note or title) after it — used by flexi, clocking, and calchemy. bagg is the exception: its format is fully todo.txt-native (leading status/priority markers, a trailing inline tag, no `#`) — see [`crates/bagg/`](crates/bagg/) for why.
- **XDG paths** — data under `~/.local/share/<tool>/`, config at `~/.config/<tool>/<tool>.toml`.
- **Independent releases** — each crate versions and ships on its own from a `<crate>-vX.Y.Z` tag.

## Development

```bash
cargo build            # build all crates
cargo test             # test all crates
cargo clippy           # lint (CI runs with -D warnings)
cargo run -p flexi     # run a specific crate
```
