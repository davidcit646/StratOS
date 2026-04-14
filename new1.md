# StratOS Update Orchestration: StratMon → StratBoot Slot Writer

## Purpose

Implement the StratOS update path where:

1. StratMon downloads the update.
2. StratMon verifies the update.
3. StratMon prepares a bootloader-readable update request.
4. StratMon requests update mode via EFI variables.
5. StratBoot performs the actual system slot write before normal OS boot.
6. Boot validation confirms the new slot or rolls back.

StratMon is the conductor. StratBoot is the surgeon.

---

## Non-Negotiable Architecture Rules

### StratMon MAY:
- Check for updates.
- Download update artifacts.
- Verify signatures and hashes.
- Stage update payloads in STRAT_CACHE.
- Create an update manifest.
- Set EFI variables requesting update mode.
- Request reboot/kexec into update flow.

### StratMon MUST NOT:
- Write to `/system`.
- Write directly to SLOT_A, SLOT_B, or SLOT_C.
- Overwrite the active slot.
- Overwrite the pinned slot.
- Treat an ISO as the normal update artifact.

### StratBoot MUST:
- Own the final system slot write.
- Run the update operation before normal OS mount.
- Use EFI variables as source of truth.
- Refuse unsafe targets.
- Leave the current bootable system untouched on failure.
- Reboot into the new staged slot only after successful write and verification.

---

## Terminology

Use these terms exactly:

- **System image**: the EROFS image used to populate a system slot.
- **Update payload**: downloaded system image plus metadata/signature.
- **Update manifest**: small bootloader-readable file describing where the payload lives on disk.
- **Target slot**: inactive, non-pinned system slot to receive the update.
- **Current slot**: slot currently booted.
- **Pinned slot**: sacred fallback slot. Never overwrite automatically.

Do not call the normal update artifact an ISO. ISO is for live USB / installer media.

---

## Recommended Design

StratBoot should not need an XFS filesystem driver.

Instead:

1. StratMon downloads the system image into STRAT_CACHE.
2. StratMon verifies the image.
3. StratMon uses FIEMAP to record the physical disk extents of the staged image file.
4. StratMon writes a small update manifest to the ESP.
5. StratMon sets EFI update variables.
6. StratBoot reads the manifest from ESP.
7. StratBoot uses EFI_BLOCK_IO_PROTOCOL to copy raw source extents into the target system slot.
8. StratBoot hashes bytes during copy and compares against the expected hash.
9. StratBoot marks the target slot as staging and makes it active.
10. Boot validation confirms or rolls back.

This avoids teaching StratBoot XFS while still keeping partition writes under StratBoot control.

---

## New EFI Variables

Add these to the existing StratOS EFI namespace.

