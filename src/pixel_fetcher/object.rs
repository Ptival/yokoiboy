use std::{
    cmp::{max, min},
    collections::VecDeque,
};

use crate::ppu::PPU;

use super::{Fetcher, TileAddressingMode};

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
pub enum ObjectPalette {
    ObjectPalette0,
    ObjectPalette1,
}

#[derive(Clone, Debug)]
pub struct ObjectFIFOItem {
    pub color: u8,
    pub palette: ObjectPalette,
}

#[derive(Clone, Debug)]
pub struct ObjectFetcher {
    state: FetcherState,
    pub fifo: VecDeque<ObjectFIFOItem>,
    sprite: Option<Sprite>,
    pub pixel_index_in_row: u8,
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
            sprite: None,
            pixel_index_in_row: 0,
            tile_row_data: [0; 8],
            selected_objects: VecDeque::new(),
        }
    }

    pub fn prepare_for_new_row(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.fifo.clear();
        self.tile_row_data = [0; 8];
        self.pixel_index_in_row = 0;
    }

    pub fn prepare_for_new_frame(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.fifo.clear();
        self.pixel_index_in_row = 0;
    }

    pub fn tick(&mut self, ppu: &mut PPU) {
        match self.state {
            FetcherState::GetTileDelay => self.state = FetcherState::GetTile,

            FetcherState::GetTile => {
                let current_x = self.pixel_index_in_row as i16;
                let x_range = (current_x, current_x + 7);
                let selected = &self.selected_objects;

                // Technically we should only tick this when there is going to be a match
                self.sprite = selected
                    .iter()
                    .find(|item| {
                        let item_x_screen = item.x_screen_plus_8 as u16 as i16 - 8;
                        inclusive_ranges_overlap(x_range, (item_x_screen, item_x_screen + 7))
                    })
                    .map(|i| i.clone());

                self.state = FetcherState::GetTileDataLowDelay
            }

            FetcherState::GetTileDataLowDelay => self.state = FetcherState::GetTileDataLow,

            FetcherState::GetTileDataLow => {
                let ly = ppu.read_ly();
                match self.sprite.clone() {
                    Some(sprite) => Fetcher::read_tile_row(
                        &ppu.vram,
                        &TileAddressingMode::UnsignedFrom0x8000,
                        (ly + ppu.scy).0,
                        sprite.tile_index,
                        false,
                        &mut self.tile_row_data,
                    ),
                    None => {
                        self.tile_row_data = [0; 8];
                    }
                }
                self.state = FetcherState::GetTileDataHighDelay
            }

            FetcherState::GetTileDataHighDelay => self.state = FetcherState::GetTileDataHigh,

            FetcherState::GetTileDataHigh => {
                let ly = ppu.read_ly();
                match self.sprite.clone() {
                    Some(sprite) => Fetcher::read_tile_row(
                        &ppu.vram,
                        &TileAddressingMode::UnsignedFrom0x8000,
                        (ly + ppu.scy).0,
                        sprite.tile_index,
                        true,
                        &mut self.tile_row_data,
                    ),
                    None => {
                        self.tile_row_data = [0; 8];
                    }
                }
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
                            self.fifo[i] = ObjectFIFOItem {
                                color: self.tile_row_data[i],
                                palette: palette_for_sprite(self.sprite.as_ref()),
                            };
                        }
                    } else {
                        // No pixel to merge with, just push
                        let color = self.tile_row_data[i];
                        self.fifo.push_back(ObjectFIFOItem {
                            color,
                            palette: palette_for_sprite(self.sprite.as_ref()),
                        });
                    }
                }
                // clean up so that GetTileData can assume 0
                self.tile_row_data = [0; 8];
                self.state = FetcherState::GetTileDelay
            }
        }
    }
}

fn palette_for_sprite(sprite: Option<&Sprite>) -> ObjectPalette {
    match sprite {
        Some(sprite) => match (sprite.attributes >> 4) & 1 {
            0b0 => ObjectPalette::ObjectPalette0,
            0b1 => ObjectPalette::ObjectPalette1,
            _ => unreachable!(),
        },
        None => ObjectPalette::ObjectPalette0, // does not matter
    }
}
