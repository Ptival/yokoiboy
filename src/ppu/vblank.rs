use tracing::{event, Level};

use crate::{
    pixel_fetcher::{background_or_window::BackgroundOrWindowFetcher, object::ObjectFetcher},
    ppu::{DOTS_PER_SCANLINE, PPU},
};

pub fn vblank(
    ppu: &mut PPU,
    bgw_fetcher: &mut BackgroundOrWindowFetcher,
    obj_fetcher: &mut ObjectFetcher,
) {
    event!(
        Level::TRACE,
        "VBlank, dots on this scanline: {scanline}, LY: {ly}",
        scanline = ppu.scanline_dots,
        ly = ppu.read_ly()
    );
    if ppu.scanline_dots == DOTS_PER_SCANLINE {
        ppu.scanline_dots = 0;
        ppu.increment_ly();
        event!(
            Level::TRACE,
            "Incremented LY to {} during VBlank (LYC = {})",
            ppu.read_ly(),
            ppu.lcd_y_compare
        );
        if ppu.read_ly().0 == 153 {
            ppu.prepare_for_new_frame(bgw_fetcher, obj_fetcher);
            ppu.switch_to_oam_scan(bgw_fetcher, obj_fetcher)
        }
    }
}
