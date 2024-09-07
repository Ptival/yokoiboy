use std::{collections::VecDeque, num::Wrapping};

use crate::{machine::Machine, utils};

use super::{LCDC_BACKGROUND_TILE_MAP_AREA_BIT, PPU};

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

#[derive(Clone, Debug)]
pub struct Fetcher {
    pub fifo: VecDeque<FIFOItem>,
    state: FetcherState,
    pub row_of_pixel_within_tile: u8,
    tile_id: u8,
    pub vram_tile_column: u8,
    tile_row_data: [u8; 8],
}

// Background and Window use one of these based on bit 4 of lcd_control.
// Sprites always use UnsignedFrom0x8000.
pub enum TileAddressingMode {
    UnsignedFrom0x8000,
    SignedFrom0x9000,
}

fn tile_index_in_palette(tile_id: u8, addressing_mode: TileAddressingMode) -> u16 {
    match addressing_mode {
        TileAddressingMode::UnsignedFrom0x8000 => tile_id as u16,
        TileAddressingMode::SignedFrom0x9000 => (256 + (tile_id as i8) as i16) as u16,
    }
}

impl Fetcher {
    pub fn new() -> Self {
        Fetcher {
            fifo: VecDeque::new(),
            state: FetcherState::GetTileDelay,
            row_of_pixel_within_tile: 0,
            tile_id: 0,
            vram_tile_column: 0,
            tile_row_data: [0; 8],
        }
    }

    pub fn reset(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.vram_tile_column = 0;
        self.fifo.clear();
    }

    fn read_tile_row(machine: &mut Machine, bit_plane: bool) {
        // WARNING: when handling sprites, will need to update this to ignore addressing mode for
        // their tiles

        // NOTE: rather than going through the MMU again with an absolute address, I'm computing the
        // address relative to VRAM and reading directly from the VRAM slice.  Should be slightly
        // faster as you don't need to perform range checks to realize you're heading into VRAM.
        let tile_index_in_palette = tile_index_in_palette(
            machine.fetcher().tile_id,
            machine.ppu().get_addressing_mode(),
        );
        let address_in_vram_slice =
            tile_index_in_palette * 16 + (machine.fetcher().row_of_pixel_within_tile as u16) * 2;
        let pixel_data = machine.ppu().vram[address_in_vram_slice as usize + bit_plane as usize];
        // We just finished reading one byte.  Each bit is half of a pixel value, we coalesce them
        // here Note: This assumes that `tile_row_data` is cleared at each loop.
        let fetcher = machine.fetcher_mut();
        // Note: it's nice to have the row data be sorted by increasing X, but the lowest bit
        // position is the highest X pixel, so using (7 - bit_position) to reorder.
        for bit_position in 0..8 {
            fetcher.tile_row_data[7 - bit_position] |=
                ((pixel_data >> bit_position) & 1) << (bit_plane as u8);
        }
    }

    pub fn step_one_dot(machine: &mut Machine) -> &mut Machine {
        match machine.fetcher().state {
            FetcherState::GetTileDelay => machine.fetcher_mut().state = FetcherState::GetTile,

            FetcherState::GetTile => {
                let vram_tile_row = (PPU::read_ly(machine) + machine.ppu().scy).0 & 255;
                machine.ppu_mut().fetcher.row_of_pixel_within_tile = vram_tile_row % 8;

                // FIXME: more complex rules for the row base address
                let row_base_address = if utils::is_bit_set(
                    &machine.ppu().lcd_control,
                    LCDC_BACKGROUND_TILE_MAP_AREA_BIT,
                ) {
                    0x9C00
                } else {
                    0x9800
                };

                let row_address = row_base_address + ((vram_tile_row as u16 / 8) * 32);

                machine.fetcher_mut().tile_id = machine
                    .read_u8(Wrapping(
                        row_address + (machine.fetcher().vram_tile_column as u16),
                    ))
                    .0;

                machine.fetcher_mut().state = FetcherState::GetTileDataLowDelay
            }

            FetcherState::GetTileDataLowDelay => {
                machine.fetcher_mut().state = FetcherState::GetTileDataLow
            }

            FetcherState::GetTileDataLow => {
                Self::read_tile_row(machine, false);
                machine.fetcher_mut().state = FetcherState::GetTileDataHighDelay
            }

            FetcherState::GetTileDataHighDelay => {
                machine.fetcher_mut().state = FetcherState::GetTileDataHigh
            }

            FetcherState::GetTileDataHigh => {
                Self::read_tile_row(machine, true);
                machine.fetcher_mut().state = FetcherState::PushRow
            }

            FetcherState::PushRow => {
                // Only supporting background tiles at the moment, and those only get pushed on an
                // empty FIFO
                if machine.fetcher().fifo.len() == 0 {
                    for i in 0..8 {
                        let color = machine.fetcher().tile_row_data[i];
                        machine.fetcher_mut().fifo.push_back(FIFOItem { color });
                    }
                    machine.fetcher_mut().vram_tile_column += 1;
                    // clean up so that GetTileData can assume 0
                    machine.fetcher_mut().tile_row_data = [0; 8];
                    machine.fetcher_mut().state = FetcherState::GetTileDelay
                }
            }
        }
        machine
    }
}

impl Machine {
    pub fn fetcher(&self) -> &Fetcher {
        &self.ppu().fetcher
    }

    pub fn fetcher_mut(&mut self) -> &mut Fetcher {
        &mut self.ppu_mut().fetcher
    }
}
