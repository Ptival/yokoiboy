use std::{
    cmp::min,
    fmt::Debug,
    fs::{File, OpenOptions},
    io::Write,
    num::{Saturating, Wrapping},
};

use circular_queue::CircularQueue;
use clap::Parser;
use cpu::{interrupts::Interrupts, timers::Timers, CPU};
use iced::{
    self,
    advanced::{
        graphics::core::font,
        image::{self},
    },
    alignment::{self},
    border::Radius,
    exit,
    keyboard::{self},
    widget::{
        self,
        container::{self},
        image::FilterMethod,
    },
    Border, Color, Settings, Size, Task, Theme,
};
use iced_aw::{grid_row, Grid};
use machine::Machine;
use memory::Memory;
use ppu::PPU;
use registers::Flag;

pub mod conditions;
pub mod cpu;
pub mod instructions;
pub mod machine;
pub mod memory;
pub mod ppu;
pub mod registers;

const CPU_SNAPS_CAPACITY: usize = 5;

#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = None)]
struct CommandLineArguments {
    #[arg(short, long)]
    boot_rom: String,
    #[arg(short, long)]
    game_rom: String,
    #[arg(short, long, default_value_t = false)]
    log_for_doctor: bool,
}

enum PreserveHistory {
    DontPreserveHistory,
    PreserveHistory,
}

#[derive(Debug)]
struct DebuggerWindow {
    pub breakpoints: Vec<u16>,
    pub output_file: Option<File>,
    pub paused: bool,
    pub snaps: CircularQueue<Machine>,
}

impl DebuggerWindow {
    fn new(args: &CommandLineArguments) -> Self {
        let mut queue = CircularQueue::with_capacity(CPU_SNAPS_CAPACITY);
        let mut machine = Machine::new(args.log_for_doctor);
        machine
            .cpu
            .memory
            .load_boot_rom(&args.boot_rom)
            .unwrap_or_else(|e| panic!("Failed to load boot ROM: {}", e))
            .load_rom(&args.game_rom)
            .unwrap_or_else(|e| panic!("Failed to load game ROM: {}", e));
        queue.push(machine);
        Self {
            breakpoints: vec![
                // 0x00F1, // passed logo check
                // 0x00FC, // passed header checksum check
                0x0100, // made it out of the boot ROM
                       // 0xC738,
                       // 0xC662,
                       // 0xDEF8,
            ],
            output_file: if args.log_for_doctor {
                Some(
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open("log")
                        .unwrap_or_else(|e| panic!("Could not create log file: {}", e)),
                )
            } else {
                None
            },
            paused: false,
            snaps: queue,
        }
    }

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

    pub fn display_breakpoint(self: &Self, address: Wrapping<u16>) -> String {
        String::from(if self.breakpoints.contains(&address.0) {
            "@"
        } else {
            ""
        })
    }

