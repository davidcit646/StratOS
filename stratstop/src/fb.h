#ifndef STRATSTOP_FB_H
#define STRATSTOP_FB_H

#include <stdint.h>
#include <stddef.h>

struct fb_var_screeninfo {
    uint32_t xres;
    uint32_t yres;
    uint32_t xres_virtual;
    uint32_t yres_virtual;
    uint32_t xoffset;
    uint32_t yoffset;
    uint32_t bits_per_pixel;
    uint32_t grayscale;
    uint32_t red;
    uint32_t green;
    uint32_t blue;
    uint32_t transp;
    uint32_t nonstd;
    uint32_t activate;
    uint32_t height;
    uint32_t width;
    uint32_t accel_flags;
    uint32_t pixclock;
    uint32_t left_margin;
    uint32_t right_margin;
    uint32_t upper_margin;
    uint32_t lower_margin;
    uint32_t hsync_len;
    uint32_t vsync_len;
    uint32_t sync;
    uint32_t vmode;
    uint32_t rotate;
    uint32_t colorspace;
    uint32_t reserved[4];
};

struct fb_fix_screeninfo {
    char id[16];
    uint32_t smem_start;
    uint32_t smem_len;
    uint32_t type;
    uint32_t type_aux;
    uint32_t visual;
    uint16_t xpanstep;
    uint16_t ypanstep;
    uint16_t ywrapstep;
    uint32_t line_length;
    uint32_t mmio_start;
    uint32_t mmio_len;
    uint32_t accel;
    uint16_t reserved[3];
};

#define FBIOGET_VSCREENINFO 0x4600
#define FBIOGET_FSCREENINFO 0x4602

struct framebuffer {
    int fd;
    uint8_t *data;
    size_t size;
    uint32_t width;
    uint32_t height;
    uint32_t bpp;
    uint32_t stride;
};

int fb_open(struct framebuffer *fb, const char *device);
void fb_close(struct framebuffer *fb);
void fb_put_pixel(struct framebuffer *fb, uint32_t x, uint32_t y, uint8_t r, uint8_t g, uint8_t b);
void fb_fill_rect(struct framebuffer *fb, uint32_t x, uint32_t y, uint32_t w, uint32_t h, uint8_t r, uint8_t g, uint8_t b);
void fb_clear(struct framebuffer *fb);

#endif
