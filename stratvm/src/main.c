#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <fcntl.h>
#include <linux/input-event-codes.h>
#include <linux/netlink.h>
#include <spawn.h>
#include <assert.h>
#include <sys/un.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <dirent.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
#include <libevdev/libevdev.h>

#include <wayland-server-core.h>
#include <wayland-server-protocol.h>
#include <xkbcommon/xkbcommon.h>
#include <xkbcommon/xkbcommon-keysyms.h>

#include <wlr/backend.h>
#include <wlr/backend/multi.h>
#include <wlr/render/wlr_renderer.h>
#include <wlr/types/wlr_compositor.h>
#include <wlr/types/wlr_surface.h>
#ifdef STRATWM_HAVE_WLR_SUBCOMPOSITOR
#include <wlr/types/wlr_subcompositor.h>
#endif
#include <wlr/types/wlr_cursor.h>
#include <wlr/types/wlr_data_device.h>
#include <wlr/types/wlr_keyboard.h>
#include <wlr/types/wlr_output.h>
#include <wlr/types/wlr_output_layout.h>
#include <wlr/types/wlr_pointer.h>
#include <wlr/types/wlr_scene.h>
#include <wlr/types/wlr_seat.h>
#include <wlr/types/wlr_xcursor_manager.h>
#include <wlr/types/wlr_xdg_shell.h>
#include <wlr/types/wlr_layer_shell_v1.h>
#include <wlr/interfaces/wlr_keyboard.h>
#include <wlr/interfaces/wlr_pointer.h>
#include <wlr/util/box.h>
#include <wlr/util/log.h>

#include "server.h"

struct stratwm_output {
    struct wl_list link;
    struct stratwm_server *server;
    struct wlr_output *wlr_output;
    struct wlr_scene_output *scene_output;
    struct wlr_scene_rect *background;
    struct wl_listener frame;
    struct wl_listener destroy;
};

struct stratwm_view {
    struct wl_list link;
    struct stratwm_server *server;
    struct wlr_xdg_toplevel *xdg_toplevel;
    struct wlr_scene_tree *scene_tree;
    struct wlr_scene_rect *border;  /* Focused/unfocused visual indicator (Phase 8.3) */
    int workspace_id;  /* Which workspace owns this view */
    bool is_floating;  /* Escape tiling; render at fixed position (Phase 8.5) */
    int float_x, float_y;  /* Floating window position */
    
    /* Titlebar (Phase 8.8) */
    struct wlr_scene_rect *titlebar_bg;     /* Titlebar background */
    struct wlr_scene_rect *close_button;    /* Close button (X) */
    struct wlr_scene_rect *max_button;      /* Maximize button */
    struct wlr_scene_rect *min_button;      /* Minimize button */
    bool decorations_visible;               /* Titlebar + window controls (right-click toggles) */

    struct wl_listener map;
    struct wl_listener unmap;
    struct wl_listener commit;
    struct wl_listener destroy;
    struct wl_listener request_move;
};

struct stratwm_keyboard {
    struct wl_list link;
    struct stratwm_server *server;
    struct wlr_input_device *device;
    struct wl_listener modifiers;
    struct wl_listener key;
    struct wl_listener destroy;
};

/* Direct evdev input device (bypassing libinput/udev) */
#define MAX_EVDEV_DEVICES 16
#define UEVENT_BUFFER_SIZE 2048

struct stratwm_evdev_device {
    struct wl_list link;
    struct stratwm_server *server;
    char path[256];
    int fd;
    struct libevdev *evdev;
    struct wl_event_source *event_source;
    bool is_keyboard;
    bool is_pointer;

    /* Wlroots device - either keyboard or pointer */
    union {
        struct wlr_keyboard *keyboard;
        struct wlr_pointer *pointer;
        struct wlr_input_device *device;
    } wlr;
};

struct stratwm_input_manager {
    struct wl_list devices;  /* stratwm_evdev_device.link */
    int uevent_fd;           /* netlink socket for hotplug */
    struct wl_event_source *uevent_source;
};

static const float STRAT_BG[4] = {0.102f, 0.102f, 0.180f, 1.0f};
static void update_view_border(struct stratwm_view *view, bool focused);
static void update_titlebar_buttons(struct stratwm_view *view, int width);
static uint32_t stratwm_seat_modifiers(struct stratwm_server *server);
static void set_view_decorations_visible(struct stratwm_view *view, bool visible);
static bool titlebar_context_region(struct stratwm_view *view, double vx, double vy);
static void move_view_to_workspace(struct stratwm_server *server, struct stratwm_view *view, int new_id);
static void stratwm_load_deco_config(struct stratwm_server *server);
static void stratwm_load_modular_chrome(struct stratwm_server *server);
static void stratwm_refresh_tiled_visibility(struct stratwm_server *server);

static void spawn_terminal(void) {
    pid_t pid = fork();
    if (pid < 0) {
#ifdef DEBUG
        fprintf(stderr, "stratwm: fork failed: %s\n", strerror(errno));
#endif
        return;
    }
    if (pid == 0) {
        setsid();
        /* StratOS terminal first, then generic host fallbacks */
        execl("/bin/stratterm", "stratterm", (char *)NULL);
        execl("/usr/bin/stratterm", "stratterm", (char *)NULL);
        execl("/usr/bin/alacritty", "alacritty", (char *)NULL);
        execl("/usr/bin/xterm", "xterm", (char *)NULL);
        execl("/usr/bin/flatpak-spawn", "flatpak-spawn", "--host", "stratterm", (char *)NULL);
        execl("/usr/bin/flatpak-spawn", "flatpak-spawn", "--host", "alacritty", (char *)NULL);
        execl("/usr/bin/flatpak-spawn", "flatpak-spawn", "--host", "xterm", (char *)NULL);
#ifdef DEBUG
        fprintf(stderr, "stratwm: no terminal found\n");
#endif
        _exit(127);
    }
}

static void spawn_autostart(const char *path, const char *wayland_display) {
    if (access(path, X_OK) != 0) {
        fprintf(stderr, "stratwm: autostart missing or not executable: %s\n", path);
        return;
    }
    pid_t pid = fork();
    if (pid < 0) {
        fprintf(stderr, "stratwm: autostart fork failed for %s: %s\n", path, strerror(errno));
        return;
    }
    if (pid == 0) {
        if (wayland_display != NULL && wayland_display[0] != '\0') {
            setenv("WAYLAND_DISPLAY", wayland_display, 1);
        }
        if (getenv("XCURSOR_THEME") == NULL || getenv("XCURSOR_THEME")[0] == '\0') {
            setenv("XCURSOR_THEME", "dmz-white", 1);
        }
        setenv("XDG_RUNTIME_DIR", "/run", 1);
        setenv("PATH", "/bin:/usr/bin:/sbin:/usr/sbin", 1);
        setenv("SHELL", "/bin/sh", 1);
        setenv("HOME", "/home", 1);
        setenv("LANG", "C.utf8", 1);
        setenv("LC_ALL", "C.utf8", 1);
        setenv("LOCPATH", "/usr/lib/locale", 1);
        setenv("XKB_CONFIG_ROOT", "/usr/share/X11/xkb", 1);
        char *const argv[] = {(char *)path, NULL};
        execv(path, argv);
        fprintf(stderr, "stratwm: autostart exec failed for %s: %s\n", path, strerror(errno));
        _exit(127);
    }
    fprintf(stderr, "stratwm: autostart spawned %s (pid=%ld)\n", path, (long)pid);
    /* Surface fast-fail cases to logs when autostart exits before mapping. */
    struct timespec delay = {.tv_sec = 0, .tv_nsec = 200 * 1000 * 1000};
    (void)nanosleep(&delay, NULL);
    int status = 0;
    pid_t waited = waitpid(pid, &status, WNOHANG);
    if (waited == pid) {
        if (WIFEXITED(status)) {
            fprintf(stderr, "stratwm: autostart exited early status=%d (pid=%ld)\n",
                WEXITSTATUS(status), (long)pid);
        } else if (WIFSIGNALED(status)) {
            fprintf(stderr, "stratwm: autostart died early signal=%d (pid=%ld)\n",
                WTERMSIG(status), (long)pid);
        }
    }
}

/* From `/config/strat/stratvm-keybinds` (written by stratsettings save / strat-ui-config export-keybinds). */
static uint32_t strat_kb_spotlite_mods = WLR_MODIFIER_LOGO;
static xkb_keysym_t strat_kb_spotlite_sym = XKB_KEY_period;
static uint32_t strat_kb_cycle_mods = WLR_MODIFIER_LOGO;
static xkb_keysym_t strat_kb_cycle_sym = XKB_KEY_space;

static void stratwm_load_keybinds(void) {
    FILE *f = fopen("/config/strat/stratvm-keybinds", "r");
    if (!f) {
        return;
    }
    char line[256];
    while (fgets(line, sizeof(line), f)) {
        if (line[0] == '#' || line[0] == '\n' || line[0] == '\r') {
            continue;
        }
        char name[64];
        unsigned long mods = 0;
        unsigned long sym = 0;
        if (sscanf(line, "%63s %lu %lu", name, &mods, &sym) < 3) {
            continue;
        }
        if (strcmp(name, "spotlite") == 0) {
            strat_kb_spotlite_mods = (uint32_t)mods;
            strat_kb_spotlite_sym = (xkb_keysym_t)sym;
        } else if (strcmp(name, "cycle_layout") == 0) {
            strat_kb_cycle_mods = (uint32_t)mods;
            strat_kb_cycle_sym = (xkb_keysym_t)sym;
        }
    }
    fclose(f);
}

static void spawn_spotlite(void) {
    pid_t pid = fork();
    if (pid < 0) {
        return;
    }
    if (pid == 0) {
        setsid();
        execl("/bin/spotlite", "spotlite", (char *)NULL);
        execl("/usr/bin/spotlite", "spotlite", (char *)NULL);
        _exit(127);
    }
}

static struct stratwm_view *view_from_surface(struct stratwm_server *server,
    struct wlr_surface *surface) {
    if (surface == NULL) {
        return NULL;
    }
    struct wlr_surface *root = wlr_surface_get_root_surface(surface);
    struct stratwm_view *view;
    wl_list_for_each(view, &server->views, link) {
        if (view->xdg_toplevel->base->surface == root) {
            return view;
        }
    }
    return NULL;
}

static void focus_view(struct stratwm_view *view, struct wlr_surface *surface) {
    if (view == NULL || surface == NULL) {
        return;
    }
    struct stratwm_server *server = view->server;
    struct wlr_keyboard *keyboard = wlr_seat_get_keyboard(server->seat);

    /* Unfocus previous view and dim border */
    if (server->focused_view != NULL && server->focused_view != view) {
        update_view_border(server->focused_view, false);
    }

    /* Focus new view and highlight border */
    update_view_border(view, true);
    server->focused_view = view;

    /* Raise only within layers_normal — view->scene_tree is a child of layers_normal,
     * so this cannot paint above LAYER_TOP panel surfaces. */
    wlr_scene_node_raise_to_top(&view->scene_tree->node);
    wlr_seat_keyboard_notify_enter(server->seat, surface,
        keyboard ? keyboard->keycodes : NULL,
        keyboard ? keyboard->num_keycodes : 0,
        keyboard ? &keyboard->modifiers : NULL);
}

/* Tiling engine (Phase 8.2) */

static int tile_leaf_count(struct stratwm_tile *tile) {
    if (!tile) return 0;
    if (tile->view) return 1;
    return tile_leaf_count(tile->left) + tile_leaf_count(tile->right);
}

static bool tile_is_empty_leaf(struct stratwm_tile *tile) {
    return tile && !tile->view && !tile->left && !tile->right;
}

static struct stratwm_tile *tile_new(struct wlr_box geometry) {
    struct stratwm_tile *tile = calloc(1, sizeof(*tile));
    if (!tile) return NULL;
    tile->geometry = geometry;
    tile->split = SPLIT_VERTICAL;
    tile->parent = NULL;
    tile->left = NULL;
    tile->right = NULL;
    tile->view = NULL;
    return tile;
}

static void tile_free(struct stratwm_tile *tile) {
    if (!tile) return;
    if (tile->left) tile_free(tile->left);
    if (tile->right) tile_free(tile->right);
    free(tile);
}

static int get_top_exclusive_zone(struct stratwm_server *server) {
    int zone = 0;
    struct stratwm_layer_surface *layer;
    wl_list_for_each(layer, &server->layer_surfaces, link) {
        if (layer->layer_surface->current.layer == ZWLR_LAYER_SHELL_V1_LAYER_TOP) {
            if (layer->layer_surface->current.exclusive_zone > zone) {
                zone = layer->layer_surface->current.exclusive_zone;
            }
        }
    }
    return zone;
}

/*
 * Recompute subtree geometry from a new bounding box while preserving each
 * internal split ratio from the previous geometry.
 */
