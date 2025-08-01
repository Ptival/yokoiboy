use tracing::{event, Level};

use crate::{
    cpu::interrupts::Interrupts,
    pixel_fetcher::{background_or_window::BackgroundOrWindowFetcher, object::ObjectFetcher},
    ppu::{DOTS_PER_SCANLINE, LCD_VERTICAL_PIXEL_COUNT, PPU},
};

pub fn hblank(
    ppu: &mut PPU,
    bgw_fetcher: &mut BackgroundOrWindowFetcher,
    obj_fetcher: &mut ObjectFetcher,
    interrupts: &mut Interrupts,
) {
    event!(
        Level::TRACE,
        "HBlank, scanline: {scanline}, LY: {ly}",
        scanline = ppu.scanline_dots,
        ly = ppu.read_ly()
    );
    if ppu.scanline_dots == DOTS_PER_SCANLINE {
        ppu.scanline_dots = 0;
        ppu.increment_ly(interrupts);
        if ppu.read_ly().0 as usize == LCD_VERTICAL_PIXEL_COUNT {
            ppu.switch_to_vertical_blank(interrupts)
        } else {
            ppu.switch_to_oam_scan(bgw_fetcher, obj_fetcher)
        }
    }
}
