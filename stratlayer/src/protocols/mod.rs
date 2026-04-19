pub mod core;
pub mod layer_shell;
pub mod seat;
pub mod shm;
pub mod xdg;

pub use core::{WlCompositor, WlDisplay, WlRegistry, WlSurface};
pub use layer_shell::{
    ZwlrLayerShellV1, ZwlrLayerSurfaceV1, LAYER_BACKGROUND, LAYER_BOTTOM, LAYER_TOP, LAYER_OVERLAY,
    ANCHOR_TOP, ANCHOR_BOTTOM, ANCHOR_LEFT, ANCHOR_RIGHT,
};
pub use seat::{WlKeyboard, WlPointer, WlSeat};
pub use shm::{WlBuffer, WlShm, WlShmPool};
pub use xdg::{XdgSurface, XdgToplevel, XdgWmBase};
