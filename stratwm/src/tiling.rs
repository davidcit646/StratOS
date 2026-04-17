use crate::surface::{StratSurface, SurfaceState};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutMode {
    Tile,
    Float,
    Tabbed,
}

#[derive(Debug)]
pub struct TilingLayout {
    // Current layout mode
    mode: LayoutMode,
    
    // Gap between tiles in pixels
    gap: u32,
    
    // Main area ratio (for master-stack layouts)
    main_ratio: f32,
    
    // Number of master windows
    main_count: usize,
}

impl TilingLayout {
    pub fn new() -> Self {
        Self {
            mode: LayoutMode::Tile,
            gap: 8,
            main_ratio: 0.6,
            main_count: 1,
        }
    }
    
    pub fn mode(&self) -> LayoutMode {
        self.mode
    }
    
    pub fn set_mode(&mut self, mode: LayoutMode) {
        self.mode = mode;
    }
    
    pub fn gap(&self) -> u32 {
        self.gap
    }
    
    pub fn set_gap(&mut self, gap: u32) {
        self.gap = gap;
    }
    
    pub fn main_ratio(&self) -> f32 {
        self.main_ratio
    }
    
    pub fn set_main_ratio(&mut self, ratio: f32) {
        self.main_ratio = ratio.clamp(0.1, 0.9);
    }
    
    pub fn main_count(&self) -> usize {
        self.main_count
    }
    
    pub fn set_main_count(&mut self, count: usize) {
        self.main_count = count;
    }
    
    /// Arrange surfaces according to current layout mode
    pub fn arrange(&self, surfaces: &mut [StratSurface], output_width: u32, output_height: u32) {
        match self.mode {
            LayoutMode::Tile => self.arrange_tile(surfaces, output_width, output_height),
            LayoutMode::Float => { /* Floating surfaces manage their own geometry */ }
            LayoutMode::Tabbed => self.arrange_tabbed(surfaces, output_width, output_height),
        }
    }
    
    fn arrange_tile(&self, surfaces: &mut [StratSurface], output_width: u32, output_height: u32) {
        todo!("arrange_tile implementation")
    }
    
    fn arrange_tabbed(&self, surfaces: &mut [StratSurface], output_width: u32, output_height: u32) {
        todo!("arrange_tabbed implementation")
    }
    
    /// Toggle a surface between tiled and floating
    pub fn toggle_float(&self, surface: &mut StratSurface) {
        match surface.state() {
            SurfaceState::Tiled => surface.set_state(SurfaceState::Floating),
            SurfaceState::Floating => surface.set_state(SurfaceState::Tiled),
            _ => {}
        }
    }
    
    /// Enter tabbed mode for a set of surfaces
    pub fn enter_tabbed(&mut self, surfaces: &mut [StratSurface]) {
        self.mode = LayoutMode::Tabbed;
        for surface in surfaces {
            surface.set_state(SurfaceState::Tabbed);
        }
    }
    
    /// Exit tabbed mode, return to tiling
    pub fn exit_tabbed(&mut self, surfaces: &mut [StratSurface]) {
        self.mode = LayoutMode::Tile;
        for surface in surfaces {
            surface.set_state(SurfaceState::Tiled);
        }
    }
}
