# stratvm (stratwm) — agent brief

## Paths

- `stratvm/src/main.c` — main server loop, input, IPC, layer shell, xdg shell, grabs.
- `stratvm/src/server.h` — `struct stratwm_server`, `layers_bg` … `layers_overlay`, workspaces.
- `stratvm/Makefile` — links wlroots, produces `stratwm` binary.

## IPC

- Socket: `/run/stratvm.sock` (see `main.c` around `stratwm_ipc_listen`).
- Panel client: `stratpanel/src/ipc.rs`.

## Hotspots

- Focus / Z-order: `focus_view`, `wlr_scene_node_raise_to_top` (must stay within `layers_normal`).
- Move grab: `stratwm_apply_move_grab`, `grabbed_view`, `view_request_move_notify`.
- Layer shell: `server_new_layer_surface_notify`, `layer_surface_commit_notify` (reparent by layer).

## Human doc

[../human/stratvm.md](../human/stratvm.md)
