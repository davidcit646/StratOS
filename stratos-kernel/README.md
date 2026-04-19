# StratOS Kernel Config

This directory contains StratOS kernel configuration fragments.

Canonical merge fragment for CI and `./build-all-and-run.sh`: **`stratos.config`**
(merged after `make defconfig`). The **`stratos_minimal.config`** snapshot is not
used by that script and is kept only as a historical / experimental reference; see
its file header.

Start from an LTS kernel tree and apply `stratos.config` on top of
the base `defconfig`. This keeps the upstream default while enabling
StratOS-required features.

Typical flow (from the kernel tree):

```
make defconfig
scripts/kconfig/merge_config.sh -m .config /path/to/StratOS/stratos-kernel/stratos.config
make olddefconfig
```

Kernel build and boot tests are deferred to hardware in this environment.
