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
    cpu::{interrupts::Interrupts, CPU},
    instructions::decode::DecodedInstruction,
    machine::Machine,
    memory::{load_boot_rom, load_game_rom},
    message::Message,
};

const CPU_SNAPS_CAPACITY: usize = 5;
const FRAME_TIME_NANOSECONDS: u32 = 16742;
const LOG_PATH: &str = "log";

#[derive(Clone, Debug)]
pub enum MapperType {
    ROMOnly,
    MBC1,
    Other, // TODO
}

#[derive(Clone, Debug)]
pub enum RAMSize {
    NoRAM,
    Ram2kb,
    Ram8kb,
    Ram4banks8kb,
    Ram16banks8kb,
    Ram8banks8kb,
}

#[derive(Clone, Debug)]
pub struct ROMInformation {
    pub mapper_type: MapperType,
    pub ram_size: RAMSize,
    pub rom_banks: u8,
}

impl ROMInformation {
    pub fn new() -> Self {
        ROMInformation {
            mapper_type: MapperType::ROMOnly,
            ram_size: RAMSize::NoRAM,
            rom_banks: 0,
        }
    }
}

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

pub struct MachineStep {
    t_cycles: u128,
    instruction_executed: Option<DecodedInstruction>,
}

pub struct InstructionStep {
    t_cycles: u128,
    _instruction_executed: DecodedInstruction,
}

impl ApplicationState {
    pub fn new(args: &CommandLineArguments, breakpoints: &[u16]) -> Self {
        let mut queue = CircularQueue::with_capacity(CPU_SNAPS_CAPACITY);
        let boot_rom = load_boot_rom(&args.boot_rom).unwrap();
        let (game_rom, rom_information) = load_game_rom(&args.game_rom).unwrap();
        println!("{:?}", rom_information);
        let machine = Machine::new(boot_rom, game_rom, rom_information, args.log_for_doctor);
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
    fn step_machine(machine: &mut Machine) -> MachineStep {
        let mut instruction_executed = None;
        let (mut t_cycles, mut _m_cycles) = Interrupts::handle_interrupts(machine);
        if t_cycles == 0 {
            (instruction_executed, (t_cycles, _m_cycles)) = CPU::execute_one_instruction(machine);
        }
        machine.timers.ticks(&mut machine.interrupts, t_cycles);
        machine.ppu.ticks(
            &mut machine.background_window_fetcher,
            &mut machine.interrupts,
            &mut machine.object_fetcher,
            &mut machine.pixel_fetcher,
            t_cycles,
        );
        machine.t_cycle_count += t_cycles as u64;

        // // Print characters written to the Link cable on the terminal (useful for blargg w/o LCD)
        // if machine.read_u8(Wrapping(0xFF02)).0 == 0x81 {
        //     let char = machine.read_u8(Wrapping(0xFF01));
        //     print!("{}", char.0 as char);
        //     machine.write_u8(Wrapping(0xFF02), Wrapping(0x01));
        // }

        MachineStep {
            t_cycles: t_cycles as u128,
            instruction_executed,
        }
    }

    // Steps cycles forward until an instruction is executed.  May take many tries when the console
    // is in HALT and awaiting an interrupt to wake up and execute an instruction.
    fn execute_one_instruction(&mut self, preserve: PreserveHistory) -> InstructionStep {
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
                let mut executed_instruction = None;
                let mut total_t_cycles: u128 = 0;

                loop {
                    match executed_instruction {
                        Some(decoded_instruction) => {
                            return InstructionStep {
                                t_cycles: total_t_cycles,
                                _instruction_executed: decoded_instruction,
                            }
                        }
                        None => {
                            let step = ApplicationState::step_machine(machine);
                            executed_instruction = step.instruction_executed;
                            total_t_cycles += step.t_cycles;
                        }
                    }
                }
            }
            PreserveHistory::PreserveHistory => {
                let mut next_machine = current_machine.clone();
                let mut executed_instruction = None;
                let mut total_t_cycles = 0;

                loop {
                    match executed_instruction {
                        Some(decoded_instruction) => {
                            self.snaps.push(next_machine);
                            return InstructionStep {
                                t_cycles: total_t_cycles,
                                _instruction_executed: decoded_instruction,
                            };
                        }
                        None => {
                            let step = ApplicationState::step_machine(&mut next_machine);
                            executed_instruction = step.instruction_executed;
                            total_t_cycles += step.t_cycles;
                        }
                    }
                }
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
                let _step = self.execute_one_instruction(PreserveHistory::PreserveHistory);
                self.current_machine().ppu_mut().render();
                Task::none()
            }

            Message::BeginRunUntilBreakpoint => {
                self.paused = false;
                // step at least once to escape current breakpoint! :D
                self.execute_one_instruction(PreserveHistory::DontPreserveHistory);
                Task::done(Message::ContinueRunUntilBreakpoint)
            }

            Message::ContinueRunUntilBreakpoint => {
                let mut pc = self.current_machine().registers().pc;

                let initial_time = time::Instant::now();

                let mut remaining_steps = Saturating(69_905);
                while remaining_steps.0 > 0 && !self.paused && !self.breakpoints.contains(&pc.0) {
                    let step = self.execute_one_instruction(PreserveHistory::DontPreserveHistory);
                    remaining_steps -= step.t_cycles as u32;
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