static void tile_apply_geometry(struct stratwm_tile *tile, struct wlr_box geometry) {
    if (!tile) return;

    struct wlr_box old = tile->geometry;
    tile->geometry = geometry;

    if (!tile->left || !tile->right) {
        return;
    }

    if (tile->split == SPLIT_VERTICAL) {
        int left_width = geometry.width / 2;
        if (old.width > 0) {
            left_width = (tile->left->geometry.width * geometry.width) / old.width;
        }
        if (left_width < 1) left_width = 1;
        if (left_width > geometry.width - 1) left_width = geometry.width - 1;

        struct wlr_box left_box = geometry;
        left_box.width = left_width;

        struct wlr_box right_box = geometry;
        right_box.x = geometry.x + left_width;
        right_box.width = geometry.width - left_width;

        tile_apply_geometry(tile->left, left_box);
        tile_apply_geometry(tile->right, right_box);
        return;
    }

    int top_height = geometry.height / 2;
    if (old.height > 0) {
        top_height = (tile->left->geometry.height * geometry.height) / old.height;
    }
    if (top_height < 1) top_height = 1;
    if (top_height > geometry.height - 1) top_height = geometry.height - 1;

    struct wlr_box top_box = geometry;
    top_box.height = top_height;

    struct wlr_box bottom_box = geometry;
    bottom_box.y = geometry.y + top_height;
    bottom_box.height = geometry.height - top_height;

    tile_apply_geometry(tile->left, top_box);
    tile_apply_geometry(tile->right, bottom_box);
}

/* Insert or replace a leaf view in the BSP tree, splitting if needed */
static struct stratwm_tile *tile_insert(struct stratwm_tile *tile,
    struct stratwm_view *view, struct wlr_box max_geometry) {
    if (!tile) {
        tile = tile_new(max_geometry);
        if (!tile) return NULL;
    }

    /* Empty leaf: claim it */
    if (!tile->view && !tile->left && !tile->right) {
        tile->view = view;
        return tile;
    }

    /* Leaf node: insert new view by splitting */
    if (tile->view != NULL) {
        struct stratwm_view *existing = tile->view;
        tile->view = NULL; /* becomes internal node */

        /* Allocate left and right children with split geometry */
        if (tile->split == SPLIT_VERTICAL) {
            int mid = tile->geometry.x + tile->geometry.width / 2;
            struct wlr_box left_box = tile->geometry;
            left_box.width = mid - tile->geometry.x;

            struct wlr_box right_box = tile->geometry;
            right_box.x = mid;
            right_box.width = tile->geometry.x + tile->geometry.width - mid;

            tile->left = tile_new(left_box);
            tile->right = tile_new(right_box);
        } else {
            int mid = tile->geometry.y + tile->geometry.height / 2;
            struct wlr_box top_box = tile->geometry;
            top_box.height = mid - tile->geometry.y;

            struct wlr_box bottom_box = tile->geometry;
            bottom_box.y = mid;
            bottom_box.height = tile->geometry.y + tile->geometry.height - mid;

            tile->left = tile_new(top_box);
            tile->right = tile_new(bottom_box);
        }

        if (!tile->left || !tile->right) {
            tile_free(tile->left);
            tile_free(tile->right);
            tile->left = NULL;
            tile->right = NULL;
            tile->view = existing;
            return tile;
        }

        tile->left->parent = tile;
        tile->right->parent = tile;

        /* Insert existing view into left child */
        tile->left->view = existing;
        /* Recursively insert new view into right child */
        tile->right = tile_insert(tile->right, view, tile->right->geometry);

        return tile;
    }

    /* Internal node: recurse into left and right */
    if (tile->left && tile->right) {
        /* Alternate split direction for next level */
        enum stratwm_split_direction next_split =
            (tile->split == SPLIT_VERTICAL) ? SPLIT_HORIZONTAL : SPLIT_VERTICAL;

        /* Insert into the less populated subtree to keep BSP reasonably balanced. */
        int left_leaves = tile_leaf_count(tile->left);
        int right_leaves = tile_leaf_count(tile->right);
        if (left_leaves <= right_leaves) {
            if (tile->left->split != tile->split) {
                tile->left->split = next_split;
            }
            tile->left = tile_insert(tile->left, view, tile->left->geometry);
        } else {
            if (tile->right->split != tile->split) {
                tile->right->split = next_split;
            }
            tile->right = tile_insert(tile->right, view, tile->right->geometry);
        }
    }

    return tile;
}

/* Remove a view from the BSP tree, collapsing if needed */
static struct stratwm_tile *tile_remove(struct stratwm_tile *tile,
    struct stratwm_view *view) {
    if (!tile) return NULL;

    if (tile->view == view) {
        /* Leaf node with this view: remove it */
        tile->view = NULL;
        return tile;
    }

    /* Recurse into children */
    if (tile->left) {
        tile->left = tile_remove(tile->left, view);
    }
    if (tile->right) {
        tile->right = tile_remove(tile->right, view);
    }

    if (tile->left && tile->right &&
        tile_is_empty_leaf(tile->left) &&
        tile_is_empty_leaf(tile->right)) {
        tile_free(tile->left);
        tile_free(tile->right);
        tile->left = NULL;
        tile->right = NULL;
        return tile;
    }

    /*
     * Promote the non-empty child when its sibling becomes empty. This keeps
     * the tree compact and lets remaining views reclaim full geometry.
     */
    if (tile->left && tile->right &&
        tile_is_empty_leaf(tile->left) &&
        !tile_is_empty_leaf(tile->right)) {
        struct stratwm_tile *promote = tile->right;
        struct stratwm_tile *empty = tile->left;
        struct wlr_box full = tile->geometry;

        tile->view = promote->view;
        tile->split = promote->split;
        tile->left = promote->left;
        tile->right = promote->right;
        if (tile->left) tile->left->parent = tile;
        if (tile->right) tile->right->parent = tile;

        tile_apply_geometry(tile, full);
        free(empty);
        free(promote);
        return tile;
    }

    if (tile->left && tile->right &&
        !tile_is_empty_leaf(tile->left) &&
        tile_is_empty_leaf(tile->right)) {
        struct stratwm_tile *promote = tile->left;
        struct stratwm_tile *empty = tile->right;
        struct wlr_box full = tile->geometry;

        tile->view = promote->view;
        tile->split = promote->split;
        tile->left = promote->left;
        tile->right = promote->right;
        if (tile->left) tile->left->parent = tile;
        if (tile->right) tile->right->parent = tile;

        tile_apply_geometry(tile, full);
        free(empty);
        free(promote);
        return tile;
    }

    return tile;
}

/* Find a view within the BSP tree */
static struct stratwm_tile *tile_find_view(struct stratwm_tile *tile,
    struct stratwm_view *view) {
    if (!tile) return NULL;
    if (tile->view == view) return tile;

    struct stratwm_tile *found = NULL;
    if (tile->left) {
        found = tile_find_view(tile->left, view);
        if (found) return found;
    }
    if (tile->right) {
        found = tile_find_view(tile->right, view);
        if (found) return found;
    }
    return NULL;
}

/* Update scene node positions based on tile geometry */
static void tile_reflow_scene(struct stratwm_tile *tile) {
    if (!tile) return;

    if (tile->view) {
        /* Position scene tree at tile geometry */
        wlr_scene_node_set_position(&tile->view->scene_tree->node,
            tile->geometry.x, tile->geometry.y);

        /* Set surface size to tile dimensions */
        wlr_xdg_toplevel_set_size(tile->view->xdg_toplevel,
            tile->geometry.width, tile->geometry.height);

        /* Keep border aligned with tile size (pad outside on each edge). */
        if (tile->view->border) {
            int pad = tile->view->server->deco_border_pad;
            int bw = tile->geometry.width + 2 * pad;
            int bh = tile->geometry.height + 2 * pad;
            if (bw < 1) bw = 1;
            if (bh < 1) bh = 1;
            wlr_scene_rect_set_size(tile->view->border, bw, bh);
            wlr_scene_node_set_position(&tile->view->border->node, -pad, -pad);
        }

        /* Update titlebar size and button positions (Phase 8.8) */
        if (tile->view->titlebar_bg) {
            int th = tile->view->server->deco_titlebar_h;
            wlr_scene_rect_set_size(tile->view->titlebar_bg, tile->geometry.width, th);
            wlr_scene_node_set_position(&tile->view->titlebar_bg->node, 0, -th);
        }
        update_titlebar_buttons(tile->view, tile->geometry.width);
    }

    if (tile->left) tile_reflow_scene(tile->left);
    if (tile->right) tile_reflow_scene(tile->right);
}

/* Find next visible leaf tile (for focus traversal) */
static struct stratwm_tile *tile_next_leaf(struct stratwm_tile *current) {
    if (!current || !current->parent) return current;

    struct stratwm_tile *parent = current->parent;

    /* If we're at left child, go to right sibling */
    if (parent->right && current == parent->left) {
        struct stratwm_tile *next = parent->right;
        while (next->left || next->right) {
            next = next->left ? next->left : next->right;
        }
        return next;
    }

    /* Otherwise, traverse up to next ancestor's right subtree */
    return tile_next_leaf(parent);
}

static struct stratwm_tile *tile_prev_leaf(struct stratwm_tile *current) {
    if (!current || !current->parent) return current;

    struct stratwm_tile *parent = current->parent;

    /* If we're at right child, go to left sibling */
    if (parent->left && current == parent->right) {
        struct stratwm_tile *prev = parent->left;
        while (prev->left || prev->right) {
            prev = prev->right ? prev->right : prev->left;
        }
        return prev;
    }

    /* Otherwise, traverse up */
    return tile_prev_leaf(parent);
}

/* Tile resizing: adjust split point with Super+Shift+{H,J,K,L} (Phase 8.7) */
static void resize_tile_horizontal(struct stratwm_server *server, int delta) {
    if (!server->focused_view) return;
    
    struct stratwm_workspace *ws = &server->workspaces[server->current_workspace];
    struct stratwm_tile *tile = tile_find_view(ws->root, server->focused_view);
    if (!tile || !tile->parent || tile->parent->split != SPLIT_VERTICAL) return;
    
    /* Adjust parent's center point (will reflow on next key press) */
    struct stratwm_tile *parent = tile->parent;
    int current_mid = parent->geometry.x + parent->geometry.width / 2;
    int new_mid = current_mid + delta;
    
    /* Clamp to reasonable bounds (20% to 80% of parent) */
    int min_mid = parent->geometry.x + parent->geometry.width / 5;
    int max_mid = parent->geometry.x + 4 * parent->geometry.width / 5;
    new_mid = (new_mid < min_mid) ? min_mid : (new_mid > max_mid) ? max_mid : new_mid;
    
    /* Update child subtree geometry from the resized split. */
    if (parent->left && parent->right) {
        struct wlr_box left_box = parent->geometry;
        left_box.width = new_mid - parent->geometry.x;
        struct wlr_box right_box = parent->geometry;
        right_box.x = new_mid;
        right_box.width = parent->geometry.x + parent->geometry.width - new_mid;
        tile_apply_geometry(parent->left, left_box);
        tile_apply_geometry(parent->right, right_box);
    }
    
    tile_reflow_scene(ws->root);
}

static void resize_tile_vertical(struct stratwm_server *server, int delta) {
    if (!server->focused_view) return;
    
    struct stratwm_workspace *ws = &server->workspaces[server->current_workspace];
    struct stratwm_tile *tile = tile_find_view(ws->root, server->focused_view);
    if (!tile || !tile->parent || tile->parent->split != SPLIT_HORIZONTAL) return;
    
    /* Adjust parent's center point */
    struct stratwm_tile *parent = tile->parent;
    int current_mid = parent->geometry.y + parent->geometry.height / 2;
    int new_mid = current_mid + delta;
    
    /* Clamp to reasonable bounds */
    int min_mid = parent->geometry.y + parent->geometry.height / 5;
    int max_mid = parent->geometry.y + 4 * parent->geometry.height / 5;
    new_mid = (new_mid < min_mid) ? min_mid : (new_mid > max_mid) ? max_mid : new_mid;
    
    /* Update child subtree geometry from the resized split. */
    if (parent->left && parent->right) {
        struct wlr_box top_box = parent->geometry;
        top_box.height = new_mid - parent->geometry.y;
        struct wlr_box bottom_box = parent->geometry;
        bottom_box.y = new_mid;
        bottom_box.height = parent->geometry.y + parent->geometry.height - new_mid;
        tile_apply_geometry(parent->left, top_box);
        tile_apply_geometry(parent->right, bottom_box);
    }
    
    tile_reflow_scene(ws->root);
}

/* Window decoration: update border color based on focus (Phase 8.3) */
static void update_view_border(struct stratwm_view *view, bool focused) {
    if (!view || !view->border) return;

    /* Focused: bright cyan (#00FFFF); unfocused: dark gray (#444444) */
    if (focused) {
        float color[4] = {0.0f, 1.0f, 1.0f, 1.0f};  /* cyan */
        wlr_scene_rect_set_color(view->border, color);
    } else {
        float color[4] = {0.267f, 0.267f, 0.267f, 1.0f};  /* dark gray */
        wlr_scene_rect_set_color(view->border, color);
    }
}

static bool point_in_titlebar_button(struct wlr_scene_rect *button,
    double view_x, double view_y) {
    if (!button) return false;
    if (!wlr_scene_node_is_enabled(&button->node)) return false;
    int bx = button->node.x;
    int by = button->node.y;
    return view_x >= bx && view_x < bx + 16 && view_y >= by && view_y < by + 16;
}

