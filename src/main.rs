use circular_queue::CircularQueue;
use cpu::CPU;
use iced::{
    self, alignment, executor,
    keyboard::{self, key::Named, Key},
    mouse,
    widget::{
        canvas::{Frame, Geometry, Path, Program},
        container,
        image::Handle,
        text, Button, Canvas, Column, Container, Image, Row,
    },
    Application, Color, Command, Length, Rectangle, Renderer, Settings, Theme,
};
use iced_aw::{grid_row, Grid};
use ppu::PPU;
use registers::Flag;

pub mod conditions;
pub mod cpu;
pub mod memory;
pub mod opcodes;
pub mod ppu;
pub mod registers;

const CPU_SNAPS_CAPACITY: usize = 5;

#[derive(Clone, Debug)]
struct DebuggerWindow {
    pub breakpoints: Vec<u16>,
    pub cpu_snaps: CircularQueue<CPU>,
    pub ppu: PPU,
}

impl DebuggerWindow {
    pub fn current_cpu(self: &Self) -> &CPU {
        self.cpu_snaps
            .iter()
            .next()
            .expect("Current CPU expected a CPU snapshot.")
    }

    pub fn display_breakpoint(self: &Self, address: u16) -> String {
        String::from(if self.breakpoints.contains(&address) {
            "@"
        } else {
            ""
        })
    }
}

#[derive(Debug)]
struct Circle {
    radius: f32,
}

// Then, we implement the `Program` trait
impl Program<Message> for Circle {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        // We prepare a new `Frame`
        let mut frame = Frame::new(renderer, bounds.size());

        // We create a `Path` representing a simple circle
        let circle = Path::circle(frame.center(), self.radius);

        // And fill it with some color
        frame.fill(&circle, Color::BLACK);

        // Finally, we produce the geometry
        vec![frame.into_geometry()]
    }
}
#[derive(Clone, Debug, Hash)]
enum Message {
    RunNextInstruction,
    RunUntilBreakpoint,
}

impl iced::Application for DebuggerWindow {
    type Executor = executor::Default;
    type Flags = CircularQueue<CPU>;
    type Message = Message;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            Self {
                breakpoints: vec![0x000C],
                cpu_snaps: flags,
                ppu: PPU::new(),
            },
            Command::none(),
        )
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        keyboard::on_key_press(|k, _m| match k {
            Key::Named(Named::ArrowRight) => Some(Message::RunNextInstruction),
            _ => None,
        })
    }

    fn title(&self) -> String {
        String::from("Rustyboi debugger")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::RunNextInstruction => {
                for r in 0..160 {
                    for c in 0..144 {
                        self.ppu.rendered_pixels[(r * 144 + c)*4] = rand::random();
                        self.ppu.rendered_pixels[(r * 144 + c)*4 + 1] = rand::random();
                        self.ppu.rendered_pixels[(r * 144 + c)*4 + 2] = rand::random();
                        self.ppu.rendered_pixels[(r * 144 + c)*4 + 3] = 255
                    }
                }
                println!("{:?}", &self.ppu.rendered_pixels[0..20]);
                let next_cpu = self.current_cpu().execute_one_instruction().expect("sad");
                self.cpu_snaps.push(next_cpu);
                Command::none()
            }
            Message::RunUntilBreakpoint => {
                while !self.breakpoints.contains(&self.current_cpu().registers.pc) {
                    let next_cpu = self.current_cpu().execute_one_instruction().expect("sad");
                    self.cpu_snaps.push(next_cpu);
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        let mut instructions_grid = Grid::new().column_spacing(5);
        let history_size = self.cpu_snaps.len() - 1;
        for old in self.cpu_snaps.asc_iter().take(history_size) {
            let instr = old
                .memory
                .decode_instruction_at(old.registers.pc)
                .expect("womp");
            instructions_grid = instructions_grid.push(grid_row![
                text(self.display_breakpoint(instr.address)),
                text(""),
                text(format!("{:04X}", instr.address)),
                text(format!("{}", instr.display_raw())),
                text(format!("{}", instr.instruction))
            ]);
        }

        let cpu = self.current_cpu();
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

        let memory = text(cpu.memory.show_memory_row(0));
        let af_row = Row::new().push(text("AF: ")).push(text(&format!(
            "{:02X} {:02X}",
            cpu.registers.get_a(),
            cpu.registers.get_f()
        )));
        let bc_row = Row::new().push(text("BC: ")).push(text(&format!(
            "{:02X} {:02X}",
            cpu.registers.get_b(),
            cpu.registers.get_c()
        )));
        let de_row = Row::new().push(text("DE: ")).push(text(&format!(
            "{:02X} {:02X}",
            cpu.registers.get_d(),
            cpu.registers.get_e()
        )));
        let hl_row = Row::new().push(text("HL: ")).push(text(&format!(
            "{:02X} {:02X}",
            cpu.registers.get_h(),
            cpu.registers.get_l()
        )));
        let sp_row = Row::new()
            .push(text("SP: "))
            .push(text(&format!("0x{:04X}", cpu.registers.sp)));
        let pc_row = Row::new()
            .push(text("PC: "))
            .push(text(&format!("0x{:04X}", cpu.registers.pc)));
        let flag_row = Row::new()
            .push(text("Flags: "))
            .push(text(&format!(
                "[Z={}]",
                cpu.registers.get_flag(Flag::Z) as u8
            )))
            .push(text(&format!(
                "[N={}]",
                cpu.registers.get_flag(Flag::N) as u8
            )))
            .push(text(&format!(
                "[H={}]",
                cpu.registers.get_flag(Flag::H) as u8
            )))
            .push(text(&format!(
                "[C={}]",
                cpu.registers.get_flag(Flag::C) as u8
            )));
        let register_column = Column::new()
            .push(af_row)
            .push(bc_row)
            .push(de_row)
            .push(hl_row)
            .push(sp_row)
            .push(pc_row)
            .push(flag_row);

        let run_next_instruction_button =
            Button::new("Run next instruction").on_press(Message::RunNextInstruction);
        let run_until_breakpoint_button =
            Button::new("Run until breakpoint").on_press(Message::RunUntilBreakpoint);

        let column = Column::new()
            .push(instructions_grid)
            .push(memory)
            .push(register_column)
            .push(run_next_instruction_button)
            .push(run_until_breakpoint_button);

        let mut grid = Grid::new().vertical_alignment(alignment::Vertical::Top);
        let debugger = Container::new(column)
            .height(600)
            .width(400)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(container::Appearance::default().with_border(Color::BLACK, 2));
        let canvas = Container::new(
            Image::new(Handle::from_pixels(160, 144, self.ppu.rendered_pixels))
                .content_fit(iced::ContentFit::Fill)
                .width(160)
                .height(144),
        )
        .width(160)
        .height(144)
        .style(container::Appearance::default().with_border(Color::BLACK, 2));
        let pixels: Vec<_> = (0..100).collect();
        let image = Image::new(Handle::from_pixels(10, 10, pixels))
            .width(10)
            .height(10);
        grid = grid.push(grid_row![debugger, canvas, image]);
        grid.into()
        // debugger.into()
    }
}

fn main() -> Result<(), iced::Error> {
    let mut queue = CircularQueue::with_capacity(CPU_SNAPS_CAPACITY);
    let mut cpu = CPU::new();
    cpu.memory
        .load_boot_rom(String::from("dmg_boot.bin"))
        .expect("sad");
    queue.push(cpu);
    DebuggerWindow::run(Settings::with_flags(queue))
}
