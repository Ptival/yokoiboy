use crate::{
    conditions::Condition,
    cpu::CPU,
    registers::{Flag, R16, R8},
};

use super::type_def::{Immediate16, Instruction};

// Checks whether adding a and b with bitsize (bit - 1) would produce a carry (1) at position bit.
// Assumes bit < 16, so that all operations can be carried without loss as u32.
fn add_produces_carry(a: impl Into<u16>, b: impl Into<u16>, bit: u8) -> bool {
    let a = a.into() as u32;
    let b = b.into() as u32;
    let bit_mask = 1 << bit;
    let input_mask = bit_mask - 1;
    ((a & input_mask) + (b & input_mask)) & bit_mask == bit_mask
}

// Checks whether subtracting b from a with bitsize (bit - 1) would produce a borrow at position
// bit.  Assumes bit < 16, so that all operations can be carried without loss as u32.
fn sub_borrows(a: impl Into<u16>, b: impl Into<u16>, bit: u8) -> bool {
    let a = a.into() as u32;
    let b = b.into() as u32;
    let bit_mask = 1 << bit;
    let input_mask = (1 << bit) - 1;
    // Put a 1 in borrowable position, then borrow occured if it became a 0
    ((bit_mask | (a & input_mask)) - (b & input_mask)) & bit_mask == 0
}

fn compare(cpu: &mut CPU, a: u8, b: u8) {
    // println!("Comparing {:02X} and {:02X}", a, b);
    cpu.registers
        .write_flag(Flag::Z, a == b)
        .set_flag(Flag::N)
        .write_flag(Flag::H, sub_borrows(a, b, 4))
        .write_flag(Flag::C, sub_borrows(a, b, 8));
}

fn add(cpu: &mut CPU, a: u8, b: u8) {
    let res = a.wrapping_add(b);
    cpu.registers
        .write_a(res)
        .write_flag(Flag::Z, res == 0)
        .unset_flag(Flag::N)
        .write_flag(Flag::H, add_produces_carry(a, b, 4))
        .write_flag(Flag::C, add_produces_carry(a, b, 8));
}

fn and(cpu: &mut CPU, a: u8, b: u8) {
    let res = a & b;
    cpu.registers
        .write_a(res)
        .write_flag(Flag::Z, res == 0)
        .unset_flag(Flag::N)
        .set_flag(Flag::H)
        .unset_flag(Flag::C);
}

fn or(cpu: &mut CPU, a: u8, b: u8) {
    let res = a | b;
    cpu.registers
        .write_a(res)
        .write_flag(Flag::Z, res == 0)
        .unset_flag(Flag::N)
        .unset_flag(Flag::H)
        .unset_flag(Flag::C);
}

fn call(cpu: &mut CPU, address: u16) {
    cpu.push_imm16(Immediate16::from_u16(cpu.registers.pc));
    cpu.registers.pc = address;
}

