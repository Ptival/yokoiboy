use iced::widget;
use iced_aw::{grid_row, Grid};

use crate::{machine::Machine, message::Message};

pub fn view(machine: &Machine) -> Grid<Message> {
    let mut lcdc_grid_right = Grid::new();
    lcdc_grid_right = lcdc_grid_right.push(grid_row![
        widget::text("7"),
        widget::text("6"),
        widget::text("5"),
        widget::text("4"),
        widget::text("3"),
        widget::text("2"),
        widget::text("1"),
        widget::text("0"),
    ]);
    let lcdc = machine.ppu.read_lcdc().0;
    lcdc_grid_right = lcdc_grid_right.push(grid_row![
        widget::text(format!("{}", (lcdc & (1 << 7)) >> 7)),
        widget::text(format!("{}", (lcdc & (1 << 6)) >> 6)),
        widget::text(format!("{}", (lcdc & (1 << 5)) >> 5)),
        widget::text(format!("{}", (lcdc & (1 << 4)) >> 4)),
        widget::text(format!("{}", (lcdc & (1 << 3)) >> 3)),
        widget::text(format!("{}", (lcdc & (1 << 2)) >> 2)),
        widget::text(format!("{}", (lcdc & (1 << 1)) >> 1)),
        widget::text(format!("{}", (lcdc & (1 << 0)) >> 0)),
    ]);

    let mut lcdc_grid = Grid::new();
    lcdc_grid = lcdc_grid.push(grid_row![widget::text("LCDC"), lcdc_grid_right]);

    lcdc_grid
}
