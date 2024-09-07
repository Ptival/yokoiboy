mod pixel_fetcher;

use std::{collections::VecDeque, num::Wrapping};

use pixel_fetcher::{object::Sprite, Fetcher, FetchingFor, TileAddressingMode};

use crate::{
    cpu::interrupts::{STAT_INTERRUPT_BIT, VBLANK_INTERRUPT_BIT},
    machine::Machine,
    utils::{self},
};

const TILE_MAP0_VRAM_OFFSET: usize = 0x1800;
const TILE_MAP1_VRAM_OFFSET: usize = 0x1C00;

const OAM_SIZE: usize = 0xA0;
const VRAM_SIZE: usize = 0x2000;
const WRAM_SIZE: usize = 0x1000;

const LCD_HORIZONTAL_PIXEL_COUNT: usize = 160;
const LCD_VERTICAL_PIXEL_COUNT: usize = 144;

pub const HORIZONTAL_PIXELS_PER_TILE: usize = 8;
pub const VERTICAL_PIXELS_PER_TILE: usize = 8;
pub const PIXELS_PER_TILE: usize = HORIZONTAL_PIXELS_PER_TILE * VERTICAL_PIXELS_PER_TILE;

pub const TILE_PALETTE_HORIZONTAL_TILE_COUNT: usize = 16;
pub const TILE_PALETTE_VERTICAL_TILE_COUNT: usize = 24;
pub const TILE_PALETTE_HORIZONTAL_PIXELS: usize =
    TILE_PALETTE_HORIZONTAL_TILE_COUNT * HORIZONTAL_PIXELS_PER_TILE;
pub const TILE_PALETTE_VERTICAL_PIXELS: usize =
    TILE_PALETTE_VERTICAL_TILE_COUNT * VERTICAL_PIXELS_PER_TILE;
pub const TILE_PALETTE_PIXELS_TOTAL: usize =
    TILE_PALETTE_HORIZONTAL_PIXELS * TILE_PALETTE_VERTICAL_PIXELS;

const TILE_MAP_HORIZONTAL_TILE_COUNT: usize = 32;
const TILE_MAP_VERTICAL_TILE_COUNT: usize = 32;
const TILE_MAP_HORIZONTAL_PIXELS: usize =
    TILE_MAP_HORIZONTAL_TILE_COUNT * HORIZONTAL_PIXELS_PER_TILE;
const TILE_MAP_VERTICAL_PIXELS: usize = TILE_MAP_VERTICAL_TILE_COUNT * VERTICAL_PIXELS_PER_TILE;
const TILE_MAP_PIXELS_TOTAL: usize = TILE_MAP_HORIZONTAL_PIXELS * TILE_MAP_VERTICAL_PIXELS;

const PIXEL_DATA_SIZE: usize = 4; // 4-bytes for R, G, B, A

// LCD control single bits of interest
const _LCDC_BACKGROUND_AND_WINDOW_ENABLE_BIT: u8 = 0;
const _LCDC_OBJECT_ENABLE_BIT: u8 = 1;
const _LCDC_OBJECT_SIZE_BIT: u8 = 2;
const LCDC_BACKGROUND_TILE_MAP_AREA_BIT: u8 = 3;
const LCDC_BACKGROUND_AND_WINDOW_TILE_AREA_BIT: u8 = 4;
const _LCDC_WINDOW_ENABLE_BIT: u8 = 5;
const _LCDC_WINDOW_TILE_MAP_AREA_BIT: u8 = 6;
const LCDC_LCD_ENABLE_BIT: u8 = 7;

// LCD status single bits of interest
const LYC_EQUALS_LY_BIT: u8 = 2;
const MODE_0_INTERRUPT_SELECT_BIT: u8 = 3;
const MODE_1_INTERRUPT_SELECT_BIT: u8 = 4;
const MODE_2_INTERRUPT_SELECT_BIT: u8 = 5;
const LYC_EQUALS_LY_INTERRUPT_SELECT_BIT: u8 = 6;

