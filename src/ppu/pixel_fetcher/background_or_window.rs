use std::{collections::VecDeque, num::Wrapping};

use crate::{machine::Machine, utils};

use super::{FIFOItem, Fetcher, FetcherState, LCDC_BACKGROUND_TILE_MAP_AREA_BIT, PPU};

#[derive(Clone, Debug)]
pub struct BackgroundOrWindowFetcher {
    state: FetcherState,
    pub fifo: VecDeque<FIFOItem>,
    pub row_of_pixel_within_tile: u8,
    tile_id: u8,
    pub vram_tile_column: u8,
    tile_row_data: [u8; 8],
}

impl BackgroundOrWindowFetcher {
    pub fn new() -> Self {
        BackgroundOrWindowFetcher {
            state: FetcherState::GetTileDelay,
            fifo: VecDeque::new(),
            row_of_pixel_within_tile: 0,
            tile_id: 0,
            vram_tile_column: 0,
            tile_row_data: [0; 8],
        }
    }

    pub fn reset(&mut self) {
        self.state = FetcherState::GetTileDelay;
        self.vram_tile_column = 0;
    }

    pub fn tick(machine: &mut Machine) -> &mut Machine {
        match machine.bgw_fetcher().state {
            FetcherState::GetTileDelay => machine.bgw_fetcher_mut().state = FetcherState::GetTile,

            FetcherState::GetTile => {
                let vram_tile_row = (PPU::read_ly(machine) + machine.ppu().scy).0 & 255;
                // machine.ppu_mut().fetcher.row_of_pixel_within_tile = vram_tile_row % 8;

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

                machine.bgw_fetcher_mut().tile_id = machine
                    .read_u8(Wrapping(
                        row_address + (machine.bgw_fetcher().vram_tile_column as u16),
                    ))
                    .0;
                machine.bgw_fetcher_mut().state = FetcherState::GetTileDataLowDelay
            }

            FetcherState::GetTileDataLowDelay => {
                machine.bgw_fetcher_mut().state = FetcherState::GetTileDataLow
            }

            FetcherState::GetTileDataLow => {
                let ly = PPU::read_ly(machine);
                let ppu = machine.ppu_mut();
                Fetcher::read_tile_row(
                    &ppu.vram,
                    &ppu.get_addressing_mode(),
                    (ly + ppu.scy).0,
                    ppu.fetcher.background_window_fetcher.tile_id,
                    false,
                    &mut ppu.fetcher.background_window_fetcher.tile_row_data,
                );
                machine.bgw_fetcher_mut().state = FetcherState::GetTileDataHighDelay
            }

            FetcherState::GetTileDataHighDelay => {
                machine.bgw_fetcher_mut().state = FetcherState::GetTileDataHigh
            }

            FetcherState::GetTileDataHigh => {
                let ly = PPU::read_ly(machine);
                let ppu = machine.ppu_mut();
                Fetcher::read_tile_row(
                    &ppu.vram,
                    &ppu.get_addressing_mode(),
                    (ly + ppu.scy).0,
                    ppu.fetcher.background_window_fetcher.tile_id,
                    true,
                    &mut ppu.fetcher.background_window_fetcher.tile_row_data,
                );
                machine.bgw_fetcher_mut().state = FetcherState::PushRow
            }

            FetcherState::PushRow => {
                // Background/Window FIFO pixels only get pushed when the FIFO is empty
                if machine.bgw_fetcher().fifo.len() == 0 {
                    for i in 0..8 {
                        let color = machine.bgw_fetcher().tile_row_data[i];
                        machine.bgw_fetcher_mut().fifo.push_back(FIFOItem { color });
                    }
                    machine.bgw_fetcher_mut().vram_tile_column += 1;
                    // clean up so that GetTileData can assume 0
                    machine.bgw_fetcher_mut().tile_row_data = [0; 8];
                    machine.bgw_fetcher_mut().state = FetcherState::GetTileDelay
                }
            }
        }
        machine
    }
}

impl Machine {
    pub fn bgw_fetcher(&self) -> &BackgroundOrWindowFetcher {
        &self.fetcher().background_window_fetcher
    }

    pub fn bgw_fetcher_mut(&mut self) -> &mut BackgroundOrWindowFetcher {
        &mut self.fetcher_mut().background_window_fetcher
    }
}
