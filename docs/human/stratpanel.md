# stratpanel (human guide)

**stratpanel** is a **Rust** client using the in-house **stratlayer** Wayland library. It creates a **layer-shell** top bar with:

- A small **launcher / search input** strip (keyboard focusable),
- **Workspace** buttons (via compositor IPC),
- A **clock** (and room for tray widgets as they are implemented).

Configuration is read from `**/config/strat/panel.conf`** (hand-rolled TOML parser in `stratpanel/src/config.rs`).

## How it connects

- Compositor IPC over `**/run/stratvm.sock`** — see `stratpanel/src/ipc.rs`.
- Started by the compositor autostart path (`stratvm/src/main.c`), not necessarily a `stratman` manifest.

## Agent brief

[../agent/stratpanel.md](../agent/stratpanel.md)