use std::num::Wrapping;

const VRAM_SIZE: usize = 0x2000;
const WRAM_SIZE: usize = 0x1000;

#[derive(Clone, Debug)]
pub enum PPUState {
    OAMScan,
    DrawingPixels,
    HorizontalBlank,
    VerticalBlank,
}

#[derive(Clone, Debug)]
pub struct PPU {
    pub rendered_pixels: [u8; 160 * 144 * 4],
    lcdc: Wrapping<u8>,
    ly: Wrapping<u8>, // max should be 153
    scanline_dots: u16,
    state: PPUState,
    vram: [u8; VRAM_SIZE],
    wram_0: [u8; WRAM_SIZE],
    wram_1: [u8; WRAM_SIZE],
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            rendered_pixels: [0; 160 * 144 * 4],
            lcdc: Wrapping(0),
            ly: Wrapping(0),
            scanline_dots: 0,
            state: PPUState::OAMScan,
            vram: [0; VRAM_SIZE],
            wram_0: [0; WRAM_SIZE],
            wram_1: [0; WRAM_SIZE],
        }
    }

    pub fn is_lcd_ppu_on(&self) -> bool {
        let mask = 1 << 7;
        self.lcdc.0 & mask == mask
    }

    pub fn increment_ly(&mut self) {
        self.ly = self.ly + Wrapping(1);
    }

    pub fn read_ly(&self) -> Wrapping<u8> {
        Wrapping(0x90) // while gbdoctoring
                       // self.ly
    }

    pub fn reset_ly(&mut self) {
        self.ly = Wrapping(0);
    }

    pub fn step_dots(&mut self, dots: u8) -> &mut Self {
        for _ in 0..dots {
            self.step_one_t_cycle();
        }
        self
    }

    pub fn step_one_t_cycle(&mut self) -> &mut Self {
        if !self.is_lcd_ppu_on() {
            return self;
        }
        match self.state {
            PPUState::OAMScan => {
                // TODO: actually scan memory
                if self.scanline_dots == 80 {
                    self.state = PPUState::DrawingPixels
                }
            }

            PPUState::DrawingPixels => {
                // TODO: actually transfer pixels
                if self.scanline_dots == 172 {
                    self.state = PPUState::HorizontalBlank
                }
            }

            PPUState::HorizontalBlank => {
                if self.scanline_dots == 456 {
                    self.scanline_dots = 0;
                    self.increment_ly();
                    if self.read_ly().0 == 144 {
                        self.state = PPUState::VerticalBlank
                    } else {
                        self.state = PPUState::OAMScan
                    }
                }
            }

            PPUState::VerticalBlank => {
                if self.scanline_dots == 456 {
                    self.scanline_dots = 0;
                    self.increment_ly();
                    if self.read_ly().0 == 153 {
                        self.reset_ly();
                        self.state = PPUState::OAMScan;
                    }
                }
            }
        }

        self.scanline_dots += 1;
        self
    }

    pub fn read_vram(&self, address: Wrapping<u16>) -> Wrapping<u8> {
        Wrapping(self.vram[address.0 as usize])
    }

    pub fn read_wram_0(&self, address: Wrapping<u16>) -> Wrapping<u8> {
        Wrapping(self.wram_0[address.0 as usize])
    }

    pub fn read_wram_1(&self, address: Wrapping<u16>) -> Wrapping<u8> {
        Wrapping(self.wram_1[address.0 as usize])
    }

    pub fn read_lcdc(&self) -> Wrapping<u8> {
        self.lcdc
    }

    pub fn write_vram(&mut self, address: Wrapping<u16>, value: Wrapping<u8>) {
        self.vram[address.0 as usize] = value.0;
    }

    pub fn write_wram_0(&mut self, address: Wrapping<u16>, value: Wrapping<u8>) {
        self.wram_0[address.0 as usize] = value.0;
    }

    pub fn write_wram_1(&mut self, address: Wrapping<u16>, value: Wrapping<u8>) {
        self.wram_1[address.0 as usize] = value.0;
    }

    pub fn write_lcdc(&mut self, value: Wrapping<u8>) {
        self.lcdc = value;
    }
}
