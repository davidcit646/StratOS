# stratpanel — agent brief

## Paths

- `stratpanel/src/main.rs` — Wayland bind, SHM buffer, render loop, input.
- `stratpanel/src/ipc.rs` — `stratvm.sock` line protocol client.
- `stratpanel/src/config.rs` — `StratSettings::load()` → panel/clock/pinned/workspace/tray; legacy `panel.conf` only if no `settings.toml`.
- `stratpanel/src/clock.rs`, `textinput.rs`.

## Invariants

- Custom Wayland types: `stratlayer` only (no `wayland-client` crate in panel).
- Config on CONFIG partition; see [../human/runtime-persistence-contract.md](../human/runtime-persistence-contract.md).

## Open work (see coding-checklist Phase 24 / 17)

Phase **24** panel items (pinned strip, workspace IPC, clock, N/V/U/B tray toggles, autohide) are implemented in-tree; see [../human/coding-checklist.md](../human/coding-checklist.md). Remaining polish: richer **volume** / **network** integration (Phase **17+**), less stubby tray **V**/**U**, optional animation tuning.

Task prompts: [prompts/panel-flesh-out.md](prompts/panel-flesh-out.md), [prompts/panel-window-chrome.md](prompts/panel-window-chrome.md).

## Human doc

[../human/stratpanel.md](../human/stratpanel.md)