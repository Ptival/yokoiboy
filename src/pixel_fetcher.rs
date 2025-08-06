pub mod background_or_window;
pub mod object;

use background_or_window::BackgroundOrWindowFetcher;
// use iced::advanced::graphics::core::event;
use object::ObjectFetcher;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::ppu::PPU;

#[derive(Clone, Debug, PartialEq)]
pub enum FlipX {
    Yes,
    No,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FlipY {
    Yes,
    No,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
enum FetcherState {
    GetTileDelay,
    GetTile,
    GetTileDataLowDelay,
    GetTileDataLow,
    GetTileDataHighDelay,
    GetTileDataHigh,
    PushRow,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct FIFOItem {
    // These fields are needed for emulation
    pub color: u8,

    // These fields are for debugging purposes
    pub tile_id: u8,
}

impl Default for FIFOItem {
    fn default() -> Self {
        Self {
            color: 0,
            tile_id: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum FetchingFor {
    BackgroundOrWindowFIFO,
    ObjectFIFO,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Fetcher {
    pub fetching_for: FetchingFor,
}

// Background and Window use one of these based on bit 4 of lcd_control.
// Sprites always use UnsignedFrom0x8000.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum TileAddressingMode {
    UnsignedFrom0x8000,
    SignedFrom0x9000,
}

pub fn get_tile_index_in_palette(tile_id: u8, addressing_mode: &TileAddressingMode) -> u16 {
    match addressing_mode {
        TileAddressingMode::UnsignedFrom0x8000 => tile_id as u16,
        TileAddressingMode::SignedFrom0x9000 => (256 + (tile_id as i8) as i16) as u16,
    }
}

impl Fetcher {
    pub fn new() -> Self {
        Fetcher {
            fetching_for: FetchingFor::BackgroundOrWindowFIFO,
        }
    }

    // pub fn reset(&mut self) {
    //     self.background_window_fetcher.reset();
    //     self.object_fetcher.reset(0);
    // }

    fn switch_to(&mut self, fetching_for: FetchingFor) {
        // self.tile_row_data = [0; 8];
        self.fetching_for = fetching_for;
        // self.state = FetcherState::GetTileDelay;
    }

    pub fn switch_to_object_fifo(&mut self) {
        self.switch_to(FetchingFor::ObjectFIFO)
    }

    pub fn switch_to_background_or_window_fifo(&mut self) {
        self.switch_to(FetchingFor::BackgroundOrWindowFIFO)
    }

    /// Reads the requested tile row data from VRAM and stores it in `tile_row_data`.
    pub fn read_tile_row(
        vram: &[u8],
        addressing_mode: &TileAddressingMode,
        current_scanline: u8,
        scy: u8,
        tile_id: u8,
        flip_x: FlipX,
        flip_y: FlipY,
        bit_plane: bool,
        tile_row_data: &mut [u8; 8],
    ) {
        // NOTE: rather than going through the MMU again with an absolute address, I'm computing the
        // address relative to VRAM and reading directly from the VRAM slice.  Should be slightly
        // faster as you don't need to perform range checks to realize you're heading into VRAM.
        let tile_index_in_palette = get_tile_index_in_palette(tile_id, addressing_mode);
        let row_of_pixel_within_tile_if_upright =
            ((current_scanline as u16 + scy as u16) & 255) % 8;
        let row_of_pixel_within_tile = if flip_y == FlipY::Yes {
            // TODO: when handling tall sprites, this might need to be 15 for them
            7 - row_of_pixel_within_tile_if_upright
        } else {
            row_of_pixel_within_tile_if_upright
        };
        let address_in_vram_slice =
            tile_index_in_palette * 16 + (row_of_pixel_within_tile as u16) * 2;
        let pixel_data = vram[address_in_vram_slice as usize + bit_plane as usize];
        // We just finished reading one byte.  Each bit is half of a pixel value, we coalesce them
        // here.
        //
        // Note: This assumes that `tile_row_data` is cleared at each loop.
        //
        // Note: it's nice to have the row data be sorted by increasing X, but the lowest bit
        // position is the highest X pixel, so using (7 - bit_position) to reorder.
        for target_bit_position in 0..8 {
            let source_bit_position = if flip_x == FlipX::Yes {
                7 - target_bit_position
            } else {
                target_bit_position
            };
            tile_row_data[7 - target_bit_position] |=
                ((pixel_data >> source_bit_position) & 1) << (bit_plane as u8);
        }
    }

    pub fn tick(
        &mut self,
        bgw_fetcher: &mut BackgroundOrWindowFetcher,
        obj_fetcher: &mut ObjectFetcher,
        ppu: &mut PPU,
    ) {
        match self.fetching_for {
            FetchingFor::BackgroundOrWindowFIFO => {
                event!(Level::DEBUG, "Fetching pixels for BGW FIFO");
                bgw_fetcher.tick(ppu);
            }
            FetchingFor::ObjectFIFO => {
                event!(Level::DEBUG, "Fetching pixels for OBJ FIFO");
                obj_fetcher.tick(ppu);
            }
        }
    }
}
