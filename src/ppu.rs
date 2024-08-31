const VRAM_SIZE: usize = 0x2000;
const WRAM_SIZE: usize = 0x1000;

#[derive(Clone, Debug)]
pub enum PPUState {
    OAMSearch,
    PixelTransfer,
    HBlank,
    VBlank,
}

#[derive(Clone, Debug)]
pub struct PPU {
    pub rendered_pixels: [u8; 160 * 144 * 4],
    ly: u8, // max should be 153
    scanline_t_cycles: u16,
    state: PPUState,
    x: u8,
    vram: [u8; VRAM_SIZE],
    wram_0: [u8; WRAM_SIZE],
    wram_1: [u8; WRAM_SIZE],
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            rendered_pixels: [0; 160 * 144 * 4],
            ly: 0,
            scanline_t_cycles: 0,
            state: PPUState::OAMSearch,
            x: 0,
            vram: [0; VRAM_SIZE],
            wram_0: [0; WRAM_SIZE],
            wram_1: [0; WRAM_SIZE],
        }
    }

    pub fn increment_ly(&mut self) {
        self.ly = self.ly + 1;
    }

    pub fn read_ly(&self) -> u8 {
        self.ly
    }

    pub fn reset_ly(&mut self) {
        self.ly = 0;
    }

    pub fn step_t_cycles(&mut self, t_cycles: u8) -> &mut Self {
        for _ in 0..t_cycles {
            self.step_one_t_cycle();
        }
        self
    }

    pub fn step_one_t_cycle(&mut self) -> &mut Self {
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
                    self.increment_ly();
                    if self.read_ly() == 144 {
                        self.state = PPUState::VBlank
                    } else {
                        self.state = PPUState::OAMSearch
                    }
                }
            }

            PPUState::VBlank => {
                if self.scanline_t_cycles == 456 {
                    self.scanline_t_cycles = 0;
                    self.increment_ly();
                    if self.read_ly() == 153 {
                        self.reset_ly();
                        self.state = PPUState::OAMSearch;
                    }
                }
            }
        }

        self.scanline_t_cycles += 1;
        self
    }

    pub fn read_vram(&self, address: u16) -> u8 {
        self.vram[address as usize]
    }

    pub fn read_wram_0(&self, address: u16) -> u8 {
        self.wram_0[address as usize]
    }

    pub fn read_wram_1(&self, address: u16) -> u8 {
        self.wram_1[address as usize]
    }

    pub fn write_vram(&mut self, address: u16, value: u8) {
        self.vram[address as usize] = value;
    }

    pub fn write_wram_0(&mut self, address: u16, value: u8) {
        self.wram_0[address as usize] = value;
    }

    pub fn write_wram_1(&mut self, address: u16, value: u8) {
        self.wram_1[address as usize] = value;
    }
}
