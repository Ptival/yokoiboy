mod debugger;

use iced::advanced::image;
use iced::border::Radius;
use iced::widget::image::FilterMethod;
use iced::widget::{column, container, row, Column};
use iced::{widget, Border, Color, Length};

use crate::application_state::ApplicationState;
use crate::message::Message;
use crate::ppu::{TILE_PALETTE_HORIZONTAL_PIXELS, TILE_PALETTE_VERTICAL_PIXELS};

impl ApplicationState {
    pub fn view(app: &ApplicationState) -> Column<Message> {
        let machine = app.current_machine_immut();
        let debugger_view = debugger::view(app);

        // let cycle_row =
        //     widget::Row::new().push(widget::text(format!("Cycles: {}", machine.t_cycle_count)));

        let debugger = widget::Container::new(debugger_view)
            .width(300)
            .height(432)
            .style(|_theme| {
                container::Style::default().border(Border {
                    color: Color::BLACK,
                    width: 2.0,
                    radius: Radius::default(),
                })
            });

        let lcd: widget::Container<'_, Message> = widget::Container::new(
            widget::Image::new(image::Handle::from_rgba(
                160,
                144,
                image::Bytes::copy_from_slice(&machine.ppu().lcd_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(480)
            .height(432),
        )
        .width(480)
        .height(432);

        let tile_palette_zoom_factor = 2;
        let wanted_width = (TILE_PALETTE_HORIZONTAL_PIXELS * tile_palette_zoom_factor) as f32;
        let wanted_height = (TILE_PALETTE_VERTICAL_PIXELS * tile_palette_zoom_factor) as f32;
        let tile_palette: widget::Container<'_, Message> = widget::Container::new(
            widget::Image::new(image::Handle::from_rgba(
                TILE_PALETTE_HORIZONTAL_PIXELS as u32,
                TILE_PALETTE_VERTICAL_PIXELS as u32,
                image::Bytes::copy_from_slice(&machine.ppu().tile_palette_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(Length::Fixed(wanted_width))
            .height(Length::Fixed(wanted_height)),
        )
        .width(wanted_width)
        .height(wanted_height);

        let tile_map0: widget::Container<'_, Message> = widget::Container::new(
            widget::Image::new(image::Handle::from_rgba(
                256,
                256,
                image::Bytes::copy_from_slice(&machine.ppu().tile_map0_pixels),
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
                image::Bytes::copy_from_slice(&machine.ppu().tile_map1_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(512)
            .height(512),
        )
        .width(512)
        .height(512);

        column![
            row![debugger, lcd, tile_palette],
            row![tile_map0, tile_map1]
        ]
    }
}
