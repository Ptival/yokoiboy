use std::{
    cmp::{max, min},
    collections::VecDeque,
};

use crate::ppu::PPU;

use super::{FIFOItem, Fetcher, TileAddressingMode};

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
pub struct Sprite {
    pub attributes: u8,
    pub tile_index: u8,
    pub x_screen_plus_8: u8,
    pub y_screen_plus_16: u8,
}

#[derive(Clone, Debug)]
pub struct ObjectFetcher {
    state: FetcherState,
    pub fifo: VecDeque<FIFOItem>,
    tile_id: u8,
    pub vram_tile_column: u8,
    tile_row_data: [u8; 8],
    pub selected_objects: VecDeque<Sprite>,
}

pub fn inclusive_ranges_overlap((s1, e1): (i16, i16), (s2, e2): (i16, i16)) -> bool {
    max(s1, s2) <= min(e1, e2)
}

impl ObjectFetcher {
    pub fn new() -> Self {
        ObjectFetcher {
            state: FetcherState::GetTileDelay,
            fifo: VecDeque::new(),
            tile_id: 0,
            vram_tile_column: 0,
            tile_row_data: [0; 8],
            selected_objects: VecDeque::new(),
        }
    }

    pub fn prepare_for_new_row(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.fifo.clear();
        self.vram_tile_column = 0;
    }

    pub fn prepare_for_new_frame(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.fifo.clear();
        self.vram_tile_column = 0;
    }

    pub fn tick(&mut self, ppu: &mut PPU) {
        match self.state {
            FetcherState::GetTileDelay => self.state = FetcherState::GetTile,

            FetcherState::GetTile => {
                // let vram_tile_row = (PPU::read_ly(machine) + machine.ppu().scy).0 & 255;
                // self.row_of_pixel_within_tile = vram_tile_row % 8;

                let tile_column = self.vram_tile_column as i16;
                let selected = &self.selected_objects;
                let tile_id = match selected.iter().find(|item| {
                    let item_x_screen = item.x_screen_plus_8 as u16 as i16 - 8;
                    inclusive_ranges_overlap(
                        (tile_column, tile_column + 7),
                        (item_x_screen, item_x_screen + 7),
                    )
                }) {
                    Some(sprite) => {
                        println!("We found an object tile!");
                        sprite.tile_index
                    }
                    None => 0,
                };
                self.tile_id = tile_id;
                self.state = FetcherState::GetTileDataLowDelay
            }

            FetcherState::GetTileDataLowDelay => self.state = FetcherState::GetTileDataLow,

            FetcherState::GetTileDataLow => {
                let ly = ppu.read_ly();
                Fetcher::read_tile_row(
                    &ppu.vram,
                    &TileAddressingMode::UnsignedFrom0x8000,
                    (ly + ppu.scy).0,
                    self.tile_id,
                    false,
                    &mut self.tile_row_data,
                );
                self.state = FetcherState::GetTileDataHighDelay
            }

            FetcherState::GetTileDataHighDelay => self.state = FetcherState::GetTileDataHigh,

            FetcherState::GetTileDataHigh => {
                let ly = ppu.read_ly();
                Fetcher::read_tile_row(
                    &ppu.vram,
                    &TileAddressingMode::UnsignedFrom0x8000,
                    (ly + ppu.scy).0,
                    self.tile_id,
                    true,
                    &mut self.tile_row_data,
                );
                self.state = FetcherState::PushRow
            }

            FetcherState::PushRow => {
                let obj_fifo_len = self.fifo.len();
                // Object FIFO pixels are merged with existing object FIFO pixels:
                // Those with ID 0 are overwritten by latter ones, otherwise the existing one wins
                for i in 0..8 {
                    if i < obj_fifo_len {
                        // Pixel merging following OBJ-to-OBJ priority
                        let old_item = self.fifo[i].clone();
                        if old_item.color == 0 {
                            self.fifo[i] = FIFOItem {
                                color: self.tile_row_data[i],
                            };
                        }
                    } else {
                        // No pixel to merge with, just push
                        let color = self.tile_row_data[i];
                        self.fifo.push_back(FIFOItem { color });
                    }
                }
                // clean up so that GetTileData can assume 0
                self.tile_row_data = [0; 8];
                self.state = FetcherState::GetTileDelay
            }
        }
    }
}
