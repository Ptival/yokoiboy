use super::{decode::DecodedInstruction, type_def::Instruction};

impl DecodedInstruction {
    fn resolve_relative(&self, i8: i8) -> u16 {
        self.address
            .wrapping_add(self.instruction_size as u16)
            .wrapping_add_signed(i8 as i16)
    }

    pub fn as_string(&self) -> String {
        match &self.instruction {
            Instruction::ADD_A_mHL => String::from("ADD A [HL]"),
            Instruction::BIT_u3_r8(bit, reg) => {
                format!("BIT {}, {}", bit, reg)
            }
            Instruction::CALL_a16(imm16) => {
                format!("CALL 0x{:04X}", imm16.as_u16())
            }
            Instruction::CP_A_r8(r8) => {
                format!("CP A, {}", r8)
            }
            Instruction::CP_A_u8(u8) => {
                format!("CP A, {:02X} (= {})", u8, u8)
            }
            Instruction::CP_A_mHL => String::from("CP A, [HL]"),
            Instruction::DEC_r8(r8) => {
                format!("DEC {}", r8)
            }
            Instruction::DEC_r16(r16) => {
                format!("DEC {}", r16)
            }
            Instruction::INC_r8(r8) => {
                format!("INC {}", r8)
            }
            Instruction::INC_r16(r16) => {
                format!("INC {}", r16)
            }
            Instruction::JR_cc_i8(cc, i8) => {
                format!("JR {}, {} (=0x{:04X})", cc, i8, self.resolve_relative(*i8))
            }
            Instruction::LD_A_FFu8(u8) => {
                format!("LD A, [0x{:04X}]", 0xFF00 + *u8 as u16)
            }
            Instruction::LD_FFC_A => String::from("LD [0xFF00 + C], A"),
            Instruction::LD_mr16_r8(mr16, r8) => {
                format!("LD [{}] {}", mr16, r8)
            }
            Instruction::LD_r8_r8(r8a, r8b) => {
                format!("LD {}, {}", r8a, r8b)
            }
            Instruction::LD_r8_mr16(r8, mr16) => {
                format!("LD {} [{}]", r8, mr16)
            }
            Instruction::LD_r16_d16(r16, imm16) => {
                format!("LD {} 0x{:04X} (= {})", r16, imm16.as_u16(), imm16.as_u16())
            }
            Instruction::LD_mHLdec_A => String::from("LD [HL-], A"),
            Instruction::LD_mHLinc_A => String::from("LD [HL+], A"),
            Instruction::LD_r8_u8(r8, u8) => {
                format!("LD {}, 0x{:02X} (= {})", r8, u8, u8)
            }
            Instruction::LD_SP_u16(imm) => {
                format!("LD SP 0x{:04X} (= {})", imm.as_u16(), imm.as_u16())
            }
            Instruction::POP_r16(r16) => {
                format!("POP {}", r16)
            }
            Instruction::PUSH_r16(r16) => {
                format!("PUSH {}", r16)
            }
            Instruction::SUB_A_r8(r8) => {
                format!("SUB A, {}", r8)
            }
            Instruction::RL_r8(r8) => {
                format!("RL {}", r8)
            }
            Instruction::XOR_r8(r8) => {
                format!("XOR {}", r8)
            }
            Instruction::ADC_A_r8(r8) => {
                format!("ADC A, {}", r8)
            }
            Instruction::ADC_A_u8(u8) => {
                format!("ADC A, {:02X}", u8)
            }
            Instruction::ADD_A_u8(u8) => {
                format!("ADD A, {:02X}", u8)
            }
            Instruction::ADD_A_r8(r8) => {
                format!("ADD A, {}", r8)
            }
            Instruction::AND_L => String::from("AND L"),
            Instruction::CALL_Z_a16(imm16) => {
                format!("CALL Z {:04X}", imm16.as_u16())
            }
            Instruction::EI => String::from("EI"),
            Instruction::JP_u16(imm16) => {
                format!("JP 0x{:04X}", imm16.as_u16())
            }
            Instruction::JR_r8(r8) => {
                format!("JP {}", r8)
            }
            Instruction::JR_i8(i8) => {
                format!("JR 0x{:02X}", i8)
            }
            Instruction::LD_mu16_SP(imm16) => {
                format!("LD [0x{:04X}] SP", imm16.as_u16())
            }
            Instruction::LD_H_mHL => String::from("LD H, [HL]"),
            Instruction::LD_L_mHL => String::from("LD L, [HL]"),
            Instruction::NOP => String::from("NOP"),
            Instruction::Prefix => String::from("TODO"),
            Instruction::RET_cc(cc) => {
                format!("RET {}", cc)
            }
            Instruction::RET => String::from("RET"),
            Instruction::RETI => String::from("RETI"),
            Instruction::RLA => String::from("RLA"),
            Instruction::SBC_A_A => String::from("SBC A, A"),
            Instruction::SBC_A_C => String::from("SBC A, C"),
            Instruction::LD_A_mHLinc => String::from("LD A [HL+]"),
            Instruction::LD_mu16_A(imm16) => {
                format!("LD [0x{:04X}], A", imm16.as_u16())
            }
            Instruction::DI => String::from("DI"),
            Instruction::LD_FFu8_A(u8) => {
                format!("LD [0x{:04X}], A", 0xFF00 + *u8 as u16)
            }
            Instruction::OR_r8(r8) => {
                format!("OR A, {}", r8)
            }
            Instruction::LD_A_mu16(imm16) => {
                format!("LD A, [0x{:04X}]", imm16.as_u16())
            }
            Instruction::AND_u8(u8) => {
                format!("AND A, 0x{:02X}", u8)
            }
            Instruction::CALL_cc_u16(cc, imm16) => {
                format!("CALL {}, 0x{:04X}", cc, imm16.as_u16())
            }
            Instruction::RET_C => String::from("RET C"),
        }
    }
}
