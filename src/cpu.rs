pub mod interrupts;
pub mod timers;

use std::num::Wrapping;

use interrupts::Interrupts;
use timers::Timers;

use crate::{
    instructions::{decode::decode_instruction_at_address, type_def::Immediate16},
    machine::Machine,
    memory::Memory,
    registers::{Registers, R16},
};

#[derive(Clone, Debug, Hash)]
pub struct CPU {
    // CPU state
    pub low_power_mode: bool,

    // Subsystems
    interrupts: Interrupts,
    memory: Memory,
    registers: Registers,
    timers: Timers,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            low_power_mode: false,
            interrupts: Interrupts::new(),
            memory: Memory::new(),
            registers: Registers::new(),
            timers: Timers::new(),
        }
    }

    pub fn execute_one_instruction(machine: &mut Machine) -> (u8, u8) {
        if machine.cpu_mut().low_power_mode {
            if Interrupts::is_interrupt_pending(machine) {
                machine.cpu_mut().low_power_mode = false;
                // Fall through on wakeup to execute one instruction
            } else {
                // Otherwise, force the other components to move forward
                return (4, 1);
            }
        }
        let next_instruction = decode_instruction_at_address(machine, machine.cpu().registers.pc);
        // This will be the default PC, unless instruction semantics overwrite it
        machine.cpu_mut().registers.pc =
            machine.cpu_mut().registers.pc + Wrapping(next_instruction.instruction_size as u16);
        next_instruction.instruction.execute(machine)
    }

    pub fn pop_r16<'a>(machine: &'a mut Machine, r16: &R16) -> &'a mut Machine {
        let lower = machine.read_u8(machine.cpu().registers.sp);
        machine.cpu_mut().registers.sp += 1;
        let higher = machine.read_u8(machine.cpu().registers.sp);
        machine.cpu_mut().registers.sp += 1;
        let imm16 = Immediate16 {
            lower_byte: lower,
            higher_byte: higher,
        };
        machine.cpu_mut().registers.write_r16(r16, imm16.as_u16());
        machine
    }

    // Note: pushes the higher byte goes to higher address!!!
    pub fn push_imm16(machine: &mut Machine, imm16: Immediate16) -> &mut Machine {
        machine.cpu_mut().registers.sp -= 1;
        machine.write_u8(machine.cpu().registers.sp, imm16.higher_byte);
        machine.cpu_mut().registers.sp -= 1;
        machine.write_u8(machine.cpu().registers.sp, imm16.lower_byte);
        machine
    }

    pub fn gbdoctor_string(machine: &Machine) -> String {
        let cpu = &machine.cpu();
        let mut res = String::new();
        res.push_str(&format!("A:{:02X} ", cpu.registers.read_a()));
        res.push_str(&format!("F:{:02X} ", cpu.registers.read_f()));
        res.push_str(&format!("B:{:02X} ", cpu.registers.read_b()));
        res.push_str(&format!("C:{:02X} ", cpu.registers.read_c()));
        res.push_str(&format!("D:{:02X} ", cpu.registers.read_d()));
        res.push_str(&format!("E:{:02X} ", cpu.registers.read_e()));
        res.push_str(&format!("H:{:02X} ", cpu.registers.read_h()));
        res.push_str(&format!("L:{:02X} ", cpu.registers.read_l()));
        res.push_str(&format!("SP:{:04X} ", cpu.registers.sp));
        let pc = cpu.registers.pc;
        res.push_str(&format!("PC:{:04X} ", pc));
        res.push_str(&format!(
            "PCMEM:{:02X},{:02X},{:02X},{:02X}",
            machine.read_u8(pc),
            machine.read_u8(pc + Wrapping(1)),
            machine.read_u8(pc + Wrapping(2)),
            machine.read_u8(pc + Wrapping(3))
        ));
        res
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }

    pub fn registers(&self) -> &Registers {
        &self.registers
    }

    pub fn registers_mut(&mut self) -> &mut Registers {
        &mut self.registers
    }
}

impl Machine {
    pub fn memory(&self) -> &Memory {
        &self.cpu().memory
    }

    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.cpu_mut().memory
    }

    pub fn registers(&self) -> &Registers {
        &self.cpu().registers
    }

    pub fn registers_mut(&mut self) -> &mut Registers {
        &mut self.cpu_mut().registers
    }
}
