use std::io::{self, Error};

use crate::instruction::decode::{decode_instruction_at_address, DecodedInstruction};

#[derive(Clone, Debug, Hash)]
pub struct Memory {
    boot_rom: [u8; 0xFF + 1],
    // DO NOT MAKE PUBLIC: we want readers to go through read_u8 to simulate DMG boot
    raw: [u8; 0xFFFF + 1],
}

impl Memory {
    pub fn decode_instruction_at(self: &Self, address: u16) -> Result<DecodedInstruction, String> {
        decode_instruction_at_address(&self, address)
    }

    pub fn decode_instructions_at(
        self: &Self,
        address: u16,
        how_many: u8,
    ) -> Result<Vec<DecodedInstruction>, String> {
        let mut res = Vec::new();
        let mut pc = address;
        for _ in 0..how_many {
            let instr = decode_instruction_at_address(&self, pc)?;
            pc = pc + instr.instruction_size as u16;
            res.push(instr);
        }
        Ok(res)
    }

    pub fn is_dmg_boot_rom_on(&self) -> bool {
        self.read_u8(0xFF50) == 0
    }

    pub fn load_boot_rom(&mut self, path: String) -> Result<&mut Self, io::Error> {
        let bytes = std::fs::read(path)?;
        let byte_length = bytes.len();
        if byte_length > 0x100 {
            return Err(Error::other(
                "Refusing to load a boot ROM larger than 0xFF bytes.",
            ));
        }
        self.boot_rom[0..byte_length].clone_from_slice(&bytes);
        Ok(self)
    }

    pub fn load_rom(self: &mut Self, path: String) -> Result<&mut Self, io::Error> {
        let bytes = std::fs::read(path)?;
        let byte_length = bytes.len();
        self.raw[0..byte_length].clone_from_slice(&bytes);
        Ok(self)
    }

    pub fn new() -> Self {
        Memory {
            boot_rom: [0; 0xFF + 1],
            raw: [0; 0xFFFF + 1],
        }
    }

    pub fn read_u8(&self, address: u16) -> u8 {
        if address <= 0xFF {
            if self.is_dmg_boot_rom_on() {
                return self.boot_rom[address as usize];
            }
        }
        self.raw[address as usize]
    }

    pub fn read_slice(&self, address: u16, size: usize) -> &[u8] {
        let address = address as usize;
        if self.is_dmg_boot_rom_on() {
            if address <= 0xFF && (address + size - 1) > 0xFF {
                panic!("Cannot return a slice overlapping the DMG boot ROM and the main memory")
            }
            if address + size <= 0xFF {
                return &self.boot_rom[address..address + size];
            }
        }
        &self.raw[address..address + size]
    }

    pub fn write_u8(&mut self, address: u16, value: u8) -> &Self {
        if address == 0xFF01 || address == 0xFF02 {
            println!("Writing {:02X} at {:04X}", value, address)
        }
        self.raw[address as usize] = value;
        self
    }

    // Note: So far I used this for CALL, where the **higher** byte of the return address goes to
    // the **higher** address!
    // pub fn write_imm16(&mut self, address: u16, value: Immediate16) -> &Self {
    //     let address = address as usize;
    //     self.raw[address + 1] = value.higher_byte;
    //     self.raw[address] = value.lower_byte;
    //     self
    // }

    pub fn show_memory_row(&self, from: u16) -> String {
        if from as usize + 7 >= self.raw.len() {
            return String::from("TODO"); // We can still display a bit
        }
        let slice = self.read_slice(from, 8);
        format!(
            "{:04x}: {:02X} {:02X} {:02X} {:02X}  {:02X} {:02X} {:02X} {:02X}",
            from, slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7]
        )
    }
}

pub fn read_u16(m: &[u8]) -> u16 {
    (m[0] as u16) << 8 | m[1] as u16
}
