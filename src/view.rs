mod debugger;
mod lcd;
mod pixel_debugger;
mod tile_debugger;
mod tile_palette;

use iced::advanced::image;
use iced::border::Radius;
use iced::widget::image::FilterMethod;
use iced::widget::{column, container, row};
use iced::{widget, Border, Color, Element};

use crate::application_state::ApplicationState;
use crate::message::Message;
use crate::ppu::{
    HORIZONTAL_PIXELS_PER_TILE, LCD_HORIZONTAL_PIXEL_COUNT, PIXEL_DATA_SIZE,
    TILE_MAP_HORIZONTAL_PIXELS, TILE_PALETTE_HORIZONTAL_PIXELS,
    TILE_PALETTE_HORIZONTAL_TILE_COUNT, VERTICAL_PIXELS_PER_TILE,
};

const RED: [u8; 4] = [255, 0, 0, 255];

impl ApplicationState {
    pub fn view(app: &ApplicationState) -> Element<'_, Message> {
        let machine = app.current_machine();
        let debugger_view = debugger::view(app);

        let debugger = widget::Container::new(debugger_view)
            .width(300)
            .height(500)
            .style(|_theme| {
                container::Style::default().border(Border {
                    color: Color::BLACK,
                    width: 2.0,
                    radius: Radius::default(),
                })
            });

        // Look up metadata for the hovered pixel on the next-frame LCD
        let (hover_x, hover_y) = app.lcd_pixel_under_mouse;
        let meta_index = hover_y as usize * LCD_HORIZONTAL_PIXEL_COUNT + hover_x as usize;
        let meta = &machine.ppu.lcd_pixels_meta[meta_index];

        // Highlight the background tile pixel in tile_map0
        let mut tile_map0_pixels = machine.ppu().tile_map0.pixels.to_vec();
        let bgw = &meta.bgw_pixel;
        let bg_pixel_x = bgw.tile_map_x as usize * HORIZONTAL_PIXELS_PER_TILE
            + bgw.pixel_in_tile_x as usize;
        let bg_pixel_y = bgw.tile_map_y as usize * VERTICAL_PIXELS_PER_TILE
            + bgw.pixel_in_tile_y as usize;
        let bg_offset =
            (bg_pixel_y * TILE_MAP_HORIZONTAL_PIXELS + bg_pixel_x) * PIXEL_DATA_SIZE;
        tile_map0_pixels[bg_offset..bg_offset + 4].copy_from_slice(&RED);

        // Highlight the object tile pixel in the tile palette
        let mut tile_palette_pixels = machine.ppu().tile_palette.pixels.to_vec();
        if let Some(sprite) = meta.obj_pixel.sprite {
            let tile_id = sprite.tile_index as usize;
            let palette_tile_x = tile_id % TILE_PALETTE_HORIZONTAL_TILE_COUNT;
            let palette_tile_y = tile_id / TILE_PALETTE_HORIZONTAL_TILE_COUNT;
            let obj_pixel_x = palette_tile_x * HORIZONTAL_PIXELS_PER_TILE
                + meta.obj_pixel.pixel_in_sprite_x as usize;
            let obj_pixel_y = palette_tile_y * VERTICAL_PIXELS_PER_TILE
                + meta.obj_pixel.pixel_in_sprite_y as usize;
            let obj_offset = (obj_pixel_y * TILE_PALETTE_HORIZONTAL_PIXELS + obj_pixel_x)
                * PIXEL_DATA_SIZE;
            tile_palette_pixels[obj_offset..obj_offset + 4].copy_from_slice(&RED);
        }

        let tile_map0: widget::Container<'_, Message> = widget::Container::new(
            widget::Image::new(image::Handle::from_rgba(
                256,
                256,
                bytes::Bytes::copy_from_slice(&tile_map0_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(512)
            .height(512),
        )
        .width(512)
        .height(512);

        let tile_map1: widget::Container<'_, Message> = widget::Container::new(
            widget::Image::new(image::Handle::from_rgba(
                256,
                256,
                bytes::Bytes::copy_from_slice(&machine.ppu().tile_map1.pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(512)
            .height(512),
        )
        .width(512)
        .height(512);

        row![
            column![
                row![
                    column![
                        debugger,
                        pixel_debugger::view(app.lcd_pixel_under_mouse),
                        tile_debugger::view(app.tile_id_under_mouse)
                    ],
                    lcd::LCD::new(&machine.ppu.lcd_last_finished_frame.pixels),
                    lcd::LCD::new(&machine.ppu.lcd_for_next_frame.pixels),
                ],
                row![tile_map0, tile_map1],
            ],
            column![tile_palette::TilePalette::new_with_pixels(&tile_palette_pixels),]
        ]
        .into()
    }
}
