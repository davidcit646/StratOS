#ifndef STRAT_FONT_H
#define STRAT_FONT_H

#include <efi.h>
#include <efilib.h>
#include "gop.h"

#define STRAT_FONT_WIDTH 8
#define STRAT_FONT_HEIGHT 8

// Draws a single ASCII character (32..127) at x,y using the given color.
EFI_STATUS strat_font_draw_char(
    StratGop *gop,
    INT32 x,
    INT32 y,
    CHAR8 ch,
    UINT8 r,
    UINT8 g,
    UINT8 b
);

// Draws ASCII text at x,y. Handles '\n' and '\r'.
// Truncates at the right/bottom edge.
EFI_STATUS strat_font_draw_text(
    StratGop *gop,
    INT32 x,
    INT32 y,
    const CHAR8 *text,
    UINT8 r,
    UINT8 g,
    UINT8 b
);

#endif