```c
STRAT_UPDATE_STATE        uint8
  0 = none
  1 = pending
  2 = applying
  3 = applied_waiting_validation
  4 = failed

STRAT_UPDATE_TARGET_SLOT  uint8
  0 = SLOT_A
  1 = SLOT_B
  2 = SLOT_C

STRAT_UPDATE_RESULT       uint8
  0 = none
  1 = success
  2 = bad_manifest
  3 = bad_signature
  4 = bad_hash
  5 = unsafe_target
  6 = block_io_failed
  7 = write_failed

STRAT_UPDATE_BOOT_COUNT   uint8
  Attempts to boot newly staged slot before rollback.

Do not overload STRAT_RESET_FLAGS for normal updates. Resets and updates are separate flows.

Update Manifest Format

Path on ESP:

/EFI/StratOS/update/update.manifest

Suggested packed struct:

#define STRAT_UPDATE_MAGIC 0x5354524154555044ULL /* "STRATUPD" */
#define STRAT_UPDATE_MANIFEST_VERSION 1
#define STRAT_MAX_UPDATE_EXTENTS 512

typedef struct {
    uint64_t source_lba;
    uint64_t byte_len;
} StratUpdateExtent;

typedef struct {
    uint64_t magic;
    uint32_t version;
    uint32_t manifest_size;

    uint8_t target_slot;
    uint8_t reserved[7];

    uint8_t image_sha256[32];
    uint64_t image_size;

    uint32_t extent_count;
    StratUpdateExtent extents[STRAT_MAX_UPDATE_EXTENTS];

    uint32_t manifest_crc32;
} StratUpdateManifest;

Manifest rules:

Must fit on ESP.
Must include image SHA256.
Must include exact byte size.
Must include raw physical extents.
Must include CRC32 over the manifest excluding manifest_crc32.
StratBoot must reject malformed, oversized, or inconsistent manifests.
StratMon Implementation

Recommended module:

stratsup/src/update.rs

Or, if the repo now has a dedicated StratMon crate:

stratmon/src/update.rs
StratMon update flow
check_for_update()
    ↓
download system image to STRAT_CACHE
    ↓
verify signature
    ↓
verify sha256
    ↓
select safe target slot
    ↓
fsync payload file
    ↓
collect physical extents with FIEMAP
    ↓
write update manifest to ESP
    ↓
set EFI vars:
    STRAT_UPDATE_STATE = pending
    STRAT_UPDATE_TARGET_SLOT = target
    STRAT_UPDATE_RESULT = none
    ↓
request reboot into StratBoot update mode
Target slot selection

Rules:

Never target the active slot.
Never target the pinned slot.
Prefer SLOT_C if available and not active/pinned.
Otherwise choose the inactive, non-pinned slot with oldest/non-confirmed status.
If no safe slot exists, fail cleanly.
StratMon user-facing text

Use calm direct messages:

"Update downloaded. Switching systems on next restart."
"Update could not be verified. Nothing was changed."
"No safe update slot is available. Nothing was changed."
StratBoot Implementation

Add files:

stratboot/src/update.c
stratboot/src/update.h
stratboot/src/update_manifest.c
stratboot/src/update_manifest.h
StratBoot update flow

In efi_main, after reset handling but before normal slot selection:

read STRAT_UPDATE_STATE

if STRAT_UPDATE_STATE == pending:
    set STRAT_UPDATE_STATE = applying
    load update manifest from ESP
    validate manifest
    validate target slot is safe
    locate target partition by GPT name
    copy source extents to target partition
    hash while copying
    compare final SHA256
    if success:
        set target slot status = staging
        set STRAT_ACTIVE_SLOT = target
        set STRAT_UPDATE_STATE = applied_waiting_validation
        warm reboot
    if failure:
        set STRAT_UPDATE_STATE = failed
        set STRAT_UPDATE_RESULT = exact failure code
        do not change active slot
        boot last good slot
StratBoot must reject update if:
Manifest missing.
Manifest CRC invalid.
Magic/version invalid.
Target slot is active.
Target slot is pinned.
Extent list is empty.
Extent list exceeds max.
Image hash mismatch.
Block read/write fails.
Image size exceeds target partition size.
Critical safety rule

If the update fails at any point:

Do not clear current active slot.
Do not mark current active slot bad.
Do not touch pinned slot.
Do not boot the partially written target as confirmed.

The failure mode must be:

"The update didn't apply. Nothing is broken."
Boot Validation Integration

After booting a newly staged slot:

Boot validation service confirms /system, /config, /home, /apps, /usr, and /var mounts.
If validation passes:
Mark active slot confirmed.
Set STRAT_LAST_GOOD_SLOT = active slot.
Set STRAT_UPDATE_STATE = none.
Set STRAT_UPDATE_RESULT = success.
If validation fails:
Increment boot count.
If boot count exceeds threshold, mark staging slot bad.
Restore STRAT_ACTIVE_SLOT = STRAT_LAST_GOOD_SLOT.
Reboot.
Session Suspension

Do not implement true session suspension in v1.

For v1:

StratMon prepares update → reboot → StratBoot applies update → boot new slot

Later, Phase 15 may add:

CRIU checkpoint → kexec/update path → restore session

But that is not required for this task.

Tests Required
Unit tests
Manifest CRC validation.
Manifest magic/version rejection.
Hash mismatch rejection.
Unsafe active-slot target rejection.
Unsafe pinned-slot target rejection.
Extent overflow rejection.
QEMU tests

Required PASS criteria:

Update pending detected
Manifest loaded
Target slot selected
Image copied
Hash verified
Target slot marked staging
Booting selected slot
No fatal signatures

Fatal signatures include:

X64 Exception Type
Kernel panic
VFS: Unable to mount root fs
BUG:
Oops:
Regression tests
Active slot is never overwritten.
Pinned slot is never overwritten.
Failed update leaves old active slot bootable.
Bad hash prevents slot activation.
Missing manifest does not brick system.
Files Likely Touched

Expected:

stratboot/src/update.c
stratboot/src/update.h
stratboot/src/update_manifest.c
stratboot/src/update_manifest.h
stratboot/src/stratboot.c
stratboot/src/efi_vars.c
stratboot/src/efi_vars.h
stratsup/src/update.rs
stratsup/src/efi_vars.rs
scripts/tests/update/
tests/update/
TALKING.md

Do not touch unrelated compositor, terminal, app store, or ISO builder files unless a test proves they are involved.

Required TALKING.md Entry

After each file touch, Codex must log:

- YYYY-MM-DD (Codex): [files touched]
  What changed:
  Why:
  Validation:
  Build output:
  QEMU result:

No “pending” validation accepted as complete.

PASS Definition

This task is complete only when:

StratMon can stage a verified update request.
StratBoot detects pending update mode.
StratBoot writes only the safe inactive target slot.
StratBoot verifies the written image hash.
StratBoot activates the new slot as staging.
Boot validation confirms or rolls back.
QEMU smoke passes.
Auditor approves no spec violations.

Nothing ships without QEMU passing.