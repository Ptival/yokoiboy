use crate::{memory::Memory, opcodes::decode_instruction_at_address, registers::Registers};

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

    pub fn execute_one_instruction(self: &CPU) -> Result<CPU, String> {
        let mut cpu = self.clone();
        let next_instruction =
            decode_instruction_at_address(cpu.registers.pc, &cpu.memory.raw, None)?;
        // This will be the default PC, unless instruction semantics overwrite it
        cpu.registers.pc = cpu.registers.pc + next_instruction.instruction_size as u16;
        next_instruction.instruction.execute(&mut cpu);
        Ok(cpu)
    }
}
