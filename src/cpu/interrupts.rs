use std::num::Wrapping;

use crate::{instructions::type_def::Immediate16, machine::Machine};

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

#[derive(Clone, Debug, Hash)]
pub struct Interrupts {
    pub interrupt_master_enable: bool,
    pub interrupt_master_enable_delayed: bool,
    pub interrupt_enable: Wrapping<u8>,
    pub interrupt_flag: Wrapping<u8>,
}

// Returns the bit index of the interrupt to handle (0 = VBlank... 4 = Joypad)
fn should_handle_interrupt(machine: &mut Machine) -> Option<u8> {
    if !machine.cpu.interrupts.interrupt_master_enable {
        return None;
    }
    let masked_ie = machine.cpu.interrupts.interrupt_enable.0 & 0x1F;
    let masked_if = machine.cpu.interrupts.interrupt_flag.0 & 0x1F;
    let conjoined = masked_ie & masked_if;
    // 0 has most priority, 4 has least
    for i in 0..5 {
        let mask = 1 << i;
        if (conjoined & mask) == mask {
            return Some(i);
        }
    }
    None
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

    pub fn handle_interrupts(machine: &mut Machine) -> (u8, u8) {
        if let Some(interrupt) = should_handle_interrupt(machine) {
            machine.cpu.interrupts.interrupt_flag =
                machine.cpu.interrupts.interrupt_flag & Wrapping(!(1 << interrupt));
            machine.cpu.interrupts.interrupt_master_enable = false;
            // Here the CPU:
            // - NOPs twice (2 M-cycles)
            // - PUSHes PC (2 M-cycles)
            // - sets PC to the handle (1 M-cycle)
            // Currently simulating this whole thing at once, but might need granularity
            CPU::push_imm16(machine, Immediate16::from_u16(machine.cpu.registers.pc));
            machine.cpu.registers.pc = interrupt_handler_offset(interrupt);
            // Execute the first instruction of the interrupt handler to match GB doctor
            let (t_cycles, m_cycles) = CPU::execute_one_instruction(machine);
            (20 + t_cycles, 5 + m_cycles)
        } else {
            (0, 0)
        }
    }

    pub fn is_interrupt_pending(machine: &Machine) -> bool {
        let masked_ie = machine.cpu.interrupts.interrupt_enable.0 & 0x1F;
        let masked_if = machine.cpu.interrupts.interrupt_flag.0 & 0x1F;
        (masked_ie & masked_if) != 0
    }

    pub fn request_interrupt(&mut self, interrupt_bit: u8) {
        self.interrupt_flag |= 1 << interrupt_bit;
    }
}
