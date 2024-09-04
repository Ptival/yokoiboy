use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    num::{Saturating, Wrapping},
    path::Path,
    thread::sleep,
    time::{self, Duration},
};

use circular_queue::CircularQueue;
use iced::{exit, keyboard, Task};

use crate::{
    command_line_arguments::CommandLineArguments,
    cpu::{interrupts::Interrupts, timers::Timers, CPU},
    machine::Machine,
    message::Message,
    ppu::PPU,
};

const CPU_SNAPS_CAPACITY: usize = 5;
const FRAME_TIME_NANOSECONDS: u32 = 16742;
const LOG_PATH: &str = "log";

#[derive(Debug)]
pub struct ApplicationState {
    pub breakpoints: Vec<u16>,
    pub output_file: Option<File>,
    pub paused: bool,
    pub snaps: CircularQueue<Machine>,
    target_frame_time: Duration,
}

enum PreserveHistory {
    DontPreserveHistory,
    PreserveHistory,
}

impl ApplicationState {
    pub fn new(args: &CommandLineArguments, breakpoints: &[u16]) -> Self {
        let mut queue = CircularQueue::with_capacity(CPU_SNAPS_CAPACITY);
        let mut machine = Machine::new(args.log_for_doctor);
        machine
            .memory_mut()
            .load_boot_rom(&args.boot_rom)
            .unwrap_or_else(|e| panic!("Failed to load boot ROM: {}", e))
            .load_rom(&args.game_rom)
            .unwrap_or_else(|e| panic!("Failed to load game ROM: {}", e));
        queue.push(machine);
        let target_frame_time = Duration::new(0, FRAME_TIME_NANOSECONDS);
        Self {
            breakpoints: breakpoints.into(),
            output_file: if args.log_for_doctor {
                Some(
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(LOG_PATH)
                        .unwrap_or_else(|e| panic!("Could not create log file: {}", e)),
                )
            } else {
                // Avoid accidentally thinking a stale log is the current log
                if Path::new(LOG_PATH).exists() {
                    fs::remove_file(LOG_PATH).unwrap();
                }
                None
            },
            paused: false,
            snaps: queue,
            target_frame_time,
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

    // TODO: move this elsewhere
    pub fn display_breakpoint(self: &Self, address: Wrapping<u16>) -> String {
        String::from(if self.breakpoints.contains(&address.0) {
            "@"
        } else {
            ""
        })
    }

    // TODO: move in machine.rs
    fn step_machine<'a>(machine: &'a mut Machine) -> u8 {
        let (mut t_cycles, mut _m_cycles) = Interrupts::handle_interrupts(machine);
        if t_cycles == 0 {
            (t_cycles, _m_cycles) = CPU::execute_one_instruction(machine)
        }
        Timers::step_dots(machine, t_cycles);
        PPU::step_dots(machine, t_cycles);
        machine.t_cycle_count += t_cycles as u64;

        // // Print characters written to the Link cable on the terminal (useful for blargg w/o LCD)
        // if machine.read_u8(Wrapping(0xFF02)).0 == 0x81 {
        //     let char = machine.read_u8(Wrapping(0xFF01));
        //     print!("{}", char.0 as char);
        //     machine.write_u8(Wrapping(0xFF02), Wrapping(0x01));
        // }

        t_cycles
    }

    fn step(&mut self, preserve: PreserveHistory) -> u8 {
        if !self.current_machine().is_dmg_boot_rom_on()
            && !self.current_machine().cpu().low_power_mode
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
                ApplicationState::step_machine(machine)
            }
            PreserveHistory::PreserveHistory => {
                let mut next_machine = current_machine.clone();
                let t_cycles = ApplicationState::step_machine(&mut next_machine);
                self.snaps.push(next_machine);
                t_cycles
            }
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
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
                self.current_machine().ppu_mut().render();
                Task::none()
            }

            Message::BeginRunUntilBreakpoint => {
                self.paused = false;
                // step at least once to escape current breakpoint! :D
                self.step(PreserveHistory::DontPreserveHistory);
                Task::done(Message::ContinueRunUntilBreakpoint)
            }

            Message::ContinueRunUntilBreakpoint => {
                let mut pc = self.current_machine().registers().pc;

                let initial_time = time::Instant::now();

                let mut remaining_steps = Saturating(69_905);
                while remaining_steps.0 > 0 && !self.paused && !self.breakpoints.contains(&pc.0) {
                    let elapsed_t_cycles = self.step(PreserveHistory::DontPreserveHistory);
                    remaining_steps -= elapsed_t_cycles as u32;
                    // self.current_machine().ppu_mut().render();
                    // let final_frame_time = time::Instant::now() - initial_time;
                    // if final_frame_time > target_frame_time {
                    //     println!("Overslept {:?}", final_frame_time - target_frame_time);
                    // } else {
                    //     println!("Did not oversleep");
                    // }
                    pc = self.current_machine().registers().pc;
                }

                if remaining_steps.0 == 0 {
                    // If we're stopping for a frame, try to get accurate frame time
                    self.current_machine().ppu_mut().render();
                    let final_time = time::Instant::now();
                    let frame_time = final_time - initial_time;
                    if frame_time.as_nanos() < FRAME_TIME_NANOSECONDS as u128 {
                        sleep(self.target_frame_time - frame_time);
                    }
                    // Note: I think technically we should save this time, so that we can account
                    // for the application rendering time as part of the next frame time.  Currently
                    // does not matter much though.
                    Task::done(Message::ContinueRunUntilBreakpoint)
                } else {
                    // If we're stopping for a breakpoint, no need for frame accuracy
                    Task::none()
                }
            }
        }
    }
}