    // TODO: move in machine.rs
    fn step_machine<'a>(machine: &'a mut Machine) -> &'a mut Machine {
        let (mut t_cycles, mut _m_cycles) = Interrupts::handle_interrupts(machine);
        if t_cycles == 0 {
            (t_cycles, _m_cycles) = CPU::execute_one_instruction(machine)
        }
        Timers::step_dots(machine, t_cycles);
        PPU::step_dots(machine, t_cycles);
        machine.t_cycle_count += t_cycles as u64;

        if machine.read_u8(Wrapping(0xFF02)).0 == 0x81 {
            let char = machine.read_u8(Wrapping(0xFF01));
            print!("{}", char.0 as char);
            machine.write_u8(Wrapping(0xFF02), Wrapping(0x01));
        }

        machine
    }

    fn step(&mut self, preserve: PreserveHistory) {
        if !self.current_machine().is_dmg_boot_rom_on()
            && !self.current_machine().cpu.low_power_mode
        {
            let string = CPU::gbdoctor_string(self.current_machine());
            if let Some(output_file) = self.output_file.as_mut() {
                write!(output_file, "{}\n", string).expect("write to log failed");
            }
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
    Quit,
    RunNextInstruction,
    BeginRunUntilBreakpoint,
    ContinueRunUntilBreakpoint,
}

impl DebuggerWindow {
    fn subscription(&self) -> iced::Subscription<Message> {
        keyboard::on_key_press(|k, _m| match k {
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                Some(Message::BeginRunUntilBreakpoint)
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                Some(Message::RunNextInstruction)
            }
            keyboard::Key::Named(keyboard::key::Named::Space) => Some(Message::Pause),
            keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::Quit),
            _ => None,
        })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Pause => {
                self.paused = true;
                Task::none()
            }

            Message::Quit => {
                if let Some(output_file) = self.output_file.as_mut() {
                    output_file.flush().expect("flush failed");
                }
                exit()
            }

            Message::RunNextInstruction => {
                self.step(PreserveHistory::PreserveHistory);
                self.current_machine().ppu.render_vram();
                Task::none()
            }

            Message::BeginRunUntilBreakpoint => {
                self.paused = false;
                // step at least once to escape current breakpoint! :D
                self.step(PreserveHistory::DontPreserveHistory);
                Task::done(Message::ContinueRunUntilBreakpoint)
            }

            Message::ContinueRunUntilBreakpoint => {
                let mut pc = self.current_machine().cpu.registers.pc;

                // Run some number of steps before updating the display
                let mut remaining_steps: u32 = 10000;
                while remaining_steps > 0 && !self.paused && !self.breakpoints.contains(&pc.0) {
                    remaining_steps -= 1;
                    self.step(PreserveHistory::DontPreserveHistory);
                    pc = self.current_machine().cpu.registers.pc;
                }

                if remaining_steps == 0 {
                    self.current_machine().ppu.render_vram();
                    Task::done(Message::ContinueRunUntilBreakpoint)
                } else {
                    Task::none()
                }
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let mut instructions_grid = Grid::new().column_spacing(5).padding(2);
        let history_size = self.snaps.len() - 1;
        let history_style = |_: &Theme| widget::text::Style {
            color: Some(Color::from_rgb(1.0, 0.0, 0.0)),
        };
        for old in self.snaps.asc_iter().take(history_size) {
            let instr = Memory::decode_instruction_at(old, old.cpu.registers.pc);
            let row = grid_row![
                widget::text(self.display_breakpoint(instr.address)).style(history_style),
                widget::text(""),
                widget::text(format!("{:04X}", instr.address)).style(history_style),
                widget::text(format!("{}", instr.display_raw())).style(history_style),
                widget::text(format!("{}", instr)).style(history_style)
            ];
            instructions_grid = instructions_grid.push(row);
        }

        let machine = self.current_machine_immut();
        let cpu = &machine.cpu;
        let pc = cpu.registers.pc;
        let instrs = Memory::decode_instructions_at(machine, pc, 10);
        instructions_grid = instructions_grid.push(grid_row![
            widget::text(self.display_breakpoint(instrs[0].address)),
            widget::text("→"),
            widget::text(format!("{:04X}", instrs[0].address)),
            widget::text(format!("{}", instrs[0].display_raw())),
            widget::text(format!("{}", instrs[0]))
        ]);
        for instr in instrs.iter().skip(1) {
            instructions_grid = instructions_grid.push(grid_row![
                widget::text(self.display_breakpoint(instr.address)),
                widget::text(""),
                widget::text(format!("{:04X}", instr.address)),
                widget::text(format!("{}", instr.display_raw())),
                widget::text(format!("{}", instr))
            ]);
        }

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
            widget::text(format!("{:02X}", cpu.registers.read_a())),
            widget::text(format!("{:02X}", cpu.registers.read_f())),
            widget::text(""),
            widget::text(format!("{:02X}", cpu.registers.read_b())),
            widget::text(format!("{:02X}", cpu.registers.read_c())),
            widget::text(""),
            widget::text(format!("{:02X}", cpu.registers.read_d())),
            widget::text(format!("{:02X}", cpu.registers.read_e())),
            widget::text(""),
            widget::text(format!("{:02X}", cpu.registers.read_h())),
            widget::text(format!("{:02X}", cpu.registers.read_l())),
            widget::text(""),
            widget::text(format!("{:01X}", cpu.registers.read_flag(Flag::Z) as u8)),
            widget::text(""),
            widget::text(format!("{:01X}", cpu.registers.read_flag(Flag::N) as u8)),
            widget::text(""),
            widget::text(format!("{:01X}", cpu.registers.read_flag(Flag::H) as u8)),
            widget::text(""),
            widget::text(format!("{:01X}", cpu.registers.read_flag(Flag::C) as u8)),
        ]);

        let dmg_row = widget::Row::new().push(widget::text(format!(
            "DMG: {}",
            machine.is_dmg_boot_rom_on()
        )));

        let mem_row1 =
            widget::Row::new().push(widget::text(machine.show_memory_row(Wrapping(0x104))));
        let mem_row2 =
            widget::Row::new().push(widget::text(machine.show_memory_row(Wrapping(0x10C))));
        let mem_row3 =
            widget::Row::new().push(widget::text(machine.show_memory_row(Wrapping(0x114))));

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

        let ly_row = widget::Row::new()
            .push(widget::text("LY: "))
            .push(widget::text(format!("{}", PPU::read_ly(machine))));

        let cycle_row =
            widget::Row::new().push(widget::text(format!("Cycles: {}", machine.t_cycle_count)));

        let register_column = widget::Column::new()
            .push(registers_grid)
            .push(dmg_row)
            .push(lcdc_grid)
            .push(ly_row)
            .push(mem_row1)
            .push(mem_row2)
            .push(mem_row3)
            .push(cycle_row);

        let run_next_instruction_button =
            widget::Button::new("Run next instruction").on_press(Message::RunNextInstruction);
        let run_until_breakpoint_button =
            widget::Button::new("Run until breakpoint").on_press(Message::BeginRunUntilBreakpoint);

        let mut stack_grid = Grid::new();
        stack_grid = stack_grid.push(grid_row![widget::text("Stack:")]);
        // Note: the stack stops at 0xFFFE, as 0xFFFF is used for interrupt enable
        let stack_top = cpu.registers.sp.0;
        let stack_until = min((Saturating(cpu.registers.sp.0) + Saturating(4)).0, 0xFFFE);
        for stack_addr in stack_top..=stack_until {
            stack_grid = stack_grid.push(grid_row![
                widget::text(format!("0x{:04X}:", stack_addr)),
                widget::text(format!("{:02X}", machine.read_u8(Wrapping(stack_addr)))),
            ]);
        }

        let column = widget::Column::new()
            .align_x(alignment::Horizontal::Left)
            .width(500)
            .push(instructions_grid)
            .push(register_column)
            .push(stack_grid)
            .push(run_next_instruction_button)
            .push(run_until_breakpoint_button);

        let mut grid = Grid::new().vertical_alignment(alignment::Vertical::Top);
        let debugger = widget::Container::new(column)
            .height(700)
            .width(600)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
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
                image::Bytes::copy_from_slice(&machine.ppu.rendered_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(480)
            .height(432),
        )
        .width(480)
        .height(432);

        let vram = widget::Container::new(
            widget::Image::new(image::Handle::from_rgba(
                128,
                128,
                image::Bytes::copy_from_slice(&machine.ppu.vram_pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(FilterMethod::Nearest)
            .width(512)
            .height(512),
        )
        .width(512)
        .height(512);

        grid = grid.push(grid_row![debugger, vram, lcd]);
        grid.into()
        // debugger.into()
    }
}

fn main() -> Result<(), iced::Error> {
    let args = CommandLineArguments::parse();

    let mut settings = Settings::default();
    settings.default_font = font::Font::MONOSPACE;
    iced::application("YokoiBoy", DebuggerWindow::update, DebuggerWindow::view)
        .subscription(DebuggerWindow::subscription)
        .settings(settings)
        .window_size(Size::new(2000.0, 1200.0))
        .run_with(move || {
            (
                DebuggerWindow::new(&args),
                Task::done(Message::BeginRunUntilBreakpoint),
            )
        })
}
