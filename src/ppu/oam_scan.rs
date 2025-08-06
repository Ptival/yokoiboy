use std::collections::VecDeque;

use tracing::{event, Level};

use crate::{
    pixel_fetcher::{
        object::{ObjectFetcher, Sprite},
        Fetcher,
    },
    ppu::{LCDC_OBJECT_SIZE_BIT, LCD_VERTICAL_PIXEL_COUNT, PPU},
    utils::is_bit_set,
};

pub fn oam_scan(ppu: &mut PPU, obj_fetcher: &mut ObjectFetcher, pixel_fetcher: &mut Fetcher) {
    event!(
        Level::TRACE,
        "OAM scanning, LY={ly}, dot {dots}/80",
        ly = ppu.read_ly(),
        dots = ppu.scanline_dots
    );
    if ppu.scanline_dots == 80 {
        let ly = ppu.read_ly().0 as usize;

        // At the start of each scanline, remember SCX
        if ly < LCD_VERTICAL_PIXEL_COUNT {
            ppu.debug.frame_scxs[ly] = ppu.scx.0;
        }

        let mut selected_objects = VecDeque::new();
        let object_size = if is_bit_set(&ppu.read_lcdc(), LCDC_OBJECT_SIZE_BIT) {
            16
        } else {
            8
        };

        for object_offset in (0x00..0x9F).step_by(4) {
            if selected_objects.len() == 10 {
                break;
            }
            let y_screen_plus_16 = ppu.object_attribute_memory[object_offset];
            let object_min_y_on_screen = (y_screen_plus_16 as u16 as i16) - 16;
            let object_max_y_on_screen = object_min_y_on_screen + object_size - 1;
            if object_min_y_on_screen <= ly as i16 && ly as i16 <= object_max_y_on_screen {
                let mut tile_index = ppu.object_attribute_memory[object_offset + 2];

                // For tall sprites, we need to adjust the tile index to grab either half
                if is_bit_set(&ppu.read_lcdc(), LCDC_OBJECT_SIZE_BIT) {
                    if object_min_y_on_screen <= ly as i16
                        && ly as i16 <= object_max_y_on_screen - 8
                    {
                        // Tile index is LY is intersecting the top half of a tall sprite
                        tile_index &= 0xFE;
                    } else {
                        // Tile index is LY is intersecting the bottom half of a tall sprite
                        tile_index |= 0x01;
                    }
                }

                selected_objects.push_back(Sprite {
                    x_screen_plus_8: ppu.object_attribute_memory[object_offset + 1],
                    y_screen_plus_16,
                    tile_index,
                    attributes: ppu.object_attribute_memory[object_offset + 3],
                });
            }
        }

        obj_fetcher.selected_objects = selected_objects;
        ppu.switch_to_drawing_pixels(pixel_fetcher);
    }
}
