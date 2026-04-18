# Spotlite — agent brief (current implementation)

## Shipped pieces

| Piece | Path |
|-------|------|
| Indexer binary | `stratterm/src/bin/stratterm-indexer.rs` |
| Indexer config | `/config/strat/indexer.conf` (defaults in binary + `strat-settings`) |
| SQLite / frecency | `stratterm/src/frecency.rs`, schema in indexer binary |
| File browser | `stratterm/src/file_browser.rs`, wired from `stratterm/src/main.rs` |
| Settings CLI | `stratterm/src/bin/strat-settings.rs` |

## Not shipped yet

- Global Wayland overlay launcher (design “Spotlite UI”) — checklist Phase 12 open item.

## Grep

`rg "indexer|Spotlite|sqlite" stratterm`

## Human doc

[../human/spotlite.md](../human/spotlite.md)
