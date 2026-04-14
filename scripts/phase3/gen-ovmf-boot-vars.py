#!/usr/bin/env python3
"""
gen-ovmf-boot-vars.py — Inject a Boot0001 EFI load option into an OVMF_VARS.fd image.

Usage:
    python3 gen-ovmf-boot-vars.py <ovmf_vars_template> <output_vars>

Hardcoded parameters (must match create-gpt-image.sh):
  - Partition GUID:   4A3B2C1D-5E6F-7A8B-9C0D-E1F2A3B4C5D6  (fixed ESP unique GUID)
  - PCI device:       PciRoot(0x0)/Pci(0x3,0x0)              (virtio-scsi-pci)
  - SCSI target:      Scsi(0x0,0x0)                           (scsi-hd, bus 0 target 0 lun 0)
  - Partition:        HD(1,GPT,<GUID>,0x800,0x1F77F)          (ESP, start=2048, size=128991)
  - Boot file:        \\\\EFI\\\\BOOT\\\\BOOTX64.EFI

The script adds two EFI NVRAM variables to the (authenticated) variable store:
  - BootOrder  (UINT16[1] = {0x0001})
  - Boot0001   (EFI_LOAD_OPTION with the full device path above)

The OVMF_VARS.fd uses the EDK2 "Authenticated Variable Store" GUID:
  AAF32C78-947B-439A-A180-2E144EC37792
Variable records start immediately after the EFI_VARIABLE_STORE_HEADER.
"""

import struct
import sys
import os


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# UEFI Global Variable GUID: {8BE4DF61-93CA-11D2-AA0D-00E098032B8C}
EFI_GLOBAL_VARIABLE_GUID = bytes([
    0x61, 0xDF, 0xE4, 0x8B,   # DWORD (LE)
    0xCA, 0x93,               # WORD  (LE)
    0xD2, 0x11,               # WORD  (LE)
    0xAA, 0x0D, 0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C,  # 8 bytes
])

# Variable attributes: NV | BS | RT
VARIABLE_ATTRS = 0x00000007

# Variable header constants (authenticated store)
VAR_START_ID = 0x55AA
VAR_ADDED    = 0x3F   # State: all bits set (written)

# Fixed ESP partition GUID: 4A3B2C1D-5E6F-7A8B-9C0D-E1F2A3B4C5D6
# Stored in UEFI GUID binary format: {DWORD-LE, WORD-LE, WORD-LE, 8-bytes}
PART_GUID_BYTES = bytes([
    0x1D, 0x2C, 0x3B, 0x4A,   # 0x4A3B2C1D LE
    0x6F, 0x5E,               # 0x5E6F LE
    0x8B, 0x7A,               # 0x7A8B LE
    0x9C, 0x0D, 0xE1, 0xF2, 0xA3, 0xB4, 0xC5, 0xD6,
])

# ESP partition parameters
PART_NUMBER = 1
PART_START  = 2048          # LBA
PART_SIZE   = 128991        # sectors (131038 - 2048 + 1)


# ---------------------------------------------------------------------------
# EFI Device Path helpers
# ---------------------------------------------------------------------------

def dp_node(typ, sub, data):
    """Build a device path node: Type, SubType, Length (LE-16), Data."""
    length = 4 + len(data)
    return bytes([typ, sub]) + struct.pack('<H', length) + data

def dp_acpi_pciroot(uid=0):
    """ACPI(PciRoot(uid)) — Type 2, SubType 1"""
    hid = 0x0A0341D0  # EISA ID for PNP0A03
    return dp_node(0x02, 0x01, struct.pack('<II', hid, uid))

def dp_pci(device, function):
    """PCI(device, function) — Type 1, SubType 1"""
    return dp_node(0x01, 0x01, bytes([function, device]))

def dp_scsi(pun, lun):
    """SCSI(pun, lun) — Type 3, SubType 2 (MSG_SCSI_DP = 0x02 per UEFI spec)"""
    return dp_node(0x03, 0x02, struct.pack('<HH', pun, lun))

