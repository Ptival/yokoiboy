use std::{
    cmp,
    fmt::Debug,
    fs::{File, OpenOptions},
    io::Write,
};

use circular_queue::CircularQueue;
use cpu::CPU;
use iced::{
    self,
    advanced::image::{Bytes, Handle},
    alignment,
    border::Radius,
    keyboard::{self, key::Named, Key},
    widget::{
        container::{self, Style},
        text, Button, Column, Container, Image, Row,
    },
    Border, Color, Task,
};
use iced_aw::{grid_row, Grid};
use ppu::PPU;
use registers::Flag;

pub mod conditions;
pub mod cpu;
pub mod instruction;
pub mod memory;
pub mod ppu;
pub mod registers;

const CPU_SNAPS_CAPACITY: usize = 5;

enum PreserveHistory {
    DontPreserveHistory,
    PreserveHistory,
}

#[derive(Clone, Debug)]
struct Machine {
    pub cpu: CPU,
    pub ppu: PPU,
}

#[derive(Debug)]
struct DebuggerWindow {
    pub breakpoints: Vec<u16>,
    pub output_file: File,
    pub paused: bool,
    pub snaps: CircularQueue<Machine>,
}

impl Default for DebuggerWindow {
    fn default() -> Self {
        let mut queue = CircularQueue::with_capacity(CPU_SNAPS_CAPACITY);
        let mut cpu = CPU::new();
        cpu.memory
            .load_boot_rom(String::from("dmg_boot.bin"))
            .expect("Failed to load boot ROM")
            .load_rom(String::from(
                "gb-test-roms/cpu_instrs/individual/07-jr,jp,call,ret,rst.gb",
            ))
            .expect("Failed to load ROM");
        queue.push(Machine {
            cpu,
            ppu: PPU::new(),
        });
        Self {
            breakpoints: vec![
                // 0x000C,
                // 0x0028,
                // 0x0034,
                // 0x0042, 0x0051, 0x0055, 0x006A, 0x0070, 0x008C, 0x00E8, // not yet
                0x00F1, // passed logo check
                0x00FC, // passed header checksum check
                0x00FF, // goal
                        //0x00A3
            ],
            output_file: OpenOptions::new()
                .write(true)
                .create(true)
                .open("log")
                .expect("could not create log"),
            paused: false,
            snaps: queue,
        }
    }
}

impl DebuggerWindow {
    pub fn current_machine(self: &mut Self) -> &mut Machine {
        self.snaps
            .iter_mut()
            .next()
            .expect("current_machine: no machine")
    }

    pub fn current_machine_immut(self: &Self) -> &Machine {
        self.snaps
            .iter()
            .next()
            .expect("current_machine_immut: no machine")
    }

    pub fn display_breakpoint(self: &Self, address: u16) -> String {
        String::from(if self.breakpoints.contains(&address) {
            "@"
        } else {
            ""
        })
    }

    fn step_machine<'a>(machine: &'a mut Machine) -> &'a mut Machine {
        // Arbitrarily stepping the CPU then the PPU
        machine.cpu.execute_one_instruction().expect("sad");
        machine.ppu.step(&mut machine.cpu); // FIXME: make this part of a machine

        if machine.cpu.memory.read_u8(0xFF02) == 0x81 {
            let char = machine.cpu.memory.read_u8(0xFF01);
            print!("{}", char);
            machine.cpu.memory.write_u8(0xFF02, 0x01);
        }

        machine
    }

    fn step(&mut self, preserve: PreserveHistory) {
        let current_machine = self.current_machine_immut();
        if !current_machine.cpu.memory.is_dmg_boot_rom_on() {
            write!(self.output_file, "{}\n", current_machine.cpu.log_string())
                .expect("write to log failed");
        }
        let current_machine = self.current_machine();
        match preserve {
            PreserveHistory::DontPreserveHistory => {
                let machine = current_machine;
                DebuggerWindow::step_machine(machine);
            }
            PreserveHistory::PreserveHistory => {
                let mut next_machine = current_machine.clone();
                DebuggerWindow::step_machine(&mut next_machine);
                self.snaps.push(next_machine);
            }
        }
    }
}

