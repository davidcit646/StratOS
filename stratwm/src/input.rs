use std::os::unix::io::RawFd;

#[derive(Debug, Clone, Copy)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub keycode: u32,
    pub state: KeyState,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
    pub caps_lock: bool,
}

#[derive(Debug, Clone)]
pub struct PointerEvent {
    pub x: f64,
    pub y: f64,
    pub button: Option<u32>,
    pub button_state: Option<ButtonState>,
    pub modifiers: Modifiers,
}

pub struct StratInput {
    // libinput context (opaque pointer for FFI)
    libinput_context: *mut libc::c_void,
    
    // Keyboard state
    keyboard_state: Modifiers,
    
    // Pointer state
    pointer_x: f64,
    pointer_y: f64,
}

impl StratInput {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: Initialize libinput context via FFI
        let libinput_context = std::ptr::null_mut();
        
        Ok(Self {
            libinput_context,
            keyboard_state: Modifiers {
                shift: false,
                ctrl: false,
                alt: false,
                super_key: false,
                caps_lock: false,
            },
            pointer_x: 0.0,
            pointer_y: 0.0,
        })
    }
    
    pub fn libinput_context(&self) -> *mut libc::c_void {
        self.libinput_context
    }
    
    pub fn keyboard_state(&self) -> Modifiers {
        self.keyboard_state
    }
    
    pub fn set_keyboard_state(&mut self, state: Modifiers) {
        self.keyboard_state = state;
    }
    
    pub fn pointer_position(&self) -> (f64, f64) {
        (self.pointer_x, self.pointer_y)
    }
    
    pub fn set_pointer_position(&mut self, x: f64, y: f64) {
        self.pointer_x = x;
        self.pointer_y = y;
    }
    
    /// Get the file descriptor for the libinput context
    pub fn fd(&self) -> RawFd {
        // TODO: Return libinput fd via FFI
        todo!("fd")
    }
    
    /// Process pending libinput events
    pub fn process_events(&mut self) -> Result<Vec<InputEvent>, Box<dyn std::error::Error>> {
        // TODO: Process libinput events via FFI
        todo!("process_events")
    }
}

pub enum InputEvent {
    Key(KeyEvent),
    Pointer(PointerEvent),
}
