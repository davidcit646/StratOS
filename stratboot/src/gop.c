#include "gop.h"

static EFI_STATUS gop_supported_format(EFI_GRAPHICS_PIXEL_FORMAT fmt) {
    if (fmt == PixelBlueGreenRedReserved8BitPerColor ||
        fmt == PixelRedGreenBlueReserved8BitPerColor) {
        return EFI_SUCCESS;
    }
    return EFI_UNSUPPORTED;
}

static UINT32 pack_pixel(EFI_GRAPHICS_PIXEL_FORMAT fmt, UINT8 r, UINT8 g, UINT8 b) {
    if (fmt == PixelRedGreenBlueReserved8BitPerColor) {
        return ((UINT32)r) | ((UINT32)g << 8) | ((UINT32)b << 16);
    }
    return ((UINT32)b) | ((UINT32)g << 8) | ((UINT32)r << 16);
}

static EFI_STATUS gop_store_active_mode(StratGop *out, EFI_GRAPHICS_OUTPUT_PROTOCOL *gop) {
    if (out == NULL || gop == NULL || gop->Mode == NULL || gop->Mode->Info == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    out->gop = gop;
    out->fb_base = gop->Mode->FrameBufferBase;
    out->fb_size = gop->Mode->FrameBufferSize;
    out->width = gop->Mode->Info->HorizontalResolution;
    out->height = gop->Mode->Info->VerticalResolution;
    out->pixels_per_scanline = gop->Mode->Info->PixelsPerScanLine;
    out->format = gop->Mode->Info->PixelFormat;

    return EFI_SUCCESS;
}

EFI_STATUS strat_gop_init(EFI_SYSTEM_TABLE *st, StratGop *out) {
    if (st == NULL || out == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    EFI_GUID gop_guid = EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID;
    EFI_GRAPHICS_OUTPUT_PROTOCOL *gop = NULL;
    EFI_STATUS status = uefi_call_wrapper(
        st->BootServices->LocateProtocol,
        3,
        &gop_guid,
        NULL,
        (VOID **)&gop
    );
    if (status != EFI_SUCCESS || gop == NULL) {
        return status;
    }

    status = gop_store_active_mode(out, gop);
    if (status != EFI_SUCCESS) {
        return EFI_DEVICE_ERROR;
    }

    if (gop_supported_format(out->format) == EFI_SUCCESS) {
        return EFI_SUCCESS;
    }

    // Some firmware boots in an unsupported GOP format (for example PixelBitMask).
    // Try switching to the first mode with a supported packed RGB/BGR layout.
    for (UINT32 mode = 0; mode < gop->Mode->MaxMode; mode++) {
        EFI_GRAPHICS_OUTPUT_MODE_INFORMATION *info = NULL;
        UINTN info_size = 0;
        EFI_STATUS query_status = uefi_call_wrapper(
            gop->QueryMode,
            4,
            gop,
            mode,
            &info_size,
            &info
        );
        if (query_status != EFI_SUCCESS || info == NULL) {
            continue;
        }

        EFI_GRAPHICS_PIXEL_FORMAT mode_format = info->PixelFormat;
        uefi_call_wrapper(st->BootServices->FreePool, 1, info);
        if (gop_supported_format(mode_format) != EFI_SUCCESS) {
            continue;
        }

        EFI_STATUS set_status = uefi_call_wrapper(gop->SetMode, 2, gop, mode);
        if (set_status != EFI_SUCCESS) {
            continue;
        }

        status = gop_store_active_mode(out, gop);
        if (status == EFI_SUCCESS && gop_supported_format(out->format) == EFI_SUCCESS) {
            return EFI_SUCCESS;
        }
    }

    return EFI_UNSUPPORTED;
}

EFI_STATUS strat_gop_clear(StratGop *gop, UINT8 r, UINT8 g, UINT8 b) {
    if (gop == NULL || gop->gop == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    return strat_gop_fill_rect(gop, 0, 0, (INT32)gop->width, (INT32)gop->height, r, g, b);
}

EFI_STATUS strat_gop_put_pixel(StratGop *gop, INT32 x, INT32 y, UINT8 r, UINT8 g, UINT8 b) {
    if (gop == NULL || gop->gop == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    if (x < 0 || y < 0 || x >= (INT32)gop->width || y >= (INT32)gop->height) {
        return EFI_INVALID_PARAMETER;
    }
    if (gop_supported_format(gop->format) != EFI_SUCCESS) {
        return EFI_UNSUPPORTED;
    }

    UINT32 *fb = (UINT32 *)(UINTN)gop->fb_base;
    UINTN index = (UINTN)y * gop->pixels_per_scanline + (UINTN)x;
    fb[index] = pack_pixel(gop->format, r, g, b);
    return EFI_SUCCESS;
}

EFI_STATUS strat_gop_fill_rect(StratGop *gop, INT32 x, INT32 y, INT32 w, INT32 h, UINT8 r, UINT8 g, UINT8 b) {
    if (gop == NULL || gop->gop == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    if (w <= 0 || h <= 0) {
        return EFI_INVALID_PARAMETER;
    }
    if (gop_supported_format(gop->format) != EFI_SUCCESS) {
        return EFI_UNSUPPORTED;
    }

    INT32 x0 = x < 0 ? 0 : x;
    INT32 y0 = y < 0 ? 0 : y;
    INT32 x1 = x + w;
    INT32 y1 = y + h;

    if (x1 > (INT32)gop->width) x1 = (INT32)gop->width;
    if (y1 > (INT32)gop->height) y1 = (INT32)gop->height;

    UINT32 color = pack_pixel(gop->format, r, g, b);
    UINT32 *fb = (UINT32 *)(UINTN)gop->fb_base;

    for (INT32 yy = y0; yy < y1; yy++) {
        UINTN row = (UINTN)yy * gop->pixels_per_scanline;
        for (INT32 xx = x0; xx < x1; xx++) {
            fb[row + (UINTN)xx] = color;
        }
    }

    return EFI_SUCCESS;
}

EFI_STATUS strat_gop_draw_line(StratGop *gop, INT32 x0, INT32 y0, INT32 x1, INT32 y1, UINT8 r, UINT8 g, UINT8 b) {
    if (gop == NULL || gop->gop == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    if (gop_supported_format(gop->format) != EFI_SUCCESS) {
        return EFI_UNSUPPORTED;
    }

    INT32 dx = (x1 > x0) ? (x1 - x0) : (x0 - x1);
    INT32 sx = (x0 < x1) ? 1 : -1;
    // Bresenham's algorithm expects a negative dy when y increases.
    INT32 dy = (y1 > y0) ? (y0 - y1) : (y1 - y0);
    INT32 sy = (y0 < y1) ? 1 : -1;
    INT32 err = dx + dy;

    while (1) {
        strat_gop_put_pixel(gop, x0, y0, r, g, b);
        if (x0 == x1 && y0 == y1) {
            break;
        }
        INT32 e2 = 2 * err;
        if (e2 >= dy) {
            err += dy;
            x0 += sx;
        }
        if (e2 <= dx) {
            err += dx;
            y0 += sy;
        }
    }

    return EFI_SUCCESS;
}

EFI_STATUS strat_gop_draw_circle(StratGop *gop, INT32 cx, INT32 cy, INT32 radius, UINT8 r, UINT8 g, UINT8 b) {
    if (gop == NULL || gop->gop == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    if (radius <= 0) {
        return EFI_INVALID_PARAMETER;
    }
    if (gop_supported_format(gop->format) != EFI_SUCCESS) {
        return EFI_UNSUPPORTED;
    }

    INT32 x = radius;
    INT32 y = 0;
    INT32 err = 1 - x;

    while (x >= y) {
        strat_gop_put_pixel(gop, cx + x, cy + y, r, g, b);
        strat_gop_put_pixel(gop, cx + y, cy + x, r, g, b);
        strat_gop_put_pixel(gop, cx - y, cy + x, r, g, b);
        strat_gop_put_pixel(gop, cx - x, cy + y, r, g, b);
        strat_gop_put_pixel(gop, cx - x, cy - y, r, g, b);
        strat_gop_put_pixel(gop, cx - y, cy - x, r, g, b);
        strat_gop_put_pixel(gop, cx + y, cy - x, r, g, b);
        strat_gop_put_pixel(gop, cx + x, cy - y, r, g, b);

        y++;
        if (err < 0) {
            err += 2 * y + 1;
        } else {
            x--;
            err += 2 * (y - x) + 1;
        }
    }

    return EFI_SUCCESS;
}

UINTN strat_gop_width(StratGop *gop) {
    if (gop == NULL) {
        return 0;
    }
    return (UINTN)gop->width;
}

UINTN strat_gop_height(StratGop *gop) {
    if (gop == NULL) {
        return 0;
    }
    return (UINTN)gop->height;
}
