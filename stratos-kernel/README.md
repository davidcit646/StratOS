# StratOS Kernel Config

This directory contains StratOS kernel configuration fragments.

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