#[derive(Clone, Debug)]
pub enum PPUState {
    OAMScan,
    DrawingPixels,
    HorizontalBlank,
    VerticalBlank,
}

#[derive(Clone, Debug)]
pub struct PPU {
    /** PPU state **/
    drawn_pixels_on_current_row: u8,
    /// Because the STAT interrupt is triggered on a rising edge of the STAT line, we need to
    /// remember its previous value.
    last_stat_line: u8,
    scanline_dots: u16,
    state: PPUState,

    // Subsystems
    fetcher: Fetcher,

    // Hardware registers
    pub background_palette_data: Wrapping<u8>,
    pub background_palette_spec: Wrapping<u8>,
    pub lcd_control: Wrapping<u8>,
    pub lcd_status: Wrapping<u8>,
    pub lcd_y_compare: Wrapping<u8>,
    /// LCD Y-coordinate.  Made private to enforce the use of `read_ly()` which allows forcing LY's
    /// value when using GB Doctor.
    lcd_y_coord: Wrapping<u8>,
    pub object_palette_data: Wrapping<u8>,
    pub object_palette_spec: Wrapping<u8>,
    pub object_palette_0: Wrapping<u8>,
    pub object_palette_1: Wrapping<u8>,
    pub scx: Wrapping<u8>,
    pub scy: Wrapping<u8>,
    pub vram_bank: Wrapping<u8>,
    pub window_x7: Wrapping<u8>,
    pub window_y: Wrapping<u8>,

    // Hardware banks
    pub object_attribute_memory: [u8; OAM_SIZE], // TODO: make private?
    pub vram: [u8; VRAM_SIZE],
    wram_0: [u8; WRAM_SIZE],
    wram_1: [u8; WRAM_SIZE],

    // Rendered pixel surfaces
    pub lcd_pixels: [u8; LCD_HORIZONTAL_PIXEL_COUNT * LCD_VERTICAL_PIXEL_COUNT * PIXEL_DATA_SIZE],
    pub tile_map0_pixels: [u8; TILE_MAP_PIXELS_TOTAL * PIXEL_DATA_SIZE],
    pub tile_map1_pixels: [u8; TILE_MAP_PIXELS_TOTAL * PIXEL_DATA_SIZE],
    pub tile_palette_pixels: [u8; TILE_PALETTE_PIXELS_TOTAL * PIXEL_DATA_SIZE],
}

