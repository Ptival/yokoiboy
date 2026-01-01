use std::{collections::VecDeque, num::Wrapping};

use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{
    pixel_fetcher::{FlipX, FlipY},
    ppu::{
        LCDC_BACKGROUND_TILE_MAP_AREA_BIT, LCDC_WINDOW_ENABLE_BIT, LCDC_WINDOW_TILE_MAP_AREA_BIT,
        PPU, TILE_MAP_HORIZONTAL_TILE_COUNT,
    },
    utils,
};

use super::{FIFOItem, Fetcher, FetcherState};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BackgroundOrWindowFetcher {
    /// BGW fetcher automaton state
    state: FetcherState,
    /// Output FIFO, onto which BGW items are pushed by the fetcher
    pub fifo: VecDeque<FIFOItem>,
    tile_id: u8,
    /// Keeping track of which VRAM tile slot we are pushing to.  Initially it's 0 for the leftmost
    /// 8 pixels on the current row, then 1 for the next 8 pixels after we pushed one row.
    pub vram_tile_column: u8,
    tile_row_data: [u8; 8],
}

impl BackgroundOrWindowFetcher {
    pub fn new() -> Self {
        BackgroundOrWindowFetcher {
            state: FetcherState::GetTileDelay,
            fifo: VecDeque::new(),
            tile_id: 0,
            vram_tile_column: 0,
            tile_row_data: [0; 8],
        }
    }

    pub fn prepare_for_new_frame(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.fifo.clear();
        self.vram_tile_column = 0;
        self.tile_row_data = [0; 8];
    }

    pub fn prepare_for_new_row(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.fifo.clear();
        self.vram_tile_column = 0;
        self.tile_row_data = [0; 8];
    }

    pub fn tick(&mut self, ppu: &mut PPU) {
        match self.state {
            FetcherState::GetTileDelay => {
                event!(Level::DEBUG, "BGW fetcher awaiting tile");
                self.state = FetcherState::GetTile
            }

            FetcherState::GetTile => {
                event!(Level::DEBUG, "BGW fetcher getting tile");
                // NOTE: Because the following operations are done via Wrapping at u8, they
                // automatically perform the necessary "mod 256"
                let vram_pixel_row = (ppu.read_ly() + ppu.scy).0;
                let vram_pixel_col = (Wrapping(self.vram_tile_column) * Wrapping(8) + ppu.scx).0;

                let tile_row = vram_pixel_row / 8;
                let tile_col = vram_pixel_col / 8;

                let tile_index_in_its_tile_map =
                    tile_row as usize * TILE_MAP_HORIZONTAL_TILE_COUNT + tile_col as usize;

                enum TileMap {
                    TileMap0,
                    TileMap1,
                }
                // Note: technically, when LCDC_BACKGROUND_AND_WINDOW_ENABLE_BIT is 0, we're not
                // using any tile map and should just return blank tiles
                let lcdc = ppu.lcd_control;
                let lcdc_bit_that_determines_tilemap =
                    if utils::is_bit_set(&lcdc, LCDC_WINDOW_ENABLE_BIT) {
                        LCDC_WINDOW_TILE_MAP_AREA_BIT
                    } else {
                        LCDC_BACKGROUND_TILE_MAP_AREA_BIT
                    };
                let tilemap_to_use = if utils::is_bit_set(&lcdc, lcdc_bit_that_determines_tilemap) {
                    TileMap::TileMap1
                } else {
                    TileMap::TileMap0
                };

                let tilemap = match tilemap_to_use {
                    TileMap::TileMap0 => {
                        ppu.debug.tile_map0_last_addressing_modes[tile_index_in_its_tile_map] =
                            ppu.get_addressing_mode();
                        &ppu.vram_tile_map0
                    }
                    TileMap::TileMap1 => {
                        ppu.debug.tile_map1_last_addressing_modes[tile_index_in_its_tile_map] =
                            ppu.get_addressing_mode();
                        &ppu.vram_tile_map1
                    }
                };

                let row_address = ((tile_row as u16) << 5) + (tile_col as u16);

                self.tile_id = tilemap[row_address as usize];
                self.state = FetcherState::GetTileDataLowDelay;
            }

            FetcherState::GetTileDataLowDelay => {
                event!(Level::DEBUG, "BGW fetcher awaiting tile low data");
                self.state = FetcherState::GetTileDataLow;
            }

            FetcherState::GetTileDataLow => {
                event!(Level::DEBUG, "BGW fetcher getting tile low data");
                let ly = ppu.read_ly();
                Fetcher::read_tile_row(
                    &ppu.vram,
                    &ppu.get_addressing_mode(),
                    ly.0,
                    ppu.scy.0,
                    self.tile_id,
                    FlipX::No,
                    FlipY::No,
                    false,
                    &mut self.tile_row_data,
                );
                self.state = FetcherState::GetTileDataHighDelay;
            }

            FetcherState::GetTileDataHighDelay => {
                event!(Level::DEBUG, "BGW fetcher awaiting tile low data");
                self.state = FetcherState::GetTileDataHigh;
            }

            FetcherState::GetTileDataHigh => {
                event!(Level::DEBUG, "BGW fetcher getting tile low data");
                let ly = ppu.read_ly();
                Fetcher::read_tile_row(
                    &ppu.vram,
                    &ppu.get_addressing_mode(),
                    ly.0,
                    ppu.scy.0,
                    self.tile_id,
                    FlipX::No,
                    FlipY::No,
                    true,
                    &mut self.tile_row_data,
                );
                self.state = FetcherState::PushRow;
            }

            FetcherState::PushRow => {
                // Background/Window FIFO pixels only get pushed when the FIFO is empty
                if self.fifo.len() == 0 {
                    event!(Level::DEBUG, "BGW fetcher pushing row of pixels");
                    for i in 0..8 {
                        let color = self.tile_row_data[i];
                        self.fifo.push_back(FIFOItem {
                            color,
                            tile_id: self.tile_id,
                        });
                    }
                    self.vram_tile_column += 1;
                    // clean up so that GetTileData can assume 0
                    self.tile_row_data = [0; 8];
                    self.state = FetcherState::GetTileDelay;
                } else {
                    event!(
                        Level::DEBUG,
                        "BGW fetcher awaiting space to push row of pixels"
                    );
                }
            }
        }
    }
}
