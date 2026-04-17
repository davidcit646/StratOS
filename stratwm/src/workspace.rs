use crate::tiling::TilingLayout;

#[derive(Debug)]
pub struct Workspace {
    // Workspace identifier
    id: u32,
    
    // Tiling layout for this workspace
    layout: TilingLayout,
    
    // Surface IDs in this workspace
    surface_ids: Vec<u32>,
}

impl Workspace {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            layout: TilingLayout::new(),
            surface_ids: Vec::new(),
        }
    }
    
    pub fn id(&self) -> u32 {
        self.id
    }
    
    pub fn layout(&self) -> &TilingLayout {
        &self.layout
    }
    
    pub fn layout_mut(&mut self) -> &mut TilingLayout {
        &mut self.layout
    }
    
    pub fn surface_ids(&self) -> &[u32] {
        &self.surface_ids
    }
    
    pub fn add_surface(&mut self, surface_id: u32) {
        if !self.surface_ids.contains(&surface_id) {
            self.surface_ids.push(surface_id);
        }
    }
    
    pub fn remove_surface(&mut self, surface_id: u32) {
        self.surface_ids.retain(|id| *id != surface_id);
    }
    
    pub fn surface_count(&self) -> usize {
        self.surface_ids.len()
    }
}
