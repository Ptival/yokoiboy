use std::{
    cmp::min,
    fmt::Debug,
    fs::{File, OpenOptions},
    io::Write,
    num::{Saturating, Wrapping},
};

use circular_queue::CircularQueue;
use cpu::CPU;
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
use machine::{Machine, EXTERNAL_RAM_SIZE};
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

enum PreserveHistory {
    DontPreserveHistory,
    PreserveHistory,
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
                // "gb-test-roms/cpu_instrs/individual/01-special.gb",
                // "gb-test-roms/cpu_instrs/individual/02-interrupts.gb",
                // "gb-test-roms/cpu_instrs/individual/03-op sp,hl.gb",
                // "gb-test-roms/cpu_instrs/individual/04-op r,imm.gb",
                // "gb-test-roms/cpu_instrs/individual/05-op rp.gb",
                // "gb-test-roms/cpu_instrs/individual/06-ld r,r.gb",
                // "gb-test-roms/cpu_instrs/individual/07-jr,jp,call,ret,rst.gb",
                // "gb-test-roms/cpu_instrs/individual/08-misc instrs.gb",
                // "gb-test-roms/cpu_instrs/individual/09-op r,r.gb",
                "gb-test-roms/cpu_instrs/individual/10-bit ops.gb",
                // "gb-test-roms/cpu_instrs/individual/11-op a,(hl).gb",
            ))
            .expect("Failed to load ROM");
        queue.push(Machine {
            t_cycle_count: 0,
            dmg_boot_rom: Wrapping(0),
            cpu,
            ppu: PPU::new(),
            bgp: Wrapping(0),
            external_ram: [0; EXTERNAL_RAM_SIZE],
            interrupt_enable: Wrapping(0),
            interrupt_flag: Wrapping(0),
            nr11: Wrapping(0),
            nr12: Wrapping(0),
            nr13: Wrapping(0),
            nr14: Wrapping(0),
            nr50: Wrapping(0),
            nr51: Wrapping(0),
            nr52: Wrapping(0),
            sb: Wrapping(0),
            sc: Wrapping(0),
            scx: Wrapping(0),
            scy: Wrapping(0),
            tac: Wrapping(0),
        });
        Self {
            breakpoints: vec![
                // 0x00F1, // passed logo check
                // 0x00FC, // passed header checksum check
                0x0100, // made it out of the boot ROM
                       // 0xC738,
                       // 0xC662,
                       // 0xDEF8,
            ],
            output_file: OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
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

    pub fn display_breakpoint(self: &Self, address: Wrapping<u16>) -> String {
        String::from(if self.breakpoints.contains(&address.0) {
            "@"
        } else {
            ""
        })
    }

    fn step_machine<'a>(machine: &'a mut Machine) -> &'a mut Machine {
        // Arbitrarily stepping the CPU then the PPU
        let (t_cycles, m_cycles) = CPU::execute_one_instruction(machine).expect("sad");
        if t_cycles != 4 * m_cycles {
            println!("T-cycle/M-cycle mismatch: {}, {}", t_cycles, m_cycles)
        }
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
        let current_machine = self.current_machine_immut();
        if !current_machine.is_dmg_boot_rom_on() {
            write!(
                self.output_file,
                "{}\n",
                CPU::gbdoctor_string(current_machine)
            )
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
                self.output_file.flush().expect("flush failed");
                exit()
            }

            Message::RunNextInstruction => {
                // let machine = self.current_machine();
                // for r in 0..160 {
                //     for c in 0..144 {
                //         machine.ppu.rendered_pixels[(r * 144 + c) * 4] = rand::random();
                //         machine.ppu.rendered_pixels[(r * 144 + c) * 4 + 1] = rand::random();
                //         machine.ppu.rendered_pixels[(r * 144 + c) * 4 + 2] = rand::random();
                //         machine.ppu.rendered_pixels[(r * 144 + c) * 4 + 3] = 255
                //     }
                // }
                self.step(PreserveHistory::PreserveHistory);
                self.current_machine().ppu.render_vram();
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
                let mut remaining_steps: u32 = 100;
                while remaining_steps > 0
                    && !self.paused
                    && !self.breakpoints.contains(&pc.0)
                    && self.current_machine().ppu.read_ly().0 != 144
                {
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
            let instr = Memory::decode_instruction_at(old, old.cpu.registers.pc).expect("womp");
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
        let instrs = Memory::decode_instructions_at(machine, pc, 10).expect("womp");
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
            .push(widget::text(format!("{}", machine.ppu.read_ly())));

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
    let mut settings = Settings::default();
    settings.default_font = font::Font::MONOSPACE;
    iced::application("Rustyboi", DebuggerWindow::update, DebuggerWindow::view)
        .subscription(DebuggerWindow::subscription)
        .settings(settings)
        .window_size(Size::new(2000.0, 1200.0))
        .run()
}
