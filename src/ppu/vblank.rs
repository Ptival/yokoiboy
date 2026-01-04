use tracing::{event, Level};

use crate::{
    cpu::interrupts::Interrupts,
    pixel_fetcher::{background_or_window::BackgroundOrWindowFetcher, object::ObjectFetcher},
    ppu::{DOTS_PER_SCANLINE, PPU},
};

pub fn vblank(
    ppu: &mut PPU,
    bgw_fetcher: &mut BackgroundOrWindowFetcher,
    obj_fetcher: &mut ObjectFetcher,
    interrupts: &mut Interrupts,
    t_cycle_count: u64,
) {
    event!(
        Level::TRACE,
        "VBlank, scanline: {scanline}, LY: {ly}",
        scanline = ppu.scanline_dots,
        ly = ppu.read_ly()
    );
    if ppu.scanline_dots == DOTS_PER_SCANLINE {
        ppu.scanline_dots = 0;
        event!(Level::DEBUG, "Incrementing LY for VBlank");
        ppu.increment_ly(interrupts, t_cycle_count);
        if ppu.read_ly().0 == 153 {
            ppu.prepare_for_new_frame(bgw_fetcher, obj_fetcher);
            ppu.switch_to_oam_scan(bgw_fetcher, obj_fetcher)
        }
    }
}
