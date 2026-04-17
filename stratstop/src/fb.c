#include "fb.h"
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <string.h>

int fb_open(struct framebuffer *fb, const char *device) {
    if (fb == NULL || device == NULL) {
        return -1;
    }

    fb->fd = open(device, O_RDWR);
    if (fb->fd < 0) {
        return -1;
    }

    struct fb_fix_screeninfo finfo;
    struct fb_var_screeninfo vinfo;

    if (ioctl(fb->fd, FBIOGET_FSCREENINFO, &finfo) < 0) {
        close(fb->fd);
        return -1;
    }

    if (ioctl(fb->fd, FBIOGET_VSCREENINFO, &vinfo) < 0) {
        close(fb->fd);
        return -1;
    }

    fb->width = vinfo.xres;
    fb->height = vinfo.yres;
    fb->bpp = vinfo.bits_per_pixel;
    fb->stride = finfo.line_length;
    fb->size = finfo.smem_len;

    fb->data = mmap(NULL, fb->size, PROT_READ | PROT_WRITE, MAP_SHARED, fb->fd, 0);
    if (fb->data == MAP_FAILED) {
        close(fb->fd);
        return -1;
    }

    return 0;
}

void fb_close(struct framebuffer *fb) {
    if (fb == NULL) {
        return;
    }

    if (fb->data != NULL && fb->data != MAP_FAILED) {
        munmap(fb->data, fb->size);
    }

    if (fb->fd >= 0) {
        close(fb->fd);
    }

    fb->data = NULL;
    fb->fd = -1;
}

void fb_put_pixel(struct framebuffer *fb, uint32_t x, uint32_t y, uint8_t r, uint8_t g, uint8_t b) {
    if (fb == NULL || fb->data == NULL) {
        return;
    }

    if (x >= fb->width || y >= fb->height) {
        return;
    }

    uint32_t offset = y * fb->stride + x * (fb->bpp / 8);
    uint8_t *pixel = fb->data + offset;

    if (fb->bpp == 32) {
        // Assume BGRA or RGBA - check red offset
        // For simplicity, use ARGB format (little-endian: BGRA)
        pixel[0] = b;
        pixel[1] = g;
        pixel[2] = r;
        pixel[3] = 0xFF;
    } else if (fb->bpp == 24) {
        pixel[0] = b;
        pixel[1] = g;
        pixel[2] = r;
    } else if (fb->bpp == 16) {
        // RGB565
        uint16_t color = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3);
        *((uint16_t *)pixel) = color;
    }
}

void fb_fill_rect(struct framebuffer *fb, uint32_t x, uint32_t y, uint32_t w, uint32_t h, uint8_t r, uint8_t g, uint8_t b) {
    if (fb == NULL || fb->data == NULL) {
        return;
    }

    for (uint32_t py = y; py < y + h && py < fb->height; py++) {
        for (uint32_t px = x; px < x + w && px < fb->width; px++) {
            fb_put_pixel(fb, px, py, r, g, b);
        }
    }
}

void fb_clear(struct framebuffer *fb) {
    if (fb == NULL || fb->data == NULL) {
        return;
    }

    memset(fb->data, 0, fb->size);
}
