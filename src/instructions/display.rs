use std::num::Wrapping;

use super::{decode::DecodedInstruction, type_def::Instruction};

impl DecodedInstruction {
    fn resolve_relative(&self, i8: Wrapping<i8>) -> u16 {
        (self.address + Wrapping(self.instruction_size as u16))
            .0
            .wrapping_add_signed(i8.0 as i16)
    }

    pub fn as_string(&self) -> String {
        match &self.instruction {
            Instruction::ADC_A_mHL => String::from("ADC A [HL]"),
            Instruction::ADC_A_r8(r8) => format!("ADC A, {}", r8),
            Instruction::ADC_A_u8(u8) => format!("ADC A, 0x{:02X}", u8),
            Instruction::ADD_A_mHL => String::from("ADD A [HL]"),
            Instruction::ADD_A_r8(r8) => format!("ADD A, {}", r8),
            Instruction::ADD_A_u8(u8) => format!("ADD A, 0x{:02X}", u8),
            Instruction::ADD_HL_r16(r16) => format!("ADD HL, {}", r16),
            Instruction::ADD_SP_i8(i8) => format!("ADD SP, 0x{:02X}", i8),
            Instruction::AND_A_mHL => String::from("AND A, [HL]"),
            Instruction::AND_A_r8(r8) => format!("AND A, {}", r8),
            Instruction::AND_u8(u8) => format!("AND A, 0x{:02X}", u8),
            Instruction::BIT_u3_r8(bit, reg) => format!("BIT {}, {}", bit, reg),
            Instruction::CALL_a16(imm16) => format!("CALL 0x{:04X}", imm16.as_u16()),
            Instruction::CALL_cc_u16(cc, imm16) => format!("CALL {}, 0x{:04X}", cc, imm16.as_u16()),
            Instruction::CCF => String::from("CCF"),
            Instruction::CP_A_mHL => String::from("CP A, [HL]"),
            Instruction::CP_A_r8(r8) => format!("CP A, {}", r8),
            Instruction::CP_A_u8(u8) => format!("CP A, 0x{:02X} (= {})", u8, u8),
            Instruction::CPL => String::from("CPL"),
            Instruction::DAA => String::from("DAA"),
            Instruction::DEC_mHL => String::from("DEC [HL]"),
            Instruction::DEC_r16(r16) => format!("DEC {}", r16),
            Instruction::DEC_r8(r8) => format!("DEC {}", r8),
            Instruction::DI => String::from("DI"),
            Instruction::EI => String::from("EI"),
            Instruction::HALT => String::from("HALT"),
            Instruction::Illegal(opcode) => format!("ILLEGAL 0x{:02X}", opcode),
            Instruction::INC_mHL => String::from("INC [HL]"),
            Instruction::INC_r16(r16) => format!("INC {}", r16),
            Instruction::INC_r8(r8) => format!("INC {}", r8),
            Instruction::JP_cc_u16(cc, imm16) => format!("JP {}, 0x{:04X}", cc, imm16.as_u16()),
            Instruction::JP_HL => String::from("JP HL"),
            Instruction::JP_u16(imm16) => format!("JP 0x{:04X}", imm16.as_u16()),
            Instruction::JR_cc_i8(cc, i8) => {
                format!("JR {}, {} (=0x{:04X})", cc, i8, self.resolve_relative(*i8))
            }
            Instruction::JR_i8(i8) => format!("JR 0x{:02X}", i8),
            Instruction::JR_r8(r8) => format!("JP {}", r8),
            Instruction::LD_FFu8_A(u8) => format!(
                "LD [0x{:04X}], A",
                Wrapping(0xFF00) + Wrapping((*u8).0 as u16)
            ),
            Instruction::LD_A_FFu8(u8) => format!(
                "LD A, [0x{:04X}]",
                Wrapping(0xFF00) + Wrapping((*u8).0 as u16)
            ),
            Instruction::LD_A_FFC => String::from("LD A, [0xFF00 + C]"),
            Instruction::LD_r16_d16(r16, imm16) => {
                format!("LD {} 0x{:04X} (= {})", r16, imm16.as_u16(), imm16.as_u16())
            }
            Instruction::LD_SP_u16(imm) => {
                format!("LD SP 0x{:04X} (= {})", imm.as_u16(), imm.as_u16())
            }
            Instruction::LD_A_mr16(r16) => format!("LD A [{}]", r16),
            Instruction::LD_A_mHLdec => String::from("LD A [HL-]"),
            Instruction::LD_A_mHLinc => String::from("LD A [HL+]"),
            Instruction::LD_A_mu16(imm16) => format!("LD A, [0x{:04X}]", imm16.as_u16()),
            Instruction::LD_FFC_A => String::from("LD [0xFF00 + C], A"),
            Instruction::LD_H_mHL => String::from("LD H, [HL]"),
            Instruction::LD_HL_SP_i8(i8) => format!("LD HL, SP+{:02X}", i8),
            Instruction::LD_L_mHL => String::from("LD L, [HL]"),
            Instruction::LD_mHL_u8(u8) => format!("LD [HL], 0x{:02X}", u8),
            Instruction::LD_mHLdec_A => String::from("LD [HL-], A"),
            Instruction::LD_mHLinc_A => String::from("LD [HL+], A"),
            Instruction::LD_mr16_r8(mr16, r8) => format!("LD [{}] {}", mr16, r8),
            Instruction::LD_mu16_A(imm16) => format!("LD [0x{:04X}], A", imm16.as_u16()),
            Instruction::LD_mu16_SP(imm16) => format!("LD [0x{:04X}] SP", imm16.as_u16()),
            Instruction::LD_r8_mr16(r8, mr16) => format!("LD {} [{}]", r8, mr16),
            Instruction::LD_r8_r8(r8a, r8b) => format!("LD {}, {}", r8a, r8b),
            Instruction::LD_r8_u8(r8, u8) => format!("LD {}, 0x{:02X} (= {})", r8, u8, u8),
            Instruction::LD_SP_HL => String::from("LD SP, HL"),
            Instruction::NOP => String::from("NOP"),
            Instruction::OR_A_mHL => String::from("OR A, [HL]"),
            Instruction::OR_A_r8(r8) => format!("OR A, {}", r8),
            Instruction::OR_A_u8(u8) => format!("OR A, 0x{:02X}", u8),
            Instruction::POP_r16(r16) => format!("POP {}", r16),
            Instruction::PUSH_r16(r16) => format!("PUSH {}", r16),
            Instruction::RES_u3_mHL(u8) => format!("RES {}, [HL]", u8),
            Instruction::RES_u3_r8(u8, r8) => format!("RES {}, {}", u8, r8),
            Instruction::RET => String::from("RET"),
            Instruction::RET_cc(cc) => format!("RET {}", cc),
            Instruction::RETI => String::from("RETI"),
            Instruction::RL_r8(r8) => format!("RL {}", r8),
            Instruction::RLA => String::from("RLA"),
            Instruction::RLCA => String::from("RLCA"),
            Instruction::RLC_r8(r8) => format!("RLC {}", r8),
            Instruction::RR_r8(r8) => format!("RRL {}", r8),
            Instruction::RRA => String::from("RRA"),
            Instruction::RRCA => String::from("RRCA"),
            Instruction::RRC_r8(r8) => format!("RRC {}", r8),
            Instruction::RST(imm16) => format!("RST 0x{:04X}", imm16.as_u16()),
            Instruction::SBC_A_mHL => String::from("SBC A, [HL]"),
            Instruction::SBC_A_r8(r8) => format!("SBC A, {}", r8),
            Instruction::SBC_A_u8(u8) => format!("SBC A, 0x{:02X}", u8),
            Instruction::SCF => String::from("SCF"),
            Instruction::SET_u3_mHL(u8) => format!("SET {}, [HL]", u8),
            Instruction::SET_u3_r8(u8, r8) => format!("SET {}, {}", u8, r8),
            Instruction::SLA_r8(r8) => format!("SLA {}", r8),
            Instruction::SRA_r8(r8) => format!("SRA {}", r8),
            Instruction::SRL_r8(r8) => format!("SRL {}", r8),
            Instruction::SUB_A_mHL => String::from("SUB A, [HL]"),
            Instruction::SUB_A_r8(r8) => format!("SUB A, {}", r8),
            Instruction::SUB_A_u8(u8) => format!("SUB A, 0x{:02X}", u8),
            Instruction::SWAP(r8) => format!("SWAP {}", r8),
            Instruction::XOR_A_mHL => String::from("XOR A, [HL]"),
            Instruction::XOR_A_r8(r8) => format!("XOR A, {}", r8),
            Instruction::XOR_A_u8(u8) => format!("XOR A, 0x{:02X}", u8),
        }
    }
}
