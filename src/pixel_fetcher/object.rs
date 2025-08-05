use std::{
    cmp::{max, min},
    collections::VecDeque,
    num::Wrapping,
};
use tracing::{event, Level};

use super::{Fetcher, TileAddressingMode};

use crate::{
    pixel_fetcher::{FlipX, FlipY},
    ppu::PPU,
    utils::is_bit_set,
};

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

    /// This is the position of the object on the screen, plus 8 pixels.  This lets one place
    /// objects off-screen entering the screen from either side.
    ///
    /// Values ≤ 8 essentially tell you how many of the object pixels are on screen, from 0 being
    /// totally off-screen to the left, and 8 where the object is fully on-screen to the left.
    /// Likewise, at 160, the object is touching the right side of the screen, then values up to 168
    /// push the object off-screen to the right.
    pub x_screen_plus_8: u8,

    /// This is the position of the object on screen, plus 16 pixels.  This lets one place objects
    /// off-screen entering the screen from top or bottom.
    ///
    /// Values ≤ 16 essentially tell you how many of a 16 pixels tall object are visible at the top.
    /// For objects that are 8 pixels tall, they count as being the top part of a 16 pixels tall
    /// object, so they only start being visible for values ≥ 9.  At 144, a 16 pixels tall object
    /// touches the bottom of the screen, and starts going off-screen.  At 160, the object is fully
    /// off-screen at the bottom.
    pub y_screen_plus_16: u8,
}

const FLIP_X_BIT: u8 = 5;
const FLIP_Y_BIT: u8 = 6;
const BG_OVER_OBJ_BIT: u8 = 7;

impl Sprite {
    pub fn bg_over_obj(&self) -> bool {
        return is_bit_set(&Wrapping(self.attributes), BG_OVER_OBJ_BIT);
    }

    pub fn flip_x(&self) -> FlipX {
        return if is_bit_set(&Wrapping(self.attributes), FLIP_X_BIT) {
            FlipX::Yes
        } else {
            FlipX::No
        };
    }

    pub fn flip_y(&self) -> FlipY {
        return if is_bit_set(&Wrapping(self.attributes), FLIP_Y_BIT) {
            FlipY::Yes
        } else {
            FlipY::No
        };
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ObjectPalette {
    ObjectPalette0,
    ObjectPalette1,
}

impl Default for ObjectPalette {
    fn default() -> Self {
        ObjectPalette::ObjectPalette0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ObjectFIFOItem {
    pub bg_over_obj: bool,
    pub color: u8,
    pub palette: ObjectPalette,
}

impl Default for ObjectFIFOItem {
    fn default() -> Self {
        Self {
            bg_over_obj: Default::default(),
            color: Default::default(),
            palette: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjectFetcher {
    state: FetcherState,
    pub fifo: VecDeque<ObjectFIFOItem>,
    sprite: Option<Sprite>,
    tile_row_data: [u8; 8],
    /// During OAM scan, the PPU will populate this with the first 10 (or fewer) objects it finds
    /// intersecting with the current scanline.  These will get rendered, additional objects on the
    /// same line do **not** get rendered!
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
            tile_row_data: [0; 8],
            selected_objects: VecDeque::new(),
        }
    }

    pub fn prepare_for_new_row(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.fifo.clear();
        self.tile_row_data = [0; 8];
    }

    pub fn prepare_for_new_frame(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.fifo.clear();
    }

    pub fn tick(&mut self, ppu: &mut PPU) {
        match self.state {
            FetcherState::GetTileDelay => {
                event!(Level::DEBUG, "OBJ fetcher awaiting tile");
                self.state = FetcherState::GetTile
            }

            FetcherState::GetTile => {
                event!(Level::DEBUG, "OBJ fetcher getting tile");
                let current_x = ppu.drawn_pixels_on_current_row as i16;
                let x_range = (current_x, current_x + 7);

                // Technically we should only tick this when there is going to be a match
                self.sprite = self
                    .selected_objects
                    .iter()
                    .find(|item| {
                        let item_x_screen = item.x_screen_plus_8 as u16 as i16 - 8;
                        inclusive_ranges_overlap(x_range, (item_x_screen, item_x_screen + 7))
                    })
                    .map(|i| i.clone());

                self.state = FetcherState::GetTileDataLowDelay
            }

            FetcherState::GetTileDataLowDelay => {
                event!(Level::DEBUG, "OBJ fetcher awaiting tile low data");
                self.state = FetcherState::GetTileDataLow
            }

            FetcherState::GetTileDataLow => {
                event!(Level::DEBUG, "OBJ fetcher getting tile low data");
                let ly = ppu.read_ly();
                match self.sprite.clone() {
                    Some(sprite) => Fetcher::read_tile_row(
                        &ppu.vram,
                        // Objects always use $8000 addressing
                        &TileAddressingMode::UnsignedFrom0x8000,
                        ly.0,
                        ppu.scy.0,
                        sprite.tile_index,
                        sprite.flip_x(),
                        sprite.flip_y(),
                        false,
                        &mut self.tile_row_data,
                    ),
                    None => {
                        self.tile_row_data = [0; 8];
                    }
                }
                self.state = FetcherState::GetTileDataHighDelay
            }

            FetcherState::GetTileDataHighDelay => {
                event!(Level::DEBUG, "OBJ fetcher awaiting tile high data");
                self.state = FetcherState::GetTileDataHigh
            }

            FetcherState::GetTileDataHigh => {
                event!(Level::DEBUG, "OBJ fetcher getting tile high data");
                let ly = ppu.read_ly();
                match self.sprite.clone() {
                    Some(sprite) => Fetcher::read_tile_row(
                        &ppu.vram,
                        &TileAddressingMode::UnsignedFrom0x8000,
                        ly.0,
                        ppu.scy.0,
                        sprite.tile_index,
                        sprite.flip_x(),
                        sprite.flip_y(),
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
                event!(
                    Level::DEBUG,
                    "OBJ fetcher pushing pixels over {obj_fifo_len} pixels"
                );
                // Object FIFO pixels are merged with existing object FIFO pixels:
                // Those with ID 0 are overwritten by latter ones, otherwise the existing one wins
                for i in 0..8 {
                    let color = self.tile_row_data[i];
                    if i < obj_fifo_len {
                        // Pixel merging following OBJ-to-OBJ priority
                        let old_item = self.fifo[i].clone();
                        if old_item.color == 0 {
                            self.fifo[i] = ObjectFIFOItem {
                                bg_over_obj: self
                                    .sprite
                                    .as_ref()
                                    .map_or(false, |s| s.bg_over_obj()),
                                color,
                                palette: palette_for_sprite(self.sprite.as_ref()),
                            };
                        }
                    } else {
                        let item = ObjectFIFOItem {
                            bg_over_obj: self.sprite.as_ref().map_or(false, |s| s.bg_over_obj()),
                            color,
                            palette: palette_for_sprite(self.sprite.as_ref()),
                        };
                        event!(Level::TRACE, "Pushing OBJ {:#?}", item);
                        // No pixel to merge with, just push
                        self.fifo.push_back(item);
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
