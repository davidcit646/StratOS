# Agent prompt: panel, stacking, window chrome, and desktop affordances

Copy everything below the line into a new agent chat (or Cursor agent) as the **user message**. Adjust scope only if you intentionally want to defer settings or file manager work.

---

## Context

StratOS boots to a **wlroots-based compositor** (`stratvm/`, binary `stratwm`) with a **Rust panel** (`stratpanel/`) and **stratterm** as the main terminal. There is **no standalone file manager app**; **`stratos-settings`** exists for merged config + keybinds, but there is **no full multi-panel “control center”** yet. Treat the session as **panel + compositor + terminal + minimal settings UI**—not a full desktop shell. The goal of this task is to make the **panel and window decorations behave as designed**, and to add **minimal, intentional** affordances (launcher strip, tray basics, stacking fix, movable windows) so the session feels usable—not to build the rest of Phase 26 unless you explicitly expand scope.

**Authoritative specs:** `docs/human/stratos-design.md`, `docs/human/runtime-persistence-contract.md`, `docs/human/coding-checklist.md` (Phases **24**, **25**, and optionally **26**).

**Already fixed in tree (verify on hardware, do not re-implement blindly):**

1. **Panel vs focus Z-order:** `stratvm` uses separate `wlr_scene_tree` layers (`layers_bg` … `layers_overlay` in `server.h`); XDG views live under `layers_normal` and `focus_view` only raises within that tree (`stratvm/src/main.c`).
2. **Interactive move:** Titlebar drag + `xdg_toplevel` `request_move` + `stratwm_apply_move_grab` / `grabbed_view` exist in `stratvm/src/main.c`.

**Still open (see checklist Phase 24–25):** pinned strip UI, real tray widgets, auto-hide animation, titlebar context menu, real minimize, decoration config from file.

**IPC:** Panel talks to compositor via Unix socket `**/run/stratvm.sock`** (see `stratpanel/src/ipc.rs`, compositor side in `stratvm/src/main.c` and related headers).

**Wayland client library:** `**stratlayer/`** (custom, minimal). Extend it only when the compositor protocol surface is required for panel or tests; avoid pulling in `wayland-client` crates unless the project explicitly changes policy.

---

## Goals (priority order)

### P0 — Regression / polish

1. Confirm stacking and move behavior under `./build-all-and-run.sh` (focus cycles, panel never covered, drag release).
2. Fix **minimize** (currently wired to `send_close` in `stratvm/src/main.c` — placeholder).

### P1 — Panel “fleshed out” (still minimal, Custom First)

Implement or complete checklist items where they are still open:

- **Pinned app strip** (scrollable): launchers from config (e.g. paths or `.desktop` ids — pick one simple scheme, document in `panel.conf` or a dedicated file under `/config/strat/`).
- **System tray (MVP):** at minimum **clock** (already), plus **stub icons** or real hooks for volume/network/power **only if** you can wire them without a giant dependency stack; otherwise placeholders with clear TODO and no fake “working” UI.
- **Auto-hide panel:** optional slide behavior driven by pointer proximity to top edge + `panel.conf` keys; must not break exclusive zone reporting to clients.

### P2 — Window decorations polish

- **Titlebar context menu** (right-click): float toggle, “move to workspace …” if feasible, hide decorations (per-window state in compositor).
- **Configurable decoration chrome** from `panel.conf` or a small `stratvm` config file: corner radius, border width, button layout — keep parser hand-rolled (project style) unless checklist explicitly allows serde for this path.

### Out of scope unless explicitly added

- Full **Phase 26** settings application (multi-panel “control center”).
- Full **file manager** / stratterm browser work — use [file-explorer.md](file-explorer.md) instead; do not conflate with this task unless scoped.
- **Cover Flow** / **tabbed mode** (Phase 25 stretch).

---

## Constraints

- **Custom First:** no new heavy UI frameworks; prefer small, explicit Rust/C in existing modules.
- **No lying UX:** stubs must look/behave as stubs (disabled state, tooltip, or log), not as broken “real” controls.
- **Update architecture:** do not change StratBoot / slot write ownership; this task is compositor + panel + optional config only.
- **Paths:** respect `/config` persistence patterns from `docs/human/runtime-persistence-contract.md` and `docs/human/application-config-resolution.md` for any new config files.

---

## Definition of done

1. **Checklist** `docs/human/coding-checklist.md`: update Phase **24** / **25** checkboxes to match reality; add short notes for any deferred items.
2. **Session:** focus cycles between **stratterm** (or another client) and **panel** — panel **never** covered by focused tiled/floating clients after your stacking fix.
3. **Floating window:** user can **drag by titlebar** to reposition; release ends grab cleanly.
4. **Brief summary** in your PR or final message: files touched, protocol/layer decisions, any new config keys with examples.

---

## Suggested execution order

1. Read `docs/human/coding-checklist.md` Phase 24–25 and grep `stratvm` for `raise_to_top`, `layer_surface`, `scene`.
2. Implement **scene layer trees** + migrate panel vs xdg nodes.
3. Add **move** grab + `request_move` path; test with floating windows.
4. Panel features: **pinned strip** → **tray MVP** → **auto-hide** last (most interaction edge cases).
5. Decoration config + context menu if time remains.

Start by summarizing current code paths (with file:line references), then implement P0 before P1.

---

*End of prompt*