#include "sha256.h"
#include "partition.h"

#define STRAT_IO_CHUNK_BYTES (1024 * 1024)
#define SHA256_BLOCK_SIZE 64

static const UINT32 K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

static UINT32 ROTRIGHT(UINT32 a, UINT32 b) {
    return (a >> b) | (a << (32 - b));
}

static UINT32 CH(UINT32 x, UINT32 y, UINT32 z) {
    return (x & y) ^ (~x & z);
}

static UINT32 MAJ(UINT32 x, UINT32 y, UINT32 z) {
    return (x & y) ^ (x & z) ^ (y & z);
}

static UINT32 EP0(UINT32 x) {
    return ROTRIGHT(x, 2) ^ ROTRIGHT(x, 13) ^ ROTRIGHT(x, 22);
}

static UINT32 EP1(UINT32 x) {
    return ROTRIGHT(x, 6) ^ ROTRIGHT(x, 11) ^ ROTRIGHT(x, 25);
}

static UINT32 SIG0(UINT32 x) {
    return ROTRIGHT(x, 7) ^ ROTRIGHT(x, 18) ^ (x >> 3);
}

static UINT32 SIG1(UINT32 x) {
    return ROTRIGHT(x, 17) ^ ROTRIGHT(x, 19) ^ (x >> 10);
}

static void transform(SHA256_CTX *ctx, const UINT8 data[64]) {
    UINT32 a, b, c, d, e, f, g, h, i, j, t1, t2, m[64];

    for (i = 0, j = 0; i < 16; ++i, j += 4) {
        m[i] = (data[j] << 24) | (data[j + 1] << 16) | (data[j + 2] << 8) | (data[j + 3]);
    }
    for (; i < 64; ++i) {
        m[i] = SIG1(m[i - 2]) + m[i - 7] + SIG0(m[i - 15]) + m[i - 16];
    }

    a = ctx->state[0];
    b = ctx->state[1];
    c = ctx->state[2];
    d = ctx->state[3];
    e = ctx->state[4];
    f = ctx->state[5];
    g = ctx->state[6];
    h = ctx->state[7];

    for (i = 0; i < 64; ++i) {
        t1 = h + EP1(e) + CH(e, f, g) + K[i] + m[i];
        t2 = EP0(a) + MAJ(a, b, c);
        h = g;
        g = f;
        f = e;
        e = d + t1;
        d = c;
        c = b;
        b = a;
        a = t1 + t2;
    }

    ctx->state[0] += a;
    ctx->state[1] += b;
    ctx->state[2] += c;
    ctx->state[3] += d;
    ctx->state[4] += e;
    ctx->state[5] += f;
    ctx->state[6] += g;
    ctx->state[7] += h;
}

