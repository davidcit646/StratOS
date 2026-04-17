#[derive(Debug, Clone)]
pub struct OutputMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
}

#[derive(Debug)]
pub struct StratOutput {
    // wlroots output handle (opaque pointer for FFI)
    wlr_output: *mut libc::c_void,
    
    // Output identifier
    id: u32,
    
    // Output name (e.g., "eDP-1")
    name: String,
    
    // Current mode
    current_mode: Option<OutputMode>,
    
    // Available modes
    modes: Vec<OutputMode>,
    
    // Scale factor for HiDPI
    scale: f32,
    
    // Transform (normal, rotated, etc.)
    transform: u32,
}

impl StratOutput {
    pub fn new(wlr_output: *mut libc::c_void, id: u32, name: String) -> Self {
        Self {
            wlr_output,
            id,
            name,
            current_mode: None,
            modes: Vec::new(),
            scale: 1.0,
            transform: 0, // WL_OUTPUT_TRANSFORM_NORMAL
        }
    }
    
    pub fn id(&self) -> u32 {
        self.id
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn current_mode(&self) -> Option<&OutputMode> {
        self.current_mode.as_ref()
    }
    
    pub fn set_current_mode(&mut self, mode: OutputMode) {
        self.current_mode = Some(mode);
    }
    
    pub fn modes(&self) -> &[OutputMode] {
        &self.modes
    }
    
    pub fn add_mode(&mut self, mode: OutputMode) {
        self.modes.push(mode);
    }
    
    pub fn scale(&self) -> f32 {
        self.scale
    }
    
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }
    
    pub fn transform(&self) -> u32 {
        self.transform
    }
    
    pub fn set_transform(&mut self, transform: u32) {
        self.transform = transform;
    }
    
    pub fn wlr_output(&self) -> *mut libc::c_void {
        self.wlr_output
    }
}

// SAFETY: StratOutput contains a raw pointer to wlroots output.
// The wlroots output is owned and managed by wlroots C library.
// StratOutput does not own the pointer and only holds it for FFI calls.
// The pointer remains valid for the lifetime of the wlroots output,
// which is guaranteed to outlive any StratOutput instance.
// No concurrent mutable access occurs - all FFI calls are serialized
// through the compositor event loop.
unsafe impl Send for StratOutput {}
