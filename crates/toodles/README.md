# toodles

A minimal plaintext todo list for the command line — todo.txt, plus due dates.

Part of the [flexi](../../) family of plaintext-storage tools; where bagg keeps a sortable list of things to buy, toodles keeps a sortable, hand-editable list of things to do.

## Quickstart

```sh
cargo install toodles
toodles add --priority A --due 2026-10-31 Renew passport
toodles add Call mum +family
toodles              # everything, sorted
toodles done 1       # mark todo 1 done (toggles back if run again)
toodles rm 2         # remove todo 2
```

## Installation

**Homebrew** (macOS/Linux):
```sh
brew tap thombruce/tap
brew install thombruce/tap/toodles
```

**crates.io** (requires Rust):
```sh
cargo install toodles
```

**From source** (from the repo root):
```sh
cargo install --path crates/toodles
```

## Usage

```sh
toodles                                   # everything, sorted (default: toodles list)
toodles add Call mum                      # plain todo (created date stamped automatically)
toodles add --due 2026-10-31 Renew passport
toodles add --priority A --due 2026-10-31 Renew passport
toodles list                              # everything, sorted (alias for bare invocation)
toodles list --open                       # only open todos
toodles list --done                       # only done todos
toodles list +house @phone                # only todos tagged +house or @phone
toodles done 2                            # toggle done/open for todo #2 as shown by `list`
toodles rm 2                              # remove todo #2 as shown by `list`
toodles edit                              # open the todo file in $EDITOR
toodles completions <shell>               # print a shell completion script
toodles man                               # print the man page (roff) to stdout
```

`$EDITOR` is run through `sh`, as git does (so `edit` isn't supported on Windows), and may carry arguments (`EDITOR="code -w"`). An editor path containing spaces must be quoted inside it: `EDITOR='"/Applications/Sublime Text.app/Contents/SharedSupport/bin/subl" -w'`.

Todos are always listed sorted, regardless of their order in the file: focused, then open, then done, then obsolete; then priority `A` before `B` before none, then soonest due date before later before undated. Open todos past their due date are shown in red. `--open`/`--done` and tag queries filter that same sorted listing without renumbering it, so `done`/`rm` always target what you expect.

Tags (`+project`, `@context`, `#hashtag`, `key:value`) are plain words in the title, todo.txt's own convention, matched case-insensitively — multiple queries match if a todo carries any of them. A shared `+project` tag is how related todos are grouped; there's no nesting. A `#hashtag` must start with a letter (`#42` isn't one), and must be quoted on the command line (`'#urgent'`), or the shell treats it as a comment.

## Storage format

```
(A) 2026-10-18 2026-10-31 Renew passport
2026-10-18 Call mum +family
x (B) 2026-10-20 2026-10-01 2026-10-15 File taxes
x 2026-10-19 2026-10-18 Water plants
! 2026-10-18 Write report
~ 2026-10-17 2026-10-01 Learn Esperanto
```

Each line is `[STATUS ][(A) ][CLOSED ][CREATED ][DUE ]TITLE`:

- `STATUS` is an optional leading marker, at most one: `!` focus (working on it now), `x` done, or `~` obsolete (won't do, kept for the record rather than removed). No marker means open. Each must be a word on its own, so `!Call` is title text.
- `(A)`–`(Z)` is an optional priority, kept on completion (unlike todo.txt, which drops it).
- Open and focused todos carry up to two dates: one is the created date, two are created then **due**.
- Closed todos (done `x` or obsolete `~`) carry up to three: closed, created, then due — todo.txt's `x CLOSED CREATED`, with due appended.

The number of dates is what identifies each one, so a due date needs a created date in front of it. `add` always stamps one, and `done` stamps (or, when reopening, clears) the closed date. Other todo.txt tools see the due date as part of the description, so the file stays readable by them.

Like calchemy, there's no delimiter: a title that opens with a `YYYY-MM-DD`-shaped word, on a line with fewer dates than its maximum, has that word read as a date. A deliberate, accepted tradeoff rather than an escaping scheme. `add` guards against it: a title starting with a date is refused unless `--due` is given (which fills both date slots, so the title is safe). Dates must be zero-padded — `2024-9-1` is plain title text.

## Configuration

Create `~/.config/toodles/toodles.toml`:

```toml
path = "/path/to/toodles.txt"
```

Without config, the list lives at `~/.local/share/toodles/toodles.txt` (or the platform equivalent).

## Not yet supported

Recurrence (`every:`, [#42](https://github.com/thombruce/flexi/issues/42)), focus/obsolete markers (`!`/`~`, [#41](https://github.com/thombruce/flexi/issues/41)), `time:`/`count:` tags, natural-language dates, and JSON output are deliberately left out of this first version.

## License

MIT
