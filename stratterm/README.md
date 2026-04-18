# stratterm

**Stratterm** is StratOS’s Wayland terminal: PTY, custom renderer, and an integrated **file browser** you can treat as the system’s **file explorer** (there is no separate file-manager app).

**Looking for a file explorer?** Read **[../docs/human/file-explorer.md](../docs/human/file-explorer.md)** — it points here and explains `F7`, indexer, and docs.

**Other docs:** [../docs/human/stratterm.md](../docs/human/stratterm.md) (OS-level) · [../docs/human/spotlite.md](../docs/human/spotlite.md) (indexer / search) · [../docs/agent/stratterm.md](../docs/agent/stratterm.md) (agent brief).

---

## File browser (explorer)

- **`F7`** — toggle the right-side **browser / preview** overlay (works on top of the normal terminal).
- **`F6`** — toggle listing mode (`flat` / `tree`) for status + state.
- **Navigation:** `Up` / `Down` select rows; **`Left`** — parent directory when the overlay has focus; **`Enter`** — open by action policy; **`Space`** — expand/collapse in tree mode.
- **Scripts:** two-step confirmation before run; paths passed to `sh` / `nano` / `xdg-open` with POSIX single-quoting (including embedded `'`).
- **Safety / feedback:** unreadable files and directories show a message in the preview strip; `chmod +x` files without a known script extension are not opened from the browser (use the shell); symlinks are labeled in preview; listing I/O errors show as `List: …`.
- **Mouse:** single-click select, double-click open/run, **wheel** scrolls selection when the pointer is over the browser column (list or preview); leaving the surface clears double-click pairing so the next click is never a stale “second click”.

Indexer (SQLite paths, frecency, `cd` helpers) and CLI for `/config/strat/indexer.conf` are documented under **Indexer & settings** below and in [spotlite.md](../docs/human/spotlite.md).

---

## Terminal core

- Lightweight Wayland + PTY (`wayland.rs`, `pty.rs`, `main.rs`).
- **Scrollback:** 10,000-line history; `Shift+PageUp` / `Shift+PageDown`.
- **Shell CWD sync:** polls `/proc/<shell-pid>/cwd`, writes `/tmp/stratterm-shell-cwd`, exposes `STRATTERM_SHELL_CWD` in-process.
- **Ghost completion:** non-destructive suffix at cursor; `Tab` or `Right` accepts.
- **Title status:** mode, item count, scroll offset, shell CWD.

---

## Indexer & settings

- **`stratterm-indexer`** — headless path indexer, SQLite storage; config `/config/strat/indexer.conf` then `~/.config/strat/indexer.conf`; modes `--once`, `--daemon`, `--boot-daemon`; backoff via `/proc/loadavg`.
- **`strat-settings`** — CLI for indexer TOML: `--show`, `--set key=value`, `--reset-defaults`, `--interactive`.

---

## Not complete yet

- Richer mouse (drag-select, context menu).
- Dedicated split-pane widget layout (browser remains a renderer overlay; list vs preview already use separate tones + a divider rule in `renderer.rs`).
- Full shell/parser-aware ghosting for non-`cd` commands.
- Richer status for indexer (live queue / daemon health; overlay may show DB row counts only).

---

## Build

```sh
make -C stratterm build
```

## Run

```sh
make -C stratterm run
```

**Shortcuts (summary):** `Shift+PageUp` / `Shift+PageDown` scrollback; `F6` / `F7` browser mode and overlay; browser active: `Up`/`Down`/`Left`/`Enter`/`Space` as above; `Tab` after `cd` / `cd -s` for completion when available.

```sh
make -C stratterm run-indexer-once
make -C stratterm run-settings
```
