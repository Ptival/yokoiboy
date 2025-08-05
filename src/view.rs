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

impl ApplicationState {
    pub fn view(app: &ApplicationState) -> Element<Message> {
        let machine = app.current_machine();
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

        row![
            column![
                row![
                    column![
                        debugger,
                        pixel_debugger::view(app.lcd_pixel_under_mouse),
                        tile_debugger::view(app.tile_id_under_mouse)
                    ],
                    lcd::LCD::new(&machine.ppu.lcd_pixels),
                ],
                row![tile_map0, tile_map1],
            ],
            column![tile_palette::TilePalette::new(
                &machine.ppu.tile_palette_pixels
            ),]
        ]
        .into()
    }
}