/* Update titlebar button positions based on current width */
static void update_titlebar_buttons(struct stratwm_view *view, int width) {
    if (!view) return;

    int th = view->server->deco_titlebar_h;
    int padding = 8;       /* Right edge padding */
    int btn_size = 16;     /* Button size */
    int btn_gap = 4;       /* Gap between buttons */
    int y_pos = -(th - 4); /* Near top of titlebar band */

    int close_x = width - padding - btn_size;
    int max_x = close_x - btn_size - btn_gap;
    int min_x = max_x - btn_size - btn_gap;

    if (view->close_button) {
        wlr_scene_node_set_position(&view->close_button->node, close_x, y_pos);
        wlr_scene_rect_set_size(view->close_button, btn_size, btn_size);
    }
    if (view->max_button) {
        wlr_scene_node_set_position(&view->max_button->node, max_x, y_pos);
        wlr_scene_rect_set_size(view->max_button, btn_size, btn_size);
    }
    if (view->min_button) {
        wlr_scene_node_set_position(&view->min_button->node, min_x, y_pos);
        wlr_scene_rect_set_size(view->min_button, btn_size, btn_size);
    }
}

/* Titlebar creation and management (Phase 8.8) */
static void create_titlebar(struct stratwm_view *view) {
    if (!view) return;

    int th = view->server->deco_titlebar_h;
    /* Titlebar background: dark slate band above the client surface */
    float bg_color[4] = {0.12f, 0.13f, 0.18f, 1.0f};  /* Dark slate */
    view->titlebar_bg = wlr_scene_rect_create(view->scene_tree, 100, th, bg_color);
    if (view->titlebar_bg) {
        wlr_scene_node_set_position(&view->titlebar_bg->node, 0, -th);  /* Above window */
    }

    /* Close button: vibrant red */
    float close_color[4] = {0.95f, 0.25f, 0.25f, 1.0f};
    view->close_button = wlr_scene_rect_create(view->scene_tree, 16, 16, close_color);

    /* Maximize button: vibrant green */
    float max_color[4] = {0.25f, 0.85f, 0.35f, 1.0f};
    view->max_button = wlr_scene_rect_create(view->scene_tree, 16, 16, max_color);

    /* Minimize button: vibrant yellow */
    float min_color[4] = {0.95f, 0.85f, 0.25f, 1.0f};
    view->min_button = wlr_scene_rect_create(view->scene_tree, 16, 16, min_color);

    /* Initial button positioning (will be updated when window sizes) */
    update_titlebar_buttons(view, 100);
    set_view_decorations_visible(view, view->server->default_decorations_visible);
}

static void destroy_titlebar(struct stratwm_view *view) {
    if (!view) return;

    if (view->titlebar_bg) {
        wlr_scene_node_destroy(&view->titlebar_bg->node);
        view->titlebar_bg = NULL;
    }
    if (view->close_button) {
        wlr_scene_node_destroy(&view->close_button->node);
        view->close_button = NULL;
    }
    if (view->max_button) {
        wlr_scene_node_destroy(&view->max_button->node);
        view->max_button = NULL;
    }
    if (view->min_button) {
        wlr_scene_node_destroy(&view->min_button->node);
        view->min_button = NULL;
    }
}

static uint32_t stratwm_seat_modifiers(struct stratwm_server *server) {
    struct wlr_keyboard *kb = wlr_seat_get_keyboard(server->seat);
    if (kb == NULL) {
        return 0;
    }
    return wlr_keyboard_get_modifiers(kb);
}

static void set_view_decorations_visible(struct stratwm_view *view, bool visible) {
    if (!view) {
        return;
    }
    view->decorations_visible = visible;
    if (view->titlebar_bg) {
        wlr_scene_node_set_enabled(&view->titlebar_bg->node, visible);
    }
    if (view->close_button) {
        wlr_scene_node_set_enabled(&view->close_button->node, visible);
    }
    if (view->max_button) {
        wlr_scene_node_set_enabled(&view->max_button->node, visible);
    }
    if (view->min_button) {
        wlr_scene_node_set_enabled(&view->min_button->node, visible);
    }
    if (view->border) {
        wlr_scene_node_set_enabled(&view->border->node, visible);
    }
}

static bool titlebar_context_region(struct stratwm_view *view, double vx, double vy) {
    if (!view || !view->titlebar_bg) {
        return false;
    }
    int th = view->server->deco_titlebar_h;
    if (vy < (double)-th || vy >= 0.0) {
        return false;
    }
    if (vx < 0.0 || vx >= (double)view->xdg_toplevel->current.width) {
        return false;
    }
    if (point_in_titlebar_button(view->close_button, vx, vy)) {
        return false;
    }
    if (point_in_titlebar_button(view->max_button, vx, vy)) {
        return false;
    }
    if (point_in_titlebar_button(view->min_button, vx, vy)) {
        return false;
    }
    return true;
}

static void stratwm_refresh_tiled_visibility(struct stratwm_server *server) {
    struct stratwm_workspace *ws = &server->workspaces[server->current_workspace];
    struct stratwm_view *v;
    wl_list_for_each(v, &server->views, link) {
        if (v->workspace_id != server->current_workspace || v->is_floating) {
            continue;
        }
        bool visible = true;
        switch (ws->layout) {
        case LAYOUT_BSP:
            visible = true;
            break;
        case LAYOUT_STACK:
        case LAYOUT_FULLSCREEN:
            visible = (v == server->focused_view);
            break;
        }
        wlr_scene_node_set_enabled(&v->scene_tree->node, visible);
    }
}

static void move_view_to_workspace(struct stratwm_server *server, struct stratwm_view *view, int new_id) {
    if (!view || new_id < 0 || new_id >= STRATWM_WORKSPACES) {
        return;
    }
    if (view->workspace_id == new_id) {
        return;
    }

    int old_id = view->workspace_id;
    struct stratwm_workspace *old_ws = &server->workspaces[old_id];
    struct stratwm_workspace *new_ws = &server->workspaces[new_id];

    if (!view->is_floating && old_ws->root) {
        old_ws->root = tile_remove(old_ws->root, view);
        tile_reflow_scene(old_ws->root);
    }

    view->workspace_id = new_id;

    if (!view->is_floating) {
        if (!new_ws->root) {
            struct wlr_box output_box = {0, 0, 1920, 1080};
            struct stratwm_output *output;
            wl_list_for_each(output, &server->outputs, link) {
                output_box.width = output->wlr_output->width;
                output_box.height = output->wlr_output->height;
                break;
            }
            int zone = get_top_exclusive_zone(server);
            output_box.y += zone;
            output_box.height -= zone;
            new_ws->root = tile_new(output_box);
        }
        if (new_ws->root) {
            new_ws->root = tile_insert(new_ws->root, view, new_ws->root->geometry);
            tile_reflow_scene(new_ws->root);
        }
    }

    bool on_current = (new_id == server->current_workspace);
    if (view->is_floating) {
        wlr_scene_node_set_enabled(&view->scene_tree->node, on_current);
    } else if (on_current) {
        stratwm_refresh_tiled_visibility(server);
    } else {
        wlr_scene_node_set_enabled(&view->scene_tree->node, false);
    }

    if (server->focused_view == view && !on_current) {
        update_view_border(view, false);
        server->focused_view = NULL;
        struct stratwm_view *v;
        wl_list_for_each(v, &server->views, link) {
            if (v->workspace_id == server->current_workspace) {
                focus_view(v, v->xdg_toplevel->base->surface);
                break;
            }
        }
        stratwm_refresh_tiled_visibility(server);
    } else if (server->focused_view == view && on_current) {
        focus_view(view, view->xdg_toplevel->base->surface);
        stratwm_refresh_tiled_visibility(server);
    }
}

static void stratwm_load_deco_config(struct stratwm_server *server) {
    FILE *f = fopen("/config/strat/stratvm.conf", "r");
    if (f == NULL) {
        return;
    }
    char line[256];
    while (fgets(line, sizeof(line), f) != NULL) {
        char *p = line;
        while (*p == ' ' || *p == '\t') {
            p++;
        }
        if (*p == '#' || *p == '\n' || *p == '\0') {
            continue;
        }
        if (strncmp(p, "titlebar_height=", 17) == 0) {
            int v = atoi(p + 17);
            if (v >= 12 && v <= 64) {
                server->deco_titlebar_h = v;
            }
        } else if (strncmp(p, "border_pad=", 12) == 0) {
            int v = atoi(p + 12);
            if (v >= 0 && v <= 12) {
                server->deco_border_pad = v;
            }
        }
    }
    fclose(f);
}

#define STRATWM_SETTINGS_D_MAX 64

static void stratwm_trim_inplace(char *s) {
    char *p = s;
    while (*p == ' ' || *p == '\t') {
        p++;
    }
    if (p != s) {
        memmove(s, p, strlen(p) + 1);
    }
    size_t len = strlen(s);
    while (len > 0 && (s[len - 1] == ' ' || s[len - 1] == '\t' || s[len - 1] == '\r'
            || s[len - 1] == '\n')) {
        s[--len] = '\0';
    }
}

static void stratwm_strip_hash_comment(char *line) {
    char *h = strchr(line, '#');
    if (h) {
        *h = '\0';
    }
}

static bool stratwm_parse_bool01(const char *v) {
    char buf[48];
    size_t i = 0;
    while (*v == ' ' || *v == '\t') {
        v++;
    }
    while (v[i] && i < sizeof(buf) - 1) {
        buf[i] = v[i];
        i++;
    }
    buf[i] = '\0';
    stratwm_trim_inplace(buf);
    if (strcasecmp(buf, "true") == 0 || strcmp(buf, "1") == 0 || strcasecmp(buf, "yes") == 0) {
        return true;
    }
    if (strcasecmp(buf, "false") == 0 || strcmp(buf, "0") == 0 || strcasecmp(buf, "no") == 0) {
        return false;
    }
    return true;
}

static void stratwm_apply_chrome_kv(struct stratwm_server *server, const char *key, const char *val) {
    if (strcmp(key, "decoration_titlebar_height") == 0) {
        int v = atoi(val);
        if (v >= 12 && v <= 64) {
            server->deco_titlebar_h = v;
        }
    } else if (strcmp(key, "border_pad") == 0) {
        int v = atoi(val);
        if (v >= 0 && v <= 12) {
            server->deco_border_pad = v;
        }
    } else if (strcmp(key, "decorations_enabled_default") == 0) {
        server->default_decorations_visible = stratwm_parse_bool01(val);
    }
}

static void stratwm_parse_chrome_from_file(struct stratwm_server *server, const char *path) {
    FILE *f = fopen(path, "r");
    if (f == NULL) {
        return;
    }
    char line[384];
    bool in_chrome = false;
    while (fgets(line, sizeof(line), f) != NULL) {
        stratwm_strip_hash_comment(line);
        stratwm_trim_inplace(line);
        if (line[0] == '\0') {
            continue;
        }
        if (line[0] == '[') {
            in_chrome = (strcmp(line, "[chrome]") == 0);
            continue;
        }
        if (!in_chrome) {
            continue;
        }
        char *eq = strchr(line, '=');
        if (eq == NULL) {
            continue;
        }
        *eq = '\0';
        char *key = line;
        char *val = eq + 1;
        stratwm_trim_inplace(key);
        stratwm_trim_inplace(val);
        if (key[0] == '\0') {
            continue;
        }
        stratwm_apply_chrome_kv(server, key, val);
    }
    fclose(f);
}

static int stratwm_cmp_cstr(const void *a, const void *b) {
    return strcmp(*(const char *const *)a, *(const char *const *)b);
}

static void stratwm_load_modular_chrome(struct stratwm_server *server) {
    stratwm_parse_chrome_from_file(server, "/config/strat/settings.toml");
    DIR *d = opendir("/config/strat/settings.d");
    if (d == NULL) {
        return;
    }
    char *names[STRATWM_SETTINGS_D_MAX];
    int n = 0;
    struct dirent *de;
    while ((de = readdir(d)) != NULL && n < STRATWM_SETTINGS_D_MAX) {
        const char *nm = de->d_name;
        size_t len = strlen(nm);
        if (len < 6 || strcmp(nm + len - 5, ".toml") != 0) {
            continue;
        }
        char *copy = strdup(nm);
        if (copy) {
            names[n++] = copy;
        }
    }
    closedir(d);
    if (n == 0) {
        return;
    }
    qsort(names, (size_t)n, sizeof(names[0]), stratwm_cmp_cstr);
    for (int i = 0; i < n; i++) {
        char path[512];
        snprintf(path, sizeof(path), "/config/strat/settings.d/%s", names[i]);
        stratwm_parse_chrome_from_file(server, path);
        free(names[i]);
    }
}

/* Layer shell handlers (Phase 24a) */
static void layer_surface_configure(struct stratwm_layer_surface *layer) {
    struct wlr_output *output = layer->layer_surface->output;
    if (!output) return;

    struct wlr_box full_area = { 0 };
    wlr_output_effective_resolution(output, &full_area.width, &full_area.height);
    struct wlr_box usable_area = full_area;

    wlr_scene_layer_surface_v1_configure(layer->scene_layer_surface,
        &full_area, &usable_area);
}

static void layer_surface_commit_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_layer_surface *layer = wl_container_of(listener, layer, commit);
    
    /* Check if layer has changed and reparent if needed */
    if (layer->layer_surface->current.layer != layer->previous_layer) {
        struct wlr_scene_tree *new_layer_tree;
        switch (layer->layer_surface->current.layer) {
            case ZWLR_LAYER_SHELL_V1_LAYER_BACKGROUND:
                new_layer_tree = layer->server->layers_bg;
                break;
            case ZWLR_LAYER_SHELL_V1_LAYER_BOTTOM:
                new_layer_tree = layer->server->layers_bottom;
                break;
            case ZWLR_LAYER_SHELL_V1_LAYER_TOP:
                new_layer_tree = layer->server->layers_top;
                break;
            case ZWLR_LAYER_SHELL_V1_LAYER_OVERLAY:
                new_layer_tree = layer->server->layers_overlay;
                break;
            default:
                new_layer_tree = layer->server->layers_bottom;
                break;
        }
        wlr_scene_node_reparent(&layer->scene_tree->node, new_layer_tree);
        layer->previous_layer = layer->layer_surface->current.layer;
    }
    
    layer_surface_configure(layer);
}

