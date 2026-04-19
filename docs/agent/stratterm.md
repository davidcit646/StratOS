# Stratterm — agent brief

## Core crate

- `stratterm/src/main.rs` — Wayland session, terminal grid, input.
- `pty.rs`, `renderer.rs`, `parser.rs`, `keyboard.rs`, `screen.rs`, `wayland.rs`, `font.rs`.
- `file_browser.rs` — in-terminal browser.

## Binaries (`src/bin/`)

- `stratterm-indexer.rs` — indexing daemon / `--once`.
- `strat-settings.rs` — indexer TOML CLI.

## Deps

- `stratlayer` path dependency; `stratsettings` for merged `settings.toml` fields; `rusqlite` for indexer/frecency.

## Human / crate README

- [../human/stratterm.md](../human/stratterm.md)
- [../human/file-explorer.md](../human/file-explorer.md) — redirect “file explorer” users to Stratterm
- [../../stratterm/README.md](../../stratterm/README.md)

