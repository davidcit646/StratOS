# Agent prompt: flesh out stratpanel (workspace switcher + system tray + pinned strip)

Copy everything below the line into a new agent chat (or Cursor agent) as the **user message**. If you only want **workspace switcher** or only **tray stubs**, say so in one line at the top.

---

## Context

StratOS’s top bar is **`stratpanel/`**: a **Rust** + **stratlayer** Wayland **layer-shell** client. It must stay **Custom first** (no `wayland-client` crate in the panel—extend **stratlayer** only if the compositor protocol requires it).

**Goal:** Make the panel feel **complete enough for daily use**: a visible **workspace switcher** wired to the compositor, a **system tray** region (clock plus stub or lightly wired indicators), **pinned apps** when configured, and—time permitting—**auto-hide** that matches IPC. “**System tree**” in the spec means the **tray / indicator strip** (power, volume, network, Bluetooth placeholders)—**not** a filesystem tree.

**Authoritative references:** `docs/human/coding-checklist.md` Phase **24** (panel), `docs/human/stratos-design.md` section **10** (panel), `docs/human/runtime-persistence-contract.md` for `/config/strat/`.

**Agent brief:** `docs/agent/stratpanel.md`. Compositor side: `docs/agent/stratvm.md`.

**Code map**

| Area | Path |
|------|------|
| Panel loop, draw, input | `stratpanel/src/main.rs` |
| `stratvm.sock` text protocol | `stratpanel/src/ipc.rs` |
| `panel.conf` TOML | `stratpanel/src/config.rs`, `/config/strat/panel.conf` |
| Clock | `stratpanel/src/clock.rs` |
| Workspace / IPC commands | `stratvm/src/main.c` (grep `switch_workspace`, `get_workspaces`, IPC parser) |

---

## Already in tree (verify on hardware—extend, do not rip out)

- Panel spawns from compositor autostart (`stratvm`); `LAYER_TOP` + exclusive zone patterns exist.
- **IPC:** `ping`, `get_workspaces`, `switch_workspace`, `set_panel_autohide`, `float_window` (see `stratpanel/src/ipc.rs` and `stratvm` handlers).
- **Clock** + basic workspace buttons may exist but can be **wrong, ugly, or out of sync** with focus—your job is to make them **correct and obvious**.

---

## Deliverables (priority order)

### P0 — Correctness

1. **Workspace switcher**
   - Call **`get_workspaces`** on a sensible interval or after relevant events; parse the compositor’s response reliably.
   - Render **one button per workspace** (or N max); **highlight the focused** workspace; **left-click** calls **`switch_workspace`** with the same 1-based indexing the compositor expects.
   - If IPC parsing is fragile, **fix the parser** and add short comments at the format boundary (`ipc.rs` + compositor send side).
2. **No lying UI:** if a tray cell is a stub, render it as **disabled / labeled** (e.g. `—` or gray icon), not a fake control that appears interactive.

### P1 — System tray strip

1. Implement the **tray area** called out in the checklist: cells **N** / **V** / **U** / **B** (network, volume, USB, Bluetooth) per `main.rs` / config—**stubs are OK** if they are visibly stubs; prefer **one** real hook (e.g. read `/proc` or a single sysfs path) only if you can do it without pulling heavy deps.
2. Keep **clock** visible and consistent (`clock.rs`, `panel.conf` clock keys).
3. Wire **`panel.conf`** keys for **which** tray cells show (checklist: hide via config).

### P2 — Pinned app strip

1. If **`pinned.apps`** (or the project’s equivalent list in `config.rs`) is non-empty, draw a **scrollable** strip: click launches **absolute paths** (document in code comment); wheel scrolls when overflowed.

### P3 — Auto-hide (if Phase 24 items still open)

1. **`set_panel_autohide`** IPC + compositor flag already sketched—make **peek bar** / debounced collapse / expand behave per checklist; must not break **exclusive zone** for maximized clients.

---

## Constraints

- **Custom first:** hand-rolled TOML / parsing style in `config.rs`; no new heavy UI stacks.
- **Paths:** new keys live under `/config/strat/` per persistence contract; document keys with an example block in a **code comment** near `PanelConfig` or in the existing human stratpanel page **only if** you add user-visible keys (keep doc edits minimal).
- **IPC contract:** panel must not invent a second socket; **`/run/stratvm.sock`** only.
- **Out of scope unless explicitly added:** Phase **26** settings app, full volume daemon, NetworkManager integration, or **panel-window-chrome.md** titlebar-only work (compositor decorations)—this prompt is **stratpanel + required stratvm IPC glue** for panel features.

---

## Definition of done

1. **`./build-all-and-run.sh`** (or your usual flow): boot to session; **click workspace buttons** and see focus / highlights change across **stratterm** or other clients.
2. Tray row: **clock + tray cells** visible per config; stubs legible.
3. **`docs/human/coding-checklist.md`:** update Phase **24** checkboxes you actually satisfied; short note for deferred items.
4. Final message / PR: file list, IPC format changes, new `panel.conf` keys (example).

---

## Suggested execution order

1. Grep `stratpanel` and `stratvm` for `workspace`, `get_workspaces`, `switch_workspace`.
2. Fix **one vertical slice**: workspaces end-to-end (IPC → UI → click).
3. Add **tray** layout + stubs + config toggles.
4. **Pinned** strip, then **auto-hide** last (most pointer edge cases).

---

*End of prompt*
