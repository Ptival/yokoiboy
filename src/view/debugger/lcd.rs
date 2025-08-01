use iced::widget::{self, row, Column};

use crate::{machine::Machine, message::Message};

pub fn view(machine: &Machine) -> Column<Message> {
    let mut lcdc_grid_right = Column::new();
    lcdc_grid_right = lcdc_grid_right.push(row![
        widget::text("      "), // same length as "LCDC: "
        widget::text("7"),
        widget::text("6"),
        widget::text("5"),
        widget::text("4"),
        widget::text("3"),
        widget::text("2"),
        widget::text("1"),
        widget::text("0"),
    ]);
    let lcdc = machine.ppu().read_lcdc().0;
    lcdc_grid_right = lcdc_grid_right.push(row![
        widget::text("LCDC: "),
        widget::text(format!("{}", (lcdc & (1 << 7)) >> 7)),
        widget::text(format!("{}", (lcdc & (1 << 6)) >> 6)),
        widget::text(format!("{}", (lcdc & (1 << 5)) >> 5)),
        widget::text(format!("{}", (lcdc & (1 << 4)) >> 4)),
        widget::text(format!("{}", (lcdc & (1 << 3)) >> 3)),
        widget::text(format!("{}", (lcdc & (1 << 2)) >> 2)),
        widget::text(format!("{}", (lcdc & (1 << 1)) >> 1)),
        widget::text(format!("{}", (lcdc & (1 << 0)) >> 0)),
    ]);
    lcdc_grid_right
}
