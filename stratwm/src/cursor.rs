pub struct StratCursor {
    // wlroots cursor handle (opaque pointer for FFI)
    wlr_cursor: *mut libc::c_void,

    // wlroots XCursor manager (loads cursor themes)
    wlr_xcursor_manager: *mut libc::c_void,

    // Current cursor image name (e.g. "left_ptr", "text", "grab")
    current_image: String,

    // Cursor position
    x: f64,
    y: f64,

    // Whether a hardware cursor is available (falls back to software)
    hardware_cursor: bool,
}

// XCursor configuration
const CURSOR_SIZE: u32 = 24;
const CURSOR_THEME: &str = "default";

impl StratCursor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: Initialize wlroots cursor via FFI
        // wlr_cursor = wlr_cursor_create();
        // wlr_xcursor_manager = wlr_xcursor_manager_create(CURSOR_THEME, CURSOR_SIZE);
        // wlr_xcursor_manager_load(wlr_xcursor_manager, 1); // scale=1
        let wlr_cursor = std::ptr::null_mut();
        let wlr_xcursor_manager = std::ptr::null_mut();

        Ok(Self {
            wlr_cursor,
            wlr_xcursor_manager,
            current_image: "left_ptr".to_string(),
            x: 0.0,
            y: 0.0,
            hardware_cursor: true,
        })
    }

    pub fn wlr_cursor(&self) -> *mut libc::c_void {
        self.wlr_cursor
    }

    pub fn wlr_xcursor_manager(&self) -> *mut libc::c_void {
        self.wlr_xcursor_manager
    }

    pub fn position(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    pub fn set_position(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
        // TODO: wlr_cursor_warp(wlr_cursor, NULL, x, y) via FFI
    }

    pub fn move_delta(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
        // TODO: wlr_cursor_move(wlr_cursor, NULL, dx, dy) via FFI
    }

    pub fn set_image(&mut self, name: &str) {
        if self.current_image != name {
            self.current_image = name.to_string();
            // TODO: wlr_xcursor_manager_set_cursor_image(
            //   wlr_xcursor_manager, name, wlr_cursor) via FFI
        }
    }

    pub fn image(&self) -> &str {
        &self.current_image
    }

    /// Set the default left_ptr cursor image
    pub fn set_default_image(&mut self) {
        self.set_image("left_ptr");
    }

    /// Called when an output is added — load cursor themes for the output's scale
    pub fn load_for_scale(&mut self, scale: f32) {
        // TODO: wlr_xcursor_manager_load(wlr_xcursor_manager, scale) via FFI
        let _ = scale;
    }

    /// Called when pointer enters a surface — update cursor image based on context
    pub fn update_for_surface(&mut self, surface_type: SurfaceCursorHint) {
        match surface_type {
            SurfaceCursorHint::Normal => self.set_image("left_ptr"),
            SurfaceCursorHint::Text => self.set_image("text"),
            SurfaceCursorHint::Grab => self.set_image("grab"),
            SurfaceCursorHint::Move => self.set_image("move"),
            SurfaceCursorHint::ResizeH => self.set_image("sb_h_double_arrow"),
            SurfaceCursorHint::ResizeV => self.set_image("sb_v_double_arrow"),
            SurfaceCursorHint::None => self.set_image("left_ptr"),
        }
    }

    /// Map the cursor to a specific output (for multi-head)
    pub fn map_to_output(&mut self, output_name: &str) {
        // TODO: wlr_cursor_map_to_output(wlr_cursor, output) via FFI
        let _ = output_name;
    }

    /// Check if hardware cursor is functioning; if not, enable software rendering
    pub fn check_hardware_cursor(&mut self) -> bool {
        // TODO: After attaching cursor to output, check if hardware cursor works.
        // If wlr_cursor->hardware_cursor is NULL, fall back to software:
        //   wlr_cursor_map_to_output(wlr_cursor, NULL) — software cursor
        //   and render the cursor image in the compositor's frame callback
        self.hardware_cursor
    }

    /// Destroy the cursor and free resources
    pub fn destroy(self) {
        // TODO: wlr_xcursor_manager_destroy(wlr_xcursor_manager) via FFI
        // TODO: wlr_cursor_destroy(wlr_cursor) via FFI
    }
}

/// Hint for what cursor image to show based on what the pointer is over
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SurfaceCursorHint {
    Normal,
    Text,
    Grab,
    Move,
    ResizeH,
    ResizeV,
    None,
}

// SAFETY: StratCursor contains raw pointers to wlroots objects.
// The wlroots objects are owned and managed by the C library.
// StratCursor does not own the pointers and only holds them for FFI calls.
// The pointers remain valid for the lifetime of the wlroots objects,
// which is guaranteed to outlive any StratCursor instance.
// No concurrent mutable access occurs - all FFI calls are serialized
// through the compositor event loop.
unsafe impl Send for StratCursor {}
