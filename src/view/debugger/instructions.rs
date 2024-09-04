use iced::{widget, Color, Theme};
use iced_aw::{grid_row, Grid};

use crate::{application_state::ApplicationState, memory::Memory, message::Message};

pub fn view(app: &ApplicationState) -> Grid<Message> {
    let mut instructions_grid = Grid::new().column_spacing(5).padding(2);
    let history_size = app.snaps.len() - 1;
    let history_style = |_: &Theme| widget::text::Style {
        color: Some(Color::from_rgb(1.0, 0.0, 0.0)),
    };

    for old in app.snaps.asc_iter().take(history_size) {
        let instr = Memory::decode_instruction_at(old, old.registers().pc);
        let row = grid_row![
            widget::text(app.display_breakpoint(instr.address)).style(history_style),
            widget::text(""),
            widget::text(format!("{:04X}", instr.address)).style(history_style),
            widget::text(format!("{}", instr.display_raw())).style(history_style),
            widget::text(format!("{}", instr)).style(history_style)
        ];
        instructions_grid = instructions_grid.push(row);
    }

    let machine = app.current_machine_immut();
    let pc = machine.registers().pc;
    let instrs = Memory::decode_instructions_at(machine, pc, 10);

    instructions_grid = instructions_grid.push(grid_row![
        widget::text(app.display_breakpoint(instrs[0].address)),
        widget::text("→"),
        widget::text(format!("{:04X}", instrs[0].address)),
        widget::text(format!("{}", instrs[0].display_raw())),
        widget::text(format!("{}", instrs[0]))
    ]);

    for instr in instrs.iter().skip(1) {
        instructions_grid = instructions_grid.push(grid_row![
            widget::text(app.display_breakpoint(instr.address)),
            widget::text(""),
            widget::text(format!("{:04X}", instr.address)),
            widget::text(format!("{}", instr.display_raw())),
            widget::text(format!("{}", instr))
        ]);
    }

    instructions_grid
}
