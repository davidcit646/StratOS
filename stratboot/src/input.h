#ifndef STRAT_INPUT_H
#define STRAT_INPUT_H

#include <efi.h>
#include <efilib.h>

typedef struct {
    SIMPLE_INPUT_INTERFACE *text_in;
} StratInput;

EFI_STATUS strat_input_init(EFI_SYSTEM_TABLE *st, StratInput *out);
EFI_STATUS strat_input_poll(StratInput *input, EFI_INPUT_KEY *out_key);
EFI_STATUS strat_input_wait(StratInput *input, EFI_INPUT_KEY *out_key);

#endif
