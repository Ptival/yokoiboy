use std::num::Wrapping;

use crate::{
    application_state::{MapperType, ROMInformation},
    cpu::{interrupts::Interrupts, timers::Timers, CPU},
    inputs::Inputs,
    pixel_fetcher::{
        background_or_window::BackgroundOrWindowFetcher, object::ObjectFetcher, Fetcher,
    },
    ppu::PPU,
};

#[derive(Clone, Debug, PartialEq)]
enum BankingMode {
    Ram,
    Rom,
}

// TODO: separate MMU from Machine?

#[derive(Clone, Debug)]
pub struct Machine {
    // Machine state
    banking_mode: BankingMode,
    pub is_ram_enabled: bool,
    pub loram_bank: u8,
    pub ram_or_hiram_bank: u8,
    pub rom_information: ROMInformation,
    pub t_cycle_count: u64,

    // Subsystems
    pub background_window_fetcher: BackgroundOrWindowFetcher,
    pub cpu: CPU,
    pub inputs: Inputs,
    pub interrupts: Interrupts,
    pub object_fetcher: ObjectFetcher,
    pub pixel_fetcher: Fetcher,
    pub ppu: PPU,
    pub timers: Timers,

    // Special registers
    pub dmg_boot_rom: Wrapping<u8>,

    // TODO: These should go in audio or other modules
    pub nr10: Wrapping<u8>,
    pub nr11: Wrapping<u8>,
    pub nr12: Wrapping<u8>,
    pub nr13: Wrapping<u8>,
    pub nr14: Wrapping<u8>,

    pub nr21: Wrapping<u8>,
    pub nr22: Wrapping<u8>,
    pub nr23: Wrapping<u8>,
    pub nr24: Wrapping<u8>,

    pub nr30: Wrapping<u8>,
    pub nr31: Wrapping<u8>,
    pub nr32: Wrapping<u8>,
    pub nr33: Wrapping<u8>,
    pub nr34: Wrapping<u8>,

    pub nr50: Wrapping<u8>,
    pub nr51: Wrapping<u8>,
    pub nr52: Wrapping<u8>,

    pub register_ff03: Wrapping<u8>,
    pub register_ff08: Wrapping<u8>,
    pub register_ff09: Wrapping<u8>,
    pub register_ff15: Wrapping<u8>,
    pub register_ff1f: Wrapping<u8>,
    pub register_ff20: Wrapping<u8>,
    pub register_ff21: Wrapping<u8>,
    pub register_ff22: Wrapping<u8>,
    pub register_ff23: Wrapping<u8>,
    pub slice_ff27_ff2f: [Wrapping<u8>; 9],
    pub slice_ff30_ff3f: [Wrapping<u8>; 16],
    pub register_ff0a: Wrapping<u8>,
    pub register_ff0b: Wrapping<u8>,
    pub register_ff0c: Wrapping<u8>,
    pub register_ff0d: Wrapping<u8>,
    pub register_ff0e: Wrapping<u8>,
    pub register_ff4d: Wrapping<u8>,
    pub register_ff72: Wrapping<u8>,
    pub register_ff73: Wrapping<u8>,
    pub register_ff75: Wrapping<u8>,

    // TODO: move these in PPU?
    pub sb: Wrapping<u8>,
    pub sc: Wrapping<u8>,
    pub wram_bank: Wrapping<u8>,
}

