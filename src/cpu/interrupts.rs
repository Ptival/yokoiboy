use std::{fmt, num::Wrapping};

use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{
    instructions::{semantics::ElapsedCycles, type_def::Immediate16},
    machine::Machine,
};

use super::CPU;

pub const VBLANK_INTERRUPT_BIT: u8 = 0;
const VBLANK_INTERRUPT_ADDRESS: u16 = 0x40;
pub const STAT_INTERRUPT_BIT: u8 = 1;
const STAT_INTERRUPT_ADDRESS: u16 = 0x48;
pub const TIMER_INTERRUPT_BIT: u8 = 2;
const TIMER_INTERRUPT_ADDRESS: u16 = 0x50;
pub const SERIAL_INTERRUPT_BIT: u8 = 3;
const SERIAL_INTERRUPT_ADDRESS: u16 = 0x58;
pub const JOYPAD_INTERRUPT_BIT: u8 = 4;
const JOYPAD_INTERRUPT_ADDRESS: u16 = 0x60;

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
pub struct Interrupts {
    pub interrupt_master_enable: bool,
    pub interrupt_master_enable_delayed: bool,
    pub interrupt_enable: Wrapping<u8>,
    pub interrupt_flag: Wrapping<u8>,
}

pub struct Interrupt {
    pub bit_index: u8,
}

impl fmt::Display for Interrupt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.bit_index {
            0 => write!(f, "VBlank"),
            1 => write!(f, "LCD"),
            2 => write!(f, "Timer"),
            3 => write!(f, "Serial"),
            4 => write!(f, "Joypad"),
            _ => unreachable!(),
        }
    }
}

fn interrupt_handler_offset(interrupt_bit: u8) -> Wrapping<u16> {
    Wrapping(match interrupt_bit {
        VBLANK_INTERRUPT_BIT => VBLANK_INTERRUPT_ADDRESS,
        STAT_INTERRUPT_BIT => STAT_INTERRUPT_ADDRESS,
        TIMER_INTERRUPT_BIT => TIMER_INTERRUPT_ADDRESS,
        SERIAL_INTERRUPT_BIT => SERIAL_INTERRUPT_ADDRESS,
        JOYPAD_INTERRUPT_BIT => JOYPAD_INTERRUPT_ADDRESS,
        _ => unreachable!(),
    })
}

impl Interrupts {
    pub fn new() -> Self {
        Interrupts {
            interrupt_master_enable: false,
            interrupt_master_enable_delayed: false,
            interrupt_enable: Wrapping(0),
            interrupt_flag: Wrapping(0),
        }
    }

    pub fn handle_interrupts(machine: &mut Machine) -> ElapsedCycles {
        if let Some(interrupt) = machine.interrupts.should_handle_interrupt() {
            event!(
                Level::DEBUG,
                "  Handling {} interrupt on T-cycle {}",
                interrupt,
                machine.t_cycle_count
            );
            event!(
                Level::DEBUG,
                "Interrupt flag before handling: {:05b}",
                machine.interrupts.interrupt_flag
            );
            machine.interrupts.interrupt_flag =
                machine.interrupts.interrupt_flag & Wrapping(!(1 << interrupt.bit_index));
            event!(
                Level::DEBUG,
                "Interrupt flag after handling: {:05b}",
                machine.interrupts.interrupt_flag
            );
            machine.interrupts.interrupt_master_enable = false;
            // Here the CPU:
            // - NOPs twice (2 M-cycles)
            // - PUSHes PC (2 M-cycles)
            // - sets PC to the handle (1 M-cycle)
            // Currently simulating this whole thing at once, but might need granularity
            CPU::push_imm16(machine, Immediate16::from_u16(machine.cpu().registers.pc));
            machine.cpu_mut().registers.pc = interrupt_handler_offset(interrupt.bit_index);
            ElapsedCycles { m_cycles: 5 }
        } else {
            ElapsedCycles { m_cycles: 0 }
        }
    }

    pub fn is_interrupt_pending(&self) -> bool {
        let masked_ie = self.interrupt_enable.0 & 0x1F;
        let masked_if = self.interrupt_flag.0 & 0x1F;
        (masked_ie & masked_if) != 0
    }

    fn request(&mut self, t_cycle_count: u64, interrupt: &Interrupt) {
        self.interrupt_flag |= 1 << interrupt.bit_index;
        event!(
            Level::DEBUG,
            "Requesting {} interrupt on T-cycle {}.  IF:{:05b} IE:{:05b}",
            interrupt,
            t_cycle_count,
            self.interrupt_flag,
            self.interrupt_enable
        );
    }

    pub fn request_stat(&mut self, t_cycle_count: u64) {
        self.request(
            t_cycle_count,
            &Interrupt {
                bit_index: STAT_INTERRUPT_BIT,
            },
        );
    }

    pub fn request_timer(&mut self, t_cycle_count: u64) {
        self.request(
            t_cycle_count,
            &Interrupt {
                bit_index: TIMER_INTERRUPT_BIT,
            },
        );
    }

    pub fn request_vblank(&mut self, t_cycle_count: u64) {
        self.request(
            t_cycle_count,
            &Interrupt {
                bit_index: VBLANK_INTERRUPT_BIT,
            },
        );
    }

    fn get_highest_priority_interrupt(&self) -> Option<Interrupt> {
        // We only want the top 5 bits of IE and IF registers
        let masked_ie = self.interrupt_enable.0 & 0x1F;
        let masked_if = self.interrupt_flag.0 & 0x1F;
        let conjoined = masked_ie & masked_if;
        // 0 has most priority, 4 has least
        for i in 0..5 {
            let mask = 1 << i;
            if (conjoined & mask) == mask {
                return Some(Interrupt { bit_index: i });
            }
        }
        None
    }

    // Returns the bit index of the interrupt to handle (0 = VBlank... 4 = Joypad)
    fn should_handle_interrupt(&self) -> Option<Interrupt> {
        if let Some(i) = self.get_highest_priority_interrupt() {
            if !self.interrupt_master_enable {
                return None;
            }
            return Some(i);
        }
        return None;
    }
}

impl Machine {
    pub fn interrupts(&self) -> &Interrupts {
        &self.interrupts
    }
    pub fn interrupts_mut(&mut self) -> &mut Interrupts {
        &mut self.interrupts
    }
}