static void layer_surface_map_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_layer_surface *layer = wl_container_of(listener, layer, map);
    wlr_scene_node_set_enabled(&layer->scene_tree->node, true);
    tile_reflow_scene(layer->server->workspaces[layer->server->current_workspace].root);
}

static void layer_surface_unmap_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_layer_surface *layer = wl_container_of(listener, layer, unmap);
    wlr_scene_node_set_enabled(&layer->scene_tree->node, false);
    tile_reflow_scene(layer->server->workspaces[layer->server->current_workspace].root);
}

static void layer_surface_destroy_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_layer_surface *layer = wl_container_of(listener, layer, destroy);
    wl_list_remove(&layer->map.link);
    wl_list_remove(&layer->unmap.link);
    wl_list_remove(&layer->commit.link);
    wl_list_remove(&layer->destroy.link);
    wl_list_remove(&layer->new_popup.link);
    wl_list_remove(&layer->link);
    free(layer);
}

static void layer_surface_new_popup_notify(struct wl_listener *listener, void *data) {
    (void)listener;
    (void)data;
    /* popups handled by wlroots scene graph automatically */
}

static void server_new_layer_surface_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, new_layer_surface);
    struct wlr_layer_surface_v1 *wlr_layer_surface = data;

    /* Assign an output if client didn't request one — required before first configure. */
    if (!wlr_layer_surface->output) {
        if (wl_list_empty(&server->outputs)) {
            wlr_layer_surface_v1_destroy(wlr_layer_surface);
            return;
        }
        struct stratwm_output *output = wl_container_of(
            server->outputs.next, output, link);
        wlr_layer_surface->output = output->wlr_output;
    }

    struct stratwm_layer_surface *layer = calloc(1, sizeof(*layer));
    if (!layer) return;

    layer->server = server;
    layer->layer_surface = wlr_layer_surface;
    layer->previous_layer = wlr_layer_surface->pending.layer;

    /* Parent to the appropriate layer tree based on pending.layer */
    struct wlr_scene_tree *layer_tree;
    switch (wlr_layer_surface->pending.layer) {
        case ZWLR_LAYER_SHELL_V1_LAYER_BACKGROUND:
            layer_tree = server->layers_bg;
            break;
        case ZWLR_LAYER_SHELL_V1_LAYER_BOTTOM:
            layer_tree = server->layers_bottom;
            break;
        case ZWLR_LAYER_SHELL_V1_LAYER_TOP:
            layer_tree = server->layers_top;
            break;
        case ZWLR_LAYER_SHELL_V1_LAYER_OVERLAY:
            layer_tree = server->layers_overlay;
            break;
        default:
            layer_tree = server->layers_bottom;
            break;
    }

    layer->scene_layer_surface = wlr_scene_layer_surface_v1_create(
        layer_tree, wlr_layer_surface);
    if (!layer->scene_layer_surface) {
        free(layer);
        return;
    }
    layer->scene_tree = layer->scene_layer_surface->tree;

    layer->map.notify = layer_surface_map_notify;
    wl_signal_add(&wlr_layer_surface->surface->events.map, &layer->map);

    layer->unmap.notify = layer_surface_unmap_notify;
    wl_signal_add(&wlr_layer_surface->surface->events.unmap, &layer->unmap);

    layer->commit.notify = layer_surface_commit_notify;
    wl_signal_add(&wlr_layer_surface->surface->events.commit, &layer->commit);

    layer->destroy.notify = layer_surface_destroy_notify;
    wl_signal_add(&wlr_layer_surface->events.destroy, &layer->destroy);

    layer->new_popup.notify = layer_surface_new_popup_notify;
    wl_signal_add(&wlr_layer_surface->events.new_popup, &layer->new_popup);

    wl_list_insert(&server->layer_surfaces, &layer->link);
}

static void output_frame_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_output *output = wl_container_of(listener, output, frame);
    (void)output->server;  /* Unused but kept for future reference */
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);

    wlr_scene_output_commit(output->scene_output, NULL);
    wlr_scene_output_send_frame_done(output->scene_output, &now);
}

static void output_destroy_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_output *output = wl_container_of(listener, output, destroy);
    wl_list_remove(&output->frame.link);
    wl_list_remove(&output->destroy.link);
    wl_list_remove(&output->link);
    free(output);
}

static void update_output_background(struct stratwm_output *output) {
    int width = output->wlr_output->width;
    int height = output->wlr_output->height;
    if (width < 1) width = 1;
    if (height < 1) height = 1;

    wlr_scene_rect_set_size(output->background, width, height);
    wlr_scene_node_set_position(&output->background->node, 0, 0);
    wlr_scene_node_lower_to_bottom(&output->background->node);
}

static void server_new_output_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, new_output);
    struct wlr_output *wlr_output = data;

    wlr_output_init_render(wlr_output, server->allocator, server->renderer);

    struct wlr_output_state state;
    wlr_output_state_init(&state);
    wlr_output_state_set_enabled(&state, true);
    struct wlr_output_mode *mode = wlr_output_preferred_mode(wlr_output);
    if (mode != NULL) {
        wlr_output_state_set_mode(&state, mode);
    }
    if (!wlr_output_commit_state(wlr_output, &state)) {
        wlr_output_state_finish(&state);
#ifdef DEBUG
        fprintf(stderr, "stratwm: output commit failed\n");
#endif
        return;
    }
    wlr_output_state_finish(&state);

    struct stratwm_output *output = calloc(1, sizeof(*output));
    if (output == NULL) {
        return;
    }

    output->server = server;
    output->wlr_output = wlr_output;
    output->scene_output = wlr_scene_output_create(server->scene, wlr_output);
    /* Parent to layers_bg so wallpaper stays below layer-shell (panel) and xdg clients. */
    output->background = wlr_scene_rect_create(server->layers_bg, 1, 1, STRAT_BG);
    update_output_background(output);

    wlr_output_layout_add_auto(server->output_layout, wlr_output);

    output->frame.notify = output_frame_notify;
    wl_signal_add(&wlr_output->events.frame, &output->frame);
    output->destroy.notify = output_destroy_notify;
    wl_signal_add(&wlr_output->events.destroy, &output->destroy);
    wl_list_insert(&server->outputs, &output->link);

#ifdef DEBUG
    fprintf(stderr, "stratwm: output added %s\n", wlr_output->name);
#endif
}

static void view_map_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_view *view = wl_container_of(listener, view, map);
    struct stratwm_server *server = view->server;
#ifdef DEBUG
    fprintf(stderr, "stratwm: view_map_notify fired (ws=%d, floating=%d)\n",
        view->workspace_id, view->is_floating ? 1 : 0);
#endif

    wlr_scene_node_set_enabled(&view->scene_tree->node, true);

    /* Damage all outputs to force repaint of new window */
    struct stratwm_output *output;
    wl_list_for_each(output, &server->outputs, link) {
        wlr_output_schedule_frame(output->wlr_output);
    }

    /* Create window border decoration (Phase 8.3) */
    int pad = server->deco_border_pad;
    float border_color[4] = {0.267f, 0.267f, 0.267f, 1.0f};  /* default unfocused: dark gray */
    view->border = wlr_scene_rect_create(view->scene_tree, 1, 1, border_color);
    if (view->border) {
        wlr_scene_node_set_position(&view->border->node, -pad, -pad);
        wlr_scene_node_lower_to_bottom(&view->border->node);
    }

    /* Create titlebar with buttons (Phase 8.8) */
    create_titlebar(view);

    /* Insert into tiling tree (Phase 8.2), unless floating (Phase 8.5) */
    struct stratwm_workspace *ws = &server->workspaces[view->workspace_id];
    bool first_tiled_in_workspace = (!ws->root || tile_leaf_count(ws->root) == 0);
    
    if (view->is_floating) {
        /* Floating window: render at fixed position, not in tree */
        wlr_scene_node_set_position(&view->scene_tree->node,
            view->float_x, view->float_y);
        int fw = view->xdg_toplevel->current.width;
        int fh = view->xdg_toplevel->current.height;
        if (fw <= 0) {
            fw = 800;
        }
        if (fh <= 0) {
            fh = 600;
        }
        int th = server->deco_titlebar_h;
        if (view->border) {
            wlr_scene_rect_set_size(view->border, fw + 2 * pad, fh + 2 * pad);
            wlr_scene_node_set_position(&view->border->node, -pad, -pad);
        }
        /* Size titlebar for floating window */
        if (view->titlebar_bg) {
            wlr_scene_rect_set_size(view->titlebar_bg, fw, th);
            wlr_scene_node_set_position(&view->titlebar_bg->node, 0, -th);
        }
        update_titlebar_buttons(view, fw);
    } else {
        /* Tiled window: insert into BSP tree */
        if (!ws->root) {
            /* Initialize root to first output's geometry, or 1920x1080 default */
            struct stratwm_output *output;
            struct wlr_box output_box = {0, 0, 1920, 1080};
            wl_list_for_each(output, &server->outputs, link) {
                output_box.width = output->wlr_output->width;
                output_box.height = output->wlr_output->height;
                break;
            }
            if (output_box.width <= 0 || output_box.height <= 0) {
#ifdef DEBUG
                fprintf(stderr,
                    "stratwm: invalid output geometry (%dx%d), using fallback 1920x1080\n",
                    output_box.width, output_box.height);
#endif
                output_box.width = 1920;
                output_box.height = 1080;
            }
            int zone = get_top_exclusive_zone(server);
            output_box.y += zone;
            output_box.height -= zone;
            ws->root = tile_new(output_box);
            if (!ws->root) {
#ifdef DEBUG
                fprintf(stderr, "stratwm: tile_new failed for workspace=%d\n", ws->id);
#endif
                focus_view(view, view->xdg_toplevel->base->surface);
                return;
            }
#ifdef DEBUG
            fprintf(stderr,
                "stratwm: workspace=%d root initialized to %dx%d+%d+%d\n",
                ws->id, output_box.width, output_box.height, output_box.x, output_box.y);
#endif
        }

        ws->root = tile_insert(ws->root, view, ws->root->geometry);
        if (!ws->root) {
#ifdef DEBUG
            fprintf(stderr, "stratwm: tile_insert returned NULL for workspace=%d\n", ws->id);
#endif
            focus_view(view, view->xdg_toplevel->base->surface);
            return;
        }
        struct stratwm_tile *mapped_tile = tile_find_view(ws->root, view);
        if (!mapped_tile) {
#ifdef DEBUG
            fprintf(stderr, "stratwm: ERROR view not found in BSP tree after insert (ws=%d)\n",
                ws->id);
#endif
#ifdef DEBUG
        } else if (first_tiled_in_workspace) {
            if (ws->root->view != view) {
                fprintf(stderr,
                    "stratwm: WARN first tiled view did not land on root leaf (ws=%d)\n",
                    ws->id);
            } else {
                fprintf(stderr, "stratwm: first tiled view inserted at workspace root (ws=%d)\n",
                    ws->id);
            }
        }
#else
        } else if (first_tiled_in_workspace) {
        }
#endif
        assert(mapped_tile != NULL);
        tile_reflow_scene(ws->root);
    }

    server->focused_view = view;
    focus_view(view, view->xdg_toplevel->base->surface);
}

static void view_commit_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_view *view = wl_container_of(listener, view, commit);

    if (!view->xdg_toplevel->base->initial_commit) {
        return;
    }

    uint32_t serial = wlr_xdg_toplevel_set_size(view->xdg_toplevel, 0, 0);
    (void)serial;
#ifdef DEBUG
    fprintf(stderr, "stratwm: initial configure queued after commit serial=%u\n", serial);
#endif
}

static void view_unmap_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_view *view = wl_container_of(listener, view, unmap);
    struct stratwm_server *server = view->server;

    wlr_scene_node_set_enabled(&view->scene_tree->node, false);

    /* Remove from tiling tree only if not floating (Phase 8.2, 8.5) */
    if (!view->is_floating) {
        struct stratwm_workspace *ws = &server->workspaces[view->workspace_id];
        if (ws->root) {
            ws->root = tile_remove(ws->root, view);
            tile_reflow_scene(ws->root);
        }
    }

    if (server->focused_view == view) {
        server->focused_view = NULL;
    }
}

static void view_request_move_notify(struct wl_listener *listener, void *data) {
    struct stratwm_view *view = wl_container_of(listener, view, request_move);
    struct stratwm_server *server = view->server;
    (void)data;

    if (!view->is_floating) {
        struct stratwm_workspace *ws = &server->workspaces[view->workspace_id];
        if (ws->root) {
            ws->root = tile_remove(ws->root, view);
            tile_reflow_scene(ws->root);
        }
        view->is_floating = true;
        view->float_x = view->scene_tree->node.x;
        view->float_y = view->scene_tree->node.y;
    }

    server->grabbed_view = view;
    server->grab_x = server->cursor->x;
    server->grab_y = server->cursor->y;
    server->grab_view_x = view->float_x;
    server->grab_view_y = view->float_y;
}

