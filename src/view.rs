mod debugger;

use iced::advanced::image;
use iced::border::Radius;
use iced::widget::container;
use iced::widget::image::FilterMethod;
use iced::{alignment, widget, Border, Color};
use iced_aw::{grid_row, Grid};

use crate::application_state::ApplicationState;
use crate::message::Message;
use crate::ppu::{TILE_PALETTE_HORIZONTAL_PIXELS, TILE_PALETTE_VERTICAL_PIXELS};

impl ApplicationState {
    pub fn view(app: &ApplicationState) -> Grid<Message> {
        let machine = app.current_machine_immut();
        let debugger_view = debugger::view(app);

        // let cycle_row =
        //     widget::Row::new().push(widget::text(format!("Cycles: {}", machine.t_cycle_count)));

        let mut grid = Grid::new().vertical_alignment(alignment::Vertical::Bottom);

        let debugger = widget::Container::new(debugger_view)
            .width(450)
            .height(520)
            .style(|_theme| {
                container::Style::default().border(Border {
                    color: Color::BLACK,
                    width: 2.0,
                    radius: Radius::default(),
                })
            });

        let lcd = widget::Container::new(
            widget::Image::new(image::Handle::from_rgba(
                160,
                144,
                image::Bytes::copy_from_slice(&machine.ppu.lcd_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(480)
            .height(432),
        )
        .width(480)
        .height(432);

        let wanted_width = (TILE_PALETTE_HORIZONTAL_PIXELS * 2) as u16;
        let wanted_height = (TILE_PALETTE_VERTICAL_PIXELS * 2) as u16;
        let tile_palette = widget::Container::new(
            widget::Image::new(image::Handle::from_rgba(
                TILE_PALETTE_HORIZONTAL_PIXELS as u32,
                TILE_PALETTE_VERTICAL_PIXELS as u32,
                image::Bytes::copy_from_slice(&machine.ppu.tile_palette_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(wanted_width)
            .height(wanted_height),
        )
        .width(wanted_width)
        .height(wanted_height);

        let tile_map0 = widget::Container::new(
            widget::Image::new(image::Handle::from_rgba(
                256,
                256,
                image::Bytes::copy_from_slice(&machine.ppu.tile_map0_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(512)
            .height(512),
        )
        .width(512)
        .height(512);

        let tile_map1 = widget::Container::new(
            widget::Image::new(image::Handle::from_rgba(
                256,
                256,
                image::Bytes::copy_from_slice(&machine.ppu.tile_map1_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(512)
            .height(512),
        )
        .width(512)
        .height(512);

        grid = grid.push(grid_row![debugger, tile_palette, lcd]);
        grid = grid.push(grid_row![tile_map0, tile_map1]);
        grid.into()
    }
}
