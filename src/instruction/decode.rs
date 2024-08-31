use std::fmt;

use crate::{
    conditions::Condition,
    memory::Memory,
    registers::{R16, R8},
};

use super::type_def::{Immediate16, Instruction};

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

pub fn decode_instruction_at_address(
    mem: &Memory,
    address: u16,
) -> Result<DecodedInstruction, String> {
    let (i, s) = match mem.read_u8(address) {
        0x00 => (Instruction::NOP, 1),
        0x01 => (
            Instruction::LD_r16_d16(R16::BC, Immediate16::from_memory(mem, address + 1)),
            3,
        ),
        0x02 => (Instruction::LD_mr16_A(R16::BC), 1),
        0x03 => (Instruction::INC_r16(R16::BC), 1),
        0x04 => (Instruction::INC_r8(R8::B), 1),
        0x05 => (Instruction::DEC_r8(R8::B), 1),
        0x06 => (Instruction::LD_r8_u8(R8::B, mem.read_u8(address + 1)), 2),
        0x08 => (
            Instruction::LD_mu16_SP(Immediate16::from_memory(mem, address + 1)),
            3,
        ),
        0x0B => (Instruction::DEC_r16(R16::BC), 1),
        0x0C => (Instruction::INC_r8(R8::C), 1),
        0x0D => (Instruction::DEC_r8(R8::C), 1),
        0x0E => (Instruction::LD_r8_u8(R8::C, mem.read_u8(address + 1)), 2),

        0x11 => (
            Instruction::LD_r16_d16(R16::DE, Immediate16::from_memory(mem, address + 1)),
            3,
        ),
        0x12 => (Instruction::LD_mr16_A(R16::DE), 1),
        0x13 => (Instruction::INC_r16(R16::DE), 1),
        0x14 => (Instruction::INC_r8(R8::D), 1),
        0x15 => (Instruction::DEC_r8(R8::D), 1),
        0x16 => (Instruction::LD_r8_u8(R8::D, mem.read_u8(address + 1)), 2),
        0x17 => (Instruction::RLA, 1),
        0x18 => (Instruction::JR_i8(mem.read_u8(address + 1) as i8), 2),
        0x1A => (Instruction::LD_r8_mr16(R8::A, R16::DE), 1),
        0x1C => (Instruction::INC_r8(R8::E), 1),
        0x1D => (Instruction::DEC_r8(R8::E), 1),
        0x1E => (Instruction::LD_r8_u8(R8::E, mem.read_u8(address + 1)), 2),

        0x20 => (
            Instruction::JR_cc_i8(Condition::NZ, mem.read_u8(address + 1) as i8),
            2,
        ),
        0x21 => (
            Instruction::LD_r16_d16(R16::HL, Immediate16::from_memory(mem, address + 1)),
            3,
        ),
        0x22 => (Instruction::LD_mHLinc_A, 1),
        0x23 => (Instruction::INC_r16(R16::HL), 1),
        0x24 => (Instruction::INC_r8(R8::H), 1),
        0x25 => (Instruction::DEC_r8(R8::H), 1),
        0x28 => (
            Instruction::JR_cc_i8(Condition::Z, mem.read_u8(address + 1) as i8),
            2,
        ),
        0x2A => (Instruction::LD_A_mHLinc, 1),
        0x2C => (Instruction::INC_r8(R8::L), 1),
        0x2D => (Instruction::DEC_r8(R8::L), 1),
        0x2E => (
            Instruction::LD_r8_u8(R8::L, mem.read_u8(address + 1) as u8),
            2,
        ),

        0x30 => (
            Instruction::JR_cc_i8(Condition::NC, mem.read_u8(address + 1) as i8),
            2,
        ),
        0x31 => (
            Instruction::LD_SP_A(Immediate16::from_memory(mem, address + 1)),
            3,
        ),
        0x32 => (Instruction::LD_mHLdec_A, 1),
        0x33 => (Instruction::INC_r16(R16::SP), 1),
        0x38 => (
            Instruction::JR_cc_i8(Condition::C, mem.read_u8(address + 1) as i8),
            2,
        ),
        0x3C => (Instruction::INC_r8(R8::A), 1),
        0x3D => (Instruction::DEC_r8(R8::A), 1),
        0x3E => (Instruction::LD_r8_u8(R8::A, mem.read_u8(address + 1)), 2),

        0x40 => (Instruction::LD_r8_r8(R8::B, R8::B), 1),
        0x41 => (Instruction::LD_r8_r8(R8::B, R8::C), 1),
        0x42 => (Instruction::LD_r8_r8(R8::B, R8::D), 1),
        0x43 => (Instruction::LD_r8_r8(R8::B, R8::E), 1),
        0x44 => (Instruction::LD_r8_r8(R8::B, R8::H), 1),
        0x45 => (Instruction::LD_r8_r8(R8::B, R8::L), 1),
        0x46 => (Instruction::LD_r8_mr16(R8::B, R16::HL), 1),
        0x47 => (Instruction::LD_r8_r8(R8::B, R8::A), 1),
        0x48 => (Instruction::LD_r8_r8(R8::C, R8::B), 1),
        0x49 => (Instruction::LD_r8_r8(R8::C, R8::C), 1),
        0x4A => (Instruction::LD_r8_r8(R8::C, R8::D), 1),
        0x4B => (Instruction::LD_r8_r8(R8::C, R8::E), 1),
        0x4C => (Instruction::LD_r8_r8(R8::C, R8::H), 1),
        0x4D => (Instruction::LD_r8_r8(R8::C, R8::L), 1),
        0x4E => (Instruction::LD_r8_mr16(R8::C, R16::HL), 1),
        0x4F => (Instruction::LD_r8_r8(R8::C, R8::A), 1),

        0x50 => (Instruction::LD_r8_r8(R8::D, R8::B), 1),
        0x51 => (Instruction::LD_r8_r8(R8::D, R8::C), 1),
        0x52 => (Instruction::LD_r8_r8(R8::D, R8::D), 1),
        0x53 => (Instruction::LD_r8_r8(R8::D, R8::E), 1),
        0x54 => (Instruction::LD_r8_r8(R8::D, R8::H), 1),
        0x55 => (Instruction::LD_r8_r8(R8::D, R8::L), 1),
        0x56 => (Instruction::LD_r8_mr16(R8::D, R16::HL), 1),
        0x57 => (Instruction::LD_r8_r8(R8::D, R8::A), 1),
        0x58 => (Instruction::LD_r8_r8(R8::E, R8::B), 1),
        0x59 => (Instruction::LD_r8_r8(R8::E, R8::C), 1),
        0x5A => (Instruction::LD_r8_r8(R8::E, R8::D), 1),
        0x5B => (Instruction::LD_r8_r8(R8::E, R8::E), 1),
        0x5C => (Instruction::LD_r8_r8(R8::E, R8::H), 1),
        0x5D => (Instruction::LD_r8_r8(R8::E, R8::L), 1),
        0x5E => (Instruction::LD_r8_mr16(R8::E, R16::HL), 1),
        0x5F => (Instruction::LD_r8_r8(R8::E, R8::A), 1),

        0x60 => (Instruction::LD_r8_r8(R8::H, R8::B), 1),
        0x61 => (Instruction::LD_r8_r8(R8::H, R8::C), 1),
        0x62 => (Instruction::LD_r8_r8(R8::H, R8::D), 1),
        0x63 => (Instruction::LD_r8_r8(R8::H, R8::E), 1),
        0x64 => (Instruction::LD_r8_r8(R8::H, R8::H), 1),
        0x65 => (Instruction::LD_r8_r8(R8::H, R8::L), 1),
        0x66 => (Instruction::LD_r8_mr16(R8::H, R16::HL), 1),
        0x67 => (Instruction::LD_r8_r8(R8::H, R8::A), 1),
        0x68 => (Instruction::LD_r8_r8(R8::L, R8::B), 1),
        0x69 => (Instruction::LD_r8_r8(R8::L, R8::C), 1),
        0x6A => (Instruction::LD_r8_r8(R8::L, R8::D), 1),
        0x6B => (Instruction::LD_r8_r8(R8::L, R8::E), 1),
        0x6C => (Instruction::LD_r8_r8(R8::L, R8::H), 1),
        0x6D => (Instruction::LD_r8_r8(R8::L, R8::L), 1),
        0x6E => (Instruction::LD_r8_mr16(R8::L, R16::HL), 1),
        0x6F => (Instruction::LD_r8_r8(R8::L, R8::A), 1),

        0x73 => (Instruction::LD_mr16_r8(R16::HL, R8::E), 1),
        0x77 => (Instruction::LD_mr16_r8(R16::HL, R8::A), 1),
        0x78 => (Instruction::LD_r8_r8(R8::A, R8::B), 1),
        0x7B => (Instruction::LD_r8_r8(R8::A, R8::E), 1),
        0x7C => (Instruction::LD_r8_r8(R8::A, R8::H), 1),
        0x7D => (Instruction::LD_r8_r8(R8::A, R8::L), 1),

        0x83 => (Instruction::ADD_A_r8(R8::E), 1),
        0x86 => (Instruction::ADD_A_mHL, 1),
        0x88 => (Instruction::ADC_A_r8(R8::B), 1),
        0x89 => (Instruction::ADC_A_r8(R8::C), 1),

        0x90 => (Instruction::SUB_r8(R8::B), 1),
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

        0xB0 => (Instruction::OR_r8(R8::B), 1),
        0xB1 => (Instruction::OR_r8(R8::C), 1),
        0xB2 => (Instruction::OR_r8(R8::D), 1),
        0xB3 => (Instruction::OR_r8(R8::E), 1),
        0xB4 => (Instruction::OR_r8(R8::H), 1),
        0xB5 => (Instruction::OR_r8(R8::L), 1),
        0xB9 => (Instruction::CP_A_r8(R8::C), 1),
        0xBB => (Instruction::CP_A_r8(R8::E), 1),
        0xBE => (Instruction::CP_A_mHL, 1),

        0xC0 => (Instruction::RET_cc(Condition::NZ), 1),
        0xC1 => (Instruction::POP_r16(R16::BC), 1),
        0xC3 => (
            Instruction::JP_u16(Immediate16::from_memory(mem, address + 1)),
            1,
        ),
        0xC4 => (
            Instruction::CALL_cc_u16(Condition::NZ, Immediate16::from_memory(mem, address + 1)),
            3,
        ),
        0xC5 => (Instruction::PUSH_r16(R16::BC), 1),
        0xC6 => (Instruction::ADD_A_u8(mem.read_u8(address + 1)), 1),
        0xC8 => (Instruction::RET_cc(Condition::Z), 1),
        0xC9 => (Instruction::RET, 1),
        0xCB => match mem.read_u8(address + 1) {
            0x10 => (Instruction::RL_r8(R8::B), 2),
            0x11 => (Instruction::RL_r8(R8::C), 2),
            0x12 => (Instruction::RL_r8(R8::D), 2),
            0x13 => (Instruction::RL_r8(R8::E), 2),
            0x14 => (Instruction::RL_r8(R8::H), 2),
            0x15 => (Instruction::RL_r8(R8::L), 2),
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
        0xCC => (
            Instruction::CALL_Z_a16(Immediate16::from_memory(mem, address + 1)),
            3,
        ),
        0xCD => (
            Instruction::CALL_a16(Immediate16::from_memory(mem, address + 1)),
            3,
        ),
        0xCE => (Instruction::ADC_A_u8(mem.read_u8(address + 1)), 2),

        0xD5 => (Instruction::PUSH_r16(R16::DE), 1),
        0xD8 => (Instruction::RET_C, 1),
        0xD9 => (Instruction::RETI, 1),

        0xE0 => (Instruction::LD__a8__A(mem.read_u8(address + 1)), 2),
        0xE1 => (Instruction::POP_r16(R16::HL), 1),
        0xE2 => (Instruction::LD_mC_A, 1),
        0xE5 => (Instruction::PUSH_r16(R16::HL), 1),
        0xE6 => (Instruction::AND_u8(mem.read_u8(address + 1)), 2),
        0xEA => (
            Instruction::LD_mu16_A(Immediate16::from_memory(mem, address + 1)),
            3,
        ),

        0xF0 => (Instruction::LD_A_mu8(mem.read_u8(address + 1)), 2),
        0xF1 => (Instruction::POP_r16(R16::AF), 1),
        0xF3 => (Instruction::DI, 1),
        0xF5 => (Instruction::PUSH_r16(R16::AF), 1),
        0xFB => (Instruction::EI, 1),
        0xFA => (
            Instruction::LD_A_mru16(Immediate16::from_memory(mem, address + 1)),
            3,
        ),
        0xFE => (Instruction::CP_A_u8(mem.read_u8(address + 1)), 2),

        b => panic!("Implement decode for {:02X}", b),
    };
    Ok(DecodedInstruction {
        address: address,
        instruction: i,
        instruction_size: s,
        raw: mem.read_slice(address, s as usize).into(),
    })
}
