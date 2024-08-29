use crate::cpu::CPU;

const LY: u16 = 0xFF44;
const MAX_LY: u8 = 153;

#[derive(Clone, Debug)]
pub struct PPU {
    pub ly_delay: u8, // fake thing
    pub rendered_pixels: [u8; 160 * 144 * 4],
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            ly_delay: 0,
            rendered_pixels: [0; 160 * 144 * 4],
        }
    }

    pub fn read_ly(&self, cpu: &CPU) -> u8 {
        cpu.memory.read_u8(LY)
    }

    pub fn step(&mut self, cpu: &mut CPU) -> &Self {
        if self.ly_delay < 3 {
            self.ly_delay += 1;
            return self;
        }
        self.ly_delay = 0;
        cpu.memory
            .write_u8(LY, (self.read_ly(cpu) + 1) % (MAX_LY + 1));
        self
    }
}
