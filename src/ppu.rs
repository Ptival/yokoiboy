mod fetcher;

use std::num::Wrapping;

use fetcher::Fetcher;

use crate::{cpu::interrupts::VBLANK_INTERRUPT_BIT, machine::Machine};

const OAM_SIZE: usize = 0xA0;
const VRAM_SIZE: usize = 0x2000;
const WRAM_SIZE: usize = 0x1000;

const LCD_HORIZONTAL_PIXEL_COUNT: usize = 160;
const LCD_VERTICAL_PIXEL_COUNT: usize = 144;

const HORIZONTAL_PIXELS_PER_TILE: usize = 8;
const VERTICAL_PIXELS_PER_TILE: usize = 8;
const PIXELS_PER_TILE: usize = HORIZONTAL_PIXELS_PER_TILE * VERTICAL_PIXELS_PER_TILE;

const TILE_PALETTE_HORIZONTAL_TILE_COUNT: usize = 16;
const TILE_PALETTE_VERTICAL_TILE_COUNT: usize = 24;
const TILE_PALETTE_HORIZONTAL_PIXELS: usize =
    TILE_PALETTE_HORIZONTAL_TILE_COUNT * HORIZONTAL_PIXELS_PER_TILE;
const TILE_PALETTE_VERTICAL_PIXELS: usize =
    TILE_PALETTE_VERTICAL_TILE_COUNT * VERTICAL_PIXELS_PER_TILE;
const TILE_PALETTE_PIXELS_TOTAL: usize =
    TILE_PALETTE_HORIZONTAL_PIXELS * TILE_PALETTE_VERTICAL_PIXELS;

const TILE_MAP_HORIZONTAL_TILE_COUNT: usize = 32;
const TILE_MAP_VERTICAL_TILE_COUNT: usize = 32;
const TILE_MAP_HORIZONTAL_PIXELS: usize =
    TILE_MAP_HORIZONTAL_TILE_COUNT * HORIZONTAL_PIXELS_PER_TILE;
const TILE_MAP_VERTICAL_PIXELS: usize = TILE_MAP_VERTICAL_TILE_COUNT * VERTICAL_PIXELS_PER_TILE;
const TILE_MAP_PIXELS_TOTAL: usize = TILE_MAP_HORIZONTAL_PIXELS * TILE_MAP_VERTICAL_PIXELS;

const PIXEL_DATA_SIZE: usize = 4; // 4-bytes for R, G, B, A

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

    pub object_attribute_memory: [u8; OAM_SIZE],
    pub lcd_pixels: [u8; LCD_HORIZONTAL_PIXEL_COUNT * LCD_VERTICAL_PIXEL_COUNT * PIXEL_DATA_SIZE],
    pub tile_palette_pixels: [u8; TILE_PALETTE_PIXELS_TOTAL * PIXEL_DATA_SIZE],
    pub tile_map0_pixels: [u8; TILE_MAP_PIXELS_TOTAL * PIXEL_DATA_SIZE],
    pub tile_map1_pixels: [u8; TILE_MAP_PIXELS_TOTAL * PIXEL_DATA_SIZE],

    pub lcd_control: Wrapping<u8>,
    lcd_y_coord: Wrapping<u8>,
    pub lcd_y_compare: Wrapping<u8>,
    pub lcd_status: Wrapping<u8>,
    pub object_palette_0: Wrapping<u8>,
    pub object_palette_1: Wrapping<u8>,
    pub window_x7: Wrapping<u8>,
    pub window_y: Wrapping<u8>,

    scanline_dots: u16,
    state: PPUState,
    vram: [u8; VRAM_SIZE],
    wram_0: [u8; WRAM_SIZE],
    wram_1: [u8; WRAM_SIZE],
    drawn_pixels_on_current_row: u8,
}