#[derive(Clone, Debug, Hash)]
enum Message {
    Pause,
    RunNextInstruction,
    BeginRunUntilBreakpoint,
    ContinueRunUntilBreakpoint,
}

impl DebuggerWindow {
    fn subscription(&self) -> iced::Subscription<Message> {
        keyboard::on_key_press(|k, _m| match k {
            Key::Named(Named::ArrowDown) => Some(Message::BeginRunUntilBreakpoint),
            Key::Named(Named::ArrowRight) => Some(Message::RunNextInstruction),
            Key::Named(Named::Space) => Some(Message::Pause),
            _ => None,
        })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Pause => {
                self.paused = true;
                Task::none()
            }

            Message::RunNextInstruction => {
                let machine = self.current_machine();
                for r in 0..160 {
                    for c in 0..144 {
                        machine.ppu.rendered_pixels[(r * 144 + c) * 4] = rand::random();
                        machine.ppu.rendered_pixels[(r * 144 + c) * 4 + 1] = rand::random();
                        machine.ppu.rendered_pixels[(r * 144 + c) * 4 + 2] = rand::random();
                        machine.ppu.rendered_pixels[(r * 144 + c) * 4 + 3] = 255
                    }
                }
                self.step(PreserveHistory::PreserveHistory);
                Task::none()
            }

            Message::BeginRunUntilBreakpoint => {
                self.paused = false;
                // make sure to step at least once! :D
                self.step(PreserveHistory::DontPreserveHistory);
                Task::done(Message::ContinueRunUntilBreakpoint)
            }

            Message::ContinueRunUntilBreakpoint => {
                let mut pc = self.current_machine().cpu.registers.pc;

                // Try to run some number of steps before updating the display
                let mut remaining_steps: u32 = 10_000_000;
                while remaining_steps > 0 && !self.paused && !self.breakpoints.contains(&pc) {
                    remaining_steps -= 1;
                    self.step(PreserveHistory::DontPreserveHistory);
                    pc = self.current_machine().cpu.registers.pc;
                }

                if remaining_steps == 0 {
                    Task::done(Message::ContinueRunUntilBreakpoint)
                } else {
                    Task::none()
                }
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let mut instructions_grid = Grid::new().column_spacing(5);
        let history_size = self.snaps.len() - 1;
        for old in self.snaps.asc_iter().take(history_size) {
            let instr = old
                .cpu
                .memory
                .decode_instruction_at(old.cpu.registers.pc)
                .expect("womp");
            instructions_grid = instructions_grid.push(grid_row![
                text(self.display_breakpoint(instr.address)),
                text(""),
                text(format!("{:04X}", instr.address)),
                text(format!("{}", instr.display_raw())),
                text(format!("{}", instr.instruction))
            ]);
        }

        let machine = self.current_machine_immut();
        let cpu = &machine.cpu;
        let pc = cpu.registers.pc;
        let instrs = cpu.memory.decode_instructions_at(pc, 10).expect("womp");
        instructions_grid = instructions_grid.push(grid_row![
            text(self.display_breakpoint(instrs[0].address)),
            text("→"),
            text(format!("{:04X}", instrs[0].address)),
            text(format!("{}", instrs[0].display_raw())),
            text(format!("{}", instrs[0].instruction))
        ]);
        for instr in instrs.iter().skip(1) {
            instructions_grid = instructions_grid.push(grid_row![
                text(self.display_breakpoint(instr.address)),
                text(""),
                text(format!("{:04X}", instr.address)),
                text(format!("{}", instr.display_raw())),
                text(format!("{}", instr.instruction))
            ]);
        }

        let af_row = Row::new().push(text("AF: ")).push(text(format!(
            "{:02X} {:02X}",
            cpu.registers.read_a(),
            cpu.registers.read_f()
        )));
        let bc_row = Row::new().push(text("BC: ")).push(text(format!(
            "{:02X} {:02X}",
            cpu.registers.read_b(),
            cpu.registers.read_c()
        )));
        let de_row = Row::new().push(text("DE: ")).push(text(format!(
            "{:02X} {:02X}",
            cpu.registers.read_d(),
            cpu.registers.read_e()
        )));
        let hl_row = Row::new().push(text("HL: ")).push(text(format!(
            "{:02X} {:02X}",
            cpu.registers.read_h(),
            cpu.registers.read_l()
        )));
        let flag_row = Row::new()
            .push(text("Flags: "))
            .push(text(format!(
                "[Z={}]",
                cpu.registers.get_flag(Flag::Z) as u8
            )))
            .push(text(format!(
                "[N={}]",
                cpu.registers.get_flag(Flag::N) as u8
            )))
            .push(text(format!(
                "[H={}]",
                cpu.registers.get_flag(Flag::H) as u8
            )))
            .push(text(format!(
                "[C={}]",
                cpu.registers.get_flag(Flag::C) as u8
            )));

        let dmg_row = Row::new().push(text(format!("DMG: {}", cpu.memory.is_dmg_boot_rom_on())));

        let mem_row1 = Row::new().push(text(cpu.memory.show_memory_row(0x104)));
        let mem_row2 = Row::new().push(text(cpu.memory.show_memory_row(0x10C)));
        let mem_row3 = Row::new().push(text(cpu.memory.show_memory_row(0x114)));

        let ly_row = Row::new()
            .push(text("LY: "))
            .push(text(format!("{}", machine.ppu.read_ly(&cpu))));

        let register_column = Column::new()
            .push(af_row)
            .push(bc_row)
            .push(de_row)
            .push(hl_row)
            .push(flag_row)
            .push(dmg_row)
            .push(ly_row)
            .push(mem_row1)
            .push(mem_row2)
            .push(mem_row3);

        let run_next_instruction_button =
            Button::new("Run next instruction").on_press(Message::RunNextInstruction);
        let run_until_breakpoint_button =
            Button::new("Run until breakpoint").on_press(Message::BeginRunUntilBreakpoint);

        let mut stack_grid = Grid::new();
        stack_grid = stack_grid.push(grid_row![text("Stack:")]);
        // Note: the stack stops at 0xFFFE, as 0xFFFF is used for interrupt enable
        for stack_addr in cmp::max(cpu.registers.sp, 0xFF80)..=0xFFFE {
            stack_grid = stack_grid.push(grid_row![
                text(format!("0x{:04X}:", stack_addr)),
                text(format!("{:02X}", cpu.memory.read_u8(stack_addr))),
            ]);
        }

        let column = Column::new()
            .push(instructions_grid)
            .push(register_column)
            .push(stack_grid)
            .push(run_next_instruction_button)
            .push(run_until_breakpoint_button);

        let mut grid = Grid::new().vertical_alignment(alignment::Vertical::Top);
        let debugger = Container::new(column)
            .height(700)
            .width(400)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(|_theme| {
                container::Style::default().border(Border {
                    color: Color::BLACK,
                    width: 2.0,
                    radius: Radius::default(),
                })
            });
        let canvas = Container::new(
            Image::new(Handle::from_rgba(
                160,
                144,
                Bytes::copy_from_slice(&machine.ppu.rendered_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .width(160)
            .height(144),
        )
        .width(160)
        .height(144)
        .style(|_theme| Style::default());
        grid = grid.push(grid_row![debugger, canvas]);
        grid.into()
        // debugger.into()
    }
}

fn main() -> Result<(), iced::Error> {
    iced::application("Rustyboi", DebuggerWindow::update, DebuggerWindow::view)
        .subscription(DebuggerWindow::subscription)
        .run()
}