impl Instruction {
    pub fn execute(self: &Instruction, cpu: &mut CPU) -> (u8, u8) {
        match self {
            Instruction::ADC_A_r8(_) => todo!(),

            Instruction::ADC_A_u8(_) => todo!(),

            Instruction::ADD_A_mHL => {
                add(
                    cpu,
                    cpu.registers.read_a(),
                    cpu.memory.read_u8(cpu.registers.hl),
                );
                (8, 2)
            }

            Instruction::ADD_A_r8(r8) => {
                add(cpu, cpu.registers.read_a(), cpu.registers.read_r8(r8));
                (4, 1)
            }

            Instruction::ADD_A_u8(u8) => {
                add(cpu, cpu.registers.read_a(), *u8);
                (8, 2)
            }

            Instruction::AND_L => todo!(),

            Instruction::AND_u8(u8) => {
                and(cpu, cpu.registers.read_a(), *u8);
                (8, 2)
            }

            Instruction::BIT_u3_r8(bit, reg) => {
                cpu.registers
                    .write_flag(Flag::Z, !cpu.registers.get_bit(reg, bit))
                    .unset_flag(Flag::N)
                    .set_flag(Flag::H);
                (8, 2)
            }

            Instruction::CALL_a16(imm16) => {
                call(cpu, imm16.as_u16());
                (24, 6)
            }

            Instruction::CALL_cc_u16(cc, imm16) => {
                if cc.holds(cpu) {
                    call(cpu, imm16.as_u16());
                    (24, 6)
                } else {
                    (12, 3)
                }
            }

            Instruction::CALL_Z_a16(_) => todo!(),

            Instruction::CP_A_r8(_) => todo!(),

            Instruction::CP_A_u8(u8) => {
                compare(cpu, cpu.registers.read_a(), *u8);
                (8, 2)
            }

            Instruction::CP_A_mHL => {
                compare(
                    cpu,
                    cpu.registers.read_a(),
                    cpu.memory.read_u8(cpu.registers.read_r16(&R16::HL)),
                );
                (8, 2)
            }

            Instruction::DEC_r8(r8) => {
                let r8val = cpu.registers.read_r8(r8);
                let res = r8val.wrapping_sub(1);
                cpu.registers
                    .write_r8(r8, res)
                    .write_flag(Flag::Z, res == 0)
                    .set_flag(Flag::N)
                    .write_flag(Flag::H, sub_borrows(r8val, 1 as u8, 4));
                (4, 1)
            }

            Instruction::DEC_r16(_) => todo!(),

            Instruction::DI => {
                cpu.registers.ime = false;
                (4, 1)
            }

            Instruction::EI => {
                // FIXME: This should apparently be delayed until the next instruction has finished.
                cpu.registers.ime = true;
                (4, 1)
            }

            Instruction::INC_r8(r8) => {
                let r8val = cpu.registers.read_r8(r8);
                let res = r8val.wrapping_add(1);
                cpu.registers
                    .write_r8(r8, res)
                    .write_flag(Flag::Z, res == 0)
                    .unset_flag(Flag::N)
                    .write_flag(Flag::H, add_produces_carry(r8val, 1 as u16, 4));
                (4, 1)
            }

            Instruction::INC_r16(r16) => {
                let res = cpu.registers.read_r16(r16).wrapping_add(1);
                cpu.registers.write_r16(r16, res);
                (8, 2)
            }

            Instruction::JR_r8(_) => todo!(),

            Instruction::JP_u16(imm16) => {
                cpu.registers.pc = imm16.as_u16();
                (16, 4)
            }

            Instruction::JR_i8(i8) => {
                cpu.registers.pc = cpu
                    .registers
                    .pc
                    .checked_add_signed(*i8 as i16)
                    .expect("JR_i8 overflowed");
                (12, 3)
            }

            Instruction::JR_cc_i8(cc, i8) => {
                if cc.holds(cpu) {
                    cpu.registers.pc = cpu
                        .registers
                        .pc
                        .checked_add_signed(*i8 as i16)
                        .expect("JR_cc_i8 overflowed");
                    (12, 3)
                } else {
                    (8, 2)
                }
            }

            Instruction::LD_A_mHLinc => {
                cpu.registers.write_a(cpu.memory.read_u8(cpu.registers.hl));
                cpu.registers.hl += 1;
                (8, 2)
            }

            Instruction::LD_FFu8_A(u8) => {
                cpu.memory
                    .write_u8(0xFF00 + *u8 as u16, cpu.registers.read_a());
                (12, 3)
            }

            Instruction::LD_mu16_A(imm16) => {
                cpu.memory.write_u8(imm16.as_u16(), cpu.registers.read_a());
                (16, 4)
            }

            Instruction::LD_mu16_SP(imm16) => {
                let sp = Immediate16::from_u16(cpu.registers.sp);
                let address = imm16.as_u16();
                cpu.memory.write_u8(address, sp.lower_byte);
                cpu.memory.write_u8(address + 1, sp.higher_byte);
                (20, 5)
            }

            Instruction::LD_H_mHL => todo!(),

            Instruction::LD_L_mHL => todo!(),

            Instruction::LD_FFC_A => {
                cpu.memory.write_u8(
                    0xFF00 + cpu.registers.read_c() as u16,
                    cpu.registers.read_a(),
                );
                (8, 2)
            }

            Instruction::LD_r8_r8(r8a, r8b) => {
                cpu.registers.write_r8(r8a, cpu.registers.read_r8(r8b));
                (4, 1)
            }

            Instruction::LD_r16_d16(r16, imm16) => {
                cpu.registers.write_r16(r16, imm16.as_u16());
                (12, 3)
            }

            Instruction::LD_mr16_r8(mr16, r8) => {
                cpu.memory
                    .write_u8(cpu.registers.read_r16(mr16), cpu.registers.read_r8(r8));
                (8, 2)
            }

            Instruction::LD_mHLdec_A => {
                cpu.memory
                    .write_u8(cpu.registers.hl, cpu.registers.read_a());
                cpu.registers.hl -= 1;
                (8, 2)
            }

            Instruction::LD_mHLinc_A => {
                cpu.memory
                    .write_u8(cpu.registers.hl, cpu.registers.read_a());
                cpu.registers.hl += 1;
                (8, 2)
            }

            Instruction::LD_A_FFu8(u8) => {
                cpu.registers
                    .write_a(cpu.memory.read_u8(0xFF00 + *u8 as u16));
                (12, 3)
            }

            Instruction::LD_A_mu16(imm16) => {
                cpu.registers.write_a(cpu.memory.read_u8(imm16.as_u16()));
                (16, 4)
            }

            Instruction::LD_r8_u8(r8, u8) => {
                cpu.registers.write_r8(r8, *u8);
                (8, 2)
            }

            Instruction::LD_r8_mr16(r8, r16) => {
                cpu.registers
                    .write_r8(r8, cpu.memory.read_u8(cpu.registers.read_r16(r16)));
                (8, 2)
            }

            Instruction::LD_SP_u16(imm16) => {
                cpu.registers.sp = imm16.as_u16();
                (12, 3)
            }

            Instruction::NOP => (4, 1),

            Instruction::OR_r8(r8) => {
                or(cpu, cpu.registers.read_a(), cpu.registers.read_r8(r8));
                (8, 2)
            }

            Instruction::POP_r16(r16) => {
                cpu.pop_r16(r16);
                (12, 3)
            }

            Instruction::PUSH_r16(r16) => {
                cpu.push_imm16(Immediate16::from_u16(cpu.registers.read_r16(r16)));
                (16, 4)
            }

            Instruction::RET => {
                cpu.pop_r16(&R16::PC);
                (16, 4)
            }

            Instruction::RET_C => {
                if Condition::C.holds(cpu) {
                    cpu.pop_r16(&R16::PC);
                    (20, 5)
                } else {
                    (8, 2)
                }
            }

            Instruction::RLA => {
                // Note: for some reason, this always unsets Z
                let carry = cpu.registers.get_flag(Flag::C) as u16;
                let result_u16 = ((cpu.registers.read_a() as u16) << 1) | carry;
                let result = result_u16 as u8;
                cpu.registers
                    .write_r8(&R8::A, result)
                    .unset_flag(Flag::Z)
                    .unset_flag(Flag::N)
                    .unset_flag(Flag::H)
                    .write_flag(Flag::C, (result_u16 & 0xFF00) != 0);
                (4, 1)
            }

            Instruction::RL_r8(r8) => {
                // Doing this as u16 to detect overflow easily
                let carry = cpu.registers.get_flag(Flag::C) as u16;
                let result_u16 = ((cpu.registers.read_r8(r8) as u16) << 1) | carry;
                let result = result_u16 as u8;
                cpu.registers
                    .write_r8(r8, result)
                    .write_flag(Flag::Z, result == 0)
                    .unset_flag(Flag::N)
                    .unset_flag(Flag::H)
                    .write_flag(Flag::C, (result_u16 & 0xFF00) != 0);
                (8, 2)
            }

            Instruction::SUB_A_r8(r8) => {
                let a = cpu.registers.read_a();
                let r8 = cpu.registers.read_r8(r8);
                let res = a.wrapping_sub(r8);
                cpu.registers
                    .write_a(res)
                    .write_flag(Flag::Z, res == 0)
                    .set_flag(Flag::N)
                    .write_flag(Flag::H, sub_borrows(a, r8, 4))
                    .write_flag(Flag::C, sub_borrows(a, r8, 8));
                (4, 1)
            }

            Instruction::XOR_r8(r8) => {
                let res = cpu.registers.read_a() ^ cpu.registers.read_r8(r8);
                cpu.registers
                    .write_a(res)
                    .write_flag(Flag::Z, res == 0)
                    .unset_flag(Flag::N)
                    .unset_flag(Flag::H)
                    .unset_flag(Flag::C);
                (4, 1)
            }

            Instruction::Prefix => todo!(),

            Instruction::RET_cc(_) => todo!(),

            Instruction::RETI => todo!(),

            Instruction::SBC_A_A => todo!(),

            Instruction::SBC_A_C => todo!(),
        }
    }
}
