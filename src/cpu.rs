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

    pub fn execute_one_instruction(&self) -> Result<CPU, String> {
        let mut cpu = self.clone();
        let next_instruction = decode_instruction_at_address(&cpu.memory, cpu.registers.pc)?;
        // This will be the default PC, unless instruction semantics overwrite it
        cpu.registers.pc = cpu.registers.pc + next_instruction.instruction_size as u16;
        next_instruction.instruction.execute(&mut cpu);
        Ok(cpu)
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
}
