# stratterm

**Docs:** [../docs/human/stratterm.md](../docs/human/stratterm.md) (how this crate fits the OS) · [../docs/human/spotlite.md](../docs/human/spotlite.md) (indexer / search) · [../docs/agent/stratterm.md](../docs/agent/stratterm.md) (agent brief).

Current StratOS terminal stack status.

## Implemented now
- `stratterm`: lightweight Wayland + PTY terminal (custom renderer/parser).
- `stratterm` scrollback core:
  - 10,000-line history buffer.
  - `Shift+PageUp` / `Shift+PageDown` viewport navigation.
- Shell CWD sync plumbing:
  - Polls `/proc/<shell-pid>/cwd`.
  - Writes latest value to `/tmp/stratterm-shell-cwd`.
  - Exposes `STRATTERM_SHELL_CWD` env var in-process.
- `stratterm-indexer`:
  - Headless filesystem path indexer with SQLite storage.
  - Config from `/config/strat/indexer.conf` then `~/.config/strat/indexer.conf`.
  - Modes: `--once`, `--daemon`, `--boot-daemon`.
  - High-usage backoff via `/proc/loadavg`.
- `strat-settings`:
  - CLI settings editor/viewer for indexer config.
  - Supports `--show`, `--set key=value`, `--reset-defaults`, `--interactive`.
- File-browser backend module (no full UI yet):
  - Directory-first sorting.
  - Flat and tree modes with expansion state.
  - Preview classification (folder/text/script/config/binary).
  - Double-click action policy classification.
- Interactive browser overlay (keyboard-driven MVP):
  - `F7` toggles the right-side browser/preview panel.
  - `Up/Down` selects entries.
  - `Enter` opens folders/files by action policy.
  - Scripts use two-press confirmation before execution.
  - `Space` toggles folder expansion in tree mode.
  - Mouse support:
    - single-click selects row
    - double-click opens/executes selected row action
- Frecency backend:
  - SQLite path-use tracking.
  - `cd` completion helper.
  - `cd -s` first-letter shortcut expansion helper.
- Ghost completion rendering:
  - Non-destructive ghost suffix drawn at cursor.
  - `Tab` or `Right` accepts suggestion.
- Lightweight status line in window title:
  - Shows mode, item count, scroll offset, and current shell CWD.

## Not complete yet
- Richer mouse interactions (scroll-wheel, drag-select, context menu).
- Dedicated split-pane widget layout (current browser is renderer overlay).
- Full shell/parser-aware ghosting for non-`cd` commands.
- Richer status chip data (indexer queue depth / service health).

## Build
```sh
make -C stratterm build
```

## Run
```sh
make -C stratterm run
```

Key shortcuts (current):
- `Shift+PageUp` / `Shift+PageDown`: scrollback viewport.
- `F6`: toggle browser backend mode (`flat`/`tree`) for status + state.
- `F7`: toggle browser overlay panel.
- `Up/Down`: navigate browser entries (when browser overlay is active).
- `Enter`: open selected browser item / confirm script run.
- `Space`: expand/collapse selected directory in tree mode.
- `Tab` after `cd ...` or `cd -s ...`: apply backend completion/expansion when possible.

Mouse controls (browser overlay active):
- Single-click row: select.
- Double-click row: open folder / run configured action.

Run indexer once:
```sh
make -C stratterm run-indexer-once
```

Run settings editor:
```sh
make -C stratterm run-settings
```
