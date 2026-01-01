use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    num::{Saturating, Wrapping},
    path::Path,
    thread::sleep,
    time::{self, Duration},
};

use anyhow::Result;
use ciborium;
use circular_queue::CircularQueue;
use iced::{exit, keyboard, Subscription, Task};
use serde::{Deserialize, Serialize};

use crate::{
    command_line_arguments::CommandLineArguments,
    cpu::CPU,
    instructions::decode::DecodedInstruction,
    machine::{FixLY, Machine, SkipBoot},
    memory::{load_boot_rom, load_game_rom},
    message::{Message, StepBeforeCheckingBreakpoint},
};

const CPU_SNAPS_CAPACITY: usize = 5;
const FRAME_TIME_NANOSECONDS: u32 = 16742;
const LOG_PATH: &str = "log";

const SAVE_STATE_PATH: &str = "save_state.cbor";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MapperType {
    ROMOnly,
    MBC1,
    Other, // TODO
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum RAMSize {
    NoRAM,
    Ram2kb,
    Ram8kb,
    Ram4banks8kb,
    Ram16banks8kb,
    Ram8banks8kb,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub lcd_pixel_under_mouse: (u8, u8),
    pub output_file: Option<File>,
    pub paused: bool,
    pub snaps: CircularQueue<Machine>,
    pub tile_id_under_mouse: u16,
    target_frame_time: Duration,
}

enum PreserveHistory {
    DontPreserveHistory,
    PreserveHistory,
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
        let machine = Machine::new(
            boot_rom,
            game_rom,
            rom_information,
            FixLY::from(args.log_for_doctor),
            SkipBoot::from(args.log_for_doctor || args.skip_boot),
        );
        queue.push(machine);
        let target_frame_time = Duration::new(0, FRAME_TIME_NANOSECONDS);
        Self {
            breakpoints: breakpoints.into(),
            lcd_pixel_under_mouse: (0, 0),
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
            tile_id_under_mouse: 0,
        }
    }

    pub fn current_machine_mut(self: &mut Self) -> &mut Machine {
        self.snaps
            .iter_mut()
            .next()
            .expect("current_machine: no machine")
    }

    pub fn current_machine(self: &Self) -> &Machine {
        self.snaps
            .iter()
            .next()
            .expect("current_machine_immut: no machine")
    }

    // TODO: move this elsewhere
    pub fn display_breakpoint(self: &Self, address: Wrapping<u16>) -> String {
        String::from(if self.breakpoints.contains(&address.0) {
            "@ "
        } else {
            "  "
        })
    }

    // Steps cycles forward until an instruction is executed.  May take many tries when the console
    // is in HALT and awaiting an interrupt to wake up and execute an instruction.
    fn execute_one_instruction(&mut self, preserve: PreserveHistory) -> InstructionStep {
        // GB doctor suggests setting the machine to the state immediately after running the boot
        // ROM.  Instead, I'm running the boot ROM and not outputting states until we leave it.
        if !self.current_machine().is_dmg_boot_rom_on() {
            let string = CPU::gbdoctor_string(self.current_machine());
            if let Some(output_file) = self.output_file.as_mut() {
                write!(output_file, "{}\n", string).expect("write to log failed");
            }
        }
        let current_machine = self.current_machine_mut();
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
                            let step = machine.step();
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
                            let step = next_machine.step();
                            executed_instruction = step.instruction_executed;
                            total_t_cycles += step.t_cycles;
                        }
                    }
                }
            }
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        Subscription::batch(vec![
            keyboard::listen().filter_map(|event| {
                let keyboard::Event::KeyPressed { key, .. } = event else {
                    return None;
                };
                match key {
                    keyboard::Key::Named(keyboard::key::Named::ArrowDown) => Some(
                        Message::BeginRunUntilBreakpoint(StepBeforeCheckingBreakpoint::Yes),
                    ),
                    keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                        Some(Message::RunNextInstruction)
                    }
                    keyboard::Key::Named(keyboard::key::Named::Space) => Some(Message::Pause),
                    keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::Quit),
                    keyboard::Key::Named(keyboard::key::Named::F5) => Some(Message::QuickSave),
                    keyboard::Key::Named(keyboard::key::Named::F9) => Some(Message::QuickLoad),
                    _ => None,
                }
            }),
            // other subscriptions possible here
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::BeginRunUntilBreakpoint(step_before_check) => {
                self.paused = false;
                if step_before_check.into() {
                    self.execute_one_instruction(PreserveHistory::DontPreserveHistory);
                }
                Task::done(Message::ContinueRunUntilBreakpoint)
            }
            Message::ContinueRunUntilBreakpoint => {
                let initial_time = time::Instant::now();

                let mut t_cycles_left_this_frame = Saturating(69_905);
                while t_cycles_left_this_frame.0 > 0
                    && !self.paused
                    && !self
                        .breakpoints
                        .contains(&self.current_machine().registers().pc.0)
                {
                    let step = self.execute_one_instruction(PreserveHistory::DontPreserveHistory);
                    t_cycles_left_this_frame -= step.t_cycles as u32;
                    // self.current_machine().ppu_mut().render();
                    // let final_frame_time = time::Instant::now() - initial_time;
                    // if final_frame_time > target_frame_time {
                    //     println!("Overslept {:?}", final_frame_time - target_frame_time);
                    // } else {
                    //     println!("Did not oversleep");
                    // }
                }

                if t_cycles_left_this_frame.0 == 0 {
                    // If we're stopping for a frame, try to get accurate frame time
                    self.current_machine_mut().ppu_mut().render();
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
            Message::MouseOnLCDPixel(x, y) => {
                self.lcd_pixel_under_mouse = (x, y);
                Task::none()
            }
            Message::MouseOnTilePalette(tile_id) => {
                self.tile_id_under_mouse = tile_id;
                Task::none()
            }
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
                self.current_machine_mut().ppu_mut().render();
                Task::none()
            }
            Message::QuickLoad => {
                let machine_or_error = read_machine();
                match machine_or_error {
                    Ok(machine) => {
                        self.snaps.clear();
                        self.snaps.push(machine);
                        println!("Quick loaded!");
                    }
                    Err(e) => {
                        println!("Could not read save state: {}", e)
                    }
                };
                Task::none()
            }
            Message::QuickSave => {
                if let Err(e) = write_machine(self.current_machine()) {
                    println!("Could not write save state: {}", e);
                } else {
                    println!("Quick saved!");
                }
                Task::none()
            }
        }
    }
}

fn read_machine() -> Result<Machine> {
    let file = File::open(SAVE_STATE_PATH)?;
    let result = ciborium::from_reader::<Machine, _>(file)?;
    Ok(result)
}

fn write_machine(machine: &Machine) -> Result<()> {
    let file = File::create(SAVE_STATE_PATH)?;
    ciborium::into_writer::<Machine, _>(machine, file)?;
    Ok(())
}
