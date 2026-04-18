pub struct StratSeat {
    // wlroots seat handle (opaque pointer for FFI)
    wlr_seat: *mut libc::c_void,

    // Seat name
    name: String,

    // Capabilities
    capabilities: SeatCapabilities,

    // Keyboard state
    keyboard_focused_surface: Option<u32>,

    // Pointer state
    pointer_focused_surface: Option<u32>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct SeatCapabilities: u32 {
        const POINTER = 1;
        const KEYBOARD = 2;
        const TOUCH = 4;
    }
}

impl StratSeat {
    pub fn new(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: Initialize wlroots seat via FFI
        // wlr_seat = wlr_seat_create(wl_display, name) via FFI
        let wlr_seat = std::ptr::null_mut();

        Ok(Self {
            wlr_seat,
            name: name.to_string(),
            capabilities: SeatCapabilities::POINTER | SeatCapabilities::KEYBOARD,
            keyboard_focused_surface: None,
            pointer_focused_surface: None,
        })
    }

    pub fn wlr_seat(&self) -> *mut libc::c_void {
        self.wlr_seat
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn capabilities(&self) -> SeatCapabilities {
        self.capabilities
    }

    pub fn set_capabilities(&mut self, caps: SeatCapabilities) {
        self.capabilities = caps;
        // TODO: wlr_seat_set_capabilities(wlr_seat, caps.bits()) via FFI
    }

    /// Notify the seat that the keyboard focus has changed
    pub fn set_keyboard_focus(&mut self, surface_id: Option<u32>) {
        self.keyboard_focused_surface = surface_id;
        // TODO: wlr_seat_keyboard_notify_enter(wlr_seat, surface, keycodes, modifiers) via FFI
    }

    /// Notify the seat that the pointer focus has changed
    pub fn set_pointer_focus(&mut self, surface_id: Option<u32>, sx: f64, sy: f64) {
        self.pointer_focused_surface = surface_id;
        // TODO: wlr_seat_pointer_notify_enter(wlr_seat, surface, sx, sy) via FFI
        let _ = (sx, sy);
    }

    /// Clear pointer focus (pointer left all surfaces)
    pub fn clear_pointer_focus(&mut self) {
        self.pointer_focused_surface = None;
        // TODO: wlr_seat_pointer_clear_focus(wlr_seat) via FFI
    }

    /// Send a keyboard key event to the focused surface
    pub fn notify_key(&self, time: u32, key: u32, state: u32) {
        // TODO: wlr_seat_keyboard_notify_key(wlr_seat, time, key, state) via FFI
        let _ = (time, key, state);
    }

    /// Send a pointer button event to the focused surface
    pub fn notify_pointer_button(&self, time: u32, button: u32, state: u32) {
        // TODO: wlr_seat_pointer_notify_button(wlr_seat, time, button, state) via FFI
        let _ = (time, button, state);
    }

    /// Send a pointer motion event to the focused surface
    pub fn notify_pointer_motion(&self, time: u32, sx: f64, sy: f64) {
        // TODO: wlr_seat_pointer_notify_motion(wlr_seat, time, sx, sy) via FFI
        let _ = (time, sx, sy);
    }

    /// Send modifier state to the focused surface
    pub fn notify_modifiers(&self, mods_depressed: u32, mods_latched: u32, mods_locked: u32, group: u32) {
        // TODO: wlr_seat_keyboard_notify_modifiers(wlr_seat, &mods) via FFI
        let _ = (mods_depressed, mods_latched, mods_locked, group);
    }

    pub fn keyboard_focused_surface(&self) -> Option<u32> {
        self.keyboard_focused_surface
    }

    pub fn pointer_focused_surface(&self) -> Option<u32> {
        self.pointer_focused_surface
    }

    /// Destroy the seat and free resources
    pub fn destroy(self) {
        // TODO: wlr_seat_destroy(wlr_seat) via FFI
    }
}

// SAFETY: StratSeat contains a raw pointer to a wlroots seat.
// The wlroots seat is owned and managed by the C library.
// StratSeat does not own the pointer and only holds it for FFI calls.
// The pointer remains valid for the lifetime of the wlroots seat,
// which is guaranteed to outlive any StratSeat instance.
// No concurrent mutable access occurs - all FFI calls are serialized
// through the compositor event loop.
unsafe impl Send for StratSeat {}
