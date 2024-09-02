mod fetcher;

use std::num::Wrapping;

use fetcher::Fetcher;

use crate::machine::Machine;

const VRAM_SIZE: usize = 0x2000;
const WRAM_SIZE: usize = 0x1000;

const TILE_HORIZONTAL_PIXELS: usize = 8;
const VRAM_HORIZONTAL_TILES: usize = 16;
const VRAM_HORIZONTAL_PIXELS: usize = VRAM_HORIZONTAL_TILES * TILE_HORIZONTAL_PIXELS;
const TILE_VERTICAL_PIXELS: usize = 8;
const VRAM_VERTICAL_TILES: usize = 16;
const VRAM_VERTICAL_PIXELS: usize = VRAM_VERTICAL_TILES * TILE_VERTICAL_PIXELS;
const VRAM_PIXELS_SIZE: usize = VRAM_HORIZONTAL_PIXELS * VRAM_VERTICAL_PIXELS * 4;

#[derive(Clone, Debug)]
pub enum PPUState {
    OAMScan,
    DrawingPixels,
    HorizontalBlank,
    VerticalBlank,
}

#[derive(Clone, Debug)]
pub struct PPU {
    pub fetcher: Fetcher,
    pub rendered_pixels: [u8; 160 * 144 * 4],
    pub vram_pixels: [u8; VRAM_PIXELS_SIZE],
    lcdc: Wrapping<u8>,
    ly: Wrapping<u8>, // max should be 153
    scanline_dots: u16,
    state: PPUState,
    vram: [u8; VRAM_SIZE],
    wram_0: [u8; WRAM_SIZE],
    wram_1: [u8; WRAM_SIZE],
    drawn_pixels_on_current_row: u8,
}

pub fn pixel_code_to_rgba(pixel_code: u8) -> [u8; 4] {
    match pixel_code {
        0b00 => [15, 56, 15, 255],
        0b01 => [48, 98, 48, 255],
        0b10 => [139, 172, 15, 255],
        0b11 => [155, 188, 15, 255],
        _ => panic!("pixel_code is: 0x{:08b}", pixel_code),
    }
}

// Each pixel takes 4 bytes (R, G, B, A).  Each y results in 160 pixels.
pub fn pixel_coordinates_in_rgba_slice(x: u8, y: u8) -> usize {
    (y as usize * 160 + x as usize) * 4
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            fetcher: Fetcher::new(),
            rendered_pixels: [0; 160 * 144 * 4],
            vram_pixels: [0; VRAM_PIXELS_SIZE],
            lcdc: Wrapping(0),
            ly: Wrapping(0),
            scanline_dots: 0,
            state: PPUState::OAMScan,
            vram: [0; VRAM_SIZE],
            wram_0: [0; WRAM_SIZE],
            wram_1: [0; WRAM_SIZE],
            drawn_pixels_on_current_row: 0,
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
        // Wrapping(144) // while gbdoctoring
        self.ly
    }

    // TODO: Eventually we could update the rendered VRAM on the fly when writes to VRAM happen
    pub fn render_vram(&mut self) {
        for tile_y in 0..VRAM_VERTICAL_TILES {
            for tile_x in 0..VRAM_HORIZONTAL_TILES {
                let tile_data_from = (tile_y * 16 + tile_x) * 16;
                let tile_data = &self.vram[tile_data_from..tile_data_from + 16];
                for tile_pixel_y in 0..TILE_VERTICAL_PIXELS {
                    let row_data_from = tile_pixel_y * 2;
                    let low_bits = tile_data[row_data_from];
                    let high_bits = tile_data[row_data_from + 1];
                    for tile_pixel_x in 0..TILE_HORIZONTAL_PIXELS {
                        let pixel_code = (((high_bits >> (7 - tile_pixel_x)) & 1) << 1)
                            | ((low_bits >> (7 - tile_pixel_x)) & 1);
                        let pixel_rgba = pixel_code_to_rgba(pixel_code);
                        let vram_pixel_x = tile_x * 8 + tile_pixel_x;
                        let vram_pixel_y = tile_y * 8 + tile_pixel_y;
                        let vram_pixels_from =
                            (vram_pixel_y * VRAM_HORIZONTAL_PIXELS + vram_pixel_x) * 4;
                        self.vram_pixels[vram_pixels_from..vram_pixels_from + 4]
                            .copy_from_slice(&pixel_rgba);
                    }
                }
            }
        }
    }

    pub fn reset_ly(&mut self) {
        self.ly = Wrapping(0);
    }

    pub fn step_dots(machine: &mut Machine, dots: u8) -> &mut Machine {
        for _ in 0..dots {
            PPU::step_one_dot(machine);
        }
        machine
    }

    pub fn step_one_dot(machine: &mut Machine) -> &mut Machine {
        if !machine.ppu.is_lcd_ppu_on() {
            return machine;
        }

        match machine.ppu.state {
            PPUState::OAMScan => {
                // TODO: actually scan memory
                if machine.ppu.scanline_dots == 80 {
                    let ly = machine.scy + machine.ppu.read_ly();
                    machine.ppu.fetcher.tile_line = ly % Wrapping(8);
                    machine.ppu.fetcher.row_address =
                        Wrapping(0x9800) + Wrapping((ly.0 as u16 / 8) * 32);
                    machine.ppu.fetcher.tile_index = Wrapping(0);
                    machine.ppu.fetcher.fifo.clear();
                    machine.ppu.state = PPUState::DrawingPixels
                }
            }

            PPUState::DrawingPixels => {
                Fetcher::step_one_dot(machine);
                if machine.ppu.fetcher.fifo.len() != 0 {
                    let pixel = machine.ppu.fetcher.fifo.pop_front().unwrap();
                    let pixel_x = machine.ppu.drawn_pixels_on_current_row;
                    let pixel_y = machine.ppu.ly.0;

                    let from = pixel_coordinates_in_rgba_slice(pixel_x, pixel_y);
                    let rgba = pixel_code_to_rgba(pixel.color);
                    machine.ppu.rendered_pixels[from..from + 4].copy_from_slice(&rgba);
                    machine.ppu.drawn_pixels_on_current_row += 1;

                    if machine.ppu.drawn_pixels_on_current_row == 160 {
                        machine.ppu.drawn_pixels_on_current_row = 0;
                        machine.ppu.state = PPUState::HorizontalBlank
                    }
                }
            }

            PPUState::HorizontalBlank => {
                if machine.ppu.scanline_dots == 456 {
                    machine.ppu.scanline_dots = 0;
                    machine.ppu.increment_ly();
                    if machine.ppu.read_ly().0 == 144 {
                        // TODO: signal VBlank
                        machine.ppu.state = PPUState::VerticalBlank
                    } else {
                        machine.ppu.state = PPUState::OAMScan
                    }
                }
            }

            PPUState::VerticalBlank => {
                if machine.ppu.scanline_dots == 456 {
                    machine.ppu.scanline_dots = 0;
                    machine.ppu.increment_ly();
                    if machine.ppu.read_ly().0 == 153 {
                        machine.ppu.reset_ly();
                        machine.ppu.state = PPUState::OAMScan;
                    }
                }
            }
        }

        machine.ppu.scanline_dots += 1;
        machine
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
