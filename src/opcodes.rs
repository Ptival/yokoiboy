use std::fmt;

use crate::{
    conditions::Condition,
    cpu::CPU,
    memory::read_u16,
    registers::{Flag, R16, R8},
};

pub const MAX_SIZE_INSTRUCTION: u16 = 4;

#[derive(Clone, Debug)]
pub struct Immediate16 {
    lower_byte: u8,
    higher_byte: u8,
}

impl Immediate16 {
    pub fn as_u16(&self) -> u16 {
        (self.higher_byte as u16) << 8 | self.lower_byte as u16
    }
    // So far, immediate 16-bit values are stored lower-byte-first
    pub fn read(m: &[u8]) -> Immediate16 {
        Immediate16 {
            lower_byte: m[0],
            higher_byte: m[1],
        }
    }
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum Instruction {
    ADC_A_B,
    ADC_A_C,
    ADC_A_d8,
    ADD_A__HL_,
    ADD_A_E,
    AND_L,
    BIT_u3_r8(u8, R8),
    CALL_a16,
    CALL_Z_a16,
    CP__HL_,
    CP_C,
    CP_d8,
    CP_E,
    DEC_A,
    DEC_B,
    DEC_BC,
    DEC_C,
    DEC_D,
    DEC_E,
    EI,
    Illegal(u8),
    INC_r8(R8),
    INC_r16(R16),
    JR_cc_s8(Condition, i8),
    JR_r8,
    JR_Z_r8,
    LD__a16__A,
    LD__a16__SP,
    LD__C__A,
    LD__HL__r8(R8),
    LD_A__DE_,
    LD_r8_d8(R8, u8),
    LD_r8_r8(R8, R8),
    LD_DE_d16(Immediate16),
    LD_H__HL_,
    LD_HL_d16(Immediate16),
    LD_HL_minus_A,
    LD_HL_plus_A,
    LD_L__HL_,
    LD_L_d8,
    LD_SP_A(Immediate16),
    LD__a8__A,
    LDH_A__a8__,
    NOP,
    Prefix,
    PUSH_BC,
    RET_NZ,
    RET,
    RETI,
    RLA,
    SBC_A_A,
    SBC_A_C,
    SUB_B,
    XOR_r8(R8),
}

#[derive(Clone, Debug)]
pub struct DecodedInstruction {
    pub address: u16,
    pub instruction: Instruction,
    pub instruction_size: u8,
    pub raw: Vec<u8>,
}

impl fmt::Display for DecodedInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04X}: {}", self.address, self.instruction)
    }
}

