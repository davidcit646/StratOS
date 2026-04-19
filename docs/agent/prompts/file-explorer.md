# Agent prompt: file explorer (Stratterm browser + indexer)

Copy everything below the line into a new agent chat (or Cursor agent) as the **user message**. Narrow or expand scope in a preamble if you only want **indexer** work, only **mouse/scroll** polish, or a **future global Spotlite** spike.

---

## Context

StratOS has **no standalone file manager application**. The **in-terminal file browser** lives in **stratterm**: a keyboard-first overlay (**`F7`**) with optional mouse, directory listing, preview classification, flat/tree modes, and script run confirmation. Indexing and frecency live in **`stratterm-indexer`** (SQLite) and **`strat-settings`** (TOML for `/config/strat/indexer.conf`).

**Authoritative specs:** `docs/human/stratos-design.md` (filesystem honesty, sections **3.4–3.5**), `docs/human/runtime-persistence-contract.md`, `docs/human/application-config-resolution.md`, `docs/human/coding-checklist.md` (Phase **11** terminal, Phase **12** Spotlite / browser / indexer).

**Agent briefs:** `docs/agent/stratos-design.md`, `docs/agent/stratterm.md`, `docs/agent/spotlite.md`.

**Crate docs:** `stratterm/README.md` (shortcuts, implemented vs not complete).

**User-facing redirect (“where is the file explorer?”):** `docs/human/file-explorer.md`.

**Primary code:**

- `stratterm/src/file_browser.rs` — browser logic, selection, modes, preview policy.
- `stratterm/src/main.rs` — toggles browser (`F7`), input routing, integration with renderer.
- `stratterm/src/bin/stratterm-indexer.rs` — indexer daemon / `--once`.
- `stratterm/src/bin/strat-settings.rs` — indexer config CLI.
- `stratterm/src/frecency.rs` — SQLite path usage (if present under that name; grep if refactored).

**Wayland client library:** `stratlayer/` — extend only when protocol support is missing; avoid `wayland-client` crates unless policy changes.

---

## Already in tree (verify before re-building)

- `F7` overlay, `Up`/`Down`, `Enter`, `Space` (tree), two-step script confirmation, basic mouse select / double-click.
- CWD sync via `/proc/…/cwd` and shell integration for `cd` / ghost completion (separate from browser but often coupled in UX).

---

## Goals (priority order — edit to match your charter)

### P0 — Correctness and safety

1. Audit **action policy** for open vs execute vs preview: binaries, scripts, symlinks, and unreadable paths must not surprise the user or bypass confirmation.
2. Ensure browser operations respect **read-only / permission errors** with clear in-UI or log feedback (no silent failure).
3. Keep **PTY + terminal** usable when browser is open: focus modes, resize, and damage must not corrupt the scrollback buffer.

### P1 — Explorer UX (still Custom First)

Pick items that match `stratterm/README.md` “Not complete yet” and Phase **12** checklist intent, for example:

- **Mouse:** scroll wheel in file list; optional drag-select if it fits the renderer without a new UI toolkit.
- **Layout:** clearer split between list and preview (still renderer-local; no GTK/Qt).
- **Paths:** visible current path / “up” affordance without breaking honest-FS rules (no hidden bind tricks).

### P2 — Indexer integration in the browser

- Optional hooks: show indexer freshness, queue depth, or “not indexed” states as ** honest stubs** (disabled styling or label), not fake progress bars.
- Respect `/config/strat/indexer.conf` and documented ignore rules; do not scan `/system` in ways that violate immutability expectations.

### Out of scope unless explicitly added

- **Dedicated global Spotlite** (search overlay outside stratterm) — checklist Phase **12** third bullet; treat as separate compositor + client design.
- **Full Phase 14** system-wide preferences API — if you only need one new key, prefer extending existing hand-rolled TOML patterns and document the key.

---

## Constraints

- **Custom first:** no heavy file-manager frameworks; small, explicit Rust changes in `stratterm` (+ `stratlayer` only if required).
- **No lying UX:** placeholders must read as placeholders.
- **Paths:** new persistent config under `/config/strat/` per `docs/human/runtime-persistence-contract.md` and `docs/human/application-config-resolution.md`; document keys in `stratterm/README.md` or the relevant human doc if behavior is user-visible.
- **Update architecture:** this task does not touch StratBoot, slot writes, or `stratmon` staging semantics.

---

## Definition of done

1. **`docs/human/coding-checklist.md`:** adjust Phase **11** / **12** notes or checkboxes only where your work changes facts; add a short deferral note for anything pushed to a follow-up.
2. **Build:** `make -C stratterm build` (or project-standard equivalent) succeeds.
3. **Manual sanity:** run stratterm on target hardware; exercise `F7`, navigation, and at least one **script** and one **non-script** open path.
4. **PR / summary:** list files touched, UX decisions, and any new config keys with an example snippet.

---

## Suggested execution order

1. Read `stratterm/README.md` and grep `rg "file_browser|F7|Browser" stratterm/src`.
2. Trace `file_browser.rs` → `main.rs` input and render paths; note `renderer.rs` assumptions.
3. Implement **P0** safety and focus/resize fixes before **P1** polish.
4. Add indexer visibility (**P2**) only after list interaction feels solid.

Start with a short **file:line map** of current behavior, then implement P0.

---

*End of prompt*
