# Spotlite (human guide)

**Spotlite** in the design is StratOS’s **search-first launcher**: fast filesystem indexing, overlay UI, and app launch. In the **current tree**, most of that surface lives inside **Stratterm**, with configuration moving onto the **modular settings** stack under `/config/strat/`.

## Where settings live (priority)

1. **Merged modular settings** — `/config/strat/settings.toml` plus optional fragments in `/config/strat/settings.d/*.toml` (later files override overlapping keys). Embedded defaults ship in-repo as `stratsettings/defaults/settings.default.toml` and are merged first.
2. **Keyboard → compositor** — `[keyboard]` in merged settings (e.g. `spotlite = "super+period"`). On save, `stratsettings` writes `/config/strat/stratvm-keybinds` for **stratvm** (`stratwm`) so the global Spotlite hotkey is available after compositor reload / restart.
3. **Indexer overlay file (optional)** — If `/config/strat/indexer.conf` exists (or `~/.config/strat/indexer.conf` when `/config` is missing), **stratterm-indexer** parses it and **overrides** the same logical keys that come from **`[spotlite.headless]`** in merged settings. So: defaults → merged TOML → first readable `indexer.conf` in search order.

## Sections (reference)

| TOML | Role |
|------|------|
| `[spotlite.headless]` | **stratterm-indexer**: enable/disable, boot daemon, scan timing, roots, excludes, batch sizing. |
| `[spotlite.ui]` | Reserved for **in-terminal** incremental / navigation hints (same field names as legacy `ui_*` lines in `indexer.conf`); not all fields are consumed by Stratterm yet—check the tree before relying on runtime behavior. |
| `[keyboard] spotlite` | Hotkey bound in stratvm; key press **attempts to launch** `/bin/spotlite` (then `/usr/bin/spotlite`). Missing binary fails in the spawned process—install the overlay from the build. |

## Components

- **Indexer:** `stratterm-indexer` maintains a **SQLite** database (`path-index.db` under `/config/strat/` or user config). It loads **`[spotlite.headless]`** from merged settings, then overlays **`indexer.conf`** if present.
- **File browser:** keyboard-driven browser / preview inside the terminal (`stratterm`), **F7**. See **[file-explorer.md](file-explorer.md)**.
- **CLI:** `strat-ui-config` (`get` / `show`) and legacy `strat-settings` for flat indexer files where still used.
- **Settings UI:** `stratos-settings` edits merged settings (including Spotlite rows) and saves `settings.toml`. It shows **`keyboard.spotlite` read-only**; **`keyboard.cycle_layout`** is only in TOML or `strat-ui-config show` (not a separate row).
- **Global overlay:** `/bin/spotlite` (from `stratterm/src/bin/spotlite.rs`) is a **fullscreen layer-shell overlay** (anchors on all edges) that searches `path-index.db`; it is not “always on,” but launched by the compositor hotkey.

**Still open (polish):** icons, `.desktop` integration, richer frecency ranking in the overlay—track against [coding-checklist.md](coding-checklist.md) Phase **12**.

## Agent brief

[../agent/spotlite.md](../agent/spotlite.md)