static void view_destroy_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_view *view = wl_container_of(listener, view, destroy);
    struct stratwm_server *server = view->server;

    /* Clean up titlebar elements (Phase 8.8) */
    destroy_titlebar(view);

    /* Clean up from tiling tree if still present */
    struct stratwm_workspace *ws = &server->workspaces[view->workspace_id];
    if (ws->root) {
        ws->root = tile_remove(ws->root, view);
        tile_reflow_scene(ws->root);
    }

    wl_list_remove(&view->map.link);
    wl_list_remove(&view->unmap.link);
    wl_list_remove(&view->commit.link);
    wl_list_remove(&view->destroy.link);
    wl_list_remove(&view->request_move.link);
    wl_list_remove(&view->link);
    free(view);
}

static void server_new_xdg_toplevel_notify(struct wl_listener *listener, void *data) {
#ifdef DEBUG
    fprintf(stderr, "stratwm: new xdg_toplevel created\n");
#endif
    struct stratwm_server *server = wl_container_of(listener, server, new_xdg_toplevel);
    struct wlr_xdg_toplevel *xdg_toplevel = data;

    struct stratwm_view *view = calloc(1, sizeof(*view));
    if (view == NULL) {
        return;
    }

    view->server = server;
    view->xdg_toplevel = xdg_toplevel;
    view->workspace_id = server->current_workspace;  /* Assign to current workspace */
    view->scene_tree = wlr_scene_xdg_surface_create(server->layers_normal,
        xdg_toplevel->base);

    if (view->scene_tree == NULL) {
        free(view);
        return;
    }

    wlr_scene_node_set_position(&view->scene_tree->node, 0, 0);
    wlr_scene_node_set_enabled(&view->scene_tree->node, false);

    view->map.notify = view_map_notify;
    wl_signal_add(&xdg_toplevel->base->surface->events.map, &view->map);
    view->unmap.notify = view_unmap_notify;
    wl_signal_add(&xdg_toplevel->base->surface->events.unmap, &view->unmap);
    view->commit.notify = view_commit_notify;
    wl_signal_add(&xdg_toplevel->base->surface->events.commit, &view->commit);
    view->destroy.notify = view_destroy_notify;
    wl_signal_add(&xdg_toplevel->events.destroy, &view->destroy);
    view->request_move.notify = view_request_move_notify;
    wl_signal_add(&xdg_toplevel->events.request_move, &view->request_move);

    xdg_toplevel->base->data = view;
    wl_list_insert(&server->views, &view->link);
}

/* Workspace switching (Phase 8.2+) */
static void switch_workspace(struct stratwm_server *server, int workspace_id) {
    if (workspace_id < 0 || workspace_id >= STRATWM_WORKSPACES) return;
    if (workspace_id == server->current_workspace) return;

    /* Hide all views from old workspace */
    struct stratwm_view *view;
    wl_list_for_each(view, &server->views, link) {
        if (view->workspace_id == server->current_workspace) {
            wlr_scene_node_set_enabled(&view->scene_tree->node, false);
        }
    }

    /* Switch workspace */
    server->current_workspace = workspace_id;

    /* Show/hide views based on new workspace's layout mode (Phase 8.6) */
    struct stratwm_workspace *new_ws = &server->workspaces[workspace_id];
    wl_list_for_each(view, &server->views, link) {
        if (view->workspace_id != server->current_workspace) continue;
        
        bool visible = true;
        if (!view->is_floating) {
            switch (new_ws->layout) {
                case LAYOUT_BSP:
                    visible = true;
                    break;
                case LAYOUT_STACK:
                case LAYOUT_FULLSCREEN:
                    /* Will set properly after focus_view call */
                    visible = false;
                    break;
            }
        } else {
            visible = true; /* Floating windows always visible */
        }
        
        wlr_scene_node_set_enabled(&view->scene_tree->node, visible);
    }

    /* Update focus to first view in new workspace */
    server->focused_view = NULL;
    struct stratwm_view *first_view = NULL;
    wl_list_for_each(view, &server->views, link) {
        if (view->workspace_id == server->current_workspace) {
            first_view = view;
            break;
        }
    }
    if (first_view) {
        wlr_scene_node_set_enabled(&first_view->scene_tree->node, true);
        focus_view(first_view, first_view->xdg_toplevel->base->surface);
    }
}

/* Float toggle: escape from tiling for single window (Phase 8.5) */
static void toggle_float(struct stratwm_server *server, struct stratwm_view *view) {
    if (!view) return;

    struct stratwm_workspace *ws = &server->workspaces[view->workspace_id];

    if (!view->is_floating) {
        /* Tiled → floating: remove from tree, save current position */
        if (ws->root) {
            ws->root = tile_remove(ws->root, view);
            tile_reflow_scene(ws->root);
        }
        view->is_floating = true;

        /* Position floating window: center of screen or 100,100 offset */
        struct stratwm_output *output = NULL;
        wl_list_for_each(output, &server->outputs, link) {
            int zone = get_top_exclusive_zone(server);
            view->float_x = 100;
            view->float_y = zone + 20;  /* Below panel with 20px padding */
            break;
        }
        wlr_scene_node_set_position(&view->scene_tree->node,
            view->float_x, view->float_y);
    } else {
        /* Floating → tiled: re-insert into tree */
        view->is_floating = false;

        if (!ws->root) {
            struct wlr_box output_box = {0, 0, 1920, 1080};
            struct stratwm_output *output;
            wl_list_for_each(output, &server->outputs, link) {
                output_box.width = output->wlr_output->width;
                output_box.height = output->wlr_output->height;
                break;
            }
            int zone = get_top_exclusive_zone(server);
            output_box.y += zone;
            output_box.height -= zone;
            ws->root = tile_new(output_box);
        }

        if (ws->root) {
            ws->root = tile_insert(ws->root, view, ws->root->geometry);
            tile_reflow_scene(ws->root);
        }
    }

    if (view->workspace_id == server->current_workspace) {
        stratwm_refresh_tiled_visibility(server);
    }
}

/* Maximize floating window to fill screen (Phase 8.5) */
static void maximize_float_window(struct stratwm_server *server, struct stratwm_view *view) {
    if (!view || !view->is_floating) return;

    /* Expand to fill usable area below panel */
    struct stratwm_output *output = NULL;
    wl_list_for_each(output, &server->outputs, link) {
        int zone = get_top_exclusive_zone(server);
        
        /* Position at output top-left in layout coordinates */
        double ox = 0.0, oy = 0.0;
        wlr_output_layout_output_coords(server->output_layout, output->wlr_output, &ox, &oy);
        view->float_x = (int)ox;
        view->float_y = (int)oy + zone;  /* Below panel */
        
        /* Resize surface to usable dimensions */
        wlr_xdg_toplevel_set_size(view->xdg_toplevel,
            output->wlr_output->width, output->wlr_output->height - zone);
        
        wlr_scene_node_set_position(&view->scene_tree->node,
            view->float_x, view->float_y);
        break;
    }
}

/* Layout switching: cycle through BSP/Stack/Fullscreen (Phase 8.6) */
static void cycle_layout(struct stratwm_server *server) {
    struct stratwm_workspace *ws = &server->workspaces[server->current_workspace];
    
    /* Cycle layout: BSP → Stack → Fullscreen → BSP */
    ws->layout = (ws->layout + 1) % 3;

    stratwm_refresh_tiled_visibility(server);
}

static bool handle_keybinding(struct stratwm_server *server, xkb_keysym_t sym,
    uint32_t modifiers) {

    /* Primary keybindings: Super+key (spec per Task F Phase 8.1) */
    bool super_pressed = (modifiers & WLR_MODIFIER_LOGO) != 0;
    bool shift_pressed = (modifiers & WLR_MODIFIER_SHIFT) != 0;

    /* Super+Return = spawn terminal */
    if (super_pressed && sym == XKB_KEY_Return) {
        spawn_terminal();
        return true;
    }

    /* Super+Q = close focused window */
    if (super_pressed && (sym == XKB_KEY_q || sym == XKB_KEY_Q)) {
        struct wlr_surface *focused = server->seat->keyboard_state.focused_surface;
        struct stratwm_view *view = view_from_surface(server, focused);
        if (view != NULL) {
            wlr_xdg_toplevel_send_close(view->xdg_toplevel);
        }
        return true;
    }

    /* Super+Shift+E = exit compositor */
    if (super_pressed && shift_pressed && (sym == XKB_KEY_e || sym == XKB_KEY_E)) {
        wl_display_terminate(server->wl_display);
        return true;
    }

    /* Fallback F-keys for testing in host WM contexts (Super key often intercepted by host) */
    if (sym == XKB_KEY_F1) {
        spawn_terminal();
        return true;
    }

    if (sym == XKB_KEY_F2) {
        wl_display_terminate(server->wl_display);
        return true;
    }

    if (sym == XKB_KEY_F3) {
        struct wlr_surface *focused = server->seat->keyboard_state.focused_surface;
        struct stratwm_view *view = view_from_surface(server, focused);
        if (view != NULL) {
            wlr_xdg_toplevel_send_close(view->xdg_toplevel);
        }
        return true;
    }

    /* Focus navigation: arrow keys cycle through tiles */
    if (super_pressed && (sym == XKB_KEY_Right || sym == XKB_KEY_l)) {
        if (server->focused_view) {
            struct stratwm_tile *current = 
                tile_find_view(server->workspaces[server->current_workspace].root,
                    server->focused_view);
            if (current) {
                struct stratwm_tile *next = tile_next_leaf(current);
                if (next && next->view) {
                    server->focused_view = next->view;
                    focus_view(next->view, next->view->xdg_toplevel->base->surface);
                    stratwm_refresh_tiled_visibility(server);
                }
            }
        }
        return true;
    }

    if (super_pressed && (sym == XKB_KEY_Left || sym == XKB_KEY_h)) {
        if (server->focused_view) {
            struct stratwm_tile *current =
                tile_find_view(server->workspaces[server->current_workspace].root,
                    server->focused_view);
            if (current) {
                struct stratwm_tile *prev = tile_prev_leaf(current);
                if (prev && prev->view) {
                    server->focused_view = prev->view;
                    focus_view(prev->view, prev->view->xdg_toplevel->base->surface);
                    stratwm_refresh_tiled_visibility(server);
                }
            }
        }
        return true;
    }

    /* Float toggle: Super+F escapes/returns to tiling (Phase 8.5) */
    if (super_pressed && (sym == XKB_KEY_f || sym == XKB_KEY_F)) {
        if (server->focused_view) {
            toggle_float(server, server->focused_view);
        }
        return true;
    }

    /* Maximize floating window: Super+M (Phase 8.5) */
    if (super_pressed && (sym == XKB_KEY_m || sym == XKB_KEY_M)) {
        if (server->focused_view) {
            maximize_float_window(server, server->focused_view);
        }
        return true;
    }

    /* Spotlite overlay + layout cycle — keys from stratvm-keybinds (defaults: Super+period / Super+Space) */
    if (modifiers == strat_kb_spotlite_mods && sym == strat_kb_spotlite_sym) {
        spawn_spotlite();
        return true;
    }
    if (modifiers == strat_kb_cycle_mods && sym == strat_kb_cycle_sym) {
        cycle_layout(server);
        return true;
    }

    /* Tile resizing with Super+Shift+{H,J,K,L} (Phase 8.7) */
    if (super_pressed && shift_pressed) {
        int delta = 50;  /* pixel adjustment per keypress */
        if (sym == XKB_KEY_h || sym == XKB_KEY_H) {
            resize_tile_horizontal(server, -delta);  /* Shrink right, grow left */
            return true;
        }
        if (sym == XKB_KEY_l || sym == XKB_KEY_L) {
            resize_tile_horizontal(server, delta);   /* Grow right, shrink left */
            return true;
        }
        if (sym == XKB_KEY_j || sym == XKB_KEY_J) {
            resize_tile_vertical(server, delta);     /* Grow bottom, shrink top */
            return true;
        }
        if (sym == XKB_KEY_k || sym == XKB_KEY_K) {
            resize_tile_vertical(server, -delta);    /* Shrink bottom, grow top */
            return true;
        }
    }

    /* Workspace switching: Super+1 through Super+9 */
    if (super_pressed && sym >= XKB_KEY_1 && sym <= XKB_KEY_9) {
        int workspace = sym - XKB_KEY_1;  /* 0-8 for keys 1-9 */
        switch_workspace(server, workspace);
        return true;
    }

    return false;
}

static void keyboard_modifiers_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_keyboard *keyboard = wl_container_of(listener, keyboard, modifiers);
    struct wlr_keyboard *wlr_kb = wlr_keyboard_from_input_device(keyboard->device);
    if (wlr_kb == NULL) {
        return;
    }
    wlr_seat_set_keyboard(keyboard->server->seat, wlr_kb);
    wlr_seat_keyboard_notify_modifiers(keyboard->server->seat, &wlr_kb->modifiers);
}

