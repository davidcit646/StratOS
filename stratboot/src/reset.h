#ifndef STRAT_RESET_H
#define STRAT_RESET_H

#include <efi.h>
#include <efilib.h>
#include "../efi/strat_efi_vars.h"
#include "slot.h"
#include "partition.h"

#define STRAT_RESET_FLAG_CONFIG  0x01
#define STRAT_RESET_FLAG_HOME    0x02
#define STRAT_RESET_FLAG_SYSTEM  0x04
#define STRAT_RESET_FLAG_FACTORY 0x08

typedef struct {
    UINT8 flags;
} StratResetState;

EFI_STATUS strat_reset_read(EFI_RUNTIME_SERVICES *rt, StratResetState *out);
EFI_STATUS strat_reset_clear(EFI_RUNTIME_SERVICES *rt);
EFI_STATUS strat_execute_resets(EFI_SYSTEM_TABLE *st, UINT8 flags);
const CHAR8 *strat_reset_describe(UINT8 flags);

#endif
