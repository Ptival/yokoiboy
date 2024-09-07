use std::{
    cmp::{max, min},
    collections::VecDeque,
};

use crate::{machine::Machine, ppu::PPU};

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

    pub fn reset(&mut self, vram_tile_column: u8) {
        self.state = FetcherState::GetTileDelay;
        self.vram_tile_column = vram_tile_column;
    }

    pub fn tick(machine: &mut Machine) -> &mut Machine {
        match machine.obj_fetcher().state {
            FetcherState::GetTileDelay => machine.obj_fetcher_mut().state = FetcherState::GetTile,

            FetcherState::GetTile => {
                // let vram_tile_row = (PPU::read_ly(machine) + machine.ppu().scy).0 & 255;
                // machine.obj_fetcher().row_of_pixel_within_tile = vram_tile_row % 8;

                let tile_column = machine.obj_fetcher().vram_tile_column as i16;
                let selected = &machine.obj_fetcher().selected_objects;
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
                machine.obj_fetcher_mut().tile_id = tile_id;
                machine.obj_fetcher_mut().state = FetcherState::GetTileDataLowDelay
            }

            FetcherState::GetTileDataLowDelay => {
                machine.obj_fetcher_mut().state = FetcherState::GetTileDataLow
            }

            FetcherState::GetTileDataLow => {
                let ly = PPU::read_ly(machine);
                let ppu = machine.ppu_mut();
                Fetcher::read_tile_row(
                    &ppu.vram,
                    &TileAddressingMode::UnsignedFrom0x8000,
                    (ly + ppu.scy).0,
                    ppu.fetcher.object_fetcher.tile_id,
                    false,
                    &mut ppu.fetcher.object_fetcher.tile_row_data,
                );
                machine.obj_fetcher_mut().state = FetcherState::GetTileDataHighDelay
            }

            FetcherState::GetTileDataHighDelay => {
                machine.obj_fetcher_mut().state = FetcherState::GetTileDataHigh
            }

            FetcherState::GetTileDataHigh => {
                let ly = PPU::read_ly(machine);
                let ppu = machine.ppu_mut();
                Fetcher::read_tile_row(
                    &ppu.vram,
                    &TileAddressingMode::UnsignedFrom0x8000,
                    (ly + ppu.scy).0,
                    ppu.fetcher.object_fetcher.tile_id,
                    true,
                    &mut ppu.fetcher.object_fetcher.tile_row_data,
                );
                machine.obj_fetcher_mut().state = FetcherState::PushRow
            }

            FetcherState::PushRow => {
                let obj_fifo_len = machine.obj_fetcher().fifo.len();
                // Object FIFO pixels are merged with existing object FIFO pixels:
                // Those with ID 0 are overwritten by latter ones, otherwise the existing one wins
                for i in 0..8 {
                    if i < obj_fifo_len {
                        // Pixel merging following OBJ-to-OBJ priority
                        let old_item = machine.obj_fetcher().fifo[i].clone();
                        if old_item.color == 0 {
                            machine.obj_fetcher_mut().fifo[i] = FIFOItem {
                                color: machine.obj_fetcher().tile_row_data[i],
                            };
                        }
                    } else {
                        // No pixel to merge with, just push
                        let color = machine.obj_fetcher().tile_row_data[i];
                        machine.obj_fetcher_mut().fifo.push_back(FIFOItem { color });
                    }
                }
                // clean up so that GetTileData can assume 0
                machine.obj_fetcher_mut().tile_row_data = [0; 8];
                machine.obj_fetcher_mut().state = FetcherState::GetTileDelay
            }
        }
        machine
    }
}

impl Machine {
    pub fn obj_fetcher(&self) -> &ObjectFetcher {
        &self.fetcher().object_fetcher
    }

    pub fn obj_fetcher_mut(&mut self) -> &mut ObjectFetcher {
        &mut self.fetcher_mut().object_fetcher
    }
}