static void keyboard_key_notify(struct wl_listener *listener, void *data) {
    struct stratwm_keyboard *keyboard = wl_container_of(listener, keyboard, key);
    struct stratwm_server *server = keyboard->server;
    struct wlr_keyboard_key_event *event = data;
    struct wlr_keyboard *wlr_keyboard = wlr_keyboard_from_input_device(keyboard->device);
    if (wlr_keyboard == NULL) {
        return;
    }

    uint32_t modifiers = wlr_keyboard_get_modifiers(wlr_keyboard);
    uint32_t keycode = event->keycode + 8;
    const xkb_keysym_t *syms = NULL;
    int nsyms = xkb_state_key_get_syms(wlr_keyboard->xkb_state, keycode, &syms);

    bool handled = false;
    if (event->state == WL_KEYBOARD_KEY_STATE_PRESSED) {
        for (int i = 0; i < nsyms; ++i) {
            if (handle_keybinding(server, syms[i], modifiers)) {
                handled = true;
                break;
            }
        }
    }

    if (!handled) {
        wlr_seat_set_keyboard(server->seat, wlr_keyboard);
        wlr_seat_keyboard_notify_key(server->seat, event->time_msec,
            event->keycode, event->state);
    }
}

static void keyboard_destroy_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_keyboard *keyboard = wl_container_of(listener, keyboard, destroy);
    wl_list_remove(&keyboard->modifiers.link);
    wl_list_remove(&keyboard->key.link);
    wl_list_remove(&keyboard->destroy.link);
    wl_list_remove(&keyboard->link);
    free(keyboard);
}

static void stratwm_apply_move_grab(struct stratwm_server *server) {
    if (!server->grabbed_view) {
        return;
    }
    int dx = (int)(server->cursor->x - server->grab_x);
    int dy = (int)(server->cursor->y - server->grab_y);
    server->grabbed_view->float_x = server->grab_view_x + dx;
    server->grabbed_view->float_y = server->grab_view_y + dy;
    wlr_scene_node_set_position(&server->grabbed_view->scene_tree->node,
        server->grabbed_view->float_x, server->grabbed_view->float_y);
}

static struct wlr_surface *stratwm_surface_at_layout(struct stratwm_server *server,
    double lx, double ly, double *sx, double *sy) {
    struct wlr_scene_tree *stack[] = {
        server->layers_overlay,
        server->layers_top,
        server->layers_normal,
        server->layers_bottom,
        server->layers_bg,
    };
    for (size_t i = 0; i < sizeof(stack) / sizeof(stack[0]); i++) {
        struct wlr_scene_node *node = wlr_scene_node_at(
            &stack[i]->node, lx, ly, sx, sy);
        if (node == NULL) {
            continue;
        }
        for (struct wlr_scene_node *n = node; n != NULL; n = n->parent) {
            if (n->type == WLR_SCENE_NODE_SURFACE) {
                struct wlr_scene_surface *ss = wlr_scene_surface_from_node(n);
                return ss->surface;
            }
        }
        return NULL;
    }
    return NULL;
}

static struct stratwm_view *stratwm_view_at_cursor(struct stratwm_server *server) {
    double lx = server->cursor->x;
    double ly = server->cursor->y;
    double sx, sy;
    struct wlr_surface *surf = stratwm_surface_at_layout(server, lx, ly, &sx, &sy);
    if (surf != NULL) {
        return view_from_surface(server, surf);
    }

    struct stratwm_view *v;
    wl_list_for_each_reverse(v, &server->views, link) {
        if (v->workspace_id != server->current_workspace) {
            continue;
        }
        if (!wlr_scene_node_is_enabled(&v->scene_tree->node)) {
            continue;
        }
        double view_x = lx - (double)v->scene_tree->node.x;
        double view_y = ly - (double)v->scene_tree->node.y;
        int th = v->server->deco_titlebar_h;
        if (v->decorations_visible && v->titlebar_bg
            && view_x >= 0 && view_x < v->xdg_toplevel->current.width
            && view_y >= (double)-th && view_y < 0) {
            return v;
        }
        if (point_in_titlebar_button(v->close_button, view_x, view_y)
            || point_in_titlebar_button(v->max_button, view_x, view_y)
            || point_in_titlebar_button(v->min_button, view_x, view_y)) {
            return v;
        }
    }
    return NULL;
}

static void stratwm_process_cursor_motion(struct stratwm_server *server, uint32_t time_msec) {
    stratwm_apply_move_grab(server);
    if (server->grabbed_view != NULL) {
        return;
    }

    double lx = server->cursor->x;
    double ly = server->cursor->y;
    double sx, sy;
    struct wlr_surface *surface = stratwm_surface_at_layout(server, lx, ly, &sx, &sy);

    if (surface != NULL) {
        wlr_seat_pointer_notify_enter(server->seat, surface, sx, sy);
        wlr_seat_pointer_notify_motion(server->seat, time_msec, sx, sy);
    } else {
        wlr_seat_pointer_clear_focus(server->seat);
        if (server->cursor_manager != NULL) {
            wlr_xcursor_manager_set_cursor_image(server->cursor_manager,
                "left_ptr", server->cursor);
        }
    }
}

static void seat_request_cursor_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, request_cursor);
    struct wlr_seat_pointer_request_set_cursor_event *event = data;
    wlr_cursor_set_surface(server->cursor, event->surface, event->hotspot_x, event->hotspot_y);
}

/* Compositor-side pointer grabs (titlebar) + libinput path; evdev reuses this. */
static void stratwm_process_pointer_button(
    struct stratwm_server *server,
    uint32_t time_msec,
    uint32_t button,
    uint32_t state
) {
    if (state == WL_POINTER_BUTTON_STATE_RELEASED && server->grabbed_view
        && button == BTN_LEFT) {
        server->grabbed_view = NULL;
        wlr_seat_pointer_notify_button(server->seat, time_msec, button, state);
        return;
    }

    struct stratwm_view *view = stratwm_view_at_cursor(server);
    if (state == WL_POINTER_BUTTON_STATE_PRESSED && view != NULL) {
        double cursor_x = server->cursor->x;
        double cursor_y = server->cursor->y;

        double view_x = cursor_x - view->scene_tree->node.x;
        double view_y = cursor_y - view->scene_tree->node.y;

        if (button == BTN_RIGHT && titlebar_context_region(view, view_x, view_y)) {
            uint32_t mods = stratwm_seat_modifiers(server);
            if (mods & WLR_MODIFIER_SHIFT) {
                toggle_float(server, view);
            } else if (mods & WLR_MODIFIER_LOGO) {
                int n = (view->workspace_id + 1) % STRATWM_WORKSPACES;
                move_view_to_workspace(server, view, n);
            } else {
                set_view_decorations_visible(view, !view->decorations_visible);
            }
            return;
        }

        if (button != BTN_LEFT) {
            wlr_seat_pointer_notify_button(server->seat, time_msec, button, state);
            return;
        }

        if (point_in_titlebar_button(view->close_button, view_x, view_y)) {
            wlr_xdg_toplevel_send_close(view->xdg_toplevel);
            return;
        }

        if (point_in_titlebar_button(view->max_button, view_x, view_y)) {
            toggle_float(server, view);
            return;
        }

        if (point_in_titlebar_button(view->min_button, view_x, view_y)) {
            wlr_xdg_toplevel_send_close(view->xdg_toplevel);
            return;
        }

        int th = view->server->deco_titlebar_h;
        if (view->decorations_visible && view->titlebar_bg
            && view_x >= 0 && view_x < view->xdg_toplevel->current.width
            && view_y >= (double)-th && view_y < 0) {
            if (!view->is_floating) {
                struct stratwm_workspace *ws = &server->workspaces[view->workspace_id];
                if (ws->root) {
                    ws->root = tile_remove(ws->root, view);
                    tile_reflow_scene(ws->root);
                }
                view->is_floating = true;
                view->float_x = view->scene_tree->node.x;
                view->float_y = view->scene_tree->node.y;
            }
            server->grabbed_view = view;
            server->grab_x = cursor_x;
            server->grab_y = cursor_y;
            server->grab_view_x = view->float_x;
            server->grab_view_y = view->float_y;
            return;
        }
    }

    wlr_seat_pointer_notify_button(server->seat, time_msec, button, state);
}

static void cursor_motion_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, cursor_motion);
    struct wlr_pointer_motion_event *event = data;

    wlr_cursor_move(server->cursor, NULL, event->delta_x, event->delta_y);
    stratwm_process_cursor_motion(server, event->time_msec);
}

static void cursor_motion_absolute_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, cursor_motion_absolute);
    struct wlr_pointer_motion_absolute_event *event = data;

    wlr_cursor_warp_absolute(server->cursor, NULL, event->x, event->y);
    stratwm_process_cursor_motion(server, event->time_msec);
}

static void cursor_button_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, cursor_button);
    struct wlr_pointer_button_event *event = data;

    stratwm_process_pointer_button(server, event->time_msec, event->button, event->state);
}

static void cursor_axis_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, cursor_axis);
    struct wlr_pointer_axis_event *event = data;

    wlr_seat_pointer_notify_axis(server->seat, event->time_msec,
        event->orientation, event->delta, event->delta_discrete, event->source,
        event->relative_direction);
}

static void cursor_frame_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_server *server = wl_container_of(listener, server, cursor_frame);
    wlr_seat_pointer_notify_frame(server->seat);
}

static void server_new_input_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, new_input);
    struct wlr_input_device *device = data;

    switch (device->type) {
    case WLR_INPUT_DEVICE_KEYBOARD: {
        struct stratwm_keyboard *keyboard = calloc(1, sizeof(*keyboard));
        if (keyboard == NULL) {
            return;
        }

        keyboard->server = server;
        keyboard->device = device;

        struct wlr_keyboard *wlr_kb = wlr_keyboard_from_input_device(device);
        if (wlr_kb == NULL) {
            free(keyboard);
            return;
        }

        struct xkb_context *context = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
        struct xkb_keymap *keymap = xkb_keymap_new_from_names(context, NULL,
            XKB_KEYMAP_COMPILE_NO_FLAGS);
        wlr_keyboard_set_keymap(wlr_kb, keymap);
        xkb_keymap_unref(keymap);
        xkb_context_unref(context);
        wlr_keyboard_set_repeat_info(wlr_kb, 25, 600);

        keyboard->modifiers.notify = keyboard_modifiers_notify;
        wl_signal_add(&wlr_kb->events.modifiers, &keyboard->modifiers);
        keyboard->key.notify = keyboard_key_notify;
        wl_signal_add(&wlr_kb->events.key, &keyboard->key);
        keyboard->destroy.notify = keyboard_destroy_notify;
        wl_signal_add(&device->events.destroy, &keyboard->destroy);

        wl_list_insert(&server->keyboards, &keyboard->link);
        wlr_seat_set_keyboard(server->seat, wlr_kb);
        break;
    }
    case WLR_INPUT_DEVICE_POINTER:
        wlr_cursor_attach_input_device(server->cursor, device);
        break;
    default:
        break;
    }

    uint32_t caps = WL_SEAT_CAPABILITY_POINTER;
    if (!wl_list_empty(&server->keyboards)) {
        caps |= WL_SEAT_CAPABILITY_KEYBOARD;
    }
    wlr_seat_set_capabilities(server->seat, caps);
}

/* IPC socket server (Phase 24a) */
static void ipc_remove_client(struct stratwm_ipc *ipc, int idx) {
    wl_event_source_remove(ipc->clients[idx].event_source);
    close(ipc->clients[idx].fd);
    ipc->clients[idx] = ipc->clients[ipc->client_count - 1];
    ipc->client_count--;
}

static void ipc_send(int fd, const char *msg) {
    write(fd, msg, strlen(msg));
}

static void ipc_dispatch_command(struct stratwm_server *server, const char *cmd, int fd) {
    if (strcmp(cmd, "ping") == 0) {
        ipc_send(fd, "OK pong\n");
    } else if (strcmp(cmd, "get focused") == 0) {
        struct stratwm_view *view = server->focused_view;
        pid_t pid = 0;
        if (view) {
            struct wl_client *wl_client = wl_resource_get_client(view->xdg_toplevel->base->resource);
            wl_client_get_credentials(wl_client, &pid, NULL, NULL);
        }
        if (view && pid > 0) {
            char buf[64];
            snprintf(buf, sizeof(buf), "OK %d\n", pid);
            ipc_send(fd, buf);
        } else {
            ipc_send(fd, "OK 0\n");
        }
    } else if (strcmp(cmd, "get workspaces") == 0) {
        char buf[512] = "{\"workspaces\":[";
        int first = 1;
        for (int i = 0; i < 9; i++) {
            char part[64];
            int count = 0;
            struct stratwm_view *v;
            wl_list_for_each(v, &server->views, link) {
                if (v->workspace_id == i) count++;
            }
            bool focused = (i == server->current_workspace);
            if (first) {
                snprintf(part, sizeof(part), "{\"id\":%d,\"name\":\"%d\",\"focused\":%s}",
                    i + 1, i + 1, focused ? "true" : "false");
                first = 0;
            } else {
                snprintf(part, sizeof(part), ",{\"id\":%d,\"name\":\"%d\",\"focused\":%s}",
                    i + 1, i + 1, focused ? "true" : "false");
            }
            strncat(buf, part, sizeof(buf) - strlen(buf) - 1);
        }
        strncat(buf, "]}\n", sizeof(buf) - strlen(buf) - 1);
        ipc_send(fd, buf);
    } else if (strncmp(cmd, "float window ", 13) == 0) {
        int pid = atoi(cmd + 13);
        struct stratwm_view *v;
        bool found = false;
        wl_list_for_each(v, &server->views, link) {
            struct wl_client *wl_client = wl_resource_get_client(v->xdg_toplevel->base->resource);
            pid_t v_pid = 0;
            wl_client_get_credentials(wl_client, &v_pid, NULL, NULL);
            if (v_pid == pid) {
                toggle_float(server, v);
                ipc_send(fd, "OK\n");
                found = true;
                break;
            }
        }
        if (!found) ipc_send(fd, "ERROR not found\n");
    } else if (strncmp(cmd, "set panel autohide ", 19) == 0) {
        server->panel_autohide = strcmp(cmd + 19, "true") == 0;
        ipc_send(fd, "OK\n");
    } else if (strncmp(cmd, "switch_workspace ", 16) == 0) {
        int workspace_id = atoi(cmd + 16) - 1;  // Convert from 1-indexed to 0-indexed
        if (workspace_id >= 0 && workspace_id < STRATWM_WORKSPACES) {
            switch_workspace(server, workspace_id);
            ipc_send(fd, "ok\n");
        } else {
            ipc_send(fd, "ERROR invalid workspace\n");
        }
    } else if (strcmp(cmd, "reload_keybinds") == 0) {
        stratwm_load_keybinds();
        ipc_send(fd, "OK\n");
    } else if (strcmp(cmd, "trigger_coverflow") == 0) {
        ipc_send(fd, "OK\n"); // stub — Phase 25
    } else if (strncmp(cmd, "trigger_pivot_overlay ", 22) == 0) {
        ipc_send(fd, "OK\n"); // stub — future
    } else {
        ipc_send(fd, "ERROR unknown command\n");
    }
}

