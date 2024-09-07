use std::collections::VecDeque;

use crate::{
    ppu::{LCDC_BACKGROUND_TILE_MAP_AREA_BIT, PPU},
    utils,
};

use super::{FIFOItem, Fetcher, FetcherState};

#[derive(Clone, Debug)]
pub struct BackgroundOrWindowFetcher {
    state: FetcherState,
    pub fifo: VecDeque<FIFOItem>,
    pub row_of_pixel_within_tile: u8,
    tile_id: u8,
    pub vram_tile_column: u8,
    tile_row_data: [u8; 8],
}

impl BackgroundOrWindowFetcher {
    pub fn new() -> Self {
        BackgroundOrWindowFetcher {
            state: FetcherState::GetTileDelay,
            fifo: VecDeque::new(),
            row_of_pixel_within_tile: 0,
            tile_id: 0,
            vram_tile_column: 0,
            tile_row_data: [0; 8],
        }
    }

    pub fn reset(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.vram_tile_column = 0;
    }

    pub fn tick(&mut self, ppu: &mut PPU) {
        match self.state {
            FetcherState::GetTileDelay => self.state = FetcherState::GetTile,

            FetcherState::GetTile => {
                let vram_tile_row = (ppu.read_ly() + ppu.scy).0 & 255;
                // machine.ppu_mut().fetcher.row_of_pixel_within_tile = vram_tile_row % 8;

                // FIXME: more complex rules for the row base address
                let row_vram_offset =
                    if utils::is_bit_set(&ppu.lcd_control, LCDC_BACKGROUND_TILE_MAP_AREA_BIT) {
                        0x1C00 // 0x9C00, but VRAM starts at 0x8000
                    } else {
                        0x1800 // 0x9800, but VRAM starts at 0x8000
                    };

                let row_address = row_vram_offset + ((vram_tile_row as u16 / 8) * 32);

                self.tile_id = ppu.vram[(row_address + (self.vram_tile_column as u16)) as usize];
                self.state = FetcherState::GetTileDataLowDelay;
            }

            FetcherState::GetTileDataLowDelay => {
                self.state = FetcherState::GetTileDataLow;
            }

            FetcherState::GetTileDataLow => {
                let ly = ppu.read_ly();
                Fetcher::read_tile_row(
                    &ppu.vram,
                    &ppu.get_addressing_mode(),
                    (ly + ppu.scy).0,
                    self.tile_id,
                    false,
                    &mut self.tile_row_data,
                );
                self.state = FetcherState::GetTileDataHighDelay;
            }

            FetcherState::GetTileDataHighDelay => {
                self.state = FetcherState::GetTileDataHigh;
            }

            FetcherState::GetTileDataHigh => {
                let ly = ppu.read_ly();
                Fetcher::read_tile_row(
                    &ppu.vram,
                    &ppu.get_addressing_mode(),
                    (ly + ppu.scy).0,
                    self.tile_id,
                    true,
                    &mut self.tile_row_data,
                );
                self.state = FetcherState::PushRow;
            }

            FetcherState::PushRow => {
                // Background/Window FIFO pixels only get pushed when the FIFO is empty
                if self.fifo.len() == 0 {
                    for i in 0..8 {
                        let color = self.tile_row_data[i];
                        self.fifo.push_back(FIFOItem { color });
                    }
                    self.vram_tile_column += 1;
                    // clean up so that GetTileData can assume 0
                    self.tile_row_data = [0; 8];
                    self.state = FetcherState::GetTileDelay;
                }
            }
        }
    }
}
