# Spotlite — agent brief (current implementation)

## Shipped pieces

| Piece | Path |
|-------|------|
| Indexer binary | `stratterm/src/bin/stratterm-indexer.rs` |
| Modular config | `stratsettings`: `[spotlite.headless]` / `[spotlite.ui]` in merged `settings.toml` (see [`stratsettings.md`](stratsettings.md)) |
| Legacy overlay | `/config/strat/indexer.conf` — parsed **after** merged `[spotlite.headless]`; per-key overrides |
| Hotkey → stratvm | `[keyboard] spotlite` / `cycle_layout` → `write_stratvm_keybind_file` → `/config/strat/stratvm-keybinds`; `stratvm` loads at boot + IPC `reload_keybinds` |
| Global overlay | `stratterm/src/bin/spotlite.rs` → `/bin/spotlite` (layer `OVERLAY`, fullscreen via all edge anchors, search `path_index`, Enter launches) |
| SQLite / frecency | `stratterm/src/frecency.rs`, schema in indexer binary |
| File browser | `stratterm/src/file_browser.rs`, wired from `stratterm/src/main.rs` |
| Settings CLI | `strat-ui-config`; legacy flat `stratterm/src/bin/strat-settings.rs` for `indexer.conf` workflows |
| Settings UI | `stratos-settings` — Spotlite/indexer rows; read-only line for `keyboard.spotlite` only (`cycle_layout` via TOML / `strat-ui-config`); saving writes `stratvm-keybinds` + IPC `reload_keybinds` |

## Not shipped yet

- Rich launcher polish (icons, `.desktop`, typed frecency ranking in overlay).

## Grep

`rg "indexer|Spotlite|sqlite|spotlite" stratterm stratsettings`

## Human doc

[../human/spotlite.md](../human/spotlite.md)
