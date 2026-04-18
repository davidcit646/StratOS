#include "partition.h"

#include <efidevp.h>

#define STRAT_IO_CHUNK_BYTES (1024 * 1024)

static EFI_STATUS partition_number_from_name(const CHAR16 *name, UINT32 *out_number) {
    if (name == NULL || out_number == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    if (StrCmp((CHAR16 *)name, L"ESP") == 0) {
        *out_number = 1;
        return EFI_SUCCESS;
    }
    if (StrCmp((CHAR16 *)name, L"SLOT_A") == 0) {
        *out_number = 2;
        return EFI_SUCCESS;
    }
    if (StrCmp((CHAR16 *)name, L"SLOT_B") == 0) {
        *out_number = 3;
        return EFI_SUCCESS;
    }
    if (StrCmp((CHAR16 *)name, L"SLOT_C") == 0) {
        *out_number = 4;
        return EFI_SUCCESS;
    }
    if (StrCmp((CHAR16 *)name, L"CONFIG") == 0) {
        *out_number = 5;
        return EFI_SUCCESS;
    }
    if (StrCmp((CHAR16 *)name, L"STRAT_CACHE") == 0) {
        *out_number = 6;
        return EFI_SUCCESS;
    }
    if (StrCmp((CHAR16 *)name, L"HOME") == 0) {
        *out_number = 7;
        return EFI_SUCCESS;
    }

    return EFI_NOT_FOUND;
}

static EFI_STATUS partition_number_from_handle(
    EFI_SYSTEM_TABLE *st,
    EFI_HANDLE handle,
    UINT32 *out_number
) {
    if (st == NULL || st->BootServices == NULL || out_number == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    EFI_DEVICE_PATH *dp = NULL;
    EFI_STATUS status = uefi_call_wrapper(
        st->BootServices->HandleProtocol,
        3,
        handle,
        &DevicePathProtocol,
        (VOID **)&dp
    );
    if (status != EFI_SUCCESS || dp == NULL) {
        return (status != EFI_SUCCESS) ? status : EFI_NOT_FOUND;
    }

    EFI_DEVICE_PATH_PROTOCOL *node = (EFI_DEVICE_PATH_PROTOCOL *)dp;
    while (!IsDevicePathEnd(node)) {
        if (DevicePathType(node) == MEDIA_DEVICE_PATH &&
            DevicePathSubType(node) == MEDIA_HARDDRIVE_DP) {
            if ((UINTN)DevicePathNodeLength(node) < sizeof(HARDDRIVE_DEVICE_PATH)) {
                return EFI_COMPROMISED_DATA;
            }

            HARDDRIVE_DEVICE_PATH *hd = (HARDDRIVE_DEVICE_PATH *)node;
            *out_number = hd->PartitionNumber;
            return EFI_SUCCESS;
        }
        node = NextDevicePathNode(node);
    }

    return EFI_NOT_FOUND;
}

static UINT64 min_u64(UINT64 a, UINT64 b) {
    return (a < b) ? a : b;
}

static BOOLEAN gpt_header_valid(const UINT8 *hdr, UINTN size) {
    if (hdr == NULL || size < 8) {
        return FALSE;
    }
    return (CompareMem(hdr, "EFI PART", 8) == 0);
}

EFI_STATUS strat_find_partition_by_name(
    EFI_SYSTEM_TABLE *st,
    const CHAR16 *name,
    EFI_BLOCK_IO **out_bio
) {
    return strat_find_partition_by_name_ex(st, name, NULL, out_bio);
}

EFI_STATUS strat_find_partition_by_name_ex(
    EFI_SYSTEM_TABLE *st,
    const CHAR16 *name,
    EFI_HANDLE *out_handle,
    EFI_BLOCK_IO **out_bio
) {
    if (st == NULL || st->BootServices == NULL || name == NULL || out_bio == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    if (out_handle != NULL) {
        *out_handle = NULL;
    }
    *out_bio = NULL;

    UINT32 expected_partition_number = 0;
    EFI_STATUS status = partition_number_from_name(name, &expected_partition_number);
    if (status != EFI_SUCCESS) {
        return status;
    }

    EFI_HANDLE *handles = NULL;
    UINTN handle_count = 0;
    status = uefi_call_wrapper(
        st->BootServices->LocateHandleBuffer,
        5,
        ByProtocol,
        &BlockIoProtocol,
        NULL,
        &handle_count,
        &handles
    );
    if (status != EFI_SUCCESS) {
        return status;
    }

    EFI_STATUS result = EFI_NOT_FOUND;
    for (UINTN i = 0; i < handle_count; i++) {
        EFI_BLOCK_IO *bio = NULL;
        status = uefi_call_wrapper(
            st->BootServices->HandleProtocol,
            3,
            handles[i],
            &BlockIoProtocol,
            (VOID **)&bio
        );
        if (status != EFI_SUCCESS || bio == NULL || bio->Media == NULL || !bio->Media->LogicalPartition) {
            continue;
        }

        UINT32 partition_number = 0;
        status = partition_number_from_handle(st, handles[i], &partition_number);
        if (status != EFI_SUCCESS) {
            continue;
        }
        if (partition_number == expected_partition_number) {
            if (out_handle != NULL) {
                *out_handle = handles[i];
            }
            *out_bio = bio;
            result = EFI_SUCCESS;
            break;
        }
    }

    if (handles != NULL) {
        uefi_call_wrapper(st->BootServices->FreePool, 1, handles);
    }
    return result;
}

EFI_STATUS strat_partition_zero_header(EFI_BLOCK_IO *bio, UINTN zero_megabytes) {
    if (bio == NULL || bio->Media == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    if (zero_megabytes == 0) {
        return EFI_SUCCESS;
    }
    if (bio->Media->ReadOnly) {
        return EFI_WRITE_PROTECTED;
    }
    if (!bio->Media->MediaPresent) {
        return EFI_NO_MEDIA;
    }

    UINT64 block_size = bio->Media->BlockSize;
    if (block_size == 0) {
        return EFI_BAD_BUFFER_SIZE;
    }

    UINT64 total_blocks = (UINT64)bio->Media->LastBlock + 1ULL;
    UINT64 total_bytes = total_blocks * block_size;

    UINT64 requested_bytes = (UINT64)zero_megabytes * 1024ULL * 1024ULL;
    UINT64 bytes_to_zero = min_u64(requested_bytes, total_bytes);
    UINT64 blocks_to_zero = bytes_to_zero / block_size;
    if ((bytes_to_zero % block_size) != 0) {
        blocks_to_zero++;
    }
    if (blocks_to_zero == 0) {
        return EFI_SUCCESS;
    }

    UINT64 chunk_blocks = STRAT_IO_CHUNK_BYTES / block_size;
    if (chunk_blocks == 0) {
        chunk_blocks = 1;
    }
    UINTN chunk_bytes = (UINTN)(chunk_blocks * block_size);

    VOID *zero_buf = AllocateZeroPool(chunk_bytes);
    if (zero_buf == NULL) {
        return EFI_OUT_OF_RESOURCES;
    }

    EFI_STATUS status = EFI_SUCCESS;
    UINT64 lba = 0;
    while (lba < blocks_to_zero) {
        UINT64 this_blocks = min_u64(chunk_blocks, blocks_to_zero - lba);
        UINTN this_bytes = (UINTN)(this_blocks * block_size);

        status = uefi_call_wrapper(bio->WriteBlocks, 5, bio, bio->Media->MediaId, (EFI_LBA)lba, this_bytes, zero_buf);
        if (status != EFI_SUCCESS) {
            break;
        }
        lba += this_blocks;
    }

    if (status == EFI_SUCCESS) {
        status = uefi_call_wrapper(bio->FlushBlocks, 1, bio);
    }

    FreePool(zero_buf);
    return status;
}

EFI_STATUS strat_partition_copy(EFI_BLOCK_IO *src_bio, EFI_BLOCK_IO *dst_bio) {
    if (src_bio == NULL || dst_bio == NULL || src_bio->Media == NULL || dst_bio->Media == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    if (!src_bio->Media->MediaPresent || !dst_bio->Media->MediaPresent) {
        return EFI_NO_MEDIA;
    }
    if (dst_bio->Media->ReadOnly) {
        return EFI_WRITE_PROTECTED;
    }
    if (src_bio->Media->BlockSize == 0 || dst_bio->Media->BlockSize == 0) {
        return EFI_BAD_BUFFER_SIZE;
    }
    if (src_bio->Media->BlockSize != dst_bio->Media->BlockSize ||
        src_bio->Media->LastBlock != dst_bio->Media->LastBlock) {
        return EFI_BAD_BUFFER_SIZE;
    }

    UINT64 block_size = src_bio->Media->BlockSize;
    UINT64 total_blocks = (UINT64)src_bio->Media->LastBlock + 1ULL;

    UINT64 chunk_blocks = STRAT_IO_CHUNK_BYTES / block_size;
    if (chunk_blocks == 0) {
        chunk_blocks = 1;
    }
    UINTN chunk_bytes = (UINTN)(chunk_blocks * block_size);

    VOID *buffer = AllocatePool(chunk_bytes);
    if (buffer == NULL) {
        return EFI_OUT_OF_RESOURCES;
    }

    EFI_STATUS status = EFI_SUCCESS;
    UINT64 lba = 0;
    while (lba < total_blocks) {
        UINT64 this_blocks = min_u64(chunk_blocks, total_blocks - lba);
        UINTN this_bytes = (UINTN)(this_blocks * block_size);

        status = uefi_call_wrapper(src_bio->ReadBlocks, 5, src_bio, src_bio->Media->MediaId, (EFI_LBA)lba, this_bytes, buffer);
        if (status != EFI_SUCCESS) {
            break;
        }

        status = uefi_call_wrapper(dst_bio->WriteBlocks, 5, dst_bio, dst_bio->Media->MediaId, (EFI_LBA)lba, this_bytes, buffer);
        if (status != EFI_SUCCESS) {
            break;
        }

        lba += this_blocks;
    }

    if (status == EFI_SUCCESS) {
        status = uefi_call_wrapper(dst_bio->FlushBlocks, 1, dst_bio);
    }

    FreePool(buffer);
    return status;
}

EFI_STATUS strat_partition_get_partuuid(
    EFI_SYSTEM_TABLE *st,
    EFI_HANDLE handle,
    CHAR16 *out_partuuid,
    UINTN out_size
) {
    if (st == NULL || st->BootServices == NULL || handle == NULL || out_partuuid == NULL || out_size < 37) {
        return EFI_INVALID_PARAMETER;
    }

    EFI_DEVICE_PATH *dp = NULL;
    EFI_STATUS status = uefi_call_wrapper(
        st->BootServices->HandleProtocol,
        3,
        handle,
        &DevicePathProtocol,
        (VOID **)&dp
    );
    if (status != EFI_SUCCESS || dp == NULL) {
        return (status != EFI_SUCCESS) ? status : EFI_NOT_FOUND;
    }

    EFI_DEVICE_PATH_PROTOCOL *node = (EFI_DEVICE_PATH_PROTOCOL *)dp;
    while (!IsDevicePathEnd(node)) {
        if (DevicePathType(node) == MEDIA_DEVICE_PATH &&
            DevicePathSubType(node) == MEDIA_HARDDRIVE_DP) {
            if ((UINTN)DevicePathNodeLength(node) < sizeof(HARDDRIVE_DEVICE_PATH)) {
                return EFI_COMPROMISED_DATA;
            }

            HARDDRIVE_DEVICE_PATH *hd = (HARDDRIVE_DEVICE_PATH *)node;
            if (hd->SignatureType != SIGNATURE_TYPE_GUID) {
                // Fallback: locate the whole-disk BlockIo and read GPT partition entry.
                // Some firmware/device paths provide an MBR-style signature here.
                // The partition number is still useful.
                UINT32 part_number = hd->PartitionNumber;
                if (part_number == 0) {
                    return EFI_NOT_FOUND;
                }

                UINTN prefix_len = (UINTN)((UINT8 *)node - (UINT8 *)dp);
                if (prefix_len < sizeof(EFI_DEVICE_PATH_PROTOCOL)) {
                    return EFI_NOT_FOUND;
                }

                // Find the whole-disk BlockIo handle by matching device-path prefix.
                EFI_HANDLE *bio_handles = NULL;
                UINTN bio_count = 0;
                EFI_STATUS loc_status = uefi_call_wrapper(
                    st->BootServices->LocateHandleBuffer,
                    5,
                    ByProtocol,
                    &BlockIoProtocol,
                    NULL,
                    &bio_count,
                    &bio_handles
                );
                if (loc_status != EFI_SUCCESS) {
                    return loc_status;
                }

                EFI_BLOCK_IO *disk_bio = NULL;
                for (UINTN hi = 0; hi < bio_count; hi++) {
                    EFI_BLOCK_IO *candidate = NULL;
                    EFI_STATUS h_status = uefi_call_wrapper(
                        st->BootServices->HandleProtocol,
                        3,
                        bio_handles[hi],
                        &BlockIoProtocol,
                        (VOID **)&candidate
                    );
                    if (h_status != EFI_SUCCESS || candidate == NULL || candidate->Media == NULL) {
                        continue;
                    }
                    if (candidate->Media->LogicalPartition) {
                        continue;
                    }

                    EFI_DEVICE_PATH *cand_dp = NULL;
                    EFI_STATUS dp_status = uefi_call_wrapper(
                        st->BootServices->HandleProtocol,
                        3,
                        bio_handles[hi],
                        &DevicePathProtocol,
                        (VOID **)&cand_dp
                    );
                    if (dp_status != EFI_SUCCESS || cand_dp == NULL) {
                        continue;
                    }

                    if (CompareMem(cand_dp, dp, prefix_len) == 0) {
                        disk_bio = candidate;
                        break;
                    }
                }

                if (bio_handles != NULL) {
                    uefi_call_wrapper(st->BootServices->FreePool, 1, bio_handles);
                }

                if (disk_bio == NULL || disk_bio->Media == NULL) {
                    return EFI_NOT_FOUND;
                }
                if (!disk_bio->Media->MediaPresent) {
                    return EFI_NO_MEDIA;
                }
                if (disk_bio->Media->BlockSize == 0) {
                    return EFI_BAD_BUFFER_SIZE;
                }

                UINTN block_size = disk_bio->Media->BlockSize;
                UINT8 *gpt_header = AllocatePool(block_size);
                if (gpt_header == NULL) {
                    return EFI_OUT_OF_RESOURCES;
                }

                EFI_STATUS read_status = uefi_call_wrapper(
                    disk_bio->ReadBlocks,
                    5,
                    disk_bio,
                    disk_bio->Media->MediaId,
                    1,
                    block_size,
                    gpt_header
                );
                if (read_status != EFI_SUCCESS) {
                    FreePool(gpt_header);
                    return read_status;
                }

                if (!gpt_header_valid(gpt_header, block_size)) {
                    FreePool(gpt_header);
                    return EFI_NOT_FOUND;
                }

                // GPT header fields (little-endian) at fixed offsets.
                UINT64 part_entry_lba = 0;
                UINT32 num_part_entries = 0;
                UINT32 part_entry_size = 0;
                CopyMem(&part_entry_lba, gpt_header + 72, sizeof(part_entry_lba));
                CopyMem(&num_part_entries, gpt_header + 80, sizeof(num_part_entries));
                CopyMem(&part_entry_size, gpt_header + 84, sizeof(part_entry_size));
                FreePool(gpt_header);

                if (part_entry_lba == 0 || num_part_entries == 0 || part_entry_size < 128) {
                    return EFI_NOT_FOUND;
                }
                if (part_number > num_part_entries) {
                    return EFI_NOT_FOUND;
                }

                UINT64 index = (UINT64)(part_number - 1);
                UINT64 entries_per_lba = (UINT64)block_size / (UINT64)part_entry_size;
                if (entries_per_lba == 0) {
                    return EFI_NOT_FOUND;
                }

                UINT64 entry_lba = part_entry_lba + (index / entries_per_lba);
                UINTN entry_offset = (UINTN)((index % entries_per_lba) * (UINT64)part_entry_size);

                UINT8 *entry_block = AllocatePool(block_size);
                if (entry_block == NULL) {
                    return EFI_OUT_OF_RESOURCES;
                }

                read_status = uefi_call_wrapper(
                    disk_bio->ReadBlocks,
                    5,
                    disk_bio,
                    disk_bio->Media->MediaId,
                    (EFI_LBA)entry_lba,
                    block_size,
                    entry_block
                );
                if (read_status != EFI_SUCCESS) {
                    FreePool(entry_block);
                    return read_status;
                }

                EFI_GUID *part_guid = (EFI_GUID *)(entry_block + entry_offset + 16);
                SPrint(out_partuuid, out_size * sizeof(CHAR16),
                       L"%08x-%04x-%04x-%02x%02x-%02x%02x%02x%02x%02x%02x",
                       part_guid->Data1,
                       part_guid->Data2,
                       part_guid->Data3,
                       part_guid->Data4[0], part_guid->Data4[1],
                       part_guid->Data4[2], part_guid->Data4[3],
                       part_guid->Data4[4], part_guid->Data4[5],
                       part_guid->Data4[6], part_guid->Data4[7]);
                FreePool(entry_block);
                return EFI_SUCCESS;
            }

            EFI_GUID *part_guid = (EFI_GUID *)hd->Signature;
            SPrint(out_partuuid, out_size * sizeof(CHAR16),
                   L"%08x-%04x-%04x-%02x%02x-%02x%02x%02x%02x%02x%02x",
                   part_guid->Data1,
                   part_guid->Data2,
                   part_guid->Data3,
                   part_guid->Data4[0], part_guid->Data4[1],
                   part_guid->Data4[2], part_guid->Data4[3],
                   part_guid->Data4[4], part_guid->Data4[5],
                   part_guid->Data4[6], part_guid->Data4[7]);

            return EFI_SUCCESS;
        }
        node = NextDevicePathNode(node);
    }

    return EFI_NOT_FOUND;
}
