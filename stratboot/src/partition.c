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

EFI_STATUS strat_find_partition_by_name(
    EFI_SYSTEM_TABLE *st,
    const CHAR16 *name,
    EFI_BLOCK_IO **out_bio
) {
    if (st == NULL || st->BootServices == NULL || name == NULL || out_bio == NULL) {
        return EFI_INVALID_PARAMETER;
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
    EFI_BLOCK_IO *bio,
    CHAR16 *out_partuuid,
    UINTN out_size
) {
    if (bio == NULL || bio->Media == NULL || out_partuuid == NULL || out_size < 37) {
        return EFI_INVALID_PARAMETER;
    }
    if (!bio->Media->MediaPresent) {
        return EFI_NO_MEDIA;
    }
    if (bio->Media->BlockSize == 0) {
        return EFI_BAD_BUFFER_SIZE;
    }

    UINT64 block_size = bio->Media->BlockSize;
    
    UINT8 *gpt_header = AllocatePool(block_size);
    if (gpt_header == NULL) {
        return EFI_OUT_OF_RESOURCES;
    }

    EFI_STATUS status = uefi_call_wrapper(bio->ReadBlocks, 5, bio, bio->Media->MediaId, 1, block_size, gpt_header);
    if (status != EFI_SUCCESS) {
        FreePool(gpt_header);
        return status;
    }

    UINT8 *signature = gpt_header;
    if (signature[0] != 'P' || signature[1] != 'A' || signature[2] != 'R' || signature[3] != 'T') {
        FreePool(gpt_header);
        return EFI_NOT_FOUND;
    }

    UINT32 part_entry_lba = *(UINT32 *)(gpt_header + 72);
    UINT32 num_part_entries = *(UINT32 *)(gpt_header + 80);
    UINT32 part_entry_size = *(UINT32 *)(gpt_header + 84);
    FreePool(gpt_header);

    if (part_entry_lba == 0 || num_part_entries == 0 || part_entry_size < 128) {
        return EFI_NOT_FOUND;
    }

    UINT8 *part_entry = AllocatePool(part_entry_size);
    if (part_entry == NULL) {
        return EFI_OUT_OF_RESOURCES;
    }

    status = uefi_call_wrapper(bio->ReadBlocks, 5, bio, bio->Media->MediaId, part_entry_lba, part_entry_size, part_entry);
    if (status != EFI_SUCCESS) {
        FreePool(part_entry);
        return status;
    }

    EFI_GUID *part_guid = (EFI_GUID *)(part_entry + 56);
    SPrint(out_partuuid, out_size,
           L"%08x-%04x-%04x-%02x%02x-%02x%02x%02x%02x%02x%02x",
           part_guid->Data1,
           part_guid->Data2,
           part_guid->Data3,
           part_guid->Data4[0], part_guid->Data4[1],
           part_guid->Data4[2], part_guid->Data4[3],
           part_guid->Data4[4], part_guid->Data4[5],
           part_guid->Data4[6], part_guid->Data4[7]);

    FreePool(part_entry);
    return EFI_SUCCESS;
}
