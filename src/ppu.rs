use std::{collections::VecDeque, num::Wrapping};

use crate::{
    cpu::interrupts::{Interrupts, STAT_INTERRUPT_BIT, VBLANK_INTERRUPT_BIT},
    pixel_fetcher::{
        background_or_window::BackgroundOrWindowFetcher,
        get_tile_index_in_palette,
        object::{ObjectFetcher, Sprite},
        Fetcher, FetchingFor, TileAddressingMode,
    },
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

pub const TILE_MAP_HORIZONTAL_TILE_COUNT: usize = 32;
pub const TILE_MAP_VERTICAL_TILE_COUNT: usize = 32;
const TILE_MAP_TILE_TOTAL: usize = TILE_MAP_HORIZONTAL_TILE_COUNT * TILE_MAP_VERTICAL_TILE_COUNT;
const TILE_MAP_HORIZONTAL_PIXELS: usize =
    TILE_MAP_HORIZONTAL_TILE_COUNT * HORIZONTAL_PIXELS_PER_TILE;
const TILE_MAP_VERTICAL_PIXELS: usize = TILE_MAP_VERTICAL_TILE_COUNT * VERTICAL_PIXELS_PER_TILE;
const TILE_MAP_PIXELS_TOTAL: usize = TILE_MAP_HORIZONTAL_PIXELS * TILE_MAP_VERTICAL_PIXELS;

const PIXEL_DATA_SIZE: usize = 4; // 4-bytes for R, G, B, A

// LCD control single bits of interest
const _LCDC_BACKGROUND_AND_WINDOW_ENABLE_BIT: u8 = 0;
const _LCDC_OBJECT_ENABLE_BIT: u8 = 1;
const _LCDC_OBJECT_SIZE_BIT: u8 = 2;
pub const LCDC_BACKGROUND_TILE_MAP_AREA_BIT: u8 = 3;
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
    fix_ly_for_gb_doctor: bool,
    /// Because the STAT interrupt is triggered on a rising edge of the STAT line, we need to
    /// remember its previous value.
    last_stat_line: u8,
    scanline_dots: u16,
    state: PPUState,

    // Hardware registers
    pub background_palette_data: u8,
    pub cgb_background_palette_data: Wrapping<u8>,
    pub cgb_background_palette_spec: Wrapping<u8>,
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

    // Transient state saved for debug view purposes
    frame_scxs: [u8; LCD_VERTICAL_PIXEL_COUNT],
    frame_scxs_valid: [bool; LCD_VERTICAL_PIXEL_COUNT],
    frame_scys_at_scanline_0: [u8; LCD_HORIZONTAL_PIXEL_COUNT],
    frame_scys_first_scanline_valid: [bool; LCD_HORIZONTAL_PIXEL_COUNT],
    // TODO: make this private? move it to pixel fetcher?
    pub tile_map0_last_addressing_modes: [TileAddressingMode; TILE_MAP_TILE_TOTAL],
    pub tile_map1_last_addressing_modes: [TileAddressingMode; TILE_MAP_TILE_TOTAL],
}

const BLACK: [u8; 4] = [0, 0, 0, 255];
const DARK_GRAY: [u8; 4] = [0x55, 0x55, 0x55, 255];
const LIGHT_GRAY: [u8; 4] = [0xAA, 0xAA, 0xAA, 255];
const WHITE: [u8; 4] = [0xFF, 0xFF, 0xFF, 255];

pub fn pixel_code_to_rgba(pixel_code: u8, palette: u8) -> [u8; PIXEL_DATA_SIZE] {
    let pixel_shade = match pixel_code {
        0b00 => palette & 0b11,
        0b01 => (palette >> 2) & 0b11,
        0b10 => (palette >> 4) & 0b11,
        0b11 => (palette >> 6) & 0b11,
        _ => panic!("Invalid pixel code: 0x{:08b}", pixel_code),
    };
    match pixel_shade {
        0b00 => WHITE,
        0b01 => LIGHT_GRAY,
        0b10 => DARK_GRAY,
        0b11 => BLACK,
        _ => unreachable!(),
    }
}