def dp_harddrive(part_num, part_start, part_size, part_guid_bytes):
    """HD(partNum, GPT, guid, start, size) — Type 4, SubType 1"""
    # PartitionNumber UINT32, PartitionStart UINT64, PartitionSize UINT64,
    # PartitionSignature[16], PartitionFormat UINT8, SignatureType UINT8
    data  = struct.pack('<I', part_num)
    data += struct.pack('<Q', part_start)
    data += struct.pack('<Q', part_size)
    data += part_guid_bytes            # 16 bytes
    data += bytes([0x02, 0x02])        # GPT format, GUID signature type
    return dp_node(0x04, 0x01, data)

def dp_filepath(path_str):
    """File path node — Type 4, SubType 4"""
    # Path is backslash-separated, stored as UTF-16LE with null terminator
    utf16 = path_str.encode('utf-16-le') + b'\x00\x00'
    return dp_node(0x04, 0x04, utf16)

def dp_end():
    """End of hardware device path — Type 0x7F, SubType 0xFF"""
    return dp_node(0x7F, 0xFF, b'')


def build_device_path():
    """
    PciRoot(0x0)/Pci(0x3,0x0)/Scsi(0x0,0x0)/HD(1,GPT,...,0x800,0x1F77F)/
        File(\\EFI\\BOOT\\BOOTX64.EFI)
    """
    return (
        dp_acpi_pciroot(0)
        + dp_pci(device=3, function=0)
        + dp_scsi(pun=0, lun=0)
        + dp_harddrive(PART_NUMBER, PART_START, PART_SIZE, PART_GUID_BYTES)
        + dp_filepath('\\EFI\\BOOT\\BOOTX64.EFI')
        + dp_end()
    )


# ---------------------------------------------------------------------------
# EFI Load Option (Boot####)
# ---------------------------------------------------------------------------

def build_load_option(description, device_path):
    """
    EFI_LOAD_OPTION:
      UINT32  Attributes
      UINT16  FilePathListLength
      CHAR16  Description[] (null-terminated)
      ...device path...
    """
    attrs = struct.pack('<I', 0x00000001)   # LOAD_OPTION_ACTIVE
    dp_len = struct.pack('<H', len(device_path))
    desc = description.encode('utf-16-le') + b'\x00\x00'
    return attrs + dp_len + desc + device_path


# ---------------------------------------------------------------------------
# Authenticated Variable Store record
# ---------------------------------------------------------------------------

def build_var_record(name_utf16, data, vendor_guid=EFI_GLOBAL_VARIABLE_GUID):
    """
    Build an AUTHENTICATED_VARIABLE_HEADER record + name + data.
    The header is 60 bytes (as used by OVMF's authenticated variable driver).

    Layout:
      UINT16  StartId       (0x55AA)
      UINT8   State         (0x3F)
      UINT8   Reserved
      UINT32  Attributes
      UINT64  MonotonicCount
      EFI_TIME TimeStamp    (16 bytes of zeros)
      UINT32  PubKeyIndex
      UINT32  NameSize      (bytes, not chars)
      UINT32  DataSize
      EFI_GUID VendorGuid   (16 bytes)
      [Name bytes]
      [Data bytes]
      [0–3 bytes of padding to 4-byte alignment]
    """
    name_bytes = name_utf16
    name_size  = len(name_bytes)
    data_size  = len(data)

    header  = struct.pack('<H', VAR_START_ID)          # StartId
    header += bytes([VAR_ADDED, 0x00])                 # State, Reserved
    header += struct.pack('<I', VARIABLE_ATTRS)        # Attributes
    header += struct.pack('<Q', 0)                     # MonotonicCount
    header += b'\x00' * 16                             # TimeStamp (EFI_TIME)
    header += struct.pack('<I', 0)                     # PubKeyIndex
    header += struct.pack('<I', name_size)             # NameSize
    header += struct.pack('<I', data_size)             # DataSize
    header += vendor_guid                              # VendorGuid

    payload = header + name_bytes + data

    # Pad to 4-byte boundary
    pad = (4 - (len(payload) % 4)) % 4
    payload += b'\x00' * pad
    return payload


