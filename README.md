# StratOS: Update Path Architecture (StratMon & StratBoot)

## I. Core Philosophy
StratOS implements a "Custom First" approach to system reliability. We minimize dependencies by building purpose-built components in **Rust** (User-space) and **C** (Low-level boot). The update system is designed to be atomic, immutable, and safe from filesystem-level corruption during the write process.

---

## II. The Conductor: StratMon
**StratMon** is the user-space conductor responsible for the "untrusted" phase of the update.

### Rules & Constraints
- **MAY**: Download update payloads, verify cryptographic signatures, and stage images.
- **MUST NOT**: Write directly to `/system` or any system slot (SLOT_A, SLOT_B, SLOT_C).
- **Format**: Updates are handled as **System images** (EROFS), never as ISOs.

### Implementation Logic (Rust)
1. **Acquisition**: Downloads the **Update payload** into `STRAT_CACHE`.
2. **Verification**: Validates the payload signature and hash.
3. **FIEMAP Mapping**: Since StratBoot lacks an XFS driver, StratMon uses `ioctl(FS_IOC_FIEMAP)` to identify the exact physical disk extents (block ranges) of the staged image file.
4. **Manifest Creation**: Writes a small, bootloader-readable **Update manifest** to the ESP. This file contains the physical block map of the source image.
5. **State Signaling**: Sets EFI variables (e.g., `Strat_Update_Pending`) to signal a requested transition to the **Target slot**.
6. **Trigger**: Initiates a reboot.

---

## III. The Surgeon: StratBoot
**StratBoot** is the low-level surgeon. It owns the hardware and performs the write in a pre-boot environment where the system is quiescent.

### Rules & Constraints
- **Ownership**: Owns the final system slot write.
- **Environment**: Runs before the normal OS mount.
- **Integrity**: Uses EFI variables as the source of truth and refuses unsafe or unverified targets.

### Implementation Logic (C)
1. **Detection**: On boot, checks EFI variables for an update request.
2. **Manifest Read**: Accesses the **Update manifest** from the ESP to find the source blocks in `STRAT_CACHE`.
3. **The Raw Copy**: Uses `EFI_BLOCK_IO_PROTOCOL` to perform a raw copy from the source physical extents directly into the **Target slot** (the inactive, non-pinned slot).
4. **Validation**: Calculates the hash of the bytes during the copy process; compares against the expected hash in the manifest.
5. **Slot Rotation**: 
   - If successful: Marks the **Target slot** as active.
   - If failed: Leaves the **Current slot** untouched and
