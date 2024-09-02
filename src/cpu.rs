use std::num::Wrapping;

use crate::{
    instructions::{decode::decode_instruction_at_address, type_def::Immediate16},
    machine::Machine,
    memory::Memory,
    registers::{Registers, R16},
};

#[derive(Clone, Debug, Hash)]
pub struct CPU {
    pub memory: Memory,
    pub registers: Registers,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            memory: Memory::new(),
            registers: Registers::new(),
        }
    }

    pub fn execute_one_instruction(machine: &mut Machine) -> (u8, u8) {
        let next_instruction = decode_instruction_at_address(machine, machine.cpu.registers.pc);
        // This will be the default PC, unless instruction semantics overwrite it
        machine.cpu.registers.pc =
            machine.cpu.registers.pc + Wrapping(next_instruction.instruction_size as u16);
        next_instruction.instruction.execute(machine)
    }

    pub fn pop_r16<'a>(machine: &'a mut Machine, r16: &R16) -> &'a mut Machine {
        let lower = machine.read_u8(machine.cpu.registers.sp);
        machine.cpu.registers.sp += 1;
        let higher = machine.read_u8(machine.cpu.registers.sp);
        machine.cpu.registers.sp += 1;
        let imm16 = Immediate16 {
            lower_byte: lower,
            higher_byte: higher,
        };
        machine.cpu.registers.write_r16(r16, imm16.as_u16());
        machine
    }

    // Note: pushes the higher byte goes to higher address!!!
    pub fn push_imm16(machine: &mut Machine, imm16: Immediate16) -> &mut Machine {
        machine.cpu.registers.sp -= 1;
        machine.write_u8(machine.cpu.registers.sp, imm16.higher_byte);
        machine.cpu.registers.sp -= 1;
        machine.write_u8(machine.cpu.registers.sp, imm16.lower_byte);
        machine
    }

    pub fn gbdoctor_string(machine: &Machine) -> String {
        let cpu = &machine.cpu;
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
}
