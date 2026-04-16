use crate::shm::pool::ShmPool;

pub struct ShmBuffer {
    pool: ShmPool,
    offset: usize,
    width: u32,
    height: u32,
    stride: u32,
}

impl ShmBuffer {
    pub fn new(pool: ShmPool, offset: usize, width: u32, height: u32, stride: u32) -> Self {
        ShmBuffer {
            pool,
            offset,
            width,
            height,
            stride,
        }
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        let ptr = self.pool.ptr().add(self.offset);
        let size = self.height as usize * self.stride as usize;
        unsafe { std::slice::from_raw_parts_mut(ptr, size) }
    }

    pub fn fill_solid_blue(&mut self) {
        let data = self.data_mut();
        let blue = 0xFF0000FFu32; // ARGB8888: A=FF, R=00, G=00, B=FF
        
        for chunk in data.chunks_exact_mut(4) {
            let bytes = blue.to_le_bytes();
            chunk[0] = bytes[0];
            chunk[1] = bytes[1];
            chunk[2] = bytes[2];
            chunk[3] = bytes[3];
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn stride(&self) -> u32 {
        self.stride
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}
