# Spotlite (human guide)

**Spotlite** in the design is StratOS’s **search-first launcher**: fast filesystem indexing, overlay UI, and app launch. In the **current tree**, most of that surface lives inside **Stratterm**:

- **Indexer:** `stratterm-indexer` (`stratterm/src/bin/stratterm-indexer.rs`) maintains a **SQLite** database of paths; config in `/config/strat/indexer.conf`.
- **File browser:** keyboard-driven browser / preview inside the terminal (`stratterm/src/file_browser.rs`, toggled from the terminal UI). For “where is the file explorer?”, see **[file-explorer.md](file-explorer.md)**.
- **CLI settings** for indexer keys: `strat-settings` (`stratterm/src/bin/strat-settings.rs`).

A **standalone global Spotlite** process (always-on overlay, independent of the terminal) is still **future work**—see [coding-checklist.md](coding-checklist.md) Phase **12**.

## Agent brief

[../agent/spotlite.md](../agent/spotlite.md)