// Each pixel takes 4 bytes (R, G, B, A).  Each y results in 160 pixels.
pub fn pixel_coordinates_in_rgba_slice(x: u8, y: u8) -> usize {
    (y as usize * LCD_HORIZONTAL_PIXEL_COUNT + x as usize) * PIXEL_DATA_SIZE
}

impl PPU {
    pub fn new(fix_ly: bool) -> Self {
        PPU {
            drawn_pixels_on_current_row: 0,
            fix_ly_for_gb_doctor: fix_ly,
            last_stat_line: 0,
            scanline_dots: 0,
            state: PPUState::OAMScan,

            background_palette_data: 0,
            cgb_background_palette_spec: Wrapping(0),
            cgb_background_palette_data: Wrapping(0),
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

            frame_scxs: [0; LCD_VERTICAL_PIXEL_COUNT],
            frame_scxs_valid: [true; LCD_VERTICAL_PIXEL_COUNT],
            frame_scys_at_scanline_0: [0; LCD_HORIZONTAL_PIXEL_COUNT],
            frame_scys_first_scanline_valid: [true; LCD_HORIZONTAL_PIXEL_COUNT],
            tile_map0_last_addressing_modes: [TileAddressingMode::UnsignedFrom0x8000;
                TILE_MAP_TILE_TOTAL],
            tile_map1_last_addressing_modes: [TileAddressingMode::UnsignedFrom0x8000;
                TILE_MAP_TILE_TOTAL],
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

    pub fn increment_ly(&mut self, interrupts: &mut Interrupts) {
        self.lcd_y_coord = self.lcd_y_coord + Wrapping(1);
        if self.lcd_y_coord == self.lcd_y_compare {
            utils::set_bit(&mut self.lcd_status, LYC_EQUALS_LY_BIT);
            if utils::is_bit_set(&self.lcd_status, LYC_EQUALS_LY_INTERRUPT_SELECT_BIT) {
                interrupts.request(STAT_INTERRUPT_BIT);
            }
        } else {
            utils::unset_bit(&mut self.lcd_status, LYC_EQUALS_LY_BIT);
        }
    }

    pub fn read_ly(&self) -> Wrapping<u8> {
        if self.fix_ly_for_gb_doctor {
            Wrapping(144)
        } else {
            self.lcd_y_coord
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
                        let pixel_rgba =
                            pixel_code_to_rgba(pixel_code, self.background_palette_data);
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
            &self.tile_map0_last_addressing_modes,
        );

        // Render the top and bottom SCY lines, where they haven't been messed with mid-frame
        let scx_top = self.frame_scxs[0] as usize;
        let scx_bot = self.frame_scxs[LCD_VERTICAL_PIXEL_COUNT - 1] as usize;
        for y in 0..LCD_HORIZONTAL_PIXEL_COUNT {
            if self.frame_scys_first_scanline_valid[y] {
                let scy = self.frame_scys_at_scanline_0[y] as usize;
                let pixel_index =
                    scy * TILE_MAP_HORIZONTAL_PIXELS + ((y + scx_top) % TILE_MAP_HORIZONTAL_PIXELS);
                self.tile_map0_pixels[pixel_index * 4..(pixel_index + 1) * 4]
                    .copy_from_slice(&[255, 0, 0, 255]);
                let pixel_index = ((scy + LCD_VERTICAL_PIXEL_COUNT) % TILE_MAP_VERTICAL_PIXELS)
                    * TILE_MAP_HORIZONTAL_PIXELS
                    + ((y + scx_bot) % TILE_MAP_HORIZONTAL_PIXELS);
                self.tile_map0_pixels[pixel_index * 4..(pixel_index + 1) * 4]
                    .copy_from_slice(&[255, 255, 0, 255]);
            }
        }

        // Render the left and right SCY lines, where they haven't been messed with mid-frame
        let scy_left = self.frame_scys_at_scanline_0[0] as usize;
        let scy_right = self.frame_scys_at_scanline_0[LCD_HORIZONTAL_PIXEL_COUNT - 1] as usize;
        for x in 0..LCD_VERTICAL_PIXEL_COUNT {
            if self.frame_scxs_valid[x] {
                let scx = self.frame_scxs[x] as usize;
                let pixel_index =
                    ((x + scy_left) % TILE_MAP_VERTICAL_PIXELS) * TILE_MAP_HORIZONTAL_PIXELS + scx;
                self.tile_map0_pixels[pixel_index * 4..(pixel_index + 1) * 4]
                    .copy_from_slice(&[0, 255, 0, 255]);
                let pixel_index = ((x + scy_right) % TILE_MAP_VERTICAL_PIXELS)
                    * TILE_MAP_HORIZONTAL_PIXELS
                    + ((scx + LCD_HORIZONTAL_PIXEL_COUNT) % TILE_MAP_HORIZONTAL_PIXELS);
                self.tile_map0_pixels[pixel_index * 4..(pixel_index + 1) * 4]
                    .copy_from_slice(&[0, 255, 255, 255]);
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
            &self.tile_map1_last_addressing_modes,
        )
    }

    // TODO: Eventually we could update on the fly on writes
    pub fn render(&mut self) {
        self.render_tile_palette();
        self.render_tile_map0();
        // self.render_tile_map1();
    }

    pub fn prepare_for_new_frame(
        &mut self,
        bgw_fetcher: &mut BackgroundOrWindowFetcher,
        obj_fetcher: &mut ObjectFetcher,
    ) {
        self.lcd_y_coord = Wrapping(0);

        bgw_fetcher.prepare_for_new_frame();
        obj_fetcher.prepare_for_new_frame();

        self.frame_scxs = [0; LCD_VERTICAL_PIXEL_COUNT];
        self.frame_scxs_valid = [true; LCD_VERTICAL_PIXEL_COUNT];

        self.frame_scys_at_scanline_0 = [0; LCD_HORIZONTAL_PIXEL_COUNT];
        self.frame_scys_first_scanline_valid = [true; LCD_HORIZONTAL_PIXEL_COUNT];
    }

    pub fn ticks(
        &mut self,
        bgw_fetcher: &mut BackgroundOrWindowFetcher,
        interrupts: &mut Interrupts,
        obj_fetcher: &mut ObjectFetcher,
        pixel_fetcher: &mut Fetcher,
        dots: u8,
    ) {
        for _ in 0..dots {
            self.tick(bgw_fetcher, obj_fetcher, interrupts, pixel_fetcher);
        }
    }

    pub fn tick(
        &mut self,
        bgw_fetcher: &mut BackgroundOrWindowFetcher,
        obj_fetcher: &mut ObjectFetcher,
        interrupts: &mut Interrupts,
        pixel_fetcher: &mut Fetcher,
    ) {
        if !self.is_lcd_ppu_on() {
            return;
        }

        self.scanline_dots += 1;
        if self.scanline_dots > 456 {
            panic!("Frame did not finish rendering in time, investigate.");
        }

        match self.state {
            // mode 2
            PPUState::OAMScan => {
                if self.scanline_dots == 80 {
                    let ly = self.read_ly().0 as usize;

                    // At the start of each scanline, remember SCX
                    if ly < LCD_VERTICAL_PIXEL_COUNT {
                        self.frame_scxs[ly] = self.scx.0;
                    }

                    let mut selected_objects = VecDeque::new();
                    let object_size = 8; // TODO: this is either 8 or 16 depending on something
                    let ly = ly as i16; // from now on it's convenient as a signed (yet >= 0)
                    for object_offset in (0x00..0x9F).step_by(4) {
                        if selected_objects.len() == 10 {
                            break;
                        }
                        let y_screen_plus_16 = self.object_attribute_memory[object_offset];
                        let object_min_y_on_screen = (y_screen_plus_16 as u16 as i16) - 16;
                        let object_max_y_on_screen = object_min_y_on_screen + object_size - 1;
                        if object_min_y_on_screen <= ly && ly <= object_max_y_on_screen {
                            selected_objects.push_back(Sprite {
                                x_screen_plus_8: self.object_attribute_memory[object_offset + 1],
                                y_screen_plus_16,
                                tile_index: self.object_attribute_memory[object_offset + 2],
                                attributes: self.object_attribute_memory[object_offset + 3],
                            });
                        }
                    }
                    obj_fetcher.selected_objects = selected_objects;
                    obj_fetcher.vram_tile_column = 0; // FIXME?
                    self.switch_to_drawing_pixels(pixel_fetcher);
                }
            }

            // mode 3
            PPUState::DrawingPixels => {
                let bgw_fifo_len = bgw_fetcher.fifo.len();
                let obj_fifo_len = obj_fetcher.fifo.len();

                let fetcher_state = &pixel_fetcher.fetching_for;
                if obj_fifo_len == 0 && bgw_fifo_len != 0 {
                    if *fetcher_state == FetchingFor::BackgroundOrWindowFIFO {
                        pixel_fetcher.switch_to_object_fifo();
                    }
                } else {
                    if *fetcher_state == FetchingFor::ObjectFIFO {
                        pixel_fetcher.switch_to_background_or_window_fifo();
                    }
                }
                pixel_fetcher.tick(bgw_fetcher, obj_fetcher, self);

                if !bgw_fetcher.fifo.is_empty() && !obj_fetcher.fifo.is_empty() {
                    // During scanline 0, remember SCY for every pixel pushed
                    let ly = self.read_ly().0 as usize;
                    if ly == 0 {
                        self.frame_scys_at_scanline_0[self.drawn_pixels_on_current_row as usize] =
                            self.scy.0;
                    }

                    let bgw_pixel = bgw_fetcher.fifo.pop_front().unwrap();
                    let obj_pixel = obj_fetcher.fifo.pop_front().unwrap();
                    let pixel_x = self.drawn_pixels_on_current_row;
                    let pixel_y = self.read_ly().0;

                    let from = pixel_coordinates_in_rgba_slice(pixel_x, pixel_y);
                    // Simulate pixel mixing
                    let (selected_pixel, palette) = if obj_pixel.color == 0 {
                        (bgw_pixel.color, self.background_palette_data)
                    } else {
                        // FIXME: need to choose between OBJ palettes based on attribute
                        (obj_pixel.color, self.object_palette_0.0)
                    };
                    let rgba = pixel_code_to_rgba(selected_pixel, palette);
                    self.lcd_pixels[from..from + 4].copy_from_slice(&rgba);
                    self.drawn_pixels_on_current_row += 1;

                    if self.drawn_pixels_on_current_row as usize == LCD_HORIZONTAL_PIXEL_COUNT {
                        self.switch_to_horizontal_blank()
                    }
                }
            }

            // mode 0
            PPUState::HorizontalBlank => {
                if self.scanline_dots == 456 {
                    self.scanline_dots = 0;
                    self.increment_ly(interrupts);
                    if self.read_ly().0 as usize == LCD_VERTICAL_PIXEL_COUNT {
                        self.switch_to_vertical_blank(interrupts)
                    } else {
                        self.switch_to_oam_scan(bgw_fetcher, obj_fetcher)
                    }
                }
            }

            // mode 1
            PPUState::VerticalBlank => {
                if self.scanline_dots == 456 {
                    self.scanline_dots = 0;
                    self.increment_ly(interrupts);
                    if self.read_ly().0 == 153 {
                        self.prepare_for_new_frame(bgw_fetcher, obj_fetcher);
                        self.switch_to_oam_scan(bgw_fetcher, obj_fetcher)
                    }
                }
            }
        }

        // STAT interrupt check
        let stat_line = (self.lcd_status.0 >> 3) & 0xF;
        if self.last_stat_line == 0 && stat_line != 0 {
            interrupts.request(STAT_INTERRUPT_BIT);
        }
        self.last_stat_line = stat_line;
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

    fn switch_to_oam_scan(
        &mut self,
        bgw_fetcher: &mut BackgroundOrWindowFetcher,
        obj_fetcher: &mut ObjectFetcher,
    ) {
        self.drawn_pixels_on_current_row = 0;
        bgw_fetcher.prepare_for_new_row();
        obj_fetcher.prepare_for_new_row();
        // Disabled because it locks LCD for Dr. Mario:
        // machine.ppu_mut().lcd_status = Wrapping((machine.ppu().lcd_status.0 & 0xFC) | 2);
        utils::unset_bit(&mut self.lcd_status, MODE_0_INTERRUPT_SELECT_BIT);
        utils::unset_bit(&mut self.lcd_status, MODE_1_INTERRUPT_SELECT_BIT);
        utils::set_bit(&mut self.lcd_status, MODE_2_INTERRUPT_SELECT_BIT);
        self.state = PPUState::OAMScan;
    }

    fn switch_to_drawing_pixels(&mut self, pixel_fetcher: &mut Fetcher) {
        pixel_fetcher.switch_to_background_or_window_fifo();
        // Disabled because it locks LCD for Dr. Mario:
        // machine.ppu_mut().lcd_status = Wrapping((machine.ppu().lcd_status.0 & 0xFC) | 3);
        self.state = PPUState::DrawingPixels;
    }

    fn switch_to_horizontal_blank(&mut self) {
        // Disabled because it locks LCD for Dr. Mario:
        // machine.ppu_mut().lcd_status = Wrapping(machine.ppu().lcd_status.0 & 0xFC);
        utils::set_bit(&mut self.lcd_status, MODE_0_INTERRUPT_SELECT_BIT);
        utils::unset_bit(&mut self.lcd_status, MODE_1_INTERRUPT_SELECT_BIT);
        utils::unset_bit(&mut self.lcd_status, MODE_2_INTERRUPT_SELECT_BIT);
        self.state = PPUState::HorizontalBlank;
    }

    fn switch_to_vertical_blank(&mut self, interrupts: &mut Interrupts) {
        // Disabled because it locks LCD for Dr. Mario:
        // machine.ppu_mut().lcd_status = Wrapping((machine.ppu().lcd_status.0 & 0xFC) | 1);
        utils::unset_bit(&mut self.lcd_status, MODE_0_INTERRUPT_SELECT_BIT);
        utils::set_bit(&mut self.lcd_status, MODE_1_INTERRUPT_SELECT_BIT);
        utils::unset_bit(&mut self.lcd_status, MODE_2_INTERRUPT_SELECT_BIT);
        interrupts.request(VBLANK_INTERRUPT_BIT);
        self.state = PPUState::VerticalBlank
    }
}

fn render_tile_map(
    vram: &[u8],
    tile_palette_pixels: &[u8],
    tile_map_pixels: &mut [u8],
    tile_map_vram_offset: usize,
    tile_map_last_addressing_modes: &[TileAddressingMode; TILE_MAP_TILE_TOTAL],
) {
    for tile_map_y in 0..TILE_MAP_VERTICAL_TILE_COUNT {
        for tile_map_x in 0..TILE_MAP_HORIZONTAL_TILE_COUNT {
            let tile_entry_offset = (tile_map_y << 5) | tile_map_x;
            let tile_id = vram[tile_map_vram_offset + tile_entry_offset];
            // Because tiles have already been rendered as pixels in the tile palette, here we
            // can just copy slices of lines for the 8 lines of the tile.
            for tile_pixel_y in 0..VERTICAL_PIXELS_PER_TILE {
                let tiles_to_skip = tile_map_y * TILE_MAP_HORIZONTAL_TILE_COUNT;
                let row_pixels_to_skip = tile_pixel_y * TILE_MAP_HORIZONTAL_PIXELS
                    + tile_map_x * HORIZONTAL_PIXELS_PER_TILE;
                let pixels_to_skip = tiles_to_skip * PIXELS_PER_TILE + row_pixels_to_skip;
                let bytes_to_skip = pixels_to_skip * PIXEL_DATA_SIZE;

                let tile_index_in_tile_map =
                    tile_map_y * TILE_MAP_HORIZONTAL_TILE_COUNT + tile_map_x;
                let addressing_mode = tile_map_last_addressing_modes[tile_index_in_tile_map];
                let tile_index_in_palette =
                    get_tile_index_in_palette(tile_id, &addressing_mode) as usize;
                let palette_tile_y = tile_index_in_palette / TILE_PALETTE_HORIZONTAL_TILE_COUNT;
                let palette_tile_x = tile_index_in_palette % TILE_MAP_HORIZONTAL_TILE_COUNT;
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
