#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <linux/input-event-codes.h>
#include <spawn.h>
#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#include <wayland-server-core.h>
#include <wayland-server-protocol.h>
#include <xkbcommon/xkbcommon.h>

#include <wlr/backend.h>
#include <wlr/backend/multi.h>
#include <wlr/render/wlr_renderer.h>
#include <wlr/types/wlr_compositor.h>
#include <wlr/types/wlr_subcompositor.h>
#include <wlr/types/wlr_cursor.h>
#include <wlr/types/wlr_data_device.h>
#include <wlr/types/wlr_keyboard.h>
#include <wlr/types/wlr_output.h>
#include <wlr/types/wlr_output_layout.h>
#include <wlr/types/wlr_scene.h>
#include <wlr/types/wlr_seat.h>
#include <wlr/types/wlr_xcursor_manager.h>
#include <wlr/types/wlr_xdg_shell.h>
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
    
    struct wl_listener map;
    struct wl_listener unmap;
    struct wl_listener commit;
    struct wl_listener destroy;
};

struct stratwm_keyboard {
    struct wl_list link;
    struct stratwm_server *server;
    struct wlr_input_device *device;
    struct wl_listener modifiers;
    struct wl_listener key;
    struct wl_listener destroy;
};

static const float STRAT_BG[4] = {0.102f, 0.102f, 0.180f, 1.0f};
static void update_view_border(struct stratwm_view *view, bool focused);

static void spawn_terminal(void) {
    pid_t pid = fork();
    if (pid < 0) {
        fprintf(stderr, "stratwm: fork failed: %s\n", strerror(errno));
        return;
    }
    if (pid == 0) {
        setsid();
        /* Try direct first, then flatpak-spawn for immutable OS hosts */
        execl("/usr/bin/foot", "foot", (char *)NULL);
        execl("/usr/bin/alacritty", "alacritty", (char *)NULL);
        execl("/usr/bin/xterm", "xterm", (char *)NULL);
        execl("/usr/bin/flatpak-spawn", "flatpak-spawn", "--host", "foot", (char *)NULL);
        fprintf(stderr, "stratwm: no terminal found\n");
        _exit(127);
    }
}