static void ipc_read_client(struct stratwm_server *server, struct stratwm_ipc *ipc, int idx) {
    struct stratwm_ipc_client *client = &ipc->clients[idx];
    int n = read(client->fd, client->buf + client->buf_len,
                 IPC_BUF_SIZE - client->buf_len - 1);
    if (n <= 0) { ipc_remove_client(ipc, idx); return; }
    client->buf_len += n;
    client->buf[client->buf_len] = '\0';
    char *newline;
    while ((newline = strchr(client->buf, '\n')) != NULL) {
        *newline = '\0';
        ipc_dispatch_command(server, client->buf, client->fd);
        int consumed = (newline - client->buf) + 1;
        client->buf_len -= consumed;
        memmove(client->buf, newline + 1, client->buf_len);
        client->buf[client->buf_len] = '\0';
    }
}

/* Direct evdev input manager (bypasses udev/libinput) */
static int input_manager_init(struct stratwm_server *server);
static void input_manager_probe_devices(struct stratwm_server *server);
static void input_manager_destroy(struct stratwm_server *server);

static int ipc_handle_event(int fd, uint32_t mask, void *data);

static void ipc_accept_client(struct stratwm_server *server) {
    struct stratwm_ipc *ipc = &server->ipc;
    if (ipc->client_count >= IPC_MAX_CLIENTS) return;
    int fd = accept(ipc->socket_fd, NULL, NULL);
    if (fd < 0) return;
    struct stratwm_ipc_client *client = &ipc->clients[ipc->client_count];
    client->fd = fd;
    client->buf_len = 0;
    client->event_source = wl_event_loop_add_fd(
        wl_display_get_event_loop(server->wl_display),
        fd, WL_EVENT_READABLE, ipc_handle_event, server);
    ipc->client_count++;
}

static int ipc_handle_event(int fd, uint32_t mask, void *data) {
    (void)mask;
    struct stratwm_server *server = data;
    struct stratwm_ipc *ipc = &server->ipc;
    if (fd == ipc->socket_fd) {
        ipc_accept_client(server);
    } else {
        for (int i = 0; i < ipc->client_count; i++) {
            if (ipc->clients[i].fd == fd) {
                ipc_read_client(server, ipc, i);
                break;
            }
        }
    }
    return 0;
}

static void ipc_init(struct stratwm_server *server) {
    struct stratwm_ipc *ipc = &server->ipc;
    ipc->socket_fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (ipc->socket_fd < 0) return;
    struct sockaddr_un addr = {0};
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, "/run/stratvm.sock", sizeof(addr.sun_path) - 1);
    unlink("/run/stratvm.sock");
    if (bind(ipc->socket_fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) return;
    listen(ipc->socket_fd, IPC_MAX_CLIENTS);
    ipc->client_count = 0;
    struct wl_event_loop *loop = wl_display_get_event_loop(server->wl_display);
    ipc->event_source = wl_event_loop_add_fd(loop, ipc->socket_fd,
        WL_EVENT_READABLE, ipc_handle_event, server);
}

static void ipc_finish(struct stratwm_ipc *ipc) {
    for (int i = 0; i < ipc->client_count; i++) {
        wl_event_source_remove(ipc->clients[i].event_source);
        close(ipc->clients[i].fd);
    }
    wl_event_source_remove(ipc->event_source);
    close(ipc->socket_fd);
    unlink("/run/stratvm.sock");
}

int main(void) {
    wlr_log_init(WLR_ERROR, NULL);

    struct stratwm_server server;
    memset(&server, 0, sizeof(server));
    wl_list_init(&server.outputs);
    wl_list_init(&server.views);
    wl_list_init(&server.keyboards);

    /* Initialize workspaces (Phase 8.2) */
    server.current_workspace = 0;
    for (int i = 0; i < STRATWM_WORKSPACES; i++) {
        server.workspaces[i].id = i;
        server.workspaces[i].root = NULL;
        server.workspaces[i].focused = NULL;
        server.workspaces[i].layout = LAYOUT_BSP;  /* Default to BSP layout (Phase 8.6) */
    }
    server.focused_view = NULL;

    server.deco_titlebar_h = 24;
    server.deco_border_pad = 2;
    server.default_decorations_visible = true;
    stratwm_load_deco_config(&server);
    stratwm_load_modular_chrome(&server);
    stratwm_load_keybinds();

    /* Guarantee VM boot even if the environment gets scrubbed */
    setenv("WLR_LIBINPUT_NO_DEVICES", "1", 1);

    server.wl_display = wl_display_create();
    if (server.wl_display == NULL) {
#ifdef DEBUG
        fprintf(stderr, "stratwm: failed to create wl_display\n");
#endif
        return 1;
    }

    server.backend = wlr_backend_autocreate(wl_display_get_event_loop(server.wl_display), NULL);
    if (server.backend == NULL) {
#ifdef DEBUG
        fprintf(stderr, "stratwm: failed to create backend\n");
#endif
        return 1;
    }

    server.renderer = wlr_renderer_autocreate(server.backend);
    server.allocator = wlr_allocator_autocreate(server.backend, server.renderer);
    if (server.renderer == NULL || server.allocator == NULL) {
#ifdef DEBUG
        fprintf(stderr, "stratwm: failed to create renderer/allocator\n");
#endif
        return 1;
    }

    wlr_compositor_create(server.wl_display, 5, server.renderer);
#ifdef STRATWM_HAVE_WLR_SUBCOMPOSITOR
    wlr_subcompositor_create(server.wl_display);
#endif

    server.layer_shell = wlr_layer_shell_v1_create(server.wl_display, 4);
    wl_list_init(&server.layer_surfaces);
    server.new_layer_surface.notify = server_new_layer_surface_notify;
    wl_signal_add(&server.layer_shell->events.new_surface, &server.new_layer_surface);
    wlr_data_device_manager_create(server.wl_display);
    wlr_renderer_init_wl_display(server.renderer, server.wl_display);

    server.output_layout = wlr_output_layout_create(server.wl_display);
    server.scene = wlr_scene_create();
    server.scene_layout = wlr_scene_attach_output_layout(server.scene, server.output_layout);
    
    /* Create layer trees for proper layer-shell stacking */
    server.layers_bg = wlr_scene_tree_create(&server.scene->tree);
    server.layers_bottom = wlr_scene_tree_create(&server.scene->tree);
    server.layers_normal = wlr_scene_tree_create(&server.scene->tree);
    server.layers_top = wlr_scene_tree_create(&server.scene->tree);
    server.layers_overlay = wlr_scene_tree_create(&server.scene->tree);
    
    server.xdg_shell = wlr_xdg_shell_create(server.wl_display, 6);
    server.seat = wlr_seat_create(server.wl_display, "seat0");
    server.request_cursor.notify = seat_request_cursor_notify;
    wl_signal_add(&server.seat->events.request_set_cursor, &server.request_cursor);

    server.cursor = wlr_cursor_create();
    wlr_cursor_attach_output_layout(server.cursor, server.output_layout);
    {
        const char *xc_theme = getenv("XCURSOR_THEME");
        if (xc_theme == NULL || xc_theme[0] == '\0') {
            xc_theme = "dmz-white";
        }
        server.cursor_manager = wlr_xcursor_manager_create(xc_theme, 24);
        if (server.cursor_manager == NULL
            || !wlr_xcursor_manager_load(server.cursor_manager, 1)) {
            fprintf(stderr, "stratwm: cursor theme \"%s\" not found, trying default\n", xc_theme);
            server.cursor_manager = wlr_xcursor_manager_create(NULL, 24);
            if (server.cursor_manager != NULL) {
                wlr_xcursor_manager_load(server.cursor_manager, 1);
            }
        }
        if (server.cursor_manager != NULL) {
            wlr_xcursor_manager_set_cursor_image(server.cursor_manager,
                "left_ptr", server.cursor);
        }
    }

    server.cursor_motion.notify = cursor_motion_notify;
    wl_signal_add(&server.cursor->events.motion, &server.cursor_motion);
    server.cursor_motion_absolute.notify = cursor_motion_absolute_notify;
    wl_signal_add(&server.cursor->events.motion_absolute, &server.cursor_motion_absolute);
    server.cursor_button.notify = cursor_button_notify;
    wl_signal_add(&server.cursor->events.button, &server.cursor_button);
    server.cursor_axis.notify = cursor_axis_notify;
    wl_signal_add(&server.cursor->events.axis, &server.cursor_axis);
    server.cursor_frame.notify = cursor_frame_notify;
    wl_signal_add(&server.cursor->events.frame, &server.cursor_frame);

    server.new_output.notify = server_new_output_notify;
    wl_signal_add(&server.backend->events.new_output, &server.new_output);
    server.new_input.notify = server_new_input_notify;
    wl_signal_add(&server.backend->events.new_input, &server.new_input);
    server.new_xdg_toplevel.notify = server_new_xdg_toplevel_notify;
    wl_signal_add(&server.xdg_shell->events.new_toplevel, &server.new_xdg_toplevel);

    const char *socket = wl_display_add_socket_auto(server.wl_display);
    if (socket == NULL) {
#ifdef DEBUG
        fprintf(stderr, "stratwm: failed to create wayland socket\n");
#endif
        return 1;
    }
    setenv("WAYLAND_DISPLAY", socket, 1);

    ipc_init(&server);

    if (!wlr_backend_start(server.backend)) {
#ifdef DEBUG
        fprintf(stderr, "stratwm: failed to start backend\n");
#endif
        return 1;
    }

    /* Initialize direct evdev input (bypasses udev/libinput) */
    input_manager_init(&server);

#ifdef DEBUG
    fprintf(stderr, "stratwm: started (%s)\n", socket);
#endif
    spawn_autostart("/bin/stratterm", socket);
    spawn_autostart("/bin/stratpanel", socket);
    wl_display_run(server.wl_display);

    wl_list_remove(&server.new_xdg_toplevel.link);
    wl_list_remove(&server.new_output.link);
    wl_list_remove(&server.new_input.link);
    wl_list_remove(&server.cursor_motion.link);
    wl_list_remove(&server.cursor_motion_absolute.link);
    wl_list_remove(&server.cursor_button.link);
    wl_list_remove(&server.cursor_axis.link);
    wl_list_remove(&server.cursor_frame.link);
    wl_list_remove(&server.request_cursor.link);

    ipc_finish(&server.ipc);

    wl_display_destroy_clients(server.wl_display);
    wl_display_destroy(server.wl_display);

    /* Clean up workspace trees (Phase 8.2) */
    for (int i = 0; i < STRATWM_WORKSPACES; i++) {
        if (server.workspaces[i].root) {
            tile_free(server.workspaces[i].root);
        }
    }

    input_manager_destroy(&server);

    return 0;
}

/* ============================================================================
 * Direct evdev input manager - bypasses udev/libinput
 * ============================================================================ */

static struct stratwm_input_manager g_input_manager = {0};

static int uevent_open_socket(void) {
    int fd = socket(AF_NETLINK, SOCK_RAW | SOCK_NONBLOCK, NETLINK_KOBJECT_UEVENT);
    if (fd < 0) return -1;

    struct sockaddr_nl addr = {
        .nl_family = AF_NETLINK,
        .nl_pid = getpid(),
        .nl_groups = 1  /* Subscribe to kernel multicast group */
    };

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        close(fd);
        return -1;
    }

    return fd;
}

static bool is_input_device(const char *path) {
    int fd = open(path, O_RDONLY | O_NONBLOCK);
    if (fd < 0) return false;

    struct libevdev *evdev = NULL;
    int rc = libevdev_new_from_fd(fd, &evdev);
    if (rc < 0) {
        close(fd);
        return false;
    }

    bool has_keys = libevdev_has_event_type(evdev, EV_KEY);
    bool has_rel = libevdev_has_event_type(evdev, EV_REL);
    bool has_abs = libevdev_has_event_type(evdev, EV_ABS);

    /* Keyboard: has keys, no relative/absolute motion */
    /* Pointer: has relative motion (mouse) or absolute (touchpad) */
    bool is_kbd = has_keys && !has_rel && !has_abs;
    bool is_ptr = has_rel || (has_abs && !has_keys);

    libevdev_free(evdev);
    close(fd);

    return is_kbd || is_ptr;
}

