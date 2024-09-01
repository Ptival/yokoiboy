use std::{fmt, num::Wrapping};

use crate::{
    conditions::Condition,
    machine::Machine,
    registers::{R16, R8},
};

use super::type_def::{Immediate16, Instruction};

#[derive(Clone, Debug)]
pub struct DecodedInstruction {
    pub address: Wrapping<u16>,
    pub instruction: Instruction,
    pub instruction_size: u8,
    pub raw: Vec<Wrapping<u8>>,
}

impl fmt::Display for DecodedInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_string())
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

pub fn decode_instruction_at_address(
    machine: &Machine,
    address: Wrapping<u16>,
) -> Result<DecodedInstruction, String> {
    let mut bytes_read: u16 = 0;
    let next_i8 = |bytes_read: &mut u16| {
        let o = *bytes_read;
        *bytes_read += 1;
        Wrapping(machine.read_u8(address + Wrapping(o)).0 as i8)
    };
    let next_u8 = |bytes_read: &mut u16| {
        let o = *bytes_read;
        *bytes_read += 1;
        machine.read_u8(address + Wrapping(o))
    };
    let next_imm16 = |bytes_read: &mut u16| {
        let o = *bytes_read;
        *bytes_read += 2;
        Immediate16::from_memory(machine, address + Wrapping(o))
    };
    let i = match next_u8(&mut bytes_read).0 {
        0x00 => Instruction::NOP,
        0x01 => Instruction::LD_r16_d16(R16::BC, next_imm16(&mut bytes_read)),
        0x02 => Instruction::LD_mr16_r8(R16::BC, R8::A),
        0x03 => Instruction::INC_r16(R16::BC),
        0x04 => Instruction::INC_r8(R8::B),
        0x05 => Instruction::DEC_r8(R8::B),
        0x06 => Instruction::LD_r8_u8(R8::B, next_u8(&mut bytes_read)),
        0x08 => Instruction::LD_mu16_SP(next_imm16(&mut bytes_read)),
        0x09 => Instruction::ADD_HL_r16(R16::BC),
        0x0B => Instruction::DEC_r16(R16::BC),
        0x0C => Instruction::INC_r8(R8::C),
        0x0D => Instruction::DEC_r8(R8::C),
        0x0E => Instruction::LD_r8_u8(R8::C, next_u8(&mut bytes_read)),

        0x11 => Instruction::LD_r16_d16(R16::DE, next_imm16(&mut bytes_read)),

        0x12 => Instruction::LD_mr16_r8(R16::DE, R8::A),
        0x13 => Instruction::INC_r16(R16::DE),
        0x14 => Instruction::INC_r8(R8::D),
        0x15 => Instruction::DEC_r8(R8::D),
        0x16 => Instruction::LD_r8_u8(R8::D, next_u8(&mut bytes_read)),
        0x17 => Instruction::RLA,
        0x18 => Instruction::JR_i8(next_i8(&mut bytes_read)),
        0x19 => Instruction::ADD_HL_r16(R16::DE),
        0x1A => Instruction::LD_r8_mr16(R8::A, R16::DE),
        0x1C => Instruction::INC_r8(R8::E),
        0x1D => Instruction::DEC_r8(R8::E),
        0x1E => Instruction::LD_r8_u8(R8::E, next_u8(&mut bytes_read)),
        0x1F => Instruction::RRA,

        0x20 => Instruction::JR_cc_i8(Condition::NZ, next_i8(&mut bytes_read)),
        0x21 => Instruction::LD_r16_d16(R16::HL, next_imm16(&mut bytes_read)),
        0x22 => Instruction::LD_mHLinc_A,
        0x23 => Instruction::INC_r16(R16::HL),
        0x24 => Instruction::INC_r8(R8::H),
        0x25 => Instruction::DEC_r8(R8::H),
        0x26 => Instruction::LD_r8_u8(R8::H, next_u8(&mut bytes_read)),
        0x28 => Instruction::JR_cc_i8(Condition::Z, next_i8(&mut bytes_read)),
        0x29 => Instruction::ADD_HL_r16(R16::HL),
        0x2A => Instruction::LD_A_mHLinc,
        0x2C => Instruction::INC_r8(R8::L),
        0x2D => Instruction::DEC_r8(R8::L),
        0x2E => Instruction::LD_r8_u8(R8::L, next_u8(&mut bytes_read)),

        0x30 => Instruction::JR_cc_i8(Condition::NC, next_i8(&mut bytes_read)),
        0x31 => Instruction::LD_SP_u16(next_imm16(&mut bytes_read)),
        0x32 => Instruction::LD_mHLdec_A,
        0x33 => Instruction::INC_r16(R16::SP),
        0x34 => Instruction::INC_mHL,
        0x35 => Instruction::DEC_mHL,
        0x38 => Instruction::JR_cc_i8(Condition::C, next_i8(&mut bytes_read)),
        0x39 => Instruction::ADD_HL_r16(R16::SP),
        0x3A => Instruction::LD_A_mHLdec,
        0x3C => Instruction::INC_r8(R8::A),
        0x3D => Instruction::DEC_r8(R8::A),
        0x3E => Instruction::LD_r8_u8(R8::A, next_u8(&mut bytes_read)),

        0x40 => Instruction::LD_r8_r8(R8::B, R8::B),
        0x41 => Instruction::LD_r8_r8(R8::B, R8::C),
        0x42 => Instruction::LD_r8_r8(R8::B, R8::D),
        0x43 => Instruction::LD_r8_r8(R8::B, R8::E),
        0x44 => Instruction::LD_r8_r8(R8::B, R8::H),
        0x45 => Instruction::LD_r8_r8(R8::B, R8::L),
        0x46 => Instruction::LD_r8_mr16(R8::B, R16::HL),
        0x47 => Instruction::LD_r8_r8(R8::B, R8::A),
        0x48 => Instruction::LD_r8_r8(R8::C, R8::B),
        0x49 => Instruction::LD_r8_r8(R8::C, R8::C),
        0x4A => Instruction::LD_r8_r8(R8::C, R8::D),
        0x4B => Instruction::LD_r8_r8(R8::C, R8::E),
        0x4C => Instruction::LD_r8_r8(R8::C, R8::H),
        0x4D => Instruction::LD_r8_r8(R8::C, R8::L),
        0x4E => Instruction::LD_r8_mr16(R8::C, R16::HL),
        0x4F => Instruction::LD_r8_r8(R8::C, R8::A),

        0x50 => Instruction::LD_r8_r8(R8::D, R8::B),
        0x51 => Instruction::LD_r8_r8(R8::D, R8::C),
        0x52 => Instruction::LD_r8_r8(R8::D, R8::D),
        0x53 => Instruction::LD_r8_r8(R8::D, R8::E),
        0x54 => Instruction::LD_r8_r8(R8::D, R8::H),
        0x55 => Instruction::LD_r8_r8(R8::D, R8::L),
        0x56 => Instruction::LD_r8_mr16(R8::D, R16::HL),
        0x57 => Instruction::LD_r8_r8(R8::D, R8::A),
        0x58 => Instruction::LD_r8_r8(R8::E, R8::B),
        0x59 => Instruction::LD_r8_r8(R8::E, R8::C),
        0x5A => Instruction::LD_r8_r8(R8::E, R8::D),
        0x5B => Instruction::LD_r8_r8(R8::E, R8::E),
        0x5C => Instruction::LD_r8_r8(R8::E, R8::H),
        0x5D => Instruction::LD_r8_r8(R8::E, R8::L),
        0x5E => Instruction::LD_r8_mr16(R8::E, R16::HL),
        0x5F => Instruction::LD_r8_r8(R8::E, R8::A),

        0x60 => Instruction::LD_r8_r8(R8::H, R8::B),
        0x61 => Instruction::LD_r8_r8(R8::H, R8::C),
        0x62 => Instruction::LD_r8_r8(R8::H, R8::D),
        0x63 => Instruction::LD_r8_r8(R8::H, R8::E),
        0x64 => Instruction::LD_r8_r8(R8::H, R8::H),
        0x65 => Instruction::LD_r8_r8(R8::H, R8::L),
        0x66 => Instruction::LD_r8_mr16(R8::H, R16::HL),
        0x67 => Instruction::LD_r8_r8(R8::H, R8::A),
        0x68 => Instruction::LD_r8_r8(R8::L, R8::B),
        0x69 => Instruction::LD_r8_r8(R8::L, R8::C),
        0x6A => Instruction::LD_r8_r8(R8::L, R8::D),
        0x6B => Instruction::LD_r8_r8(R8::L, R8::E),
        0x6C => Instruction::LD_r8_r8(R8::L, R8::H),
        0x6D => Instruction::LD_r8_r8(R8::L, R8::L),
        0x6E => Instruction::LD_r8_mr16(R8::L, R16::HL),
        0x6F => Instruction::LD_r8_r8(R8::L, R8::A),

        0x70 => Instruction::LD_mr16_r8(R16::HL, R8::B),
        0x71 => Instruction::LD_mr16_r8(R16::HL, R8::C),
        0x72 => Instruction::LD_mr16_r8(R16::HL, R8::D),
        0x73 => Instruction::LD_mr16_r8(R16::HL, R8::E),
        0x74 => Instruction::LD_mr16_r8(R16::HL, R8::H),
        0x75 => Instruction::LD_mr16_r8(R16::HL, R8::L),
        0x77 => Instruction::LD_mr16_r8(R16::HL, R8::A),
        0x78 => Instruction::LD_r8_r8(R8::A, R8::B),
        0x79 => Instruction::LD_r8_r8(R8::A, R8::C),
        0x7A => Instruction::LD_r8_r8(R8::A, R8::D),
        0x7B => Instruction::LD_r8_r8(R8::A, R8::E),
        0x7C => Instruction::LD_r8_r8(R8::A, R8::H),
        0x7D => Instruction::LD_r8_r8(R8::A, R8::L),
        0x7E => Instruction::LD_A_mHL,

        0x83 => Instruction::ADD_A_r8(R8::E),
        0x86 => Instruction::ADD_A_mHL,
        0x88 => Instruction::ADC_A_r8(R8::B),
        0x89 => Instruction::ADC_A_r8(R8::C),

        0x90 => Instruction::SUB_A_r8(R8::B),
        0x99 => Instruction::SBC_A_C,
        0x9F => Instruction::SBC_A_A,

        0xA5 => Instruction::AND_L,
        0xA8 => Instruction::XOR_A_r8(R8::B),
        0xA9 => Instruction::XOR_A_r8(R8::C),
        0xAA => Instruction::XOR_A_r8(R8::D),
        0xAB => Instruction::XOR_A_r8(R8::E),
        0xAC => Instruction::XOR_A_r8(R8::H),
        0xAD => Instruction::XOR_A_r8(R8::L),
        0xAE => Instruction::XOR_A_mHL,
        0xAF => Instruction::XOR_A_r8(R8::A),

        0xB0 => Instruction::OR_r8(R8::B),
        0xB1 => Instruction::OR_r8(R8::C),
        0xB2 => Instruction::OR_r8(R8::D),
        0xB3 => Instruction::OR_r8(R8::E),
        0xB4 => Instruction::OR_r8(R8::H),
        0xB5 => Instruction::OR_r8(R8::L),
        0xB6 => Instruction::OR_A_mHL,
        0xB7 => Instruction::OR_r8(R8::A),
        0xB9 => Instruction::CP_A_r8(R8::C),
        0xBB => Instruction::CP_A_r8(R8::E),
        0xBE => Instruction::CP_A_mHL,

        0xC0 => Instruction::RET_cc(Condition::NZ),
        0xC1 => Instruction::POP_r16(R16::BC),
        0xC2 => Instruction::JP_cc_u16(Condition::NZ, next_imm16(&mut bytes_read)),
        0xC3 => Instruction::JP_u16(next_imm16(&mut bytes_read)),
        0xC4 => Instruction::CALL_cc_u16(Condition::NZ, next_imm16(&mut bytes_read)),
        0xC5 => Instruction::PUSH_r16(R16::BC),
        0xC6 => Instruction::ADD_A_u8(next_u8(&mut bytes_read)),
        0xC7 => Instruction::RST(Immediate16::from_u16(Wrapping(0x0000))),
        0xC8 => Instruction::RET_cc(Condition::Z),
        0xC9 => Instruction::RET,
        0xCA => Instruction::JP_cc_u16(Condition::Z, next_imm16(&mut bytes_read)),
        0xCB => {
            match next_u8(&mut bytes_read).0 {
                0x10 => Instruction::RL_r8(R8::B),
                0x11 => Instruction::RL_r8(R8::C),
                0x12 => Instruction::RL_r8(R8::D),
                0x13 => Instruction::RL_r8(R8::E),
                0x14 => Instruction::RL_r8(R8::H),
                0x15 => Instruction::RL_r8(R8::L),
                0x18 => Instruction::RR_r8(R8::B),
                0x19 => Instruction::RR_r8(R8::C),
                0x1A => Instruction::RR_r8(R8::D),
                0x1B => Instruction::RR_r8(R8::E),
                0x1C => Instruction::RR_r8(R8::H),
                0x1D => Instruction::RR_r8(R8::L),
                0x1F => Instruction::RR_r8(R8::A),

                0x38 => Instruction::SRL_r8(R8::B),

                0x78 => Instruction::BIT_u3_r8(7, R8::B),
                0x79 => Instruction::BIT_u3_r8(7, R8::C),
                0x7A => Instruction::BIT_u3_r8(7, R8::D),
                0x7B => Instruction::BIT_u3_r8(7, R8::E),
                0x7C => Instruction::BIT_u3_r8(7, R8::H),

                unhandled => {
                    println!("TODO: CB-prefixed opcode 0x{:02x}", unhandled);
                    Instruction::Prefix // 1 for prefix, 1 for extension?
                }
            }
        }
        0xCC => Instruction::CALL_cc_u16(Condition::Z, next_imm16(&mut bytes_read)),
        0xCD => Instruction::CALL_a16(next_imm16(&mut bytes_read)),
        0xCE => Instruction::ADC_A_u8(next_u8(&mut bytes_read)),
        0xCF => Instruction::RST(Immediate16::from_u16(Wrapping(0x0008))),

        0xD0 => Instruction::RET_cc(Condition::NC),
        0xD1 => Instruction::POP_r16(R16::DE),
        0xD2 => Instruction::JP_cc_u16(Condition::NC, next_imm16(&mut bytes_read)),
        0xD4 => Instruction::CALL_cc_u16(Condition::NC, next_imm16(&mut bytes_read)),
        0xD5 => Instruction::PUSH_r16(R16::DE),
        0xD6 => Instruction::SUB_A_u8(next_u8(&mut bytes_read)),
        0xD7 => Instruction::RST(Immediate16::from_u16(Wrapping(0x0010))),
        0xD8 => Instruction::RET_cc(Condition::C),
        0xD9 => Instruction::RETI,
        0xDA => Instruction::JP_cc_u16(Condition::C, next_imm16(&mut bytes_read)),
        0xDC => Instruction::CALL_cc_u16(Condition::C, next_imm16(&mut bytes_read)),
        0xDF => Instruction::RST(Immediate16::from_u16(Wrapping(0x0018))),

        0xE0 => Instruction::LD_FFu8_A(next_u8(&mut bytes_read)),
        0xE1 => Instruction::POP_r16(R16::HL),
        0xE2 => Instruction::LD_FFC_A,
        0xE5 => Instruction::PUSH_r16(R16::HL),
        0xE6 => Instruction::AND_u8(next_u8(&mut bytes_read)),
        0xE7 => Instruction::RST(Immediate16::from_u16(Wrapping(0x0020))),
        0xE9 => Instruction::JP_HL,
        0xEA => Instruction::LD_mu16_A(next_imm16(&mut bytes_read)),
        0xEE => Instruction::XOR_A_u8(next_u8(&mut bytes_read)),
        0xEF => Instruction::RST(Immediate16::from_u16(Wrapping(0x0028))),

        0xF0 => Instruction::LD_A_FFu8(next_u8(&mut bytes_read)),
        0xF1 => Instruction::POP_r16(R16::AF),
        0xF3 => Instruction::DI,
        0xF5 => Instruction::PUSH_r16(R16::AF),
        0xF7 => Instruction::RST(Immediate16::from_u16(Wrapping(0x0030))),
        0xF9 => Instruction::LD_SP_HL,
        0xFB => Instruction::EI,
        0xFA => Instruction::LD_A_mu16(next_imm16(&mut bytes_read)),
        0xFE => Instruction::CP_A_u8(next_u8(&mut bytes_read)),
        0xFF => Instruction::RST(Immediate16::from_u16(Wrapping(0x0038))),

        unhandled => panic!("Implement decode for {:02X}", unhandled),
    };
    Ok(DecodedInstruction {
        address: address,
        instruction: i,
        instruction_size: bytes_read as u8,
        raw: machine.read_range(address, bytes_read as usize).into(),
    })
}
