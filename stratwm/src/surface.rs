#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecorationState {
    Full,
    NoTitlebar,
    NoButtons,
    NoBorders,
    Minimal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SurfaceState {
    Tiled,
    Floating,
    Tabbed,
    Maximized,
    Fullscreen,
}

#[derive(Debug, Clone)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct StratSurface {
    // wlroots surface handle (opaque pointer for FFI)
    wlr_surface: *mut libc::c_void,
    
    // Surface identifier
    id: u32,
    
    // Surface title (from app)
    title: String,
    
    // App ID (from app)
    app_id: String,
    
    // Current geometry
    geometry: Geometry,
    
    // Surface state
    state: SurfaceState,
    
    // Decoration state
    decoration: DecorationState,
    
    // Whether surface is mapped (visible)
    mapped: bool,
    
    // Workspace index
    workspace: u32,
}

impl StratSurface {
    pub fn new(wlr_surface: *mut libc::c_void, id: u32) -> Self {
        Self {
            wlr_surface,
            id,
            title: String::new(),
            app_id: String::new(),
            geometry: Geometry { x: 0, y: 0, width: 0, height: 0 },
            state: SurfaceState::Tiled,
            decoration: DecorationState::Full,
            mapped: false,
            workspace: 0,
        }
    }
    
    pub fn id(&self) -> u32 {
        self.id
    }
    
    pub fn title(&self) -> &str {
        &self.title
    }
    
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }
    
    pub fn app_id(&self) -> &str {
        &self.app_id
    }
    
    pub fn set_app_id(&mut self, app_id: String) {
        self.app_id = app_id;
    }
    
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
    
    pub fn set_geometry(&mut self, geometry: Geometry) {
        self.geometry = geometry;
    }
    
    pub fn state(&self) -> SurfaceState {
        self.state
    }
    
    pub fn set_state(&mut self, state: SurfaceState) {
        self.state = state;
    }
    
    pub fn decoration(&self) -> DecorationState {
        self.decoration
    }
    
    pub fn set_decoration(&mut self, decoration: DecorationState) {
        self.decoration = decoration;
    }
    
    pub fn is_mapped(&self) -> bool {
        self.mapped
    }
    
    pub fn set_mapped(&mut self, mapped: bool) {
        self.mapped = mapped;
    }
    
    pub fn workspace(&self) -> u32 {
        self.workspace
    }
    
    pub fn set_workspace(&mut self, workspace: u32) {
        self.workspace = workspace;
    }
    
    pub fn wlr_surface(&self) -> *mut libc::c_void {
        self.wlr_surface
    }
}

// SAFETY: StratSurface contains a raw pointer to wlroots surface.
// The wlroots surface is owned and managed by wlroots C library.
// StratSurface does not own the pointer and only holds it for FFI calls.
// The pointer remains valid for the lifetime of the wlroots surface,
// which is guaranteed to outlive any StratSurface instance.
// No concurrent mutable access occurs - all FFI calls are serialized
// through the compositor event loop.
unsafe impl Send for StratSurface {}