static int evdev_device_event(int fd, uint32_t mask, void *data);

static struct stratwm_evdev_device *evdev_device_create(
    struct stratwm_server *server, const char *path) {

    struct stratwm_evdev_device *dev = calloc(1, sizeof(*dev));
    if (!dev) return NULL;

    dev->server = server;
    strncpy(dev->path, path, sizeof(dev->path) - 1);
    dev->fd = -1;

    /* Open read-write - some devices require this for grab/ioctl */
    dev->fd = open(path, O_RDWR | O_NONBLOCK | O_CLOEXEC);
    if (dev->fd < 0) {
        /* Fallback to read-only */
        dev->fd = open(path, O_RDONLY | O_NONBLOCK | O_CLOEXEC);
    }
    if (dev->fd < 0) {
        free(dev);
        return NULL;
    }

    int rc = libevdev_new_from_fd(dev->fd, &dev->evdev);
    if (rc < 0) {
        close(dev->fd);
        free(dev);
        return NULL;
    }

    /* Determine device type */
    bool has_keys = libevdev_has_event_type(dev->evdev, EV_KEY);
    bool has_rel = libevdev_has_event_type(dev->evdev, EV_REL);
    bool has_abs = libevdev_has_event_type(dev->evdev, EV_ABS);

    dev->is_keyboard = has_keys && !has_rel && !has_abs;
    dev->is_pointer = has_rel || (has_abs && !has_keys);

    /* Create wlroots input device */
    if (dev->is_keyboard) {
        struct wlr_keyboard *kbd = calloc(1, sizeof(*kbd));
        if (!kbd) goto fail;

        wlr_keyboard_init(kbd, NULL, path);

        /* Set up keymap */
        struct xkb_context *context = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
        struct xkb_keymap *keymap = xkb_keymap_new_from_names(context, NULL,
            XKB_KEYMAP_COMPILE_NO_FLAGS);
        wlr_keyboard_set_keymap(kbd, keymap);
        xkb_keymap_unref(keymap);
        xkb_context_unref(context);

        wlr_keyboard_set_repeat_info(kbd, 25, 600);
        dev->wlr.keyboard = kbd;

        /* Set seat keyboard and capabilities */
        wlr_seat_set_keyboard(server->seat, kbd);
        wlr_seat_set_capabilities(server->seat,
            WL_SEAT_CAPABILITY_KEYBOARD |
            (wl_list_empty(&server->keyboards) ? 0 : WL_SEAT_CAPABILITY_POINTER));

        /* Emit new_input signal */
        wl_signal_emit(&server->backend->events.new_input, &kbd->base);
    } else if (dev->is_pointer) {
        struct wlr_pointer *ptr = calloc(1, sizeof(*ptr));
        if (!ptr) goto fail;

        wlr_pointer_init(ptr, NULL, path);
        dev->wlr.pointer = ptr;

        /* Set seat pointer and capabilities */
        wlr_seat_set_capabilities(server->seat,
            WL_SEAT_CAPABILITY_POINTER |
            (wl_list_empty(&server->keyboards) ? 0 : WL_SEAT_CAPABILITY_KEYBOARD));

        /* Emit new_input signal */
        wl_signal_emit(&server->backend->events.new_input, &ptr->base);
    }

    /* Add to event loop */
    dev->event_source = wl_event_loop_add_fd(
        wl_display_get_event_loop(server->wl_display),
        dev->fd, WL_EVENT_READABLE, evdev_device_event, dev);

    if (!dev->event_source) {
        goto fail;
    }

    wl_list_insert(&g_input_manager.devices, &dev->link);
    goto success;

fail:
    if (dev->evdev) libevdev_free(dev->evdev);
    if (dev->fd >= 0) close(dev->fd);
    free(dev);
    return NULL;

success:
#ifdef DEBUG
    fprintf(stderr, "stratwm: evdev opened %s (kbd=%d, ptr=%d)\n",
            path, dev->is_keyboard, dev->is_pointer);
#endif
    return dev;
}

static void evdev_device_destroy(struct stratwm_evdev_device *dev) {
    if (!dev) return;

    /* Clean up wlroots device */
    if (dev->wlr.device) {
        wl_signal_emit(&dev->wlr.device->events.destroy, dev->wlr.device);
        if (dev->is_keyboard) {
            wlr_keyboard_finish(dev->wlr.keyboard);
            free(dev->wlr.keyboard);
        } else if (dev->is_pointer) {
            wlr_pointer_finish(dev->wlr.pointer);
            free(dev->wlr.pointer);
        }
    }

    if (dev->event_source) {
        wl_event_source_remove(dev->event_source);
    }
    if (dev->evdev) {
        libevdev_free(dev->evdev);
    }
    if (dev->fd >= 0) {
        close(dev->fd);
    }

    wl_list_remove(&dev->link);
    free(dev);
}

static int evdev_device_event(int fd, uint32_t mask, void *data) {
    (void)fd;
    (void)mask;
    struct stratwm_evdev_device *dev = data;
    struct input_event ev;
    static double accum_dx = 0, accum_dy = 0;

    while (libevdev_next_event(dev->evdev, LIBEVDEV_READ_FLAG_NORMAL, &ev) == LIBEVDEV_READ_STATUS_SUCCESS) {
        if (ev.type == EV_SYN && ev.code == SYN_REPORT) {
            /* Send accumulated pointer motion on SYN_REPORT */
            if (dev->is_pointer && (accum_dx != 0 || accum_dy != 0)) {
                struct wlr_pointer_motion_event motion = {
                    .pointer = dev->wlr.pointer,
                    .time_msec = ev.time.tv_sec * 1000 + ev.time.tv_usec / 1000,
                    .delta_x = accum_dx,
                    .delta_y = accum_dy,
                    .unaccel_dx = accum_dx,
                    .unaccel_dy = accum_dy
                };
                wl_signal_emit(&dev->wlr.pointer->events.motion, &motion);

                /* Move the cursor directly (bypasses the cursor_motion_notify handler) */
                wlr_cursor_move(dev->server->cursor, NULL, accum_dx, accum_dy);
                stratwm_process_cursor_motion(dev->server, motion.time_msec);

                accum_dx = 0;
                accum_dy = 0;
            }
            continue;
        }

        /* Handle keyboard events */
        if (dev->is_keyboard && ev.type == EV_KEY) {
            /* wlroots seat keycode: Linux evdev code; xkb: evdev + 8 */
            uint32_t wl_key = ev.code;
            uint32_t xkb_key = ev.code + 8;

            struct wlr_keyboard_key_event key_event = {
                .time_msec = ev.time.tv_sec * 1000 + ev.time.tv_usec / 1000,
                .keycode = wl_key,
                .update_state = true,
                .state = ev.value ? WL_KEYBOARD_KEY_STATE_PRESSED : WL_KEYBOARD_KEY_STATE_RELEASED
            };

            /* Update xkb state */
            if (dev->wlr.keyboard->xkb_state) {
                xkb_state_update_key(dev->wlr.keyboard->xkb_state,
                    xkb_key, ev.value ? XKB_KEY_DOWN : XKB_KEY_UP);
            }

            wl_signal_emit(&dev->wlr.keyboard->events.key, &key_event);

            /* Update and emit modifiers */
            if (dev->wlr.keyboard->xkb_state) {
                struct wlr_keyboard_modifiers mods = {
                    .depressed = xkb_state_serialize_mods(dev->wlr.keyboard->xkb_state, XKB_STATE_MODS_DEPRESSED),
                    .latched = xkb_state_serialize_mods(dev->wlr.keyboard->xkb_state, XKB_STATE_MODS_LATCHED),
                    .locked = xkb_state_serialize_mods(dev->wlr.keyboard->xkb_state, XKB_STATE_MODS_LOCKED),
                    .group = xkb_state_serialize_layout(dev->wlr.keyboard->xkb_state, XKB_STATE_LAYOUT_EFFECTIVE)
                };
                if (memcmp(&mods, &dev->wlr.keyboard->modifiers, sizeof(mods)) != 0) {
                    dev->wlr.keyboard->modifiers = mods;
                    wlr_keyboard_notify_modifiers(dev->wlr.keyboard,
                        mods.depressed, mods.latched, mods.locked, mods.group);
                }
            }
        }
        /* Handle pointer motion (accumulate for SYN_REPORT) */
        else if (dev->is_pointer && ev.type == EV_REL) {
            if (ev.code == REL_X) {
                accum_dx += ev.value;
            } else if (ev.code == REL_Y) {
                accum_dy += ev.value;
            }
        }
        /* Handle pointer buttons */
        else if (dev->is_pointer && ev.type == EV_KEY) {
            /* Convert Linux button codes (BTN_*) to wayland codes */
            uint32_t button;
            switch (ev.code) {
                case BTN_LEFT: button = BTN_LEFT; break;
                case BTN_RIGHT: button = BTN_RIGHT; break;
                case BTN_MIDDLE: button = BTN_MIDDLE; break;
                default: button = ev.code; break;
            }

            uint32_t tm = (uint32_t)(ev.time.tv_sec * 1000 + ev.time.tv_usec / 1000);
            uint32_t st = ev.value ? WL_POINTER_BUTTON_STATE_PRESSED
                                   : WL_POINTER_BUTTON_STATE_RELEASED;
            stratwm_process_pointer_button(dev->server, tm, button, st);
        }
    }

    return 0;
}

static int uevent_handle(int fd, uint32_t mask, void *data) {
    (void)data;
    (void)mask;

    char buf[UEVENT_BUFFER_SIZE];
    ssize_t n = recv(fd, buf, sizeof(buf) - 1, 0);
    if (n <= 0) return 0;

    buf[n] = '\0';

    /* Parse uevent - look for add/change events for input devices */
    if (strstr(buf, "@/devices") && strstr(buf, "/input")) {
        char *action = buf;  /* First line is action@path */
        char *devpath = NULL;

        /* Find DEVPATH in the message */
        char *p = buf;
        while (*p) {
            if (strncmp(p, "DEVPATH=", 8) == 0) {
                devpath = p + 8;
                break;
            }
            p += strlen(p) + 1;
        }

        if (devpath && strncmp(action, "add@", 4) == 0) {
            char full_path[512];
            snprintf(full_path, sizeof(full_path), "/sys%s/event", devpath);

            /* Find the event device node */
            DIR *dir = opendir(full_path);
            if (dir) {
                struct dirent *entry;
                while ((entry = readdir(dir)) != NULL) {
                    if (strncmp(entry->d_name, "event", 5) == 0) {
                        char device_path[512];
                        snprintf(device_path, sizeof(device_path),
                                 "/dev/input/%s", entry->d_name);

                        /* Check if already known */
                        struct stratwm_evdev_device *dev;
                        wl_list_for_each(dev, &g_input_manager.devices, link) {
                            if (strcmp(dev->path, device_path) == 0) {
                                goto skip;
                            }
                        }

                        if (is_input_device(device_path)) {
                            evdev_device_create((struct stratwm_server *)data, device_path);
                        }
                    skip:
                        break;
                    }
                }
                closedir(dir);
            }
        }
    }

    return 0;
}

static int input_manager_init(struct stratwm_server *server) {
    wl_list_init(&g_input_manager.devices);

    /* Open uevent socket for hotplug */
    g_input_manager.uevent_fd = uevent_open_socket();
    if (g_input_manager.uevent_fd >= 0) {
        g_input_manager.uevent_source = wl_event_loop_add_fd(
            wl_display_get_event_loop(server->wl_display),
            g_input_manager.uevent_fd,
            WL_EVENT_READABLE,
            uevent_handle,
            server);
    }

    /* Probe existing devices */
    input_manager_probe_devices(server);

    fprintf(stderr, "stratwm: input manager initialized, uevent_fd=%d\n",
            g_input_manager.uevent_fd);

    return 0;
}

static void input_manager_probe_devices(struct stratwm_server *server) {
    DIR *dir = opendir("/dev/input");
    if (!dir) {
        fprintf(stderr, "stratwm: /dev/input not found\n");
        return;
    }

    fprintf(stderr, "stratwm: scanning /dev/input for devices...\n");

    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL) {
        if (strncmp(entry->d_name, "event", 5) != 0) continue;

        char path[256];
        int n = snprintf(path, sizeof(path), "/dev/input/%s", entry->d_name);
        if (n < 0 || (size_t)n >= sizeof(path)) continue;

        /* Check if already known */
        struct stratwm_evdev_device *dev;
        bool found = false;
        wl_list_for_each(dev, &g_input_manager.devices, link) {
            if (strcmp(dev->path, path) == 0) {
                found = true;
                break;
            }
        }
        if (found) continue;

        if (is_input_device(path)) {
            evdev_device_create(server, path);
        }
    }

    closedir(dir);
}

static void input_manager_destroy(struct stratwm_server *server) {
    (void)server;

    struct stratwm_evdev_device *dev, *tmp;
    wl_list_for_each_safe(dev, tmp, &g_input_manager.devices, link) {
        evdev_device_destroy(dev);
    }

    if (g_input_manager.uevent_source) {
        wl_event_source_remove(g_input_manager.uevent_source);
    }
    if (g_input_manager.uevent_fd >= 0) {
        close(g_input_manager.uevent_fd);
    }
}
