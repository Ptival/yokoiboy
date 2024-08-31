use crate::cpu::CPU;

const _LCDC: u16 = 0xFF40;
const _TILE_SEL: u8 = 4;
const LY: u16 = 0xFF44;
const MAX_LY: u8 = 153;

#[derive(Clone, Debug)]
pub enum PPUState {
    OAMSearch,
    PixelTransfer,
    HBlank,
    VBlank,
}

#[derive(Clone, Debug)]
pub struct PPU {
    pub scanline_t_cycles: u16,
    pub rendered_pixels: [u8; 160 * 144 * 4],
    pub state: PPUState,
    pub x: u8,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            scanline_t_cycles: 0,
            rendered_pixels: [0; 160 * 144 * 4],
            state: PPUState::OAMSearch,
            x: 0,
        }
    }

    pub fn increment_ly(&self, cpu: &mut CPU) {
        // TODO: maybe PPU should hold this value, and CPU should ask for it
        cpu.memory
            .write_u8(LY, (self.read_ly(cpu) + 1) % (MAX_LY + 1));
    }

    pub fn read_ly(&self, cpu: &CPU) -> u8 {
        cpu.memory.read_u8(LY)
    }

    pub fn reset_ly(&self, cpu: &mut CPU) {
        cpu.memory.write_u8(LY, 0);
    }

    pub fn step_t_cycles(&mut self, cpu: &mut CPU, t_cycles: u8) -> &mut Self {
        for _ in 0..t_cycles {
            self.step_one_t_cycle(cpu);
        }
        self
    }

    pub fn step_one_t_cycle(&mut self, cpu: &mut CPU) -> &Self {
        match self.state {
            PPUState::OAMSearch => {
                // TODO: actually scan memory
                if self.scanline_t_cycles == 40 {
                    self.x = 0;
                    self.state = PPUState::PixelTransfer
                }
            }

            PPUState::PixelTransfer => {
                // TODO: actually transfer pixels
                self.x += 1;
                if self.x == 160 {
                    self.state = PPUState::HBlank
                }
            }

            PPUState::HBlank => {
                if self.scanline_t_cycles == 456 {
                    self.scanline_t_cycles = 0;
                    self.increment_ly(cpu);
                    if self.read_ly(cpu) == 144 {
                        self.state = PPUState::VBlank
                    } else {
                        self.state = PPUState::OAMSearch
                    }
                }
            }

            PPUState::VBlank => {
                if self.scanline_t_cycles == 456 {
                    self.scanline_t_cycles = 0;
                    self.increment_ly(cpu);
                    if self.read_ly(cpu) == 153 {
                        self.reset_ly(cpu);
                        self.state = PPUState::OAMSearch;
                    }
                }
            }
        }

        self.scanline_t_cycles += 1;

        self
    }
}