impl Machine {
    pub fn new(
        boot_rom: Vec<u8>,
        game_rom: Vec<u8>,
        rom_information: ROMInformation,
        fix_ly: bool,
    ) -> Self {
        let cpu = CPU::new(boot_rom, game_rom, &rom_information);
        Machine {
            banking_mode: BankingMode::Rom,
            is_ram_enabled: false,
            loram_bank: 1,
            ram_or_hiram_bank: 0,
            rom_information,
            t_cycle_count: 0,
            dmg_boot_rom: Wrapping(0),

            background_window_fetcher: BackgroundOrWindowFetcher::new(),
            cpu,
            inputs: Inputs::new(),
            interrupts: Interrupts::new(),
            object_fetcher: ObjectFetcher::new(),
            pixel_fetcher: Fetcher::new(),
            ppu: PPU::new(fix_ly),
            timers: Timers::new(),

            nr10: Wrapping(0),
            nr11: Wrapping(0),
            nr12: Wrapping(0),
            nr13: Wrapping(0),
            nr14: Wrapping(0),

            nr21: Wrapping(0),
            nr22: Wrapping(0),
            nr23: Wrapping(0),
            nr24: Wrapping(0),

            nr30: Wrapping(0),
            nr31: Wrapping(0),
            nr32: Wrapping(0),
            nr33: Wrapping(0),
            nr34: Wrapping(0),

            nr50: Wrapping(0),
            nr51: Wrapping(0),
            nr52: Wrapping(0),

            register_ff03: Wrapping(0),
            register_ff08: Wrapping(0),
            register_ff09: Wrapping(0),
            register_ff15: Wrapping(0),
            register_ff1f: Wrapping(0),
            register_ff20: Wrapping(0),
            register_ff21: Wrapping(0),
            register_ff22: Wrapping(0),
            register_ff23: Wrapping(0),
            slice_ff27_ff2f: [Wrapping(0); 9],
            slice_ff30_ff3f: [Wrapping(0); 16],
            register_ff0a: Wrapping(0),
            register_ff0b: Wrapping(0),
            register_ff0c: Wrapping(0),
            register_ff0d: Wrapping(0),
            register_ff0e: Wrapping(0),
            register_ff4d: Wrapping(0),
            register_ff72: Wrapping(0),
            register_ff73: Wrapping(0),
            register_ff75: Wrapping(0),

            sb: Wrapping(0),
            sc: Wrapping(0),
            wram_bank: Wrapping(0),
        }
    }

    pub fn is_dmg_boot_rom_on(&self) -> bool {
        self.dmg_boot_rom.0 == 0
    }