static void spawn_autostart(const char *path) {
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
        setenv("WAYLAND_DISPLAY", "wayland-0", 1);
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

static struct stratwm_view *view_from_surface(struct stratwm_server *server,
    struct wlr_surface *surface) {
    struct stratwm_view *view;
    wl_list_for_each(view, &server->views, link) {
        if (view->xdg_toplevel->base->surface == surface) {
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

        /* Keep border aligned with tile size (2px outside on each edge). */
        if (tile->view->border) {
            int bw = tile->geometry.width + 4;
            int bh = tile->geometry.height + 4;
            if (bw < 1) bw = 1;
            if (bh < 1) bh = 1;
            wlr_scene_rect_set_size(tile->view->border, bw, bh);
            wlr_scene_node_set_position(&tile->view->border->node, -2, -2);
        }

        /* Update titlebar size and button positions (Phase 8.8) */
        if (tile->view->titlebar_bg) {
            wlr_scene_rect_set_size(tile->view->titlebar_bg, tile->geometry.width, 24);
            wlr_scene_node_set_position(&tile->view->titlebar_bg->node, 0, -24);
        }
        if (tile->view->close_button) {
            wlr_scene_node_set_position(&tile->view->close_button->node, 
                tile->geometry.width - 20, -22);
        }
        if (tile->view->max_button) {
            wlr_scene_node_set_position(&tile->view->max_button->node, 
                tile->geometry.width - 45, -22);
        }
        if (tile->view->min_button) {
            wlr_scene_node_set_position(&tile->view->min_button->node, 
                tile->geometry.width - 70, -22);
        }
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
    int bx = button->node.x;
    int by = button->node.y;
    return view_x >= bx && view_x < bx + 20 && view_y >= by && view_y < by + 20;
}

/* Titlebar creation and management (Phase 8.8) */
static void create_titlebar(struct stratwm_view *view) {
    if (!view) return;

    /* Titlebar background: dark blue-gray, spans full window width, 24px height */
    float bg_color[4] = {0.15f, 0.15f, 0.25f, 1.0f};  /* dark blue-gray */
    view->titlebar_bg = wlr_scene_rect_create(view->scene_tree, 800, 24, bg_color);
    if (view->titlebar_bg) {
        wlr_scene_node_set_position(&view->titlebar_bg->node, 0, -24);  /* Above window */
    }

    /* Close button: red, 20x20px, positioned at top-right */
    float close_color[4] = {0.8f, 0.2f, 0.2f, 1.0f};  /* red */
    view->close_button = wlr_scene_rect_create(view->scene_tree, 20, 20, close_color);
    if (view->close_button) {
        wlr_scene_node_set_position(&view->close_button->node, 780, -22);  /* Top-right corner */
    }

    /* Maximize button: green, 20x20px, left of close */
    float max_color[4] = {0.2f, 0.8f, 0.2f, 1.0f};  /* green */
    view->max_button = wlr_scene_rect_create(view->scene_tree, 20, 20, max_color);
    if (view->max_button) {
        wlr_scene_node_set_position(&view->max_button->node, 755, -22);  /* Left of close */
    }

    /* Minimize button: yellow, 20x20px, left of maximize */
    float min_color[4] = {0.8f, 0.8f, 0.2f, 1.0f};  /* yellow */
    view->min_button = wlr_scene_rect_create(view->scene_tree, 20, 20, min_color);
    if (view->min_button) {
        wlr_scene_node_set_position(&view->min_button->node, 730, -22);  /* Left of maximize */
    }
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

static void output_frame_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_output *output = wl_container_of(listener, output, frame);
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
        fprintf(stderr, "stratwm: output commit failed\n");
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
    output->background = wlr_scene_rect_create(&server->scene->tree, 1, 1, STRAT_BG);
    update_output_background(output);

    wlr_output_layout_add_auto(server->output_layout, wlr_output);

    output->frame.notify = output_frame_notify;
    wl_signal_add(&wlr_output->events.frame, &output->frame);
    output->destroy.notify = output_destroy_notify;
    wl_signal_add(&wlr_output->events.destroy, &output->destroy);
    wl_list_insert(&server->outputs, &output->link);

    fprintf(stderr, "stratwm: output added %s\n", wlr_output->name);
}

static void view_map_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_view *view = wl_container_of(listener, view, map);
    struct stratwm_server *server = view->server;
    fprintf(stderr, "stratwm: view_map_notify fired (ws=%d, floating=%d)\n",
        view->workspace_id, view->is_floating ? 1 : 0);

    wlr_scene_node_set_enabled(&view->scene_tree->node, true);

    /* Damage all outputs to force repaint of new window */
    struct stratwm_output *output;
    wl_list_for_each(output, &server->outputs, link) {
        wlr_output_schedule_frame(output->wlr_output);
    }

    /* Create window border decoration (Phase 8.3) */
    float border_color[4] = {0.267f, 0.267f, 0.267f, 1.0f};  /* default unfocused: dark gray */
    view->border = wlr_scene_rect_create(view->scene_tree, 1, 1, border_color);
    if (view->border) {
        wlr_scene_node_set_position(&view->border->node, -2, -2);
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
        if (view->border) {
            wlr_scene_rect_set_size(view->border, 804, 604);
            wlr_scene_node_set_position(&view->border->node, -2, -2);
        }
        /* Size titlebar for floating window */
        if (view->titlebar_bg) {
            wlr_scene_rect_set_size(view->titlebar_bg, 800, 24);
            wlr_scene_node_set_position(&view->titlebar_bg->node, 0, -24);
        }
        if (view->close_button) {
            wlr_scene_node_set_position(&view->close_button->node, 780, -22);
        }
        if (view->max_button) {
            wlr_scene_node_set_position(&view->max_button->node, 755, -22);
        }
        if (view->min_button) {
            wlr_scene_node_set_position(&view->min_button->node, 730, -22);
        }
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
                fprintf(stderr,
                    "stratwm: invalid output geometry (%dx%d), using fallback 1920x1080\n",
                    output_box.width, output_box.height);
                output_box.width = 1920;
                output_box.height = 1080;
            }
            ws->root = tile_new(output_box);
            if (!ws->root) {
                fprintf(stderr, "stratwm: tile_new failed for workspace=%d\n", ws->id);
                focus_view(view, view->xdg_toplevel->base->surface);
                return;
            }
            fprintf(stderr,
                "stratwm: workspace=%d root initialized to %dx%d+%d+%d\n",
                ws->id, output_box.width, output_box.height, output_box.x, output_box.y);
        }

        ws->root = tile_insert(ws->root, view, ws->root->geometry);
        if (!ws->root) {
            fprintf(stderr, "stratwm: tile_insert returned NULL for workspace=%d\n", ws->id);
            focus_view(view, view->xdg_toplevel->base->surface);
            return;
        }
        struct stratwm_tile *mapped_tile = tile_find_view(ws->root, view);
        if (!mapped_tile) {
            fprintf(stderr, "stratwm: ERROR view not found in BSP tree after insert (ws=%d)\n",
                ws->id);
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
    fprintf(stderr, "stratwm: initial configure queued after commit serial=%u\n", serial);
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
    wl_list_remove(&view->link);
    free(view);
}

static void server_new_xdg_toplevel_notify(struct wl_listener *listener, void *data) {
    fprintf(stderr, "stratwm: new xdg_toplevel created\n");
    struct stratwm_server *server = wl_container_of(listener, server, new_xdg_toplevel);
    struct wlr_xdg_toplevel *xdg_toplevel = data;

    struct stratwm_view *view = calloc(1, sizeof(*view));
    if (view == NULL) {
        return;
    }

    view->server = server;
    view->xdg_toplevel = xdg_toplevel;
    view->workspace_id = server->current_workspace;  /* Assign to current workspace */
    view->scene_tree = wlr_scene_xdg_surface_create(&server->scene->tree,
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
            view->float_x = 100;
            view->float_y = 100;
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
            ws->root = tile_new(output_box);
        }

        if (ws->root) {
            ws->root = tile_insert(ws->root, view, ws->root->geometry);
            tile_reflow_scene(ws->root);
        }
    }
}

/* Maximize floating window to fill screen (Phase 8.5) */
static void maximize_float_window(struct stratwm_server *server, struct stratwm_view *view) {
    if (!view || !view->is_floating) return;

    /* Expand to full output size */
    struct stratwm_output *output = NULL;
    wl_list_for_each(output, &server->outputs, link) {
        /* Position at output top-left in layout coordinates */
        double ox = 0.0, oy = 0.0;
        wlr_output_layout_output_coords(server->output_layout, output->wlr_output, &ox, &oy);
        view->float_x = (int)ox;
        view->float_y = (int)oy;
        
        /* Resize surface to output dimensions */
        wlr_xdg_toplevel_set_size(view->xdg_toplevel,
            output->wlr_output->width, output->wlr_output->height);
        
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
    
    /* Update visibility of all windows based on new layout mode */
    struct stratwm_view *view;
    wl_list_for_each(view, &server->views, link) {
        if (view->workspace_id != server->current_workspace || view->is_floating) {
            continue; /* Skip views in other workspaces or floating windows */
        }
        
        bool visible = true;
        switch (ws->layout) {
            case LAYOUT_BSP:
                /* All tiled windows visible */
                visible = true;
                break;
            case LAYOUT_STACK:
                /* Only focused window visible in stack mode */
                visible = (view == server->focused_view);
                break;
            case LAYOUT_FULLSCREEN:
                /* Only focused window visible in fullscreen mode */
                visible = (view == server->focused_view);
                break;
        }
        
        wlr_scene_node_set_enabled(&view->scene_tree->node, visible);
    }
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

    /* Cycle layout mode: Super+Space (BSP → Stack → Fullscreen → BSP) (Phase 8.6) */
    if (super_pressed && sym == XKB_KEY_space) {
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

static void cursor_motion_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, cursor_motion);
    struct wlr_pointer_motion_event *event = data;

    wlr_cursor_move(server->cursor, NULL, event->delta_x, event->delta_y);
    wlr_seat_pointer_notify_motion(server->seat, event->time_msec,
        server->cursor->x, server->cursor->y);
}

static void cursor_motion_absolute_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, cursor_motion_absolute);
    struct wlr_pointer_motion_absolute_event *event = data;

    wlr_cursor_warp_absolute(server->cursor, NULL, event->x, event->y);
    wlr_seat_pointer_notify_motion(server->seat, event->time_msec,
        server->cursor->x, server->cursor->y);
}

