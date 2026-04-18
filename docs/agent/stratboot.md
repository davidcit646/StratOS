# StratBoot — agent brief

## Scope

- **Tree:** `stratboot/` (C, gnu-efi). Output: `BOOTX64.EFI` → copied to ESP `EFI/BOOT/` and used with slot paths under `EFI/STRAT/SLOT_`*.
- **Entry:** `stratboot/src/stratboot.c` (`efi_main`).
- **Slots / state:** `stratboot/src/slot.c`, `stratboot/src/reset.c`, `stratboot/efi/strat_efi_vars.c` + `strat_efi_vars.h`.
- **Partitions:** `stratboot/src/partition.c` (GPT name → partition index; `strat_partition_get_partuuid`, block I/O).
- **Kernel handoff:** `start_kernel_efi` — cmdline includes `root=PARTUUID=…`, `rootfstype=erofs`, initrd path.

## Invariants

- StratBoot **owns** raw slot writes / surgery; user-space **StratMon** does not write slot partitions directly (see [stratos-design.md](../human/stratos-design.md)).
- Do not reintroduce removed EFI vars without updating `efi-variables.md` and `efi_var_test.c`.

## Tests / build

- `stratboot/Makefile`, `stratboot/tests/efi_var_test.c`.
- Host needs gnu-efi / elf linker scripts (see Makefile `check` target).

## Human doc

[../human/stratboot.md](../human/stratboot.md)