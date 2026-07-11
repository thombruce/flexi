# flexi workspace

A Cargo workspace for a family of minimal, plaintext-storage CLI tools
inspired by [todo.txt](https://github.com/todotxt/todo.txt).

## Crates

| Crate | Description |
|-------|-------------|
| [`flexi`](crates/flexi/) | Track your flexi-time balance. |

## Development

```bash
cargo build            # build all crates
cargo test             # test all crates
cargo clippy           # lint (CI runs with -D warnings)
cargo run -p flexi     # run a specific crate
```
