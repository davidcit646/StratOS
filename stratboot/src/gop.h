#ifndef STRAT_GOP_H
#define STRAT_GOP_H

#include <efi.h>
#include <efilib.h>

typedef struct {
    EFI_GRAPHICS_OUTPUT_PROTOCOL *gop;
    EFI_PHYSICAL_ADDRESS fb_base;
    UINTN fb_size;
    UINT32 width;
    UINT32 height;
    UINT32 pixels_per_scanline;
    EFI_GRAPHICS_PIXEL_FORMAT format;
} StratGop;

EFI_STATUS strat_gop_init(EFI_SYSTEM_TABLE *st, StratGop *out);
EFI_STATUS strat_gop_clear(StratGop *gop, UINT8 r, UINT8 g, UINT8 b);
EFI_STATUS strat_gop_put_pixel(StratGop *gop, INT32 x, INT32 y, UINT8 r, UINT8 g, UINT8 b);
EFI_STATUS strat_gop_fill_rect(StratGop *gop, INT32 x, INT32 y, INT32 w, INT32 h, UINT8 r, UINT8 g, UINT8 b);
EFI_STATUS strat_gop_draw_line(StratGop *gop, INT32 x0, INT32 y0, INT32 x1, INT32 y1, UINT8 r, UINT8 g, UINT8 b);
EFI_STATUS strat_gop_draw_circle(StratGop *gop, INT32 cx, INT32 cy, INT32 radius, UINT8 r, UINT8 g, UINT8 b);
UINTN strat_gop_width(StratGop *gop);
UINTN strat_gop_height(StratGop *gop);

#endif
