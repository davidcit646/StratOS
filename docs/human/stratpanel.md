# stratpanel (human guide)

**stratpanel** is a **Rust** client using the in-house **stratlayer** Wayland library. It creates a **layer-shell** top bar with:

- A small **launcher / search input** strip (keyboard focusable),
- **Workspace** switcher (compositor IPC; `get workspaces` / `switch_workspace` on `/run/stratvm.sock`),
- A **clock** (`clock.rs`),
- **Pinned apps** strip (`pinned.apps` in `panel.conf`; scroll + click to launch),
- **Tray** cells (**N**etwork / **V**olume / **U**pdates / **B**attery toggles in `[tray]` — N/B read sysfs when enabled; V/U are stubs),
- Optional **auto-hide** (IPC `set panel autohide` + pointer enter/leave handling; see checklist Phase 24).

**Layering:** the panel is a **layer-shell** surface; a focused **XDG** (application) window can still paint **above** it in the current compositor tree. On a live ISO this is easy to notice; it is tracked as UX polish, not a panel config bug.

Configuration is loaded via **`stratsettings::StratSettings`** (`/config/strat/settings.toml`, `settings.d/`, embedded defaults); legacy **`/config/strat/panel.conf`** is merged only when **`settings.toml` is absent** (`stratpanel/src/config.rs`).

## How it connects

- Compositor IPC over `**/run/stratvm.sock`** — see `stratpanel/src/ipc.rs`.
- Started by the compositor autostart path (`stratvm/src/main.c`), not necessarily a `stratman` manifest.

## Agent brief

[../agent/stratpanel.md](../agent/stratpanel.md)