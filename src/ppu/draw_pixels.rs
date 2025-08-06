use tracing::{event, Level};

use crate::{
    pixel_fetcher::{
        background_or_window::BackgroundOrWindowFetcher,
        object::{ObjectFetcher, ObjectPalette},
        Fetcher, FetchingFor,
    },
    ppu::{
        pixel_code_to_rgba, pixel_coordinates_in_rgba_slice, LCDPixelMetadata, PPUState,
        LCD_HORIZONTAL_PIXEL_COUNT, LCD_VERTICAL_PIXEL_COUNT, PPU,
    },
};

pub fn draw_pixels(
    ppu: &mut PPU,
    bgw_fetcher: &mut BackgroundOrWindowFetcher,
    obj_fetcher: &mut ObjectFetcher,
    pixel_fetcher: &mut Fetcher,
    dropped_pixels: u8,
) {
    if ppu.read_lcdc().0 == 0 {
        panic!("TODO: cancel object fetching")
    }

    let bgw_fifo_len = bgw_fetcher.fifo.len();
    let obj_fifo_len = obj_fetcher.fifo.len();

    event!(
        Level::TRACE,
        "Drawing pixels, drawn: {drawn}/{LCD_HORIZONTAL_PIXEL_COUNT}, dropped: {dropped}/{to_be_dropped}, LY: {ly}, BGW FIFO: {bgw_fifo_len} items, OBJ FIFO: {obj_fifo_len} items",
        drawn = ppu.drawn_pixels_on_current_row,
        dropped = dropped_pixels,
        to_be_dropped = ppu.scx.0 % 8,
        ly = ppu.read_ly(),
    );

    if ppu.drawn_pixels_on_current_row as usize == LCD_HORIZONTAL_PIXEL_COUNT {
        return;
    }

    let fetcher_state = &pixel_fetcher.fetching_for;
    if obj_fifo_len == 0 && bgw_fifo_len != 0 {
        if *fetcher_state == FetchingFor::BackgroundOrWindowFIFO {
            pixel_fetcher.switch_to_object_fifo();
        }
    } else {
        if *fetcher_state == FetchingFor::ObjectFIFO {
            pixel_fetcher.switch_to_background_or_window_fifo();
        }
    }
    pixel_fetcher.tick(bgw_fetcher, obj_fetcher, ppu);

    // Pixel mixing only happens when both FIFO are non-empty
    if !bgw_fetcher.fifo.is_empty() && !obj_fetcher.fifo.is_empty() {
        // To support fine scrolling, the first (scx % 8) pixels are dropped from FIFOs
        if dropped_pixels < ppu.scx.0 % 8 {
            bgw_fetcher.fifo.pop_front();
            obj_fetcher.fifo.pop_front();
            ppu.state = PPUState::DrawingPixels(dropped_pixels + 1);
            return;
        }

        // During scanline 0, remember SCY for every pixel pushed.  This is only for printing out
        // the nice VRAM frame, not something the console cares about.
        let ly = ppu.read_ly().0;
        if ly == 0 {
            ppu.debug.frame_scys_at_scanline_0[ppu.drawn_pixels_on_current_row as usize] =
                ppu.scy.0;
        }

        let bgw_pixel = bgw_fetcher.fifo.pop_front().unwrap();
        let obj_pixel = obj_fetcher.fifo.pop_front().unwrap();
        let pixel_x = ppu.drawn_pixels_on_current_row;
        let pixel_y = ly;

        // Simulate pixel mixing
        let choose_bgw = // We choose the background pixel if either:
                        // the object pixel is transparent
                        obj_pixel.color == 0
                        // or the object should be behind a non-transparent background
                        || (obj_pixel.bg_over_obj && bgw_pixel.color != 0);
        let (selected_pixel, palette) = if choose_bgw {
            if ppu.is_background_and_window_enabled() {
                (bgw_pixel.color, ppu.background_palette_data)
            } else {
                (0, 0)
            }
        } else {
            (
                obj_pixel.color,
                match obj_pixel.palette {
                    ObjectPalette::ObjectPalette0 => ppu.object_palette_0,
                    ObjectPalette::ObjectPalette1 => ppu.object_palette_1,
                },
            )
        };
        let rgba = pixel_code_to_rgba(selected_pixel, palette);

        if ppu.read_ly().0 as usize >= LCD_VERTICAL_PIXEL_COUNT {
            event!(
                Level::WARN,
                "Skipping writing pixels as they are out-of-bounds in LCD"
            );
        } else {
            let from = pixel_coordinates_in_rgba_slice(pixel_x, pixel_y);
            ppu.lcd.pixels[from..from + 4].copy_from_slice(&rgba);
            for i in from..from + 4 {
                ppu.lcd_pixels_meta[i] = LCDPixelMetadata {
                    bgw_pixel,
                    obj_pixel,
                    choose_bgw,
                };
            }
        }
        ppu.drawn_pixels_on_current_row += 1;

        if ppu.drawn_pixels_on_current_row as usize == LCD_HORIZONTAL_PIXEL_COUNT {
            ppu.switch_to_horizontal_blank()
        }
    }
}
