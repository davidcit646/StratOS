#ifndef STRAT_SHA256_H
#define STRAT_SHA256_H

#include <efi.h>
#include <efilib.h>
#include "slot.h"

typedef struct {
    UINT32 state[8];
    UINT8 data[64];
    UINT32 bitlen[2];
    UINTN datalen;
} SHA256_CTX;

EFI_STATUS strat_sha256_init(SHA256_CTX *ctx);
EFI_STATUS strat_sha256_update(SHA256_CTX *ctx, const UINT8 *data, UINTN len);
EFI_STATUS strat_sha256_final(SHA256_CTX *ctx, UINT8 *digest);
EFI_STATUS strat_slot_verify_hash(EFI_SYSTEM_TABLE *st, StratSlotId slot, const UINT8 *expected_hash);

#endif
