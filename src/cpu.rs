use crate::{
    instruction::{decode::decode_instruction_at_address, type_def::Immediate16},
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

    pub fn execute_one_instruction(&mut self) -> Result<(u8, u8), String> {
        let next_instruction = decode_instruction_at_address(&self.memory, self.registers.pc)?;
        // This will be the default PC, unless instruction semantics overwrite it
        self.registers.pc = self.registers.pc + next_instruction.instruction_size as u16;
        Ok(next_instruction.instruction.execute(self))
    }

    pub fn pop_r16(&mut self, r16: &R16) -> &Self {
        let lower = self.memory.read_u8(self.registers.sp);
        self.registers.sp += 1;
        let higher = self.memory.read_u8(self.registers.sp);
        self.registers.sp += 1;
        let imm16 = Immediate16 {
            lower_byte: lower,
            higher_byte: higher,
        };
        self.registers.write_r16(r16, imm16.as_u16());
        self
    }

    // Note: pushes the higher byte goes to higher address!!!
    pub fn push_imm16(&mut self, imm16: Immediate16) -> &Self {
        self.registers.sp -= 1;
        self.memory.write_u8(self.registers.sp, imm16.higher_byte);
        self.registers.sp -= 1;
        self.memory.write_u8(self.registers.sp, imm16.lower_byte);
        self
    }

    pub fn log_string(&self) -> String {
        let mut res = String::new();
        res.push_str(&format!("A: {:02X} ", self.registers.read_a()));
        res.push_str(&format!("F: {:02X} ", self.registers.read_f()));
        res.push_str(&format!("B: {:02X} ", self.registers.read_b()));
        res.push_str(&format!("C: {:02X} ", self.registers.read_c()));
        res.push_str(&format!("D: {:02X} ", self.registers.read_d()));
        res.push_str(&format!("E: {:02X} ", self.registers.read_e()));
        res.push_str(&format!("H: {:02X} ", self.registers.read_h()));
        res.push_str(&format!("L: {:02X} ", self.registers.read_l()));
        res.push_str(&format!("SP: {:04X} ", self.registers.sp));
        let pc = self.registers.pc;
        let mem = &self.memory;
        res.push_str(&format!("PC: 00:{:04X} ", pc));
        res.push_str(&format!(
            "({:02X} {:02X} {:02X} {:02X})",
            mem.read_u8(pc),
            mem.read_u8(pc + 1),
            mem.read_u8(pc + 2),
            mem.read_u8(pc + 3)
        ));
        res
    }
}