pub fn pixel_code_to_rgba(pixel_code: u8) -> [u8; PIXEL_DATA_SIZE] {
    match pixel_code {
        0b00 => [0, 0, 0, 255],
        0b01 => [0x55, 0x55, 0x55, 255],
        0b10 => [0xAA, 0xAA, 0xAA, 255],
        0b11 => [0xFF, 0xFF, 0xFF, 255],
        // 0b00 => [15, 56, 15, 255],
        // 0b01 => [48, 98, 48, 255],
        // 0b10 => [139, 172, 15, 255],
        // 0b11 => [155, 188, 15, 255],
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
            drawn_pixels_on_current_row: 0,
            last_stat_line: 0,
            scanline_dots: 0,
            state: PPUState::OAMScan,

            fetcher: Fetcher::new(),

            background_palette_spec: Wrapping(0),
            background_palette_data: Wrapping(0),
            lcd_control: Wrapping(0),
            lcd_status: Wrapping(2), // initially set Mode 2
            lcd_y_compare: Wrapping(0),
            lcd_y_coord: Wrapping(0),
            object_palette_data: Wrapping(0),
            object_palette_0: Wrapping(0),
            object_palette_1: Wrapping(0),
            object_palette_spec: Wrapping(0),
            scx: Wrapping(0),
            scy: Wrapping(0),
            vram_bank: Wrapping(0),
            window_x7: Wrapping(0),
            window_y: Wrapping(0),

            object_attribute_memory: [0; OAM_SIZE],
            vram: [0; VRAM_SIZE],
            wram_0: [0; WRAM_SIZE],
            wram_1: [0; WRAM_SIZE],

            lcd_pixels: [0; LCD_HORIZONTAL_PIXEL_COUNT
                * LCD_VERTICAL_PIXEL_COUNT
                * PIXEL_DATA_SIZE],
            tile_map0_pixels: [0; TILE_MAP_PIXELS_TOTAL * PIXEL_DATA_SIZE],
            tile_map1_pixels: [0; TILE_MAP_PIXELS_TOTAL * PIXEL_DATA_SIZE],
            tile_palette_pixels: [0; TILE_PALETTE_PIXELS_TOTAL * PIXEL_DATA_SIZE],
        }
    }

    pub fn get_addressing_mode(&self) -> TileAddressingMode {
        if utils::is_bit_set(&self.lcd_control, LCDC_BACKGROUND_AND_WINDOW_TILE_AREA_BIT) {
            TileAddressingMode::UnsignedFrom0x8000
        } else {
            TileAddressingMode::SignedFrom0x9000
        }
    }

    pub fn is_lcd_ppu_on(&self) -> bool {
        utils::is_bit_set(&self.lcd_control, LCDC_LCD_ENABLE_BIT)
    }

    pub fn increment_ly(machine: &mut Machine) {
        machine.ppu_mut().lcd_y_coord = machine.ppu().lcd_y_coord + Wrapping(1);
        if machine.ppu().lcd_y_coord == machine.ppu().lcd_y_compare {
            utils::set_bit(machine.lcd_status_mut(), LYC_EQUALS_LY_BIT);
            if utils::is_bit_set(
                &machine.ppu().lcd_status,
                LYC_EQUALS_LY_INTERRUPT_SELECT_BIT,
            ) {
                machine.request_interrupt(STAT_INTERRUPT_BIT);
            }
        } else {
            utils::unset_bit(machine.lcd_status_mut(), LYC_EQUALS_LY_BIT);
        }
    }

    pub fn read_ly(machine: &Machine) -> Wrapping<u8> {
        if machine.fix_ly_for_gb_doctor {
            Wrapping(144)
        } else {
            machine.ppu().lcd_y_coord
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
            TILE_MAP0_VRAM_OFFSET,
        );
        let scx = self.scx.0 as usize;
        let scy = self.scy.0 as usize;
        let bottom = (scy + 143) % 256;
        let right = (scx + 159) % 256;

        // TODO: Technically we should render the scx on the fly as it may change between scanlines
        // (that's how you get cool effects)

        // Draw red horizontal lines above/below the viewport
        let mut x = scx;
        while x != right {
            // write pixel of top border
            let top_pixel_index = scy * TILE_MAP_HORIZONTAL_PIXELS + x;
            self.tile_map0_pixels[top_pixel_index * 4..(top_pixel_index + 1) * 4]
                .copy_from_slice(&[255, 0, 0, 255]);
            // write pixel of bottom border
            let bottom_pixel_index = bottom * TILE_MAP_HORIZONTAL_PIXELS + x;
            self.tile_map0_pixels[bottom_pixel_index * 4..(bottom_pixel_index + 1) * 4]
                .copy_from_slice(&[255, 0, 0, 255]);
            // increment and wrap around if necessary
            x += 1;
            if x == TILE_MAP_HORIZONTAL_PIXELS {
                x = 0;
            }
        }

        // Draw red horizontal lines left/right of the viewport
        let mut y = scy;
        while y != bottom {
            // write pixel of left border
            let left_pixel_index = y * TILE_MAP_HORIZONTAL_PIXELS + scx;
            self.tile_map0_pixels[left_pixel_index * 4..(left_pixel_index + 1) * 4]
                .copy_from_slice(&[255, 0, 0, 255]);
            // write pixel of right border
            let right_pixel_index = y * TILE_MAP_HORIZONTAL_PIXELS + right;
            self.tile_map0_pixels[right_pixel_index * 4..(right_pixel_index + 1) * 4]
                .copy_from_slice(&[255, 0, 0, 255]);
            // increment and wrap around if necessary
            y += 1;
            if y == TILE_MAP_VERTICAL_PIXELS {
                y = 0;
            }
        }
    }

    // NOTE: Assumes the tile palette has been rendered first
    pub fn render_tile_map1(&mut self) {
        render_tile_map(
            &self.vram,
            &self.tile_palette_pixels,
            &mut self.tile_map1_pixels,
            TILE_MAP1_VRAM_OFFSET,
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
        if !machine.ppu().is_lcd_ppu_on() {
            return machine;
        }

        machine.ppu_mut().scanline_dots += 1;
        if machine.ppu().scanline_dots > 456 {
            panic!("Frame did not finish rendering in time, investigate.");
        }

        match machine.ppu().state {
            // mode 2
            PPUState::OAMScan => {
                let ppu = machine.ppu();
                if ppu.scanline_dots == 80 {
                    let ly = PPU::read_ly(machine).0 as u16 as i16;
                    let mut selected_objects = VecDeque::new();
                    let object_size = 8; // TODO: this is either 8 or 16 depending on something
                    for object_offset in (0x00..0x9F).step_by(4) {
                        if selected_objects.len() == 10 {
                            break;
                        }
                        let y_screen_plus_16 = ppu.object_attribute_memory[object_offset];
                        let object_min_y_on_screen = (y_screen_plus_16 as u16 as i16) - 16;
                        let object_max_y_on_screen = object_min_y_on_screen + object_size - 1;
                        if object_min_y_on_screen <= ly && ly <= object_max_y_on_screen {
                            selected_objects.push_back(Sprite {
                                x_screen_plus_8: ppu.object_attribute_memory[object_offset + 1],
                                y_screen_plus_16,
                                tile_index: ppu.object_attribute_memory[object_offset + 2],
                                attributes: ppu.object_attribute_memory[object_offset + 3],
                            });
                        }
                    }
                    machine.fetcher_mut().reset();
                    machine.obj_fetcher_mut().selected_objects = selected_objects;
                    Self::switch_to_drawing_pixels(machine)
                }
            }

            // mode 3
            PPUState::DrawingPixels => {
                let bgw_fifo_len = machine.bgw_fetcher().fifo.len();
                let obj_fifo_len = machine.obj_fetcher().fifo.len();

                let fetcher_state = &machine.fetcher().fetching_for;
                if obj_fifo_len == 0 && bgw_fifo_len != 0 {
                    if *fetcher_state == FetchingFor::BackgroundOrWindowFIFO {
                        machine.fetcher_mut().switch_to_object_fifo();
                    }
                } else {
                    if *fetcher_state == FetchingFor::ObjectFIFO {
                        machine.fetcher_mut().switch_to_background_or_window_fifo();
                    }
                }
                Fetcher::tick(machine);

                if !machine.bgw_fetcher().fifo.is_empty() && !machine.obj_fetcher().fifo.is_empty()
                {
                    let bgw_pixel = machine.bgw_fetcher_mut().fifo.pop_front().unwrap();
                    let obj_pixel = machine.obj_fetcher_mut().fifo.pop_front().unwrap();
                    let pixel_x = machine.ppu().drawn_pixels_on_current_row;
                    let pixel_y = machine.ppu().lcd_y_coord.0;

                    let from = pixel_coordinates_in_rgba_slice(pixel_x, pixel_y);
                    let rgba = pixel_code_to_rgba(if obj_pixel.color == 0 {
                        bgw_pixel.color
                    } else {
                        obj_pixel.color
                    });
                    machine.ppu_mut().lcd_pixels[from..from + 4].copy_from_slice(&rgba);
                    machine.ppu_mut().drawn_pixels_on_current_row += 1;

                    if machine.ppu().drawn_pixels_on_current_row == 160 {
                        Self::switch_to_horizontal_blank(machine)
                    }
                }
            }

            // mode 0
            PPUState::HorizontalBlank => {
                if machine.ppu().scanline_dots == 456 {
                    machine.ppu_mut().scanline_dots = 0;
                    PPU::increment_ly(machine);
                    if PPU::read_ly(machine).0 == 144 {
                        Self::switch_to_vertical_blank(machine)
                    } else {
                        Self::switch_to_oam_scan(machine)
                    }
                }
            }

            // mode 1
            PPUState::VerticalBlank => {
                if machine.ppu().scanline_dots == 456 {
                    machine.ppu_mut().scanline_dots = 0;
                    PPU::increment_ly(machine);
                    if PPU::read_ly(machine).0 == 153 {
                        machine.ppu_mut().reset_ly();
                        Self::switch_to_oam_scan(machine)
                    }
                }
            }
        }

        // STAT interrupt check
        let stat_line = (machine.ppu().lcd_status.0 >> 3) & 0xF;
        if machine.ppu().last_stat_line == 0 && stat_line != 0 {
            machine.interrupts_mut().request(STAT_INTERRUPT_BIT);
        }
        machine.ppu_mut().last_stat_line = stat_line;

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

    fn switch_to_oam_scan(machine: &mut Machine) {
        machine.ppu_mut().drawn_pixels_on_current_row = 0;
        // Disabled because it locks LCD for Dr. Mario:
        // machine.ppu_mut().lcd_status = Wrapping((machine.ppu().lcd_status.0 & 0xFC) | 2);
        utils::unset_bit(machine.lcd_status_mut(), MODE_0_INTERRUPT_SELECT_BIT);
        utils::unset_bit(machine.lcd_status_mut(), MODE_1_INTERRUPT_SELECT_BIT);
        utils::set_bit(machine.lcd_status_mut(), MODE_2_INTERRUPT_SELECT_BIT);
        machine.ppu_mut().state = PPUState::OAMScan;
    }

    fn switch_to_drawing_pixels(machine: &mut Machine) {
        machine.fetcher_mut().switch_to_background_or_window_fifo();
        // Disabled because it locks LCD for Dr. Mario:
        // machine.ppu_mut().lcd_status = Wrapping((machine.ppu().lcd_status.0 & 0xFC) | 3);
        machine.ppu_mut().state = PPUState::DrawingPixels;
    }

    fn switch_to_horizontal_blank(machine: &mut Machine) {
        // Disabled because it locks LCD for Dr. Mario:
        // machine.ppu_mut().lcd_status = Wrapping(machine.ppu().lcd_status.0 & 0xFC);
        utils::set_bit(machine.lcd_status_mut(), MODE_0_INTERRUPT_SELECT_BIT);
        utils::unset_bit(machine.lcd_status_mut(), MODE_1_INTERRUPT_SELECT_BIT);
        utils::unset_bit(machine.lcd_status_mut(), MODE_2_INTERRUPT_SELECT_BIT);
        machine.ppu_mut().state = PPUState::HorizontalBlank;
    }

    fn switch_to_vertical_blank(machine: &mut Machine) {
        // Disabled because it locks LCD for Dr. Mario:
        // machine.ppu_mut().lcd_status = Wrapping((machine.ppu().lcd_status.0 & 0xFC) | 1);
        utils::unset_bit(machine.lcd_status_mut(), MODE_0_INTERRUPT_SELECT_BIT);
        utils::set_bit(machine.lcd_status_mut(), MODE_1_INTERRUPT_SELECT_BIT);
        utils::unset_bit(machine.lcd_status_mut(), MODE_2_INTERRUPT_SELECT_BIT);
        machine.request_interrupt(VBLANK_INTERRUPT_BIT);
        machine.ppu_mut().state = PPUState::VerticalBlank
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
            // Because tiles have already been rendered as pixels in the tile palette, here we
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

impl Machine {
    pub fn lcd_status(&self) -> &Wrapping<u8> {
        &self.ppu().lcd_status
    }
    pub fn lcd_status_mut(&mut self) -> &mut Wrapping<u8> {
        &mut self.ppu_mut().lcd_status
    }
}
