# StratMon — agent brief

## Paths

- `stratmon/src/main.rs` — CLI, `--stage-update`, EFI var writes via `stratsup::efi_vars`.
- `stratmon/src/manifest.rs` — on-disk manifest structs / write.
- `stratmon/src/fiemap.rs` — `FS_IOC_FIEMAP` mapping.

## Paths on disk (staging)

- Manifest target path appears as `/EFI/STRAT/UPDATE.MAN` in sources — **requires ESP mounted** at that prefix on the host where stratmon runs (dev caveat).

## Invariants

- Must not write SLOT partitions from this crate; StratBoot owns block writes.
- Uses `sha2` crate for hashing today — note checklist / Custom-first policy if touching deps.

## Human doc

[../human/stratmon.md](../human/stratmon.md)