# ---------------------------------------------------------------------------
# OVMF_VARS.fd manipulation
# ---------------------------------------------------------------------------

def find_var_store_offset(data):
    """
    Locate the EFI_VARIABLE_STORE_HEADER within the firmware volume.
    The FV header ends at FV_header.HeaderLength; the var store follows.
    HeaderLength is a UINT16 at offset 0x30 in the FV header.
    """
    # FV Signature "_FVH" is at offset 0x28
    if data[0x28:0x32] != b'_FVH\xff\xfe':
        # Try searching for it
        sig_off = data.find(b'_FVH')
        if sig_off < 0:
            raise ValueError('Cannot find EFI FV header signature (_FVH)')
    else:
        sig_off = 0x28

    # FV header starts at sig_off - 0x28 (the full FV header starts at 0)
    fv_start = sig_off - 0x28
    header_len = struct.unpack_from('<H', data, fv_start + 0x30)[0]
    var_store_offset = fv_start + header_len

    # Sanity: EFI_VARIABLE_STORE_HEADER is 28 bytes; first field is a GUID
    # We'll just return the offset; the Format byte at +16+4 should be 0x5A
    fmt_byte = data[var_store_offset + 20]  # offset 20 within var store header
    if fmt_byte != 0x5A:
        raise ValueError(
            f'Variable store Format byte at 0x{var_store_offset+20:X} '
            f'= 0x{fmt_byte:02X}, expected 0x5A (VARIABLE_STORE_FORMATTED)'
        )

    # Variable records start after the 28-byte var store header
    return var_store_offset + 28


def inject_variables(template_path, output_path):
    with open(template_path, 'rb') as f:
        data = bytearray(f.read())

    var_data_start = find_var_store_offset(data)
    print(f'Variable data area starts at offset 0x{var_data_start:X}')

    # Build the two variables to inject
    boot_order_name  = 'BootOrder'.encode('utf-16-le')
    boot_order_data  = struct.pack('<H', 0x0001)   # Boot0001 first

    boot0001_name    = 'Boot0001'.encode('utf-16-le')
    device_path      = build_device_path()
    boot0001_data    = build_load_option('StratBoot', device_path)

    rec_boot_order = build_var_record(boot_order_name, boot_order_data)
    rec_boot0001   = build_var_record(boot0001_name,   boot0001_data)

    print(f'BootOrder record: {len(rec_boot_order)} bytes')
    print(f'Boot0001 record:  {len(rec_boot0001)} bytes')
    print(f'Device path:      {len(device_path)} bytes')
    print(f'Load option:      {len(boot0001_data)} bytes')

    needed = len(rec_boot_order) + len(rec_boot0001)
    available = len(data) - var_data_start
    print(f'Space needed: {needed} bytes, available: {available} bytes')

    if needed > available:
        raise ValueError('Not enough space in variable store for boot entries')

    # Write at the start of the variable data area (store is empty = all 0xFF)
    offset = var_data_start
    if data[offset] != 0xFF:
        raise ValueError(
            f'Variable data area at 0x{offset:X} is not empty (0xFF) — '
            'refusing to overwrite existing variables'
        )

    data[offset:offset + len(rec_boot_order)] = rec_boot_order
    offset += len(rec_boot_order)
    data[offset:offset + len(rec_boot0001)] = rec_boot0001

    with open(output_path, 'wb') as f:
        f.write(data)

    print(f'Written: {output_path}')


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(f'Usage: {sys.argv[0]} <ovmf_vars_template> <output_vars>')
        sys.exit(1)

    template = sys.argv[1]
    output   = sys.argv[2]

    if not os.path.isfile(template):
        print(f'Error: template not found: {template}', file=sys.stderr)
        sys.exit(1)

    try:
        inject_variables(template, output)
    except Exception as e:
        print(f'Error: {e}', file=sys.stderr)
        sys.exit(1)