EFI_STATUS strat_sha256_init(SHA256_CTX *ctx) {
    if (ctx == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    ctx->datalen = 0;
    ctx->bitlen[0] = 0;
    ctx->bitlen[1] = 0;
    ctx->state[0] = 0x6a09e667;
    ctx->state[1] = 0xbb67ae85;
    ctx->state[2] = 0x3c6ef372;
    ctx->state[3] = 0xa54ff53a;
    ctx->state[4] = 0x510e527f;
    ctx->state[5] = 0x9b05688c;
    ctx->state[6] = 0x1f83d9ab;
    ctx->state[7] = 0x5be0cd19;

    return EFI_SUCCESS;
}

EFI_STATUS strat_sha256_update(SHA256_CTX *ctx, const UINT8 *data, UINTN len) {
    if (ctx == NULL || (data == NULL && len > 0)) {
        return EFI_INVALID_PARAMETER;
    }

    for (UINTN i = 0; i < len; ++i) {
        ctx->data[ctx->datalen] = data[i];
        ctx->datalen++;
        if (ctx->datalen == 64) {
            transform(ctx, ctx->data);
            ctx->bitlen[0] += 512;
            if (ctx->bitlen[0] < 512) {
                ctx->bitlen[1]++;
            }
            ctx->datalen = 0;
        }
    }

    return EFI_SUCCESS;
}

EFI_STATUS strat_sha256_final(SHA256_CTX *ctx, UINT8 *digest) {
    if (ctx == NULL || digest == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    UINTN i = ctx->datalen;

    if (ctx->datalen < 56) {
        ctx->data[i++] = 0x80;
        while (i < 56) {
            ctx->data[i++] = 0x00;
        }
    } else {
        ctx->data[i++] = 0x80;
        while (i < 64) {
            ctx->data[i++] = 0x00;
        }
        transform(ctx, ctx->data);
        for (i = 0; i < 56; ++i) {
            ctx->data[i] = 0x00;
        }
    }

    ctx->bitlen[0] += ctx->datalen * 8;
    if (ctx->bitlen[0] < ctx->datalen * 8) {
        ctx->bitlen[1]++;
    }
    ctx->bitlen[0] <<= 3;
    if (ctx->bitlen[0] == 0) {
        ctx->bitlen[1] <<= 3;
    }

    ctx->data[63] = ctx->bitlen[0];
    ctx->data[62] = ctx->bitlen[0] >> 8;
    ctx->data[61] = ctx->bitlen[0] >> 16;
    ctx->data[60] = ctx->bitlen[0] >> 24;
    ctx->data[59] = ctx->bitlen[1];
    ctx->data[58] = ctx->bitlen[1] >> 8;
    ctx->data[57] = ctx->bitlen[1] >> 16;
    ctx->data[56] = ctx->bitlen[1] >> 24;

    transform(ctx, ctx->data);

    for (i = 0; i < 4; ++i) {
        digest[i] = (ctx->state[0] >> (24 - i * 8)) & 0x000000ff;
        digest[i + 4] = (ctx->state[1] >> (24 - i * 8)) & 0x000000ff;
        digest[i + 8] = (ctx->state[2] >> (24 - i * 8)) & 0x000000ff;
        digest[i + 12] = (ctx->state[3] >> (24 - i * 8)) & 0x000000ff;
        digest[i + 16] = (ctx->state[4] >> (24 - i * 8)) & 0x000000ff;
        digest[i + 18] = (ctx->state[5] >> (24 - i * 8)) & 0x000000ff;
        digest[i + 20] = (ctx->state[6] >> (24 - i * 8)) & 0x000000ff;
        digest[i + 24] = (ctx->state[7] >> (24 - i * 8)) & 0x000000ff;
    }

    return EFI_SUCCESS;
}

static const CHAR16 *slot_name_from_id(StratSlotId slot) {
    switch (slot) {
        case STRAT_SLOT_A: return L"SLOT_A";
        case STRAT_SLOT_B: return L"SLOT_B";
        case STRAT_SLOT_C: return L"SLOT_C";
        default: return NULL;
    }
}

EFI_STATUS strat_slot_verify_hash(EFI_SYSTEM_TABLE *st, StratSlotId slot, const UINT8 *expected_hash) {
    if (st == NULL || st->BootServices == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    if (slot != STRAT_SLOT_A && slot != STRAT_SLOT_B && slot != STRAT_SLOT_C) {
        return EFI_INVALID_PARAMETER;
    }

    if (expected_hash == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    const CHAR16 *slot_name = slot_name_from_id(slot);
    if (slot_name == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    EFI_BLOCK_IO *bio = NULL;
    EFI_STATUS status = strat_find_partition_by_name(st, slot_name, &bio);
    if (status != EFI_SUCCESS || bio == NULL) {
        return (status != EFI_SUCCESS) ? status : EFI_NOT_FOUND;
    }

    if (bio->Media == NULL || !bio->Media->MediaPresent) {
        return EFI_NO_MEDIA;
    }

    if (bio->Media->BlockSize == 0) {
        return EFI_BAD_BUFFER_SIZE;
    }

    UINT64 block_size = bio->Media->BlockSize;
    UINT64 total_blocks = (UINT64)bio->Media->LastBlock + 1ULL;

    UINT64 chunk_blocks = STRAT_IO_CHUNK_BYTES / block_size;
    if (chunk_blocks == 0) {
        chunk_blocks = 1;
    }
    UINTN chunk_bytes = (UINTN)(chunk_blocks * block_size);

    VOID *buffer = AllocatePool(chunk_bytes);
    if (buffer == NULL) {
        return EFI_OUT_OF_RESOURCES;
    }

    SHA256_CTX ctx;
    status = strat_sha256_init(&ctx);
    if (status != EFI_SUCCESS) {
        FreePool(buffer);
        return status;
    }

    UINT64 lba = 0;
    while (lba < total_blocks) {
        UINT64 this_blocks = (total_blocks - lba < chunk_blocks) ? (total_blocks - lba) : chunk_blocks;
        UINTN this_bytes = (UINTN)(this_blocks * block_size);

        status = uefi_call_wrapper(bio->ReadBlocks, 5, bio, bio->Media->MediaId, (EFI_LBA)lba, this_bytes, buffer);
        if (status != EFI_SUCCESS) {
            FreePool(buffer);
            return status;
        }

        status = strat_sha256_update(&ctx, buffer, this_bytes);
        if (status != EFI_SUCCESS) {
            FreePool(buffer);
            return status;
        }

        lba += this_blocks;
    }

    UINT8 computed_hash[32];
    status = strat_sha256_final(&ctx, computed_hash);
    if (status != EFI_SUCCESS) {
        FreePool(buffer);
        return status;
    }

    FreePool(buffer);

    for (UINTN i = 0; i < 32; i++) {
        if (computed_hash[i] != expected_hash[i]) {
            return EFI_SECURITY_VIOLATION;
        }
    }

    return EFI_SUCCESS;
}
