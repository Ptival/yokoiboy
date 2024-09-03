use std::{
    cmp::min,
    num::{Saturating, Wrapping},
};

use iced::widget;
use iced_aw::{grid_row, Grid};

use crate::{machine::Machine, message::Message};

pub fn view(machine: &Machine) -> Grid<Message> {
    let mut stack_grid = Grid::new();
    stack_grid = stack_grid.push(grid_row![widget::text("Stack:")]);

    // Note: the stack stops at 0xFFFE, as 0xFFFF is used for interrupt enable
    let stack_top = machine.cpu.registers.sp.0;
    let stack_until = min(
        (Saturating(machine.cpu.registers.sp.0) + Saturating(4)).0,
        0xFFFE,
    );

    for stack_addr in stack_top..=stack_until {
        stack_grid = stack_grid.push(grid_row![
            widget::text(format!("0x{:04X}:", stack_addr)),
            widget::text(format!("{:02X}", machine.read_u8(Wrapping(stack_addr)))),
        ]);
    }

    stack_grid
}
