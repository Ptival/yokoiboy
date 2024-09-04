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
    pub tile_line: Wrapping<u8>,
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

impl Fetcher {
    pub fn new() -> Self {
        Fetcher {
            fifo: VecDeque::new(),
            state: FetcherState::GetTileDelay,
            row_address: 0,
            tile_line: Wrapping(0),
            tile_id: Wrapping(0),
            tile_index: Wrapping(0),
            tile_row_data: [0; 8],
        }
    }

    fn read_tile_line(machine: &mut Machine, bit_plane: bool) {
        // WARNING: when handling sprites, will need to update this to ignore addressing mode for
        // their tiles
        let offset = match machine.ppu().get_addressing_mode() {
            TileAddressingMode::UnsignedFrom0x8000 => {
                0x8000 + (machine.fetcher().tile_id.0 as u16) * 16
            }
            TileAddressingMode::SignedFrom0x9000 => {
                // as i8 interprets the correct bit sign
                // i32 allows signed arithmetic without accidentally identifying a sign bit
                // final result interpreted as a u16
                (0x9000 + ((machine.fetcher().tile_id.0 as i8) as i32) * 16) as u16
            }
        };
        let address = offset + (machine.fetcher().tile_line.0 as u16) * 2;
        let pixel_data = machine.read_u8(Wrapping(address + bit_plane as u16)).0;
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
                Self::read_tile_line(machine, false);
                machine.fetcher_mut().state = FetcherState::GetTileDataHighDelay
            }

            FetcherState::GetTileDataHighDelay => {
                machine.fetcher_mut().state = FetcherState::GetTileDataHigh
            }

            FetcherState::GetTileDataHigh => {
                Self::read_tile_line(machine, true);
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
