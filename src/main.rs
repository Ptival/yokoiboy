use std::fmt;

use opcodes::{decode_next_instruction, DecodedInstruction};

pub mod opcodes;

impl fmt::Display for DecodedInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:02X} {:?}", self.address, self.instruction)
    }
}

fn main() {
    let boot_ROM_bytes = std::fs::read("dmg_boot.bin").unwrap();
    let mut pc: usize = 0;
    while (pc < boot_ROM_bytes.len()) {
        let decoded = decode_next_instruction(pc as u16, &boot_ROM_bytes[pc..]);
        pc += decoded.instruction_size as usize;
        println!("{}", decoded)
    }
}
