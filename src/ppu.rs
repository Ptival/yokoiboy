#[derive(Clone, Debug)]
pub struct PPU {
    pub rendered_pixels: [u8; 160 * 144 * 4],
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            rendered_pixels: [0; 160 * 144 * 4],
        }
    }
}
