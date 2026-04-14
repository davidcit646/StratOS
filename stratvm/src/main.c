#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <linux/input-event-codes.h>
#include <spawn.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#include <wayland-server-core.h>
#include <wayland-server-protocol.h>
#include <xkbcommon/xkbcommon.h>

#include <wlr/backend.h>
#include <wlr/backend/libinput.h>
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
    struct wl_listener map;
    struct wl_listener unmap;
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

    wlr_scene_node_raise_to_top(&view->scene_tree->node);
    wlr_seat_keyboard_notify_enter(server->seat, surface,
        keyboard ? keyboard->keycodes : NULL,
        keyboard ? keyboard->num_keycodes : 0,
        keyboard ? &keyboard->modifiers : NULL);
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
    wlr_scene_node_set_enabled(&view->scene_tree->node, true);
    focus_view(view, view->xdg_toplevel->base->surface);
}

static void view_unmap_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_view *view = wl_container_of(listener, view, unmap);
    wlr_scene_node_set_enabled(&view->scene_tree->node, false);
}

static void view_destroy_notify(struct wl_listener *listener, void *data) {
    (void)data;
    struct stratwm_view *view = wl_container_of(listener, view, destroy);
    wl_list_remove(&view->map.link);
    wl_list_remove(&view->unmap.link);
    wl_list_remove(&view->destroy.link);
    wl_list_remove(&view->link);
    free(view);
}

static void server_new_xdg_toplevel_notify(struct wl_listener *listener, void *data) {
    struct stratwm_server *server = wl_container_of(listener, server, new_xdg_toplevel);
    struct wlr_xdg_toplevel *xdg_toplevel = data;

    struct stratwm_view *view = calloc(1, sizeof(*view));
    if (view == NULL) {
        return;
    }

    view->server = server;
    view->xdg_toplevel = xdg_toplevel;
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
    view->destroy.notify = view_destroy_notify;
    wl_signal_add(&xdg_toplevel->events.destroy, &view->destroy);

    wl_list_insert(&server->views, &view->link);
}

static bool handle_keybinding(struct stratwm_server *server, xkb_keysym_t sym,
    uint32_t modifiers) {
    (void)modifiers;

    /* F1 = spawn terminal, F2 = exit (testing keybinds — Super is eaten by host WM) */
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
    return 0;
}
