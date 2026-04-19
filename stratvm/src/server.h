#ifndef STRATVM_SERVER_H
#define STRATVM_SERVER_H

#include <stdbool.h>
#include <wayland-server-core.h>

#include <wlr/backend.h>
#include <wlr/render/wlr_renderer.h>
#include <wlr/render/allocator.h>
#include <wlr/types/wlr_scene.h>
#include <wlr/types/wlr_output_layout.h>
#include <wlr/types/wlr_xdg_shell.h>
#include <wlr/types/wlr_seat.h>
#include <wlr/types/wlr_cursor.h>
#include <wlr/types/wlr_xcursor_manager.h>
#include <wlr/types/wlr_layer_shell_v1.h>

/* Tiling engine data structures (Phase 8.2) */
enum stratwm_split_direction {
    SPLIT_VERTICAL,   /* children split left/right */
    SPLIT_HORIZONTAL, /* children split top/bottom */
};

enum stratwm_layout_mode {
    LAYOUT_BSP,       /* Binary space partition (default) */
    LAYOUT_STACK,     /* Stack: all windows stacked, one visible */
    LAYOUT_FULLSCREEN, /* Fullscreen: only focused window visible */
};

struct stratwm_tile {
    struct stratwm_tile *parent;
    struct stratwm_tile *left;   /* child node (vertical split) or NULL */
    struct stratwm_tile *right;  /* child node (vertical split) or NULL */
    struct wlr_box geometry;      /* tile bounding box */
    enum stratwm_split_direction split;
    struct stratwm_view *view;    /* NULL for internal nodes, set for leaves */
};

struct stratwm_workspace {
    int id;
    struct stratwm_tile *root;    /* root of BSP tree */
    struct stratwm_view *focused; /* currently focused view on this workspace */
    enum stratwm_layout_mode layout; /* BSP/Stack/Fullscreen (Phase 8.6) */
};

/* Layer shell (Phase 24a) */
struct stratwm_layer_surface {
    struct wl_list link;
    struct stratwm_server *server;
    struct wlr_layer_surface_v1 *layer_surface;
    struct wlr_scene_tree *scene_tree;
    struct wlr_scene_layer_surface_v1 *scene_layer_surface;
    uint32_t previous_layer;  /* Track layer changes for reparenting */
    struct wl_listener map;
    struct wl_listener unmap;
    struct wl_listener commit;
    struct wl_listener destroy;
    struct wl_listener new_popup;
};

/* IPC (Phase 24a) */
#define IPC_MAX_CLIENTS 16
#define IPC_BUF_SIZE 512

struct stratwm_ipc_client {
    int fd;
    char buf[IPC_BUF_SIZE];
    int buf_len;
    struct wl_event_source *event_source;
};

struct stratwm_ipc {
    int socket_fd;
    struct wl_event_source *event_source;
    struct stratwm_ipc_client clients[IPC_MAX_CLIENTS];
    int client_count;
};

struct stratwm_server {
    struct wl_display *wl_display;
    struct wlr_backend *backend;
    struct wlr_renderer *renderer;
    struct wlr_allocator *allocator;
    struct wlr_scene *scene;
    struct wlr_scene_output_layout *scene_layout;
    struct wlr_xdg_shell *xdg_shell;
    struct wlr_seat *seat;
    struct wl_list outputs;
    struct wl_list views;
    struct wl_listener new_output;
    struct wl_listener new_xdg_toplevel;
    struct wl_listener new_input;

    struct wlr_layer_shell_v1 *layer_shell;
    struct wl_list layer_surfaces;
    struct wl_listener new_layer_surface;
    struct stratwm_ipc ipc;
    bool panel_autohide;

    /* Optional `/config/strat/stratvm.conf` — titlebar / border padding (see stratwm_load_deco_config). */
    int deco_titlebar_h;
    int deco_border_pad;
    /* `[chrome] decorations_enabled_default` in settings.toml / settings.d (stratwm_load_modular_chrome). */
    bool default_decorations_visible;

    struct wlr_output_layout *output_layout;
    struct wlr_cursor *cursor;
    struct wlr_xcursor_manager *cursor_manager;

    struct wl_list keyboards;

    struct wl_listener cursor_motion;
    struct wl_listener cursor_motion_absolute;
    struct wl_listener cursor_button;
    struct wl_listener cursor_axis;
    struct wl_listener cursor_frame;
    struct wl_listener request_cursor;

    /* Tiling engine (Phase 8.2) */
    #define STRATWM_WORKSPACES 9
    struct stratwm_workspace workspaces[STRATWM_WORKSPACES];
    int current_workspace;
    struct stratwm_view *focused_view;

    /* Layer-shell stacking (Phase 24a) */
    struct wlr_scene_tree *layers_bg;
    struct wlr_scene_tree *layers_bottom;
    struct wlr_scene_tree *layers_normal;
    struct wlr_scene_tree *layers_top;
    struct wlr_scene_tree *layers_overlay;

    /* Interactive window move (titlebar drag) */
    struct stratwm_view *grabbed_view;
    int grab_x, grab_y;  /* Initial cursor position */
    int grab_view_x, grab_view_y;  /* Initial view position */
};

#endif