pub fn pixel_code_to_rgba(pixel_code: u8) -> [u8; PIXEL_DATA_SIZE] {
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
    (y as usize * LCD_HORIZONTAL_PIXEL_COUNT + x as usize) * PIXEL_DATA_SIZE
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            fetcher: Fetcher::new(),

            object_attribute_memory: [0; OAM_SIZE],
            lcd_pixels: [0; LCD_HORIZONTAL_PIXEL_COUNT
                * LCD_VERTICAL_PIXEL_COUNT
                * PIXEL_DATA_SIZE],
            tile_palette_pixels: [0; TILE_PALETTE_PIXELS_TOTAL * PIXEL_DATA_SIZE],
            tile_map0_pixels: [0; TILE_MAP_PIXELS_TOTAL * PIXEL_DATA_SIZE],
            tile_map1_pixels: [0; TILE_MAP_PIXELS_TOTAL * PIXEL_DATA_SIZE],

            lcd_control: Wrapping(0),
            lcd_y_coord: Wrapping(0),
            lcd_y_compare: Wrapping(0),
            lcd_status: Wrapping(0),
            object_palette_0: Wrapping(0),
            object_palette_1: Wrapping(0),
            window_x7: Wrapping(0),
            window_y: Wrapping(0),

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
        self.lcd_control.0 & mask == mask
    }

    pub fn increment_ly(&mut self) {
        self.lcd_y_coord = self.lcd_y_coord + Wrapping(1);
    }

    pub fn read_ly(machine: &Machine) -> Wrapping<u8> {
        if machine.fix_ly_for_gb_doctor {
            Wrapping(144)
        } else {
            machine.ppu.lcd_y_coord
        }
    }

    // TODO: Eventually we could update on the fly on writes
    pub fn render_tile_palette(&mut self) {
        for tile_palette_y in 0..TILE_PALETTE_VERTICAL_TILE_COUNT {
            for tile_palette_x in 0..TILE_PALETTE_HORIZONTAL_TILE_COUNT {
                let tile_data_from = (tile_palette_y * 16 + tile_palette_x) * 16;
                let tile_data = &self.vram[tile_data_from..tile_data_from + 16];
                for tile_pixel_y in 0..VERTICAL_PIXELS_PER_TILE {
                    let row_data_from = tile_pixel_y * 2;
                    let low_bits = tile_data[row_data_from];
                    let high_bits = tile_data[row_data_from + 1];
                    for tile_pixel_x in 0..HORIZONTAL_PIXELS_PER_TILE {
                        let pixel_code = (((high_bits >> (7 - tile_pixel_x)) & 1) << 1)
                            | ((low_bits >> (7 - tile_pixel_x)) & 1);
                        let pixel_rgba = pixel_code_to_rgba(pixel_code);
                        let vram_pixel_x = tile_palette_x * 8 + tile_pixel_x;
                        let vram_pixel_y = tile_palette_y * 8 + tile_pixel_y;
                        let vram_pixels_from =
                            (vram_pixel_y * TILE_PALETTE_HORIZONTAL_PIXELS + vram_pixel_x) * 4;
                        self.tile_palette_pixels[vram_pixels_from..vram_pixels_from + 4]
                            .copy_from_slice(&pixel_rgba);
                    }
                }
            }
        }
    }

    // NOTE: Assumes the tile palette has been rendered first
    pub fn render_tile_map0(&mut self) {
        render_tile_map(
            &self.vram,
            &self.tile_palette_pixels,
            &mut self.tile_map0_pixels,
            0x1800,
        )
    }

    // NOTE: Assumes the tile palette has been rendered first
    pub fn render_tile_map1(&mut self) {
        render_tile_map(
            &self.vram,
            &self.tile_palette_pixels,
            &mut self.tile_map1_pixels,
            0x1C00,
        )
    }

    // TODO: Eventually we could update on the fly on writes
    pub fn render(&mut self) {
        self.render_tile_palette();
        self.render_tile_map0();
        self.render_tile_map1();
    }

    pub fn reset_ly(&mut self) {
        self.lcd_y_coord = Wrapping(0);
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

        machine.ppu.scanline_dots += 1;

        match machine.ppu.state {
            PPUState::OAMScan => {
                // TODO: actually scan memory
                if machine.ppu.scanline_dots == 80 {
                    let lcd_y_coord = machine.scy + PPU::read_ly(machine);
                    machine.ppu.fetcher.tile_line = lcd_y_coord % Wrapping(8);
                    machine.ppu.fetcher.row_address =
                        Wrapping(0x9800) + Wrapping((lcd_y_coord.0 as u16 / 8) * 32);
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
                    let pixel_y = machine.ppu.lcd_y_coord.0;

                    let from = pixel_coordinates_in_rgba_slice(pixel_x, pixel_y);
                    let rgba = pixel_code_to_rgba(pixel.color);
                    machine.ppu.lcd_pixels[from..from + 4].copy_from_slice(&rgba);
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
                    if PPU::read_ly(machine).0 == 144 {
                        // println!("Requesting VBLANK interrupt");
                        // println!("IME: {}", machine.cpu.interrupts.interrupt_master_enable);
                        // println!("IE: {:08b}", machine.cpu.interrupts.interrupt_enable);
                        // println!("IF: {:08b}", machine.cpu.interrupts.interrupt_flag);
                        machine.request_interrupt(VBLANK_INTERRUPT_BIT);
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
                    if PPU::read_ly(machine).0 == 153 {
                        machine.ppu.reset_ly();
                        machine.ppu.state = PPUState::OAMScan;
                    }
                }
            }
        }

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
        self.lcd_control
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
        self.lcd_control = value;
    }
}

