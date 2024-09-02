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
    pub palette: u8,
}

#[derive(Clone, Debug)]
pub struct Fetcher {
    pub fifo: VecDeque<FIFOItem>,
    state: FetcherState,
    pub row_address: Wrapping<u16>,
    pub tile_line: Wrapping<u8>,
    tile_id: Wrapping<u8>,
    pub tile_index: Wrapping<u8>,
    tile_row_data: [u8; 8],
}

impl Fetcher {
    pub fn new() -> Self {
        Fetcher {
            fifo: VecDeque::new(),
            state: FetcherState::GetTileDelay,
            row_address: Wrapping(0x9800),
            tile_line: Wrapping(0),
            tile_id: Wrapping(0),
            tile_index: Wrapping(0),
            tile_row_data: [0; 8],
        }
    }

    fn read_tile_line(machine: &mut Machine, bit_plane: bool) {
        let offset =
            Wrapping(0x8000) + (Wrapping(machine.ppu.fetcher.tile_id.0 as u16) * Wrapping(16));
        let address = offset + (Wrapping(machine.ppu.fetcher.tile_line.0 as u16) * Wrapping(2));
        let pixel_data = machine.read_u8(address + Wrapping(bit_plane as u16)).0;
        // We just read one byte.  Each bit is half of a pixel value, we coalesce them here
        // Note: This assumes that `tile_row_data` is cleared at each loop.
        for bit_position in 0..8 {
            machine.ppu.fetcher.tile_row_data[bit_position] |=
                ((pixel_data >> bit_position) & 1) << (bit_plane as u8);
        }
    }

    pub fn step_one_dot(machine: &mut Machine) -> &mut Machine {
        match machine.ppu.fetcher.state {
            FetcherState::GetTileDelay => machine.ppu.fetcher.state = FetcherState::GetTile,

            FetcherState::GetTile => {
                machine.ppu.fetcher.tile_id = machine.read_u8(
                    machine.ppu.fetcher.row_address
                        + Wrapping(machine.ppu.fetcher.tile_index.0 as u16),
                );
                machine.ppu.fetcher.state = FetcherState::GetTileDataLowDelay
            }

            FetcherState::GetTileDataLowDelay => {
                machine.ppu.fetcher.state = FetcherState::GetTileDataLow
            }

            FetcherState::GetTileDataLow => {
                Self::read_tile_line(machine, false);
                machine.ppu.fetcher.state = FetcherState::GetTileDataHighDelay
            }

            FetcherState::GetTileDataHighDelay => {
                machine.ppu.fetcher.state = FetcherState::GetTileDataHigh
            }

            FetcherState::GetTileDataHigh => {
                Self::read_tile_line(machine, true);
                machine.ppu.fetcher.state = FetcherState::Sleep1
            }

            FetcherState::Sleep1 => machine.ppu.fetcher.state = FetcherState::Sleep2,
            FetcherState::Sleep2 => machine.ppu.fetcher.state = FetcherState::PushRow,

            FetcherState::PushRow => {
                // Only supporting background tiles at the moment, and those only get pushed on an
                // empty FIFO
                if machine.ppu.fetcher.fifo.len() == 0 {
                    for i in (0..8).rev() {
                        machine.ppu.fetcher.fifo.push_back(FIFOItem {
                            color: machine.ppu.fetcher.tile_row_data[i],
                            palette: 0,
                        });
                    }
                    machine.ppu.fetcher.tile_index += 1;
                    // clean up so that GetTileData can assume 0
                    machine.ppu.fetcher.tile_row_data = [0; 8];
                    machine.ppu.fetcher.state = FetcherState::GetTileDelay
                }
            }
        }
        machine
    }
}
