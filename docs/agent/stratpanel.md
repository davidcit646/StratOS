# stratpanel — agent brief

## Paths

- `stratpanel/src/main.rs` — Wayland bind, SHM buffer, render loop, input.
- `stratpanel/src/ipc.rs` — `stratvm.sock` line protocol client.
- `stratpanel/src/config.rs` — `/config/strat/panel.conf`, `pinned`, `tray`, `panel`, `clock`.
- `stratpanel/src/clock.rs`, `textinput.rs`.

## Invariants

- Custom Wayland types: `stratlayer` only (no `wayland-client` crate in panel).
- Config on CONFIG partition; see [../human/runtime-persistence-contract.md](../human/runtime-persistence-contract.md).

## Open work (see coding-checklist Phase 24)

- Pinned strip UI if `pinned.apps` non-empty.
- Tray widgets beyond clock.
- Auto-hide animation vs flag-only IPC.

## Human doc

[../human/stratpanel.md](../human/stratpanel.md)