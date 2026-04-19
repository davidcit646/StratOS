# stratman — agent brief

## Paths

- `stratman/src/main.rs` — PID 1, `mount_filesystems`, `service::load_and_run_all`, `emergency_shell`.
- `stratman/src/service.rs` — manifest parse, spawn, reap, namespaces.
- `stratman/src/maint.rs` — idle maintenance queue.
- `stratman/src/network.rs` — DHCP / interface logic; invoked via `--network`.
- `stratman/manifests/*.toml` — `stratwm`, `strat-wpa` (`wpa_supplicant`), `strat-network`, `seatd`, maintenance tasks, etc.

## Invariants

- PID 1 must not exit unexpectedly; failures tend to drop to `emergency_shell`.
- Respect **Custom first**: avoid new heavy deps unless checklist explicitly allows.
- Mount assumptions: often `/dev/sda5`–`sda7` in Rust path — keep aligned with `sysroot/initramfs-init.c` and GPT layout scripts.

## Grep starters

`rg "load_and_run_all|parse_manifest|strat-network|strat-wpa" stratman`

## Human doc

[../human/stratman.md](../human/stratman.md)