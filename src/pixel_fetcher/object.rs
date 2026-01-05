use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    collections::VecDeque,
    fmt,
    num::Wrapping,
};
use tracing::{event, Level};

use super::{Fetcher, TileAddressingMode};

use crate::{
    pixel_fetcher::{FlipX, FlipY},
    ppu::PPU,
    utils::is_bit_set,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FetcherNonIdleState {
    GetTileDelay,
    GetTile,
    GetTileDataLowDelay,
    GetTileDataLow,
    GetTileDataHighDelay,
    GetTileDataHigh,
    PushRow,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FetcherState {
    Idle,
    NonIdle(FetcherNonIdleState, Sprite),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

    /// This is the offset of the object in the OAM.  This helps with "Drawing priority", as when
    /// two objects have been selected that have the same X coordinate, the pixel should come from
    /// the object lowest in OAM.
    pub oam_offset: usize,
}

impl fmt::Display for Sprite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Sprite(x={:3},y={:3}, OAM#{})",
            self.x_screen_plus_8 - 8,
            self.y_screen_plus_16 - 16,
            self.oam_offset
        )
    }
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

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ObjectPalette {
    ObjectPalette0,
    ObjectPalette1,
}

impl Default for ObjectPalette {
    fn default() -> Self {
        ObjectPalette::ObjectPalette0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObjectFetcher {
    pub state: FetcherState,
    pub fifo: VecDeque<ObjectFIFOItem>,
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
            state: FetcherState::Idle,
            fifo: VecDeque::new(),
            tile_row_data: [0; 8],
            selected_objects: VecDeque::new(),
        }
    }

    pub fn is_idle(&self) -> bool {
        return self.state == FetcherState::Idle;
    }

    pub fn prepare_for_fetch(&mut self, sprite: Sprite) {
        self.state = FetcherState::NonIdle(FetcherNonIdleState::GetTileDelay, sprite);
        self.tile_row_data = [0; 8];
    }

    pub fn prepare_for_new_row(&mut self) {
        self.state = FetcherState::Idle;
        self.fifo.clear();
        self.tile_row_data = [0; 8];
    }

    pub fn prepare_for_new_frame(&mut self) {
        self.state = FetcherState::Idle;
        self.fifo.clear();
        self.tile_row_data = [0; 8];
    }

    // TODO: Technically, the OBJ fetcher might have to wait if the BGW fetcher is already accessing
    // VRAM
    pub fn tick(&mut self, ppu: &mut PPU) {
        match self.state {
            FetcherState::Idle => {
                panic!("The object fetcher was ticked while idle, please report.")
            }

            FetcherState::NonIdle(non_idle_state, sprite) => {
                match non_idle_state {
                    FetcherNonIdleState::GetTileDelay => {
                        event!(Level::TRACE, "OBJ fetcher awaiting tile");
                        self.state = FetcherState::NonIdle(FetcherNonIdleState::GetTile, sprite)
                    }

                    FetcherNonIdleState::GetTile => {
                        event!(Level::TRACE, "OBJ fetcher selected sprite");
                        self.state =
                            FetcherState::NonIdle(FetcherNonIdleState::GetTileDataLowDelay, sprite)
                    }

                    FetcherNonIdleState::GetTileDataLowDelay => {
                        event!(Level::TRACE, "OBJ fetcher awaiting tile low data");
                        self.state =
                            FetcherState::NonIdle(FetcherNonIdleState::GetTileDataLow, sprite)
                    }

                    FetcherNonIdleState::GetTileDataLow => {
                        event!(Level::TRACE, "OBJ fetcher getting tile low data");
                        let ly = ppu.read_ly();
                        Fetcher::read_tile_row(
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
                        );
                        self.state =
                            FetcherState::NonIdle(FetcherNonIdleState::GetTileDataHighDelay, sprite)
                    }

                    FetcherNonIdleState::GetTileDataHighDelay => {
                        event!(Level::TRACE, "OBJ fetcher awaiting tile high data");
                        self.state =
                            FetcherState::NonIdle(FetcherNonIdleState::GetTileDataHigh, sprite)
                    }

                    FetcherNonIdleState::GetTileDataHigh => {
                        event!(Level::TRACE, "OBJ fetcher getting tile high data");
                        let ly = ppu.read_ly();
                        Fetcher::read_tile_row(
                            &ppu.vram,
                            &TileAddressingMode::UnsignedFrom0x8000,
                            ly.0,
                            ppu.scy.0,
                            sprite.tile_index,
                            sprite.flip_x(),
                            sprite.flip_y(),
                            true,
                            &mut self.tile_row_data,
                        );
                        self.state = FetcherState::NonIdle(FetcherNonIdleState::PushRow, sprite)
                    }

                    FetcherNonIdleState::PushRow => {
                        let obj_fifo_len = self.fifo.len();
                        event!(
                            Level::TRACE,
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
                                        bg_over_obj: sprite.bg_over_obj(),
                                        color,
                                        palette: palette_for_sprite(&sprite),
                                    };
                                }
                            } else {
                                let item = ObjectFIFOItem {
                                    bg_over_obj: sprite.bg_over_obj(),
                                    color,
                                    palette: palette_for_sprite(&sprite),
                                };
                                event!(Level::TRACE, "Pushing OBJ {:#?}", item);
                                // No pixel to merge with, just push
                                self.fifo.push_back(item);
                            }
                        }
                        self.state = FetcherState::Idle
                    }
                }
            }
        }
    }
}

fn palette_for_sprite(sprite: &Sprite) -> ObjectPalette {
    match (sprite.attributes >> 4) & 1 {
        0b0 => ObjectPalette::ObjectPalette0,
        0b1 => ObjectPalette::ObjectPalette1,
        _ => unreachable!(),
    }
}