fn render_tile_map(
    vram: &[u8],
    tile_palette_pixels: &[u8],
    tile_map_pixels: &mut [u8],
    tile_map_vram_offset: usize,
) {
    for tile_map_y in 0..TILE_MAP_VERTICAL_TILE_COUNT {
        for tile_map_x in 0..TILE_MAP_HORIZONTAL_TILE_COUNT {
            let tile_entry_offset = (tile_map_y << 5) | tile_map_x;
            let tile_id = vram[tile_map_vram_offset + tile_entry_offset] as usize;
            // Because tiles have already been rendered as pixels in the  tile palette, here we
            // can just copy slices of lines for the 8 lines of the tile.
            for tile_pixel_y in 0..VERTICAL_PIXELS_PER_TILE {
                let tiles_to_skip = tile_map_y * TILE_MAP_HORIZONTAL_TILE_COUNT;
                let row_pixels_to_skip = tile_pixel_y * TILE_MAP_HORIZONTAL_PIXELS
                    + tile_map_x * HORIZONTAL_PIXELS_PER_TILE;
                let pixels_to_skip = tiles_to_skip * PIXELS_PER_TILE + row_pixels_to_skip;
                let bytes_to_skip = pixels_to_skip * PIXEL_DATA_SIZE;

                let palette_tile_y = tile_id / TILE_PALETTE_HORIZONTAL_TILE_COUNT;
                let palette_tile_x = tile_id % TILE_MAP_HORIZONTAL_TILE_COUNT;
                let palette_tiles_to_skip = palette_tile_y * TILE_PALETTE_HORIZONTAL_TILE_COUNT;
                let palette_row_pixels_to_skip = tile_pixel_y * TILE_PALETTE_HORIZONTAL_PIXELS
                    + palette_tile_x * HORIZONTAL_PIXELS_PER_TILE;
                let palette_pixels_to_skip =
                    palette_tiles_to_skip * PIXELS_PER_TILE + palette_row_pixels_to_skip;
                let palette_bytes_to_skip = palette_pixels_to_skip * PIXEL_DATA_SIZE;

                tile_map_pixels
                    [bytes_to_skip..bytes_to_skip + HORIZONTAL_PIXELS_PER_TILE * PIXEL_DATA_SIZE]
                    .copy_from_slice(
                        &tile_palette_pixels[palette_bytes_to_skip
                            ..palette_bytes_to_skip + HORIZONTAL_PIXELS_PER_TILE * PIXEL_DATA_SIZE],
                    );
            }
        }
    }
}