impl DecodedInstruction {
    pub fn display_raw(&self) -> String {
        let mut res = String::new();
        for b in &self.raw {
            res.push_str(&format!("{:02X} ", b));
        }
        res
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::BIT_u3_r8(bit, reg) => {
                write!(f, "BIT {}, {}", bit, reg)
            }
            Instruction::INC_r8(r8) => {
                write!(f, "INC {}", r8)
            }
            Instruction::INC_r16(r16) => {
                write!(f, "INC {}", r16)
            }
            Instruction::JR_cc_s8(cc, s8) => {
                write!(f, "JR {}, {}", cc, s8)
            }
            Instruction::LD__C__A => {
                write!(f, "LD (0xFF00 + C), A")
            }
            Instruction::LD__HL__r8(r8) => {
                write!(f, "LD (HL) {}", r8)
            }
            Instruction::LD_HL_d16(imm) => {
                write!(f, "LD HL 0x{:04X} (= {})", imm.as_u16(), imm.as_u16())
            }
            Instruction::LD_HL_minus_A => {
                write!(f, "LD (HL-) A")
            }
            Instruction::LD_r8_d8(r8, d8) => {
                write!(f, "LD {}, 0x{:02X} (= {})", r8, d8, d8)
            }
            Instruction::LD_SP_A(imm) => {
                write!(f, "LD SP 0x{:04X} (= {})", imm.as_u16(), imm.as_u16())
            }
            Instruction::XOR_r8(r8) => {
                write!(f, "XOR {}", r8)
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

pub fn decode_instruction_at_address(
    address: u16,
    slice: &[u8],
    // If you pass in a slice that's not at "real address zero", you can give the address of the
    // slice to adjust the address that will be given to the instruction.  `None` behaves like
    // `Some(0)`.
    slice_address: Option<u16>,
) -> Result<DecodedInstruction, String> {
    let address_u16 = address;
    let address = address as usize;
    let (i, s) = match slice[address] {
        0x00 => (Instruction::NOP, 1),
        0x03 => (Instruction::INC_r16(R16::BC), 1),
        0x04 => (Instruction::INC_r8(R8::B), 1),
        0x05 => (Instruction::DEC_B, 1),
        0x06 => (Instruction::LD_r8_d8(R8::B, slice[address + 1]), 2),
        0x08 => (Instruction::LD__a16__SP, 3),
        0x0B => (Instruction::DEC_BC, 1),
        0x0C => (Instruction::INC_r8(R8::C), 1),
        0x0D => (Instruction::DEC_C, 1),
        0x0E => (Instruction::LD_r8_d8(R8::C, slice[address + 1]), 2),
        0x11 => (
            Instruction::LD_DE_d16(Immediate16::read(&slice[address + 1..])),
            3,
        ),
        0x13 => (Instruction::INC_r16(R16::DE), 1),
        0x15 => (Instruction::DEC_D, 1),
        0x16 => (Instruction::LD_r8_d8(R8::D, slice[address + 1]), 2),
        0x17 => (Instruction::RLA, 1),
        0x18 => (Instruction::JR_r8, 2),
        0x1A => (Instruction::LD_A__DE_, 1),
        0x1D => (Instruction::DEC_E, 1),
        0x1E => (Instruction::LD_r8_d8(R8::E, slice[address + 1]), 2),
        0x20 => (
            Instruction::JR_cc_s8(Condition::NZ, slice[address + 1] as i8),
            2,
        ),
        0x21 => (
            Instruction::LD_HL_d16(Immediate16::read(&slice[address + 1..])),
            3,
        ),
        0x22 => (Instruction::LD_HL_plus_A, 1),
        0x23 => (Instruction::INC_r16(R16::HL), 1),
        0x24 => (Instruction::INC_r8(R8::H), 1),
        0x28 => (Instruction::JR_Z_r8, 2),
        0x2E => (Instruction::LD_L_d8, 2),
        0x31 => (
            Instruction::LD_SP_A(Immediate16::read(&slice[address + 1..])),
            3,
        ),
        0x32 => (Instruction::LD_HL_minus_A, 1),
        0x33 => (Instruction::INC_r16(R16::SP), 1),
        0x3C => (Instruction::INC_r8(R8::A), 1),
        0x3D => (Instruction::DEC_A, 1),
        0x3E => (Instruction::LD_r8_d8(R8::A, slice[address + 1]), 2),
        0x42 => (Instruction::LD_r8_r8(R8::B, R8::D), 1),
        0x4F => (Instruction::LD_r8_r8(R8::C, R8::A), 1),
        0x57 => (Instruction::LD_r8_r8(R8::D, R8::A), 1),
        0x63 => (Instruction::LD_r8_r8(R8::H, R8::E), 1),
        0x66 => (Instruction::LD_H__HL_, 1),
        0x67 => (Instruction::LD_r8_r8(R8::H, R8::A), 1),
        0x6E => (Instruction::LD_L__HL_, 1),
        0x73 => (Instruction::LD__HL__r8(R8::E), 1),
        0x77 => (Instruction::LD__HL__r8(R8::A), 1),
        0x78 => (Instruction::LD_r8_r8(R8::A, R8::B), 1),
        0x7B => (Instruction::LD_r8_r8(R8::A, R8::E), 1),
        0x7C => (Instruction::LD_r8_r8(R8::A, R8::H), 1),
        0x7D => (Instruction::LD_r8_r8(R8::A, R8::L), 1),
        0x83 => (Instruction::ADD_A_E, 1),
        0x86 => (Instruction::ADD_A__HL_, 1),
        0x88 => (Instruction::ADC_A_B, 1),
        0x89 => (Instruction::ADC_A_C, 1),
        0x90 => (Instruction::SUB_B, 1),
        0x99 => (Instruction::SBC_A_C, 1),
        0x9F => (Instruction::SBC_A_A, 1),
        0xA5 => (Instruction::AND_L, 1),
        0xA8 => (Instruction::XOR_r8(R8::B), 1),
        0xA9 => (Instruction::XOR_r8(R8::C), 1),
        0xAA => (Instruction::XOR_r8(R8::D), 1),
        0xAB => (Instruction::XOR_r8(R8::E), 1),
        0xAC => (Instruction::XOR_r8(R8::H), 1),
        0xAD => (Instruction::XOR_r8(R8::L), 1),
        0xAF => (Instruction::XOR_r8(R8::A), 1),
        0xB9 => (Instruction::CP_C, 1),
        0xBB => (Instruction::CP_E, 1),
        0xBE => (Instruction::CP__HL_, 1),
        0xC1 => (Instruction::RET_NZ, 1),
        0xC9 => (Instruction::RET, 1),
        0xCB => match slice[address + 1] {
            0x78 => (Instruction::BIT_u3_r8(7, R8::B), 2),
            0x79 => (Instruction::BIT_u3_r8(7, R8::C), 2),
            0x7A => (Instruction::BIT_u3_r8(7, R8::D), 2),
            0x7B => (Instruction::BIT_u3_r8(7, R8::E), 2),
            0x7C => (Instruction::BIT_u3_r8(7, R8::H), 2),
            _ => {
                (Instruction::Prefix, 2) // 1 for prefix, 1 for extension?
                                         // println!("TODO: CB-prefixed opcode 0x{:x}", slice[1]);
                                         // todo!()
            }
        },
        0xC5 => (Instruction::PUSH_BC, 1),
        0xCC => (Instruction::CALL_Z_a16, 3),
        0xCD => (Instruction::CALL_a16, 3),
        0xCE => (Instruction::ADC_A_d8, 2),
        0xD9 => (Instruction::RETI, 1),
        0xE0 => (Instruction::LD__a8__A, 2),
        0xE2 => (Instruction::LD__C__A, 1),
        0xEA => (Instruction::LD__a16__A, 3),
        0xF0 => (Instruction::LDH_A__a8__, 2),
        0xFB => (Instruction::EI, 1),
        0xFE => (Instruction::CP_d8, 2),
        b => (Instruction::Illegal(b), 1),
    };
    Ok(DecodedInstruction {
        address: slice_address.unwrap_or(0) + address_u16,
        instruction: i,
        instruction_size: s,
        raw: slice[address..address + s as usize].into(),
    })
}

impl Instruction {
    // /// Returns the number of cycles to execute this [`Instruction`].
    // pub fn get_cycles(self: &Instruction) -> i8 {
    //     match self {
    //         Instruction::LD_SP_A => todo!(),
    //         Instruction::XOR_A => todo!(),
    //         Instruction::LD_HL_d16 => todo!(),
    //         Instruction::LD_HL_minus_A => todo!(),
    //         Instruction::Prefix => todo!(),
    //         Instruction::EI => todo!(),
    //         Instruction::LD_C_d8 => todo!(),
    //         Instruction::LD_A_d8 => todo!(),
    //         Instruction::LD__C__A => todo!(),
    //     }
    // }

    pub fn execute(self: &Instruction, cpu: &mut CPU) {
        match self {
            Instruction::BIT_u3_r8(bit, reg) => {
                cpu.registers
                    .write_flag(Flag::Z, !cpu.registers.get_bit(reg, bit))
                    .unset_flag(Flag::N)
                    .set_flag(Flag::H);
            }
            Instruction::INC_r8(r8) => {
                let res = cpu.registers.get_r8(r8).wrapping_add(1);
                cpu.registers
                    .set_r8(r8, res)
                    .write_flag(Flag::Z, res == 0)
                    .unset_flag(Flag::N)
                    .write_flag(Flag::H, res == 0x10);
            }
            Instruction::INC_r16(r16) => {
                let res = cpu.registers.get_r16(r16).wrapping_add(1);
                cpu.registers.set_r16(r16, res);
            }
            Instruction::JR_cc_s8(cc, s8) => {
                if cc.holds(cpu) {
                    cpu.registers.pc = cpu
                        .registers
                        .pc
                        .checked_add_signed(*s8 as i16)
                        .expect("JR_cc_s8 overflowed")
                }
            }
            Instruction::LD__C__A => cpu
                .memory
                .write_u8(0x00FF + cpu.registers.get_c() as u16, cpu.registers.get_a()),
            Instruction::LD_HL_d16(imm) => {
                cpu.registers.hl = imm.as_u16();
            }
            Instruction::LD__HL__r8(r8) => cpu
                .memory
                .write_u8(cpu.registers.hl, cpu.registers.get_r8(r8)),
            Instruction::LD_HL_minus_A => {
                cpu.memory.write_u8(cpu.registers.hl, cpu.registers.get_a());
                cpu.registers.hl -= 1;
            }
            Instruction::LD_r8_d8(r8, d8) => {
                cpu.registers.set_r8(r8, *d8);
            }
            Instruction::LD_SP_A(imm) => {
                cpu.registers.sp = imm.as_u16();
            }
            Instruction::XOR_r8(r8) => {
                let res = cpu.registers.get_a() ^ cpu.registers.get_r8(r8);
                cpu.registers
                    .set_a(res)
                    .write_flag(Flag::Z, res == 0)
                    .unset_flag(Flag::N)
                    .unset_flag(Flag::H)
                    .unset_flag(Flag::C);
            }
            _ => todo!(),
        };
    }
}
