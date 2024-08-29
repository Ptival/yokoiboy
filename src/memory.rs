use std::io::{self, Error};

use crate::opcodes::{decode_instruction_at_address, DecodedInstruction};

#[derive(Clone, Debug, Hash)]
pub struct Memory {
    pub raw: [u8; 0xFFFF],
}

impl Memory {
    pub fn decode_instruction_at(self: &Self, address: u16) -> Result<DecodedInstruction, String> {
        decode_instruction_at_address(address, &self.raw, None)
    }

    pub fn decode_instructions_at(
        self: &Self,
        address: u16,
        how_many: u8,
    ) -> Result<Vec<DecodedInstruction>, String> {
        let mut res = Vec::new();
        let mut pc = address;
        for _ in 0..how_many {
            let instr = decode_instruction_at_address(pc, &self.raw, None)?;
            pc = pc + instr.instruction_size as u16;
            res.push(instr);
        }
        Ok(res)
    }

    pub fn load_boot_rom(self: &mut Self, path: String) -> Result<(), io::Error> {
        let bytes = std::fs::read(path)?;
        let byte_length = bytes.len();
        if byte_length > 0x100 {
            return Err(Error::other(
                "Refusing to load a boot ROM larger than 0xFF bytes.",
            ));
        }
        self.raw[0..byte_length].clone_from_slice(&bytes);
        Ok(())
    }

    pub fn new() -> Self {
        Memory { raw: [0; 0xFFFF] }
    }

    pub fn write_u8(&mut self, address: u16, value: u8) {
        self.raw[address as usize] = value;
    }

    pub fn show_memory_row(&self, from: u16) -> String {
        let from = from as usize;
        let raw = self.raw;
        if from + 7 >= raw.len() {
            return String::from("TODO"); // We can still display a bit
        }
        format!(
            "{:08x}: {:2X} {:2X} {:2X} {:2X}  {:2X} {:2X} {:2X} {:2X}",
            0,
            raw[from + 0],
            raw[from + 1],
            raw[from + 2],
            raw[from + 3],
            raw[from + 4],
            raw[from + 5],
            raw[from + 6],
            raw[from + 7]
        )
    }
}

pub fn read_u16(m: &[u8]) -> u16 {
    (m[0] as u16) << 8 | m[1] as u16
}
