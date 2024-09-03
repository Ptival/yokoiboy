use iced::widget;
use iced_aw::{grid_row, Grid};

use crate::{
    message::Message,
    registers::{Flag, Registers},
};

pub fn view(registers: &Registers) -> Grid<Message> {
    let mut registers_grid = Grid::new();

    registers_grid = registers_grid.push(grid_row![
        widget::text(" A"),
        widget::text(" F"),
        widget::text(""),
        widget::text(" B"),
        widget::text(" C"),
        widget::text(""),
        widget::text(" D"),
        widget::text(" E"),
        widget::text(""),
        widget::text(" H"),
        widget::text(" L"),
        widget::text(""),
        widget::text("Z"),
        widget::text(""),
        widget::text("N"),
        widget::text(""),
        widget::text("H"),
        widget::text(""),
        widget::text("C")
    ]);

    registers_grid = registers_grid.push(grid_row![
        widget::text(format!("{:02X}", registers.read_a())),
        widget::text(format!("{:02X}", registers.read_f())),
        widget::text(""),
        widget::text(format!("{:02X}", registers.read_b())),
        widget::text(format!("{:02X}", registers.read_c())),
        widget::text(""),
        widget::text(format!("{:02X}", registers.read_d())),
        widget::text(format!("{:02X}", registers.read_e())),
        widget::text(""),
        widget::text(format!("{:02X}", registers.read_h())),
        widget::text(format!("{:02X}", registers.read_l())),
        widget::text(""),
        widget::text(format!("{:01X}", registers.read_flag(Flag::Z) as u8)),
        widget::text(""),
        widget::text(format!("{:01X}", registers.read_flag(Flag::N) as u8)),
        widget::text(""),
        widget::text(format!("{:01X}", registers.read_flag(Flag::H) as u8)),
        widget::text(""),
        widget::text(format!("{:01X}", registers.read_flag(Flag::C) as u8)),
    ]);

    registers_grid
}
