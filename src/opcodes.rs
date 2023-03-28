#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum Instruction {
    ADC_A_B,
    ADC_A_C,
    ADC_A_d8,
    ADD_A__HL_,
    ADD_A_E,
    AND_L,
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
    INC_A,
    INC_B,
    INC_BC,
    INC_C,
    INC_DE,
    INC_H,
    INC_HL,
    INC_SP,
    JR_NZ_r8,
    JR_r8,
    JR_Z_r8,
    LD__a16__A,
    LD__a16__SP,
    LD__C__A,
    LD__HL__A,
    LD__HL__E,
    LD_A__DE_,
    LD_A_B,
    LD_A_d8,
    LD_A_E,
    LD_A_H,
    LD_A_L,
    LD_B_D,
    LD_B_d8,
    LD_C_A,
    LD_C_d8,
    LD_D_A,
    LD_D_d8,
    LD_DE_d16,
    LD_E_d8,
    LD_H__HL_,
    LD_H_A,
    LD_H_E,
    LD_HL_d16,
    LD_HL_minus_A,
    LD_HL_plus_A,
    LD_L__HL_,
    LD_L_d8,
    LD_SP_A,
    LDH__a8__A,
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
    XOR_A,
}

/*
0x00 LD SP, 0xFFFE
0x03 XOR A
0x04 LD HL, 0x9FFF
0x07 LD (HL-), A
0x08 BIT 7, H
0x0A JR NZ, 0xFB
0x0C LD HL, 0xFF26
0x0F LD C, 0x11
0x11 LD A, 0x80
0x13 LD (HL-), A
0x14 LD (C), A
0x15 INC C

00000000  31 fe ff af 21 ff 9f 32  cb 7c 20 fb 21 26 ff 0e
00000010  11 3e 80 32 e2 0c 3e f3  e2 32 3e 77 77 3e fc e0

*/

#[derive(Debug)]
pub struct DecodedInstruction {
    pub address: u16,
    pub instruction: Instruction,
    pub instruction_size: u8,
}

pub fn decode_next_instruction(address: u16, slice_at_program_counter: &[u8]) -> DecodedInstruction {
    let (i, s) = match slice_at_program_counter[0] {
        0x00 => (Instruction::NOP, 1),
        0x03 => (Instruction::INC_BC, 1),
        0x04 => (Instruction::INC_B, 1),
        0x05 => (Instruction::DEC_B, 1),
        0x06 => (Instruction::LD_B_d8, 2),
        0x08 => (Instruction::LD__a16__SP, 3),
        0x0B => (Instruction::DEC_BC, 1),
        0x0C => (Instruction::INC_C, 1),
        0x0D => (Instruction::DEC_C, 1),
        0x0E => (Instruction::LD_C_d8, 2),
        0x11 => (Instruction::LD_DE_d16, 3),
        0x13 => (Instruction::INC_DE, 1),
        0x15 => (Instruction::DEC_D, 1),
        0x16 => (Instruction::LD_D_d8, 2),
        0x17 => (Instruction::RLA, 1),
        0x18 => (Instruction::JR_r8, 2),
        0x1A => (Instruction::LD_A__DE_, 1),
        0x1D => (Instruction::DEC_E, 1),
        0x1E => (Instruction::LD_E_d8, 2),
        0x20 => (Instruction::JR_NZ_r8, 2),
        0x21 => (Instruction::LD_HL_d16, 3),
        0x22 => (Instruction::LD_HL_plus_A, 1),
        0x23 => (Instruction::INC_HL, 1),
        0x24 => (Instruction::INC_H, 1),
        0x28 => (Instruction::JR_Z_r8, 2),
        0x2E => (Instruction::LD_L_d8, 2),
        0x31 => (Instruction::LD_SP_A, 3),
        0x32 => (Instruction::LD_HL_minus_A, 1),
        0x33 => (Instruction::INC_SP, 1),
        0x3C => (Instruction::INC_A, 1),
        0x3D => (Instruction::DEC_A, 1),
        0x3E => (Instruction::LD_A_d8, 2),
        0x42 => (Instruction::LD_B_D, 1),
        0x4F => (Instruction::LD_C_A, 1),
        0x57 => (Instruction::LD_D_A, 1),
        0x63 => (Instruction::LD_H_E, 1),
        0x66 => (Instruction::LD_H__HL_, 1),
        0x67 => (Instruction::LD_H_A, 1),
        0x6E => (Instruction::LD_L__HL_, 1),
        0x73 => (Instruction::LD__HL__E, 1),
        0x77 => (Instruction::LD__HL__A, 1),
        0x78 => (Instruction::LD_A_B, 1),
        0x7B => (Instruction::LD_A_E, 1),
        0x7C => (Instruction::LD_A_H, 1),
        0x7D => (Instruction::LD_A_L, 1),
        0x83 => (Instruction::ADD_A_E, 1),
        0x86 => (Instruction::ADD_A__HL_, 1),
        0x88 => (Instruction::ADC_A_B, 1),
        0x89 => (Instruction::ADC_A_C, 1),
        0x90 => (Instruction::SUB_B, 1),
        0x99 => (Instruction::SBC_A_C, 1),
        0x9F => (Instruction::SBC_A_A, 1),
        0xA5 => (Instruction::AND_L, 1),
        0xAF => (Instruction::XOR_A, 1),
        0xB9 => (Instruction::CP_C, 1),
        0xBB => (Instruction::CP_E, 1),
        0xBE => (Instruction::CP__HL_, 1),
        0xC1 => (Instruction::RET_NZ, 1),
        0xC9 => (Instruction::RET, 1),
        0xCB => match slice_at_program_counter[1] {
            _ => {
                (Instruction::Prefix, 2) // 1 for prefix, 1 for extension?
                                         // println!("TODO: CB-prefixed opcode 0x{:x}", slice_at_program_counter[1]);
                                         // todo!()
            }
        },
        0xC5 => (Instruction::PUSH_BC, 1),
        0xCC => (Instruction::CALL_Z_a16, 3),
        0xCD => (Instruction::CALL_a16, 3),
        0xCE => (Instruction::ADC_A_d8, 2),
        0xD9 => (Instruction::RETI, 1),
        0xE0 => (Instruction::LDH__a8__A, 2),
        0xE2 => (Instruction::LD__C__A, 1),
        0xEA => (Instruction::LD__a16__A, 3),
        0xF0 => (Instruction::LDH_A__a8__, 2),
        0xFB => (Instruction::EI, 1),
        0xFE => (Instruction::CP_d8, 2),
        b => (Instruction::Illegal(b), 1),
    };
    DecodedInstruction {
        instruction: i,
        instruction_size: s,
        address,
    }
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
}
