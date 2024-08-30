use std::fmt;

use super::type_def::Instruction;

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::BIT_u3_r8(bit, reg) => {
                write!(f, "BIT {}, {}", bit, reg)
            }
            Instruction::CALL_a16(imm16) => {
                write!(f, "CALL 0x{:04X}", imm16.as_u16())
            }
            Instruction::CP_A_r8(r8) => {
                write!(f, "CP A, {}", r8)
            }
            Instruction::CP_A_u8(u8) => {
                write!(f, "CP A, {:02X} (= {})", u8, u8)
            }
            Instruction::CP_A_mHL => {
                write!(f, "CP A, [HL]")
            }
            Instruction::DEC_r8(r8) => {
                write!(f, "DEC {}", r8)
            }
            Instruction::DEC_r16(r16) => {
                write!(f, "DEC {}", r16)
            }
            Instruction::INC_r8(r8) => {
                write!(f, "INC {}", r8)
            }
            Instruction::INC_r16(r16) => {
                write!(f, "INC {}", r16)
            }
            Instruction::JR_cc_i8(cc, i8) => {
                write!(f, "JR {}, {}", cc, i8)
            }
            Instruction::LD_A_mu8(u8) => {
                write!(f, "LD A, [{:04X}]", 0xFF00 + *u8 as u16)
            }
            Instruction::LD__a8__A(u8) => {
                write!(f, "LD [0x{:04X}], A", 0xFF00 + *u8 as u16)
            }
            Instruction::LD_mC_A => {
                write!(f, "LD [0xFF00 + C], A")
            }
            Instruction::LD_mr16_A(mr16) => {
                write!(f, "LD [0x{:04X}] A", mr16.as_u16())
            }
            Instruction::LD_mr16_r8(mr16, r8) => {
                write!(f, "LD [{}] {}", mr16, r8)
            }
            Instruction::LD_r8_r8(r8a, r8b) => {
                write!(f, "LD {}, {}", r8a, r8b)
            }
            Instruction::LD_r8_mr16(r8, mr16) => {
                write!(f, "LD {} [{}]", r8, mr16)
            }
            Instruction::LD_r16_d16(r16, imm16) => {
                write!(
                    f,
                    "LD {} 0x{:04X} (= {})",
                    r16,
                    imm16.as_u16(),
                    imm16.as_u16()
                )
            }
            Instruction::LD_mHLdec_A => {
                write!(f, "LD [HL-], A")
            }
            Instruction::LD_mHLinc_A => {
                write!(f, "LD [HL+], A")
            }
            Instruction::LD_r8_u8(r8, u8) => {
                write!(f, "LD {}, 0x{:02X} (= {})", r8, u8, u8)
            }
            Instruction::LD_SP_A(imm) => {
                write!(f, "LD SP 0x{:04X} (= {})", imm.as_u16(), imm.as_u16())
            }
            Instruction::POP_r16(r16) => {
                write!(f, "POP {}", r16)
            }
            Instruction::PUSH_r16(r16) => {
                write!(f, "PUSH {}", r16)
            }
            Instruction::SUB_r8(r8) => {
                write!(f, "SUB {}", r8)
            }
            Instruction::RL_r8(r8) => {
                write!(f, "RL {}", r8)
            }
            Instruction::XOR_r8(r8) => {
                write!(f, "XOR {}", r8)
            }
            _ => write!(f, "{:?}", self),
        }
    }
}
