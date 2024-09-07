pub mod background_or_window;
pub mod object;

use background_or_window::BackgroundOrWindowFetcher;
use object::ObjectFetcher;

use crate::ppu::PPU;

#[derive(Clone, Debug)]
enum FetcherState {
    GetTileDelay,
    GetTile,
    GetTileDataLowDelay,
    GetTileDataLow,
    GetTileDataHighDelay,
    GetTileDataHigh,
    PushRow,
}

#[derive(Clone, Debug)]
pub struct FIFOItem {
    pub color: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FetchingFor {
    BackgroundOrWindowFIFO,
    ObjectFIFO,
}

#[derive(Clone, Debug)]
pub struct Fetcher {
    pub fetching_for: FetchingFor,
}

// Background and Window use one of these based on bit 4 of lcd_control.
// Sprites always use UnsignedFrom0x8000.
pub enum TileAddressingMode {
    UnsignedFrom0x8000,
    SignedFrom0x9000,
}

fn tile_index_in_palette(tile_id: u8, addressing_mode: &TileAddressingMode) -> u16 {
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

    pub fn read_tile_row(
        vram: &[u8],
        addressing_mode: &TileAddressingMode,
        current_line: u8,
        tile_id: u8,
        bit_plane: bool,
        tile_row_data: &mut [u8],
    ) {
        // WARNING: when handling sprites, will need to update this to ignore addressing mode for
        // their tiles

        // NOTE: rather than going through the MMU again with an absolute address, I'm computing the
        // address relative to VRAM and reading directly from the VRAM slice.  Should be slightly
        // faster as you don't need to perform range checks to realize you're heading into VRAM.
        let tile_index_in_palette = tile_index_in_palette(tile_id, addressing_mode);
        let row_of_pixel_within_tile = (current_line & 255) % 8;
        let address_in_vram_slice =
            tile_index_in_palette * 16 + (row_of_pixel_within_tile as u16) * 2;
        let pixel_data = vram[address_in_vram_slice as usize + bit_plane as usize];
        // We just finished reading one byte.  Each bit is half of a pixel value, we coalesce them
        // here Note: This assumes that `tile_row_data` is cleared at each loop.
        // Note: it's nice to have the row data be sorted by increasing X, but the lowest bit
        // position is the highest X pixel, so using (7 - bit_position) to reorder.
        for bit_position in 0..8 {
            tile_row_data[7 - bit_position] |=
                ((pixel_data >> bit_position) & 1) << (bit_plane as u8);
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
                bgw_fetcher.tick(ppu);
            }
            FetchingFor::ObjectFIFO => {
                obj_fetcher.tick(ppu);
            }
        }
    }
}
