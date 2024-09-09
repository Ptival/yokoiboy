use std::{
    io::{self, Error},
    num::Wrapping,
};

use crate::{
    application_state::{MapperType, RAMSize, ROMInformation},
    instructions::decode::{decode_instruction_at_address, DecodedInstruction},
    machine::Machine,
};

const HRAM_SIZE: usize = 0x7F;

#[derive(Clone, Debug, Hash)]
pub struct Memory {
    boot_rom: Vec<u8>,
    pub game_rom: Vec<u8>,
    pub game_ram: Vec<u8>,
    pub hram: [u8; HRAM_SIZE],
}

impl Memory {
    pub fn decode_instruction_at(machine: &Machine, address: Wrapping<u16>) -> DecodedInstruction {
        decode_instruction_at_address(machine, address)
    }

    pub fn decode_instructions_at(
        machine: &Machine,
        address: Wrapping<u16>,
        how_many: u8,
    ) -> Vec<DecodedInstruction> {
        let mut res = Vec::new();
        let mut pc = address;
        for _ in 0..how_many {
            let instr = decode_instruction_at_address(machine, pc);
            pc = pc + Wrapping(instr.instruction_size as u16);
            res.push(instr);
        }
        res
    }

    pub fn new(boot_rom: Vec<u8>, game_rom: Vec<u8>, rom_information: &ROMInformation) -> Self {
        let game_ram = match rom_information.ram_size {
            RAMSize::NoRAM => Vec::new(),
            RAMSize::Ram2kb => Vec::from([0; 0x800]),
            RAMSize::Ram8kb => Vec::from([0; 0x2000]),
            RAMSize::Ram4banks8kb => todo!(),
            RAMSize::Ram16banks8kb => todo!(),
            RAMSize::Ram8banks8kb => todo!(),
        };
        Memory {
            boot_rom,
            game_rom,
            game_ram,
            hram: [0; HRAM_SIZE],
        }
    }

    pub fn read_boot_rom(&self, address: Wrapping<u16>) -> Wrapping<u8> {
        Wrapping(self.boot_rom[address.0 as usize])
    }
}

// TODO: move somewhere
pub fn load_boot_rom(path: &String) -> Result<Vec<u8>, io::Error> {
    let bytes = std::fs::read(path)?;
    let byte_length = bytes.len();
    if byte_length > 0x100 {
        return Err(Error::other(
            "Refusing to load a boot ROM larger than 0xFF bytes.",
        ));
    }
    Ok(bytes)
}

pub fn load_game_rom(path: &String) -> Result<(Vec<u8>, ROMInformation), io::Error> {
    let bytes = std::fs::read(path)?;
    let byte_length = bytes.len();
    if byte_length > 0x8000 {
        println!("[WARNING] ROM larger than 0x8000 bytes, errors may occur.");
    }

    // Now compute ROM information
    let mapper_type = match bytes[0x147] {
        0x00 => MapperType::ROMOnly,
        0x01..=0x03 => MapperType::MBC1,
        byte => {
            println!("Unhandled mapper type: 0x{:02X}", byte);
            MapperType::Other
        }
    };
    let rom_banks = match bytes[0x148] {
        0x00 => 0,
        0x01 => 4,
        0x02 => 8,
        0x03 => 16,
        0x04 => 32,
        byte => panic!("Unhandled ROM bank size: 0x{:02X}", byte),
    };
    let ram_size = match bytes[0x149] {
        0x00 => RAMSize::NoRAM,
        0x01 => RAMSize::Ram2kb,
        0x02 => RAMSize::Ram8kb,
        0x03 => RAMSize::Ram4banks8kb,
        0x04 => RAMSize::Ram16banks8kb,
        0x05 => RAMSize::Ram8banks8kb,
        byte => panic!("Unhandled RAM size: 0x{:02X}", byte),
    };

    Ok((
        bytes,
        ROMInformation {
            mapper_type,
            ram_size,
            rom_banks,
        },
    ))
}
