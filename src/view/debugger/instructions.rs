use iced::{
    widget::{self, row, Column},
    Color, Theme,
};

use crate::{application_state::ApplicationState, memory::Memory, message::Message};

pub fn view(app: &ApplicationState) -> Column<Message> {
    let mut instructions_grid = Column::new();
    let history_size = app.snaps.len() - 1;
    let history_style = |_: &Theme| widget::text::Style {
        color: Some(Color::from_rgb(1.0, 0.0, 0.0)),
    };

    for old in app.snaps.asc_iter().take(history_size) {
        let instr = Memory::decode_instruction_at(old, old.registers().pc);
        let row = row![
            widget::text(app.display_breakpoint(instr.address)).style(history_style),
            widget::text("  "),
            widget::text(format!("{:04X}", instr.address)).style(history_style),
            widget::text(" "),
            widget::text(format!("{}", instr)).style(history_style)
        ];
        instructions_grid = instructions_grid.push(row);
    }

    let machine = app.current_machine_immut();
    let pc = machine.registers().pc;
    let instrs = Memory::decode_instructions_at(machine, pc, 10 - history_size as u8);

    instructions_grid = instructions_grid.push(row![
        widget::text(app.display_breakpoint(instrs[0].address)),
        widget::text("→ "),
        widget::text(format!("{:04X}", instrs[0].address)),
        widget::text(" "),
        widget::text(format!("{}", instrs[0]))
    ]);

    for instr in instrs.iter().skip(1) {
        instructions_grid = instructions_grid.push(row![
            widget::text(app.display_breakpoint(instr.address)),
            widget::text("  "),
            widget::text(format!("{:04X}", instr.address)),
            widget::text(" "),
            widget::text(format!("{}", instr))
        ]);
    }

    instructions_grid
}