    pub fn read_u8(&self, address: Wrapping<u16>) -> Wrapping<u8> {
        if self.is_dmg_boot_rom_on() && address.0 <= 0xFF {
            return self.memory().read_boot_rom(address);
        }
        match address.0 {
            0x0000..=0x3FFF => Wrapping(self.memory().game_rom[address.0 as usize]),
            0x4000..=0x7FFF => match self.rom_information.mapper_type {
                crate::application_state::MapperType::ROMOnly => {
                    Wrapping(self.memory().game_rom[address.0 as usize])
                }
                crate::application_state::MapperType::MBC1 => {
                    let mut bank_number = self.loram_bank;
                    if self.banking_mode == BankingMode::Rom {
                        bank_number |= self.ram_or_hiram_bank << 5;
                    }
                    let base_address = bank_number as usize * 0x4000;
                    Wrapping(self.memory().game_rom[base_address + address.0 as usize - 0x4000])
                }
                crate::application_state::MapperType::Other => todo!(),
            },
            0x8000..=0x9FFF => self.ppu.read_vram(address - Wrapping(0x8000)),

            0xA000..=0xBFFF => {
                Wrapping(self.memory().game_ram[(address - Wrapping(0xA000)).0 as usize])
            }
            0xC000..=0xCFFF => self.ppu.read_wram_0(address - Wrapping(0xC000)),
            0xD000..=0xDFFF => self.ppu.read_wram_1(address - Wrapping(0xD000)),
            0xE000..=0xFDFF => self.read_u8(address - Wrapping(0x2000)),

            0xFE00..=0xFE9F => {
                Wrapping(self.ppu.object_attribute_memory[address.0 as usize - 0xFE00])
            }
            0xFEA0..=0xFEFF => Wrapping(0xFF),

            0xFF00..=0xFF00 => self.inputs.read(),
            0xFF01..=0xFF01 => self.sb,
            0xFF02..=0xFF02 => self.sc,
            0xFF03..=0xFF03 => self.register_ff03,
            0xFF04..=0xFF07 => self.timers().read_u8(address),
            0xFF08..=0xFF08 => self.register_ff08,
            0xFF09..=0xFF09 => self.register_ff09,
            0xFF0A..=0xFF0A => self.register_ff0a,
            0xFF0B..=0xFF0B => self.register_ff0b,
            0xFF0C..=0xFF0C => self.register_ff0c,
            0xFF0D..=0xFF0D => self.register_ff0d,
            0xFF0E..=0xFF0E => self.register_ff0e,
            0xFF0F..=0xFF0F => self.interrupts().interrupt_flag,

            0xFF10..=0xFF10 => self.nr10,
            0xFF11..=0xFF11 => self.nr11,
            0xFF12..=0xFF12 => self.nr12,
            0xFF13..=0xFF13 => self.nr13,
            0xFF14..=0xFF14 => self.nr14,
            0xFF15..=0xFF15 => self.register_ff15,
            0xFF16..=0xFF16 => self.nr21,
            0xFF17..=0xFF17 => self.nr22,
            0xFF18..=0xFF18 => self.nr23,
            0xFF19..=0xFF19 => self.nr24,
            0xFF1A..=0xFF1A => self.nr30,
            0xFF1B..=0xFF1B => self.nr31,
            0xFF1C..=0xFF1C => self.nr32,
            0xFF1D..=0xFF1D => self.nr33,
            0xFF1E..=0xFF1E => self.nr34,
            0xFF1F..=0xFF1F => self.register_ff1f,
            0xFF20..=0xFF20 => self.register_ff20,
            0xFF21..=0xFF21 => self.register_ff21,
            0xFF22..=0xFF22 => self.register_ff22,
            0xFF23..=0xFF23 => self.register_ff23,
            0xFF24..=0xFF24 => self.nr50,
            0xFF25..=0xFF25 => self.nr51,
            0xFF26..=0xFF26 => self.nr52,
            0xFF27..=0xFF2F => self.slice_ff27_ff2f[address.0 as usize - 0xFF27],

            // Wave RAM
            0xFF30..=0xFF3F => self.slice_ff30_ff3f[address.0 as usize - 0xFF30],

            0xFF40..=0xFF40 => self.ppu.read_lcdc(),
            0xFF41..=0xFF41 => self.ppu.lcd_status,
            0xFF42..=0xFF42 => self.ppu.scy,
            0xFF43..=0xFF43 => self.ppu.scx,
            0xFF44..=0xFF44 => self.ppu.read_ly(),
            0xFF45..=0xFF45 => self.ppu.lcd_y_compare,
            0xFF46..=0xFF46 => {
                print!("WARNING: Faking read attempt of 0xFF46");
                Wrapping(0xFF)
            }
            0xFF47..=0xFF47 => Wrapping(self.ppu.background_palette_data),
            0xFF48..=0xFF48 => Wrapping(self.ppu.object_palette_0),
            0xFF49..=0xFF49 => Wrapping(self.ppu.object_palette_1),
            0xFF4A..=0xFF4A => self.ppu.window_y,
            0xFF4B..=0xFF4B => self.ppu.window_x7,
            0xFF4D..=0xFF4D => self.register_ff4d,
            0xFF4F..=0xFF4F => self.ppu.vram_bank,

            0xFF50..=0xFF50 => self.dmg_boot_rom,

            0xFF68..=0xFF68 => self.ppu.cgb_background_palette_spec,
            0xFF69..=0xFF69 => self.ppu.cgb_background_palette_data,
            0xFF6A..=0xFF6A => self.ppu.object_palette_spec,
            0xFF6B..=0xFF6B => self.ppu.object_palette_data,

            0xFF70..=0xFF70 => self.wram_bank,
            0xFF72..=0xFF72 => self.register_ff72,
            0xFF73..=0xFF73 => self.register_ff73,
            0xFF74..=0xFF74 => Wrapping(0xFF),
            0xFF75..=0xFF75 => self.register_ff75,

            0xFF80..=0xFFFE => Wrapping(self.memory().hram[address.0 as usize - 0xFF80]),
            0xFFFF..=0xFFFF => self.interrupts().interrupt_enable,
            _ => panic!(
                "Memory read at address {:04X} needs to be handled (at PC 0x{:04X})",
                address,
                self.registers().pc
            ),
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

    pub fn request_interrupt(&mut self, interrupt_bit: u8) {
        self.interrupts_mut().request(interrupt_bit);
    }

    pub fn write_u8(&mut self, address: Wrapping<u16>, value: Wrapping<u8>) {
        if self.is_dmg_boot_rom_on() && address.0 <= 0xFF {
            panic!("Attempted write in boot ROM")
        }
        match address.0 {
            0x0000..=0x1FFF => match self.rom_information.mapper_type {
                MapperType::ROMOnly => {
                    print!("WARNING: Ignoring write at 0x{:04X}", address.0)
                }
                MapperType::MBC1 => {
                    self.is_ram_enabled = value.0 & 0x0F == 0x0A;
                }
                MapperType::Other => todo!(),
            },
            0x2000..=0x3FFF => match self.rom_information.mapper_type {
                MapperType::ROMOnly => {
                    println!("WARNING: Ignoring write at 0x{:04X}", address.0)
                }
                MapperType::MBC1 => {
                    self.loram_bank = value.0 & 0x1F;
                }
                MapperType::Other => todo!(),
            },
            0x4000..=0x5FFF => match self.rom_information.mapper_type {
                MapperType::ROMOnly => {
                    print!("WARNING: Ignoring write at 0x{:04X}", address.0)
                }
                MapperType::MBC1 => {
                    self.ram_or_hiram_bank = value.0 & 0b11;
                }
                MapperType::Other => todo!(),
            },
            0x6000..=0x7FFF => match self.rom_information.mapper_type {
                MapperType::ROMOnly => {
                    print!("WARNING: Ignoring write at 0x{:04X}", address.0)
                }
                MapperType::MBC1 => {
                    self.banking_mode = if value.0 & 1 == 0 {
                        BankingMode::Rom
                    } else {
                        BankingMode::Ram
                    }
                }
                MapperType::Other => todo!(),
            },
            0x8000..=0x9FFF => PPU::write_vram(&mut self.ppu, address - Wrapping(0x8000), value),

            0xA000..=0xBFFF => match self.rom_information.ram_size {
                crate::application_state::RAMSize::NoRAM => {
                    println!(
                        "WARNING: Ignoring write to non-existing RAM at 0x{:04X}",
                        address
                    )
                }
                _ => self.memory_mut().game_ram[address.0 as usize - 0xA000] = value.0,
            },
            0xC000..=0xCFFF => PPU::write_wram_0(&mut self.ppu, address - Wrapping(0xC000), value),
            0xD000..=0xDFFF => PPU::write_wram_1(&mut self.ppu, address - Wrapping(0xD000), value),
            0xE000..=0xFDFF => {
                panic!("Echo RAM write")
            }

            0xFE00..=0xFE9F => {
                self.ppu.object_attribute_memory[address.0 as usize - 0xFE00] = value.0
            }
            0xFEA0..=0xFEFF => {
                // println!("[WARNING] Ignoring write to 0x{:04X}", address.0)
            }

            0xFF00..=0xFF00 => self.inputs.write(value),
            0xFF01..=0xFF01 => self.sb = value,
            0xFF02..=0xFF02 => self.sc = value,
            0xFF03..=0xFF03 => self.register_ff03 = value,
            0xFF04..=0xFF07 => self.timers_mut().write_u8(address, value),
            0xFF08..=0xFF08 => self.register_ff08 = value,
            0xFF09..=0xFF09 => self.register_ff09 = value,
            0xFF0A..=0xFF0A => self.register_ff0a = value,
            0xFF0B..=0xFF0B => self.register_ff0b = value,
            0xFF0C..=0xFF0C => self.register_ff0c = value,
            0xFF0D..=0xFF0D => self.register_ff0d = value,
            0xFF0E..=0xFF0E => self.register_ff0e = value,
            0xFF0F..=0xFF0F => self.interrupts_mut().interrupt_flag = value,

            // AUDIO
            0xFF10..=0xFF10 => self.nr10 = value,
            0xFF11..=0xFF11 => self.nr11 = value,
            0xFF12..=0xFF12 => self.nr12 = value,
            0xFF13..=0xFF13 => self.nr13 = value,
            0xFF14..=0xFF14 => self.nr14 = value,
            0xFF15..=0xFF15 => self.register_ff15 = value,
            0xFF16..=0xFF16 => self.nr21 = value,
            0xFF17..=0xFF17 => self.nr22 = value,
            0xFF18..=0xFF18 => self.nr23 = value,
            0xFF19..=0xFF19 => self.nr24 = value,
            0xFF1A..=0xFF1A => self.nr30 = value,
            0xFF1B..=0xFF1B => self.nr31 = value,
            0xFF1C..=0xFF1C => self.nr32 = value,
            0xFF1D..=0xFF1D => self.nr33 = value,
            0xFF1E..=0xFF1E => self.nr34 = value,
            0xFF1F..=0xFF1F => self.register_ff1f = value,

            0xFF20..=0xFF20 => self.register_ff20 = value,
            0xFF21..=0xFF21 => self.register_ff21 = value,
            0xFF22..=0xFF22 => self.register_ff22 = value,
            0xFF23..=0xFF23 => self.register_ff23 = value,
            0xFF24..=0xFF24 => self.nr50 = value,
            0xFF25..=0xFF25 => self.nr51 = value,
            0xFF26..=0xFF26 => self.nr52 = value,
            0xFF27..=0xFF2F => self.slice_ff27_ff2f[address.0 as usize - 0xFF27] = value,

            // WAVE RAM
            0xFF30..=0xFF3F => self.slice_ff30_ff3f[address.0 as usize - 0xFF30] = value,

            0xFF40..=0xFF40 => self.ppu.write_lcdc(value),
            0xFF41..=0xFF41 => self.ppu.lcd_status = value,
            0xFF42..=0xFF42 => self.ppu.scy = value,
            0xFF43..=0xFF43 => self.ppu.scx = value,
            0xFF44..=0xFF44 => {
                panic!("Something attempted to write to LY")
            }
            0xFF45..=0xFF45 => self.ppu.lcd_y_compare = value,
            0xFF46..=0xFF46 => {
                // TODO: extract
                // OAM DMA transfer (should take 640 dots)
                if value.0 > 0xDF {
                    panic!("OAM DMA transfer outside of valid range!");
                }
                let base_source_address = (value.0 as u16) << 8;
                for offset in 0..=0x9F {
                    let byte = self.read_u8(Wrapping(base_source_address | offset));
                    self.write_u8(Wrapping(0xFE00 + offset), byte)
                }
            }
            0xFF47..=0xFF47 => self.ppu.background_palette_data = value.0,
            0xFF48..=0xFF48 => self.ppu.object_palette_0 = value.0,
            0xFF49..=0xFF49 => self.ppu.object_palette_1 = value.0,
            0xFF4A..=0xFF4A => self.ppu.window_y = value,
            0xFF4B..=0xFF4B => self.ppu.window_x7 = value,
            0xFF4D..=0xFF4D => self.register_ff4d = value,
            0xFF4F..=0xFF4F => self.ppu.vram_bank = value,

            0xFF50..=0xFF50 => self.dmg_boot_rom = value,

            0xFF68..=0xFF68 => self.ppu.cgb_background_palette_spec = value,
            0xFF69..=0xFF69 => self.ppu.cgb_background_palette_data = value,
            0xFF6A..=0xFF6A => self.ppu.object_palette_spec = value,
            0xFF6B..=0xFF6B => self.ppu.object_palette_data = value,

            0xFF70..=0xFF70 => self.wram_bank = value,
            0xFF72..=0xFF72 => self.register_ff72 = value,
            0xFF73..=0xFF73 => self.register_ff73 = value,
            0xFF74..=0xFF74 => {}
            0xFF75..=0xFF75 => self.register_ff75 = Wrapping(value.0 & 0x07),
            0xFF7F..=0xFF7F => {
                // println!("[WARNING] Ignoring write to 0x{:04X}", address.0)
            }

            0xFF80..=0xFFFE => self.memory_mut().hram[address.0 as usize - 0xFF80] = value.0,
            0xFFFF..=0xFFFF => self.interrupts_mut().interrupt_enable = value,
            _ => panic!(
                "Memory write at address {:04X} needs to be handle (at PC 0x{:04X})",
                address,
                self.registers().pc
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

    pub fn cpu(&self) -> &CPU {
        &self.cpu
    }

    pub fn cpu_mut(&mut self) -> &mut CPU {
        &mut self.cpu
    }

    pub fn pixel_fetcher(&self) -> &Fetcher {
        &self.pixel_fetcher
    }

    pub fn pixel_fetcher_mut(&mut self) -> &mut Fetcher {
        &mut self.pixel_fetcher
    }

    pub fn ppu(&self) -> &PPU {
        &self.ppu
    }

    pub fn ppu_mut(&mut self) -> &mut PPU {
        &mut self.ppu
    }
}
