use crate::{
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
    println!("Comparing {:02X} and {:02X}", a, b);
    cpu.registers
        .write_flag(Flag::Z, a == b)
        .set_flag(Flag::N)
        .write_flag(Flag::H, sub_borrows(a, b, 4))
        .write_flag(Flag::C, sub_borrows(a, b, 8));
}

impl Instruction {
    pub fn execute(self: &Instruction, cpu: &mut CPU) {
        match self {
            Instruction::ADD_A_r8(r8) => {
                let a = cpu.registers.read_a();
                let r8 = cpu.registers.read_r8(r8);
                let res = a.wrapping_add(r8);
                cpu.registers
                    .write_a(res)
                    .write_flag(Flag::Z, res == 0)
                    .unset_flag(Flag::N)
                    .write_flag(Flag::H, add_produces_carry(a, r8, 4))
                    .write_flag(Flag::C, add_produces_carry(a, r8, 8));
            }
            Instruction::BIT_u3_r8(bit, reg) => {
                cpu.registers
                    .write_flag(Flag::Z, !cpu.registers.get_bit(reg, bit))
                    .unset_flag(Flag::N)
                    .set_flag(Flag::H);
            }
            Instruction::CALL_a16(imm16) => {
                // Note: This assumes the PC is already pointing to the next instruction
                cpu.push_imm16(Immediate16::from_u16(cpu.registers.pc));
                cpu.registers.pc = imm16.as_u16();
            }
            Instruction::CP_A_u8(u8) => {
                compare(cpu, cpu.registers.read_a(), *u8);
            }
            Instruction::CP_A_mHL => {
                compare(
                    cpu,
                    cpu.registers.read_a(),
                    cpu.memory.read_u8(cpu.registers.read_r16(&R16::HL)),
                );
            }
            Instruction::DEC_r8(r8) => {
                let r8val = cpu.registers.read_r8(r8);
                let res = r8val.wrapping_sub(1);
                cpu.registers
                    .write_r8(r8, res)
                    .write_flag(Flag::Z, res == 0)
                    .set_flag(Flag::N)
                    .write_flag(Flag::H, sub_borrows(r8val, 1 as u8, 4));
            }
            Instruction::INC_r8(r8) => {
                let res = cpu.registers.read_r8(r8).wrapping_add(1);
                cpu.registers
                    .write_r8(r8, res)
                    .write_flag(Flag::Z, res == 0)
                    .unset_flag(Flag::N)
                    .write_flag(Flag::H, res == 0x10);
            }
            Instruction::INC_r16(r16) => {
                let res = cpu.registers.read_r16(r16).wrapping_add(1);
                cpu.registers.write_r16(r16, res);
            }
            Instruction::JR_i8(i8) => {
                cpu.registers.pc = cpu
                    .registers
                    .pc
                    .checked_add_signed(*i8 as i16)
                    .expect("JR_i8 overflowed");
            }
            Instruction::JR_cc_i8(cc, i8) => {
                if cc.holds(cpu) {
                    cpu.registers.pc = cpu
                        .registers
                        .pc
                        .checked_add_signed(*i8 as i16)
                        .expect("JR_cc_i8 overflowed")
                }
            }
            Instruction::LD__a8__A(u8) => {
                cpu.memory
                    .write_u8(0xFF00 + *u8 as u16, cpu.registers.read_a());
            }
            Instruction::LD_mC_A => {
                cpu.memory.write_u8(
                    0xFF00 + cpu.registers.read_c() as u16,
                    cpu.registers.read_a(),
                );
            }
            Instruction::LD_r8_r8(r8a, r8b) => {
                cpu.registers.write_r8(r8a, cpu.registers.read_r8(r8b));
            }
            Instruction::LD_r16_d16(r16, imm16) => {
                cpu.registers.write_r16(r16, imm16.as_u16());
            }
            Instruction::LD_mr16_A(mr16) => {
                cpu.memory.write_u8(mr16.as_u16(), cpu.registers.read_a());
            }
            Instruction::LD_mr16_r8(mr16, r8) => {
                cpu.memory
                    .write_u8(cpu.registers.read_r16(mr16), cpu.registers.read_r8(r8));
            }
            Instruction::LD_mHLdec_A => {
                cpu.memory
                    .write_u8(cpu.registers.hl, cpu.registers.read_a());
                cpu.registers.hl -= 1;
            }
            Instruction::LD_mHLinc_A => {
                cpu.memory
                    .write_u8(cpu.registers.hl, cpu.registers.read_a());
                cpu.registers.hl += 1;
            }
            Instruction::LD_A_mu8(u8) => {
                cpu.registers
                    .write_a(cpu.memory.read_u8(0xFF00 + *u8 as u16));
            }
            Instruction::LD_r8_u8(r8, u8) => {
                cpu.registers.write_r8(r8, *u8);
            }
            Instruction::LD_r8_mr16(r8, r16) => {
                cpu.registers
                    .write_r8(r8, cpu.memory.read_u8(cpu.registers.read_r16(r16)));
            }
            Instruction::LD_SP_A(imm) => {
                cpu.registers.sp = imm.as_u16();
            }
            Instruction::NOP => {}
            Instruction::POP_r16(r16) => {
                cpu.pop_r16(r16);
            }
            Instruction::PUSH_r16(r16) => {
                cpu.push_imm16(Immediate16::from_u16(cpu.registers.read_r16(r16)));
            }
            Instruction::RET => {
                cpu.pop_r16(&R16::PC);
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
            }
            Instruction::SUB_r8(r8) => {
                let a = cpu.registers.read_a();
                let r8 = cpu.registers.read_r8(r8);
                let res = a.wrapping_sub(r8);
                cpu.registers
                    .write_a(res)
                    .write_flag(Flag::Z, res == 0)
                    .set_flag(Flag::N)
                    .write_flag(Flag::H, sub_borrows(a, r8, 4))
                    .write_flag(Flag::C, sub_borrows(a, r8, 8));
            }
            Instruction::XOR_r8(r8) => {
                let res = cpu.registers.read_a() ^ cpu.registers.read_r8(r8);
                cpu.registers
                    .write_a(res)
                    .write_flag(Flag::Z, res == 0)
                    .unset_flag(Flag::N)
                    .unset_flag(Flag::H)
                    .unset_flag(Flag::C);
            }
            i => panic!("No semantics for: {}", i),
        };
    }
}
