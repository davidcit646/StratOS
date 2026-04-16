#ifndef STRAT_PARTITION_H
#define STRAT_PARTITION_H

#include <efi.h>
#include <efilib.h>

EFI_STATUS strat_find_partition_by_name(
    EFI_SYSTEM_TABLE *st,
    const CHAR16 *name,
    EFI_BLOCK_IO **out_bio
);

EFI_STATUS strat_partition_zero_header(EFI_BLOCK_IO *bio, UINTN zero_megabytes);
EFI_STATUS strat_partition_copy(EFI_BLOCK_IO *src_bio, EFI_BLOCK_IO *dst_bio);

EFI_STATUS strat_partition_get_partuuid(
    EFI_BLOCK_IO *bio,
    CHAR16 *out_partuuid,
    UINTN out_size
);

#endif
