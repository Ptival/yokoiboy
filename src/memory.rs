use std::{
    cmp::min,
    io::{self, Error},
    num::Wrapping,
};

use crate::{
    instruction::decode::{decode_instruction_at_address, DecodedInstruction},
    machine::Machine,
};

const BANK_SIZE: usize = 0x4000;
const HRAM_SIZE: usize = 0x7F;

#[derive(Clone, Debug, Hash)]
pub struct Memory {
    boot_rom: [u8; 0xFF + 1],
    // DO NOT MAKE PUBLIC: we want readers to go through read_u8 to simulate DMG boot
    bank_00: [u8; BANK_SIZE],
    bank_01: [u8; BANK_SIZE],
    hram: [u8; HRAM_SIZE],
}

impl Memory {
    pub fn decode_instruction_at(
        machine: &Machine,
        address: Wrapping<u16>,
    ) -> Result<DecodedInstruction, String> {
        decode_instruction_at_address(machine, address)
    }

    pub fn decode_instructions_at(
        machine: &Machine,
        address: Wrapping<u16>,
        how_many: u8,
    ) -> Result<Vec<DecodedInstruction>, String> {
        let mut res = Vec::new();
        let mut pc = address;
        for _ in 0..how_many {
            let instr = decode_instruction_at_address(machine, pc)?;
            pc = pc + Wrapping(instr.instruction_size as u16);
            res.push(instr);
        }
        Ok(res)
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
        if byte_length > 0x8000 {
            panic!("Loading ROM larger than 0x8000 bytes not supported.");
        }
        println!("ROM byte length: {:08X}", byte_length);
        self.bank_00[0..min(BANK_SIZE, byte_length)].clone_from_slice(&bytes[..BANK_SIZE]);
        self.bank_01[0..(byte_length - BANK_SIZE)].clone_from_slice(&bytes[BANK_SIZE..]);
        Ok(self)
    }

    pub fn new() -> Self {
        Memory {
            boot_rom: [0; 0xFF + 1],
            bank_00: [0; BANK_SIZE],
            bank_01: [0; BANK_SIZE],
            hram: [0; HRAM_SIZE],
        }
    }

    pub fn read_boot_rom(&self, address: Wrapping<u16>) -> Wrapping<u8> {
        Wrapping(self.boot_rom[address.0 as usize])
    }

    pub fn read_bank_00(&self, address: Wrapping<u16>) -> Wrapping<u8> {
        Wrapping(self.bank_00[address.0 as usize])
    }

    pub fn read_bank_01(&self, address: Wrapping<u16>) -> Wrapping<u8> {
        Wrapping(self.bank_01[address.0 as usize])
    }

    pub fn read_hram(&self, address: Wrapping<u16>) -> Wrapping<u8> {
        Wrapping(self.hram[address.0 as usize])
    }

    // fn rread_u8(machine: &Machine, address: Wrapping<u16>) -> Wrapping<u8> {
    //     let mem = &machine.cpu.memory;
    //     if address <= 0xFF && machine.is_dmg_boot_rom_on() {
    //         mem.read_boot_rom(address);
    //     }
    //     match address.0 as usize {
    //         0x0000..=0x3FFF => mem.read_bank_00(address),
    //         0x4000..=0x7FFF => mem.read_bank_01(address - 0x4000),
    //         0xFF80..=0xFFFE => mem.read_hram(address - 0xFF80),
    //         _ => {
    //             panic!(
    //                 "Memory was asked to read address {:04X} outside its range",
    //                 address
    //             )
    //         }
    //     }
    // }

    // pub fn read_range(&self, machine: &Machine, address: Wrapping<u16>, size: usize) -> Vec<u8> {
    //     let mut res = Vec::new();
    //     for a in address..address.saturating_add(size as u16) {
    //         res.push(machine.read_u8(a));
    //     }
    //     res
    // }

    pub fn write_u8(machine: &mut Machine, address: Wrapping<u16>, value: u8) -> &mut Machine {
        if address.0 <= 0xFF {
            if machine.is_dmg_boot_rom_on() {
                panic!("Program is attempting to write in DMG boot ROM")
            }
        }
        match address.0 as usize {
            0x0000..=0x3FFF => {
                machine.cpu.memory.bank_00[address.0 as usize] = value;
            }
            0x4000..=0x7FFF => {
                machine.cpu.memory.bank_01[(address.0 - 0x4000) as usize];
            }
            0xFF80..=0xFFFE => {
                machine.cpu.memory.hram[(address.0 - 0xFF80) as usize];
            }
            _ => {
                panic!(
                    "Memory was asked to write address {:04X} outside its range",
                    address
                )
            }
        }
        machine
    }

    pub fn write_bank_00(machine: &mut Machine, address: Wrapping<u16>, value: Wrapping<u8>) {
        machine.cpu.memory.bank_00[address.0 as usize] = value.0;
    }

    pub fn write_bank_01(machine: &mut Machine, address: Wrapping<u16>, value: Wrapping<u8>) {
        machine.cpu.memory.bank_01[address.0 as usize] = value.0;
    }

    pub fn write_hram(machine: &mut Machine, address: Wrapping<u16>, value: Wrapping<u8>) {
        machine.cpu.memory.hram[address.0 as usize] = value.0;
    }
}
