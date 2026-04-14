# stratterm

StratOS Phase 9 terminal MVP.

## Features (MVP)
- Modernized UI theme with layered surfaces, typography hierarchy, and styled prompt/action controls.
- Live status chip summarizing mode/view/item count/indexer queue state.
- GTK4 window suitable for Wayland sessions.
- File browser with directories-first sorting.
- `.. (go up)` row + double-click folder navigation.
- Clickable breadcrumb path navigation.
- Flat/Tree view toggle for directory listing.
- Tree mode supports per-folder inline expansion state via the `Expand/Collapse` action.
- Prompt line with ghost completion support.
- Ghost suggestions are non-destructive until accepted (`Tab`/Right), and dismissed with `Esc`.
- Single-click previews:
  - Folder: inline summary of contents.
  - File: first lines when text.
  - Script: quick preview + run behavior hint.
  - Config: plain-language summary + preview.
- Double-click file actions:
  - Script: confirmation flow (double-click once to arm, again to run).
  - Config: open in editor (`nano`/`vi` fallback).
  - File: open with `xdg-open` fallback to `less`.
  - Executables that are not clear scripts are not auto-run.
- PTY-backed interactive shell in VTE (`fish` preferred, `bash` fallback, then `/bin/sh`).
- Terminal scrollback buffer (10,000 lines).
- File browser sync with shell CWD when `cd` is used.
  - Source of truth is `/proc/<shell-pid>/cwd` polling.
- Background path index (paths + metadata only) stored in SQLite:
  - Prefers `/config/strat/index.db` when `/config` exists
  - Falls back to `~/.config/strat/index.db`, then `/tmp/stratterm_index.db`
- Indexing policy:
  - Quiet startup indexing pass
  - Quiet idle-time indexing when user input is inactive
  - High-usage backoff: indexing pauses when system load is elevated, then resumes automatically
  - Best-effort close-time flush of queued paths
  - Event-driven queueing when files/paths are viewed, opened, edited, or navigated
- `cd` and `cd -s` suggestions query the index (fast) instead of scanning HOME at startup.
- `cd` ghost suggestions ranked using SQLite frecency data (`~/.config/strat/frecency.db`) plus cwd/home/system-path matches.
- `cd -s` smart shortcut expansion (first-letter segment abbreviation).
- Command ghosting from local prompt command history.

## Build
```sh
make -C stratterm build
```

Build only the lightweight headless indexer runner:
```sh
make -C stratterm run-indexer-once
```

## Run
```sh
make -C stratterm run
```

Run the simple settings app:
```sh
make -C stratterm run-settings
```

## Rootfs/boot integration
- Boot-time background indexing is supported with a dedicated lightweight process: `stratterm-indexer`.
- `system-init` runs `/bin/strat-indexer-boot.sh`, which starts `/bin/stratterm-indexer --boot-daemon` in the background.
- This keeps indexing warm even if no one opens the GUI terminal app.
- Daemon indexing roots are intentionally scoped to `/home`, `/config`, and `/apps`.
- Disable switch:
  - create `/config/strat/disable-indexer`, or
  - set `STRAT_INDEXER_DISABLE=1` before startup script execution.

## Indexer settings backend (for future System Settings app)
- Config file locations:
  - `/config/strat/indexer.conf` (preferred)
  - `~/.config/strat/indexer.conf` (fallback)
- Template: `stratterm/indexer.conf.example`
- Temporary MVP editor app:
  - binary: `strat-settings`
  - purpose: edit/save indexer backend settings until full System Settings app exists
  - UX: icon-based main settings page; open `Terminal` icon to access terminal/indexer settings panel
  - usability: terminal/indexer fields include hover tooltips explaining each setting
- Current supported keys:
  - `enabled`
  - `boot_start`
  - `frequency_ms`
  - `rescan_secs`
  - `batch_limit`
  - `high_usage_load_per_cpu`
  - `roots`
  - `exclude_prefixes`
  - `ui_enabled`
  - `ui_tick_ms`
  - `ui_batch_limit`
  - `ui_idle_after_secs`
  - `ui_startup_grace_secs`
  - `ui_post_nav_delay_ms`
  - `ui_post_nav_scan_limit`
  - `ui_post_nav_force_secs`
