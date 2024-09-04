mod instructions;
mod lcd;
mod registers;
mod stack;

use iced::widget::{self, Column};

use crate::{application_state::ApplicationState, message::Message};

pub fn view(app: &ApplicationState) -> Column<Message> {
    let machine = app.current_machine_immut();
    let instructions = instructions::view(app);
    let registers = registers::view(&machine.registers());
    let stack = stack::view(machine);
    let lcd = lcd::view(machine);

    widget::Column::new()
        .width(450)
        .height(520)
        .push(instructions)
        .push(registers)
        .push(stack)
        .push(lcd)
}