static void cursor_button_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, cursor_button);
    struct wlr_pointer_button_event *event = data;

    /* Check for titlebar button clicks (Phase 8.8) */
    if (event->state == WLR_BUTTON_PRESSED && server->focused_view) {
        struct stratwm_view *view = server->focused_view;
        double cursor_x = server->cursor->x;
        double cursor_y = server->cursor->y;

        /* Convert to view-relative coordinates */
        double view_x = cursor_x - view->scene_tree->node.x;
        double view_y = cursor_y - view->scene_tree->node.y;

        /* Dynamic hit-test based on current scene-node button positions. */
        if (point_in_titlebar_button(view->close_button, view_x, view_y)) {
            wlr_xdg_toplevel_send_close(view->xdg_toplevel);
            return;
        }

        if (point_in_titlebar_button(view->max_button, view_x, view_y)) {
            toggle_float(server, view);
            return;
        }

        if (point_in_titlebar_button(view->min_button, view_x, view_y)) {
            /* For now, just close on minimize (can be improved later) */
            wlr_xdg_toplevel_send_close(view->xdg_toplevel);
            return;
        }
    }

    wlr_seat_pointer_notify_button(server->seat, event->time_msec,
        event->button, event->state);
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

    server.wl_display = wl_display_create();
    if (server.wl_display == NULL) {
        fprintf(stderr, "stratwm: failed to create wl_display\n");
        return 1;
    }

    server.backend = wlr_backend_autocreate(wl_display_get_event_loop(server.wl_display), NULL);
    if (server.backend == NULL) {
        fprintf(stderr, "stratwm: failed to create backend\n");
        return 1;
    }

    server.renderer = wlr_renderer_autocreate(server.backend);
    server.allocator = wlr_allocator_autocreate(server.backend, server.renderer);
    if (server.renderer == NULL || server.allocator == NULL) {
        fprintf(stderr, "stratwm: failed to create renderer/allocator\n");
        return 1;
    }

    wlr_compositor_create(server.wl_display, 5, server.renderer);
    wlr_subcompositor_create(server.wl_display);
    wlr_data_device_manager_create(server.wl_display);
    wlr_renderer_init_wl_display(server.renderer, server.wl_display);

    server.output_layout = wlr_output_layout_create(server.wl_display);
    server.scene = wlr_scene_create();
    server.scene_layout = wlr_scene_attach_output_layout(server.scene, server.output_layout);
    server.xdg_shell = wlr_xdg_shell_create(server.wl_display, 6);
    server.seat = wlr_seat_create(server.wl_display, "seat0");

    server.cursor = wlr_cursor_create();
    wlr_cursor_attach_output_layout(server.cursor, server.output_layout);
    server.cursor_manager = wlr_xcursor_manager_create(NULL, 24);
    wlr_xcursor_manager_load(server.cursor_manager, 1);

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
        fprintf(stderr, "stratwm: failed to create wayland socket\n");
        return 1;
    }
    setenv("WAYLAND_DISPLAY", socket, 1);

    if (!wlr_backend_start(server.backend)) {
        fprintf(stderr, "stratwm: failed to start backend\n");
        return 1;
    }

    fprintf(stderr, "stratwm: started (%s)\n", socket);
    spawn_autostart("/bin/foot");
    wl_display_run(server.wl_display);

    wl_list_remove(&server.new_xdg_toplevel.link);
    wl_list_remove(&server.new_output.link);
    wl_list_remove(&server.new_input.link);
    wl_list_remove(&server.cursor_motion.link);
    wl_list_remove(&server.cursor_motion_absolute.link);
    wl_list_remove(&server.cursor_button.link);
    wl_list_remove(&server.cursor_axis.link);
    wl_list_remove(&server.cursor_frame.link);

    wl_display_destroy_clients(server.wl_display);
    wl_display_destroy(server.wl_display);

    /* Clean up workspace trees (Phase 8.2) */
    for (int i = 0; i < STRATWM_WORKSPACES; i++) {
        if (server.workspaces[i].root) {
            tile_free(server.workspaces[i].root);
        }
    }

    return 0;
}
