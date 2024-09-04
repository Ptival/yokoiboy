use std::{collections::VecDeque, num::Wrapping};

use crate::machine::Machine;

#[derive(Clone, Debug)]
enum FetcherState {
    GetTileDelay,
    GetTile,
    GetTileDataLowDelay,
    GetTileDataLow,
    GetTileDataHighDelay,
    GetTileDataHigh,
    Sleep1,
    Sleep2,
    PushRow,
}

#[derive(Clone, Debug)]
pub struct FIFOItem {
    pub color: u8,
    _palette: u8,
}

#[derive(Clone, Debug)]
pub struct Fetcher {
    pub fifo: VecDeque<FIFOItem>,
    state: FetcherState,
    pub row_address: u16,
    pub tile_row: Wrapping<u8>,
    tile_id: Wrapping<u8>,
    pub tile_index: Wrapping<u8>,
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
            row_address: 0,
            tile_row: Wrapping(0),
            tile_id: Wrapping(0),
            tile_index: Wrapping(0),
            tile_row_data: [0; 8],
        }
    }

    fn read_tile_row(machine: &mut Machine, bit_plane: bool) {
        // WARNING: when handling sprites, will need to update this to ignore addressing mode for
        // their tiles

        // NOTE: rather than going through the MMU again, I'm computing the address relative to VRAM
        // and reading directly from the VRAM slice.
        let tile_index_in_palette = tile_index_in_palette(
            machine.fetcher().tile_id.0,
            machine.ppu().get_addressing_mode(),
        );
        let address_in_vram_slice =
            tile_index_in_palette * 16 + (machine.fetcher().tile_row.0 as u16) * 2;
        let pixel_data = machine.ppu().vram[address_in_vram_slice as usize + bit_plane as usize];
        // We just finished reading one byte.  Each bit is half of a pixel value, we coalesce them
        // here Note: This assumes that `tile_row_data` is cleared at each loop.
        for bit_position in 0..8 {
            machine.fetcher_mut().tile_row_data[bit_position] |=
                ((pixel_data >> bit_position) & 1) << (bit_plane as u8);
        }
    }

    pub fn step_one_dot(machine: &mut Machine) -> &mut Machine {
        match machine.fetcher().state {
            FetcherState::GetTileDelay => machine.fetcher_mut().state = FetcherState::GetTile,

            FetcherState::GetTile => {
                machine.fetcher_mut().tile_id = machine.read_u8(Wrapping(
                    machine.fetcher().row_address + (machine.fetcher().tile_index.0 as u16),
                ));
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
                machine.fetcher_mut().state = FetcherState::Sleep1
            }

            FetcherState::Sleep1 => machine.fetcher_mut().state = FetcherState::Sleep2,
            FetcherState::Sleep2 => machine.fetcher_mut().state = FetcherState::PushRow,

            FetcherState::PushRow => {
                // Only supporting background tiles at the moment, and those only get pushed on an
                // empty FIFO
                if machine.fetcher().fifo.len() == 0 {
                    for i in (0..8).rev() {
                        let color = machine.fetcher().tile_row_data[i];
                        machine
                            .fetcher_mut()
                            .fifo
                            .push_back(FIFOItem { color, _palette: 0 });
                    }
                    machine.fetcher_mut().tile_index += 1;
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
