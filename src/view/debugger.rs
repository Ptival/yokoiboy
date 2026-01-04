mod instructions;
mod lcd;
mod registers;
mod stack;

use iced::widget::{self, row, Column, Row};

use crate::{application_state::ApplicationState, message::Message};

pub fn view(app: &ApplicationState) -> Column<Message> {
    let machine = app.current_machine();
    let instructions = instructions::view(app);
    let registers = registers::view(&machine.registers());
    let stack = stack::view(machine);
    let lcd = lcd::view(machine);
    let ly_grid = Row::new().push(row![
        widget::text("LY: "),
        widget::text(format!("{}", machine.ppu.read_ly()))
    ]);

    widget::Column::new()
        .width(450)
        .height(600)
        .push(instructions)
        .push(registers)
        .push(stack)
        .push(lcd)
        .push(ly_grid)
}
