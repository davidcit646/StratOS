# stratman (human guide)

**stratman** is **PID 1** on a normal StratOS boot: the initramfs execs `/bin/stratman` after mounting `/system` (EROFS), `/config`, `/apps`, and `/home` per the persistence contract.

## Responsibilities (today)

- Bring up basic mounts and environment (`stratman/src/main.rs`).
- Load **TOML service manifests** from `stratman/manifests/` and supervise child processes (`stratman/src/service.rs`).
- Run **maintenance** tasks when the machine is idle (`stratman/src/maint.rs`).
- **Networking:** `strat-wpa` runs `wpa_supplicant` when `/config/strat/wpa_supplicant.conf` has a `network={}` block; `strat-network` runs `stratman --network` (DHCP, `[network]` in `settings.toml`, optional `/config/strat/network.toml` shim). `interface = "auto"` prefers wired or USB Ethernet, then Wi-Fi.

## What it does *not* replace

- **StratBoot** still owns firmware-time slot surgery.
- **stratvm** (compositor) autostarts the panel and terminal from its own code; the panel is **not** always a stratman manifest service—check `stratvm/src/main.c` autostart list vs `stratman/manifests/`.

## Related reading

- [runtime-persistence-contract.md](runtime-persistence-contract.md) — mount expectations.
- [coding-checklist.md](coding-checklist.md) — Phase **12b**.
- Agent file: [../agent/stratman.md](../agent/stratman.md).

