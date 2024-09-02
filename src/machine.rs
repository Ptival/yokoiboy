use std::num::Wrapping;

use crate::{cpu::CPU, memory::Memory, ppu::PPU};

pub const EXTERNAL_RAM_SIZE: usize = 0x2000;

#[derive(Clone, Debug)]
pub struct Machine {
    pub t_cycle_count: u64,
    pub cpu: CPU,
    pub ppu: PPU,
    pub external_ram: [u8; EXTERNAL_RAM_SIZE],
    // Special registers
    pub bgp: Wrapping<u8>,
    pub dmg_boot_rom: Wrapping<u8>,
    pub interrupt_enable: Wrapping<u8>,
    pub interrupt_flag: Wrapping<u8>,
    pub nr11: Wrapping<u8>,
    pub nr12: Wrapping<u8>,
    pub nr13: Wrapping<u8>,
    pub nr14: Wrapping<u8>,
    pub nr50: Wrapping<u8>,
    pub nr51: Wrapping<u8>,
    pub nr52: Wrapping<u8>,
    pub sb: Wrapping<u8>,
    pub sc: Wrapping<u8>,
    pub scx: Wrapping<u8>,
    pub scy: Wrapping<u8>,
    pub tac: Wrapping<u8>,
}

impl Machine {
    pub fn new() -> Self {
        Machine {
            t_cycle_count: 0,
            dmg_boot_rom: Wrapping(0),
            cpu: CPU::new(),
            ppu: PPU::new(),
            bgp: Wrapping(0),
            external_ram: [0; EXTERNAL_RAM_SIZE],
            interrupt_enable: Wrapping(0),
            interrupt_flag: Wrapping(0),
            nr11: Wrapping(0),
            nr12: Wrapping(0),
            nr13: Wrapping(0),
            nr14: Wrapping(0),
            nr50: Wrapping(0),
            nr51: Wrapping(0),
            nr52: Wrapping(0),
            sb: Wrapping(0),
            sc: Wrapping(0),
            scx: Wrapping(0),
            scy: Wrapping(0),
            tac: Wrapping(0),
        }
    }

    pub fn is_dmg_boot_rom_on(&self) -> bool {
        self.dmg_boot_rom.0 == 0
    }

    pub fn read_u8(&self, address: Wrapping<u16>) -> Wrapping<u8> {
        if self.is_dmg_boot_rom_on() && address.0 <= 0xFF {
            return self.cpu.memory.read_boot_rom(address);
        }
        match address.0 {
            0x0000..=0x3FFF => self.cpu.memory.read_bank_00(address),
            0x4000..=0x7FFF => self.cpu.memory.read_bank_01(address - Wrapping(0x4000)),
            0x8000..=0x9FFF => self.ppu.read_vram(address - Wrapping(0x8000)),
            0xA000..=0xBFFF => Wrapping(self.external_ram[(address - Wrapping(0xA000)).0 as usize]),
            0xC000..=0xCFFF => self.ppu.read_wram_0(address - Wrapping(0xC000)),
            0xD000..=0xDFFF => self.ppu.read_wram_1(address - Wrapping(0xD000)),
            0xE000..=0xFDFF => self.read_u8(address - Wrapping(0x2000)),
            0xFF01..=0xFF01 => self.sb,
            0xFF02..=0xFF02 => self.sc,
            0xFF07..=0xFF07 => self.tac,
            0xFF0F..=0xFF0F => self.interrupt_flag,
            0xFF11..=0xFF11 => self.nr11,
            0xFF12..=0xFF12 => self.nr12,
            0xFF13..=0xFF13 => self.nr13,
            0xFF14..=0xFF14 => self.nr14,
            0xFF24..=0xFF24 => self.nr50,
            0xFF25..=0xFF25 => self.nr51,
            0xFF26..=0xFF26 => self.nr52,
            0xFF40..=0xFF40 => self.ppu.read_lcdc(),
            0xFF42..=0xFF42 => self.scy,
            0xFF43..=0xFF43 => self.scx,
            0xFF44..=0xFF44 => self.ppu.read_ly(),
            0xFF47..=0xFF47 => self.bgp,
            0xFF50..=0xFF50 => self.dmg_boot_rom,
            0xFF80..=0xFFFE => self.cpu.memory.read_hram(address - Wrapping(0xFF80)),
            0xFFFF..=0xFFFF => self.interrupt_enable,
            _ => panic!("Memory read at address {:04X} needs to be handled", address),
        }
    }

    pub fn read_range(&self, address: Wrapping<u16>, size: usize) -> Vec<Wrapping<u8>> {
        let address = address.0;
        let mut res = Vec::new();
        for a in address..address.saturating_add(size as u16) {
            res.push(self.read_u8(Wrapping(a)));
        }
        res
    }

    pub fn write_u8(&mut self, address: Wrapping<u16>, value: Wrapping<u8>) {
        if self.is_dmg_boot_rom_on() && address.0 <= 0xFF {
            panic!("Attempted write in boot ROM")
        }
        match address.0 {
            0x0000..=0x3FFF => Memory::write_bank_00(self, address, value),
            0x4000..=0x7FFF => Memory::write_bank_01(self, address - Wrapping(0x4000), value),
            0x8000..=0x9FFF => PPU::write_vram(&mut self.ppu, address - Wrapping(0x8000), value),
            0xA000..=0xBFFF => self.external_ram[(address - Wrapping(0xA000)).0 as usize] = value.0,
            0xC000..=0xCFFF => PPU::write_wram_0(&mut self.ppu, address - Wrapping(0xC000), value),
            0xD000..=0xDFFF => PPU::write_wram_1(&mut self.ppu, address - Wrapping(0xD000), value),
            0xFF01..=0xFF01 => self.sb = value,
            0xFF02..=0xFF02 => self.sc = value,
            0xFF07..=0xFF07 => self.tac = value,
            0xFF0F..=0xFF0F => self.interrupt_flag = value,
            0xFF11..=0xFF11 => self.nr11 = value,
            0xFF12..=0xFF12 => self.nr12 = value,
            0xFF13..=0xFF13 => self.nr13 = value,
            0xFF14..=0xFF14 => self.nr14 = value,
            0xFF24..=0xFF24 => self.nr50 = value,
            0xFF25..=0xFF25 => self.nr51 = value,
            0xFF26..=0xFF26 => self.nr52 = value,
            0xFF40..=0xFF40 => self.ppu.write_lcdc(value),
            0xFF42..=0xFF42 => self.scy = value,
            0xFF43..=0xFF43 => self.scx = value,
            0xFF47..=0xFF47 => self.bgp = value,
            0xFF50..=0xFF50 => self.dmg_boot_rom = value,
            0xFF80..=0xFFFE => Memory::write_hram(self, address - Wrapping(0xFF80), value),
            0xFFFF..=0xFFFF => self.interrupt_enable = value,
            _ => panic!(
                "Memory write at address {:04X} needs to be handled",
                address
            ),
        }
    }

    pub fn show_memory_row(&self, from: Wrapping<u16>) -> String {
        let range = self.read_range(from, 8);
        format!(
            "{:04x}: {:02X} {:02X} {:02X} {:02X}  {:02X} {:02X} {:02X} {:02X}",
            from, range[0], range[1], range[2], range[3], range[4], range[5], range[6], range[7]
        )
    }
}
