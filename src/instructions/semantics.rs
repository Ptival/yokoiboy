use std::num::Wrapping;

use crate::{
    cpu::{interrupts::Interrupts, CPU},
    machine::Machine,
    registers::{Flag, R16},
};

use super::type_def::{Immediate16, Instruction};

// Checks whether adding a and b with bitsize (bit - 1) would produce a carry (1) at position bit.
// Assumes bit < 16, so that all operations can be carried without loss as u32.
fn add_produces_carry(a: impl Into<u16>, b: impl Into<i32>, c: bool, bit: u8) -> bool {
    let a = a.into() as i32;
    let b = b.into();
    let bit_mask = 1 << bit;
    let input_mask = bit_mask - 1;
    ((a & input_mask) + (b & input_mask) + c as i32) & bit_mask == bit_mask
}

// Checks whether subtracting b from a with bitsize (bit - 1) would produce a borrow at position
// bit.  Assumes bit < 16, so that all operations can be carried without loss as u32.
fn sub_borrows(a: impl Into<u16>, b: impl Into<u16>, c: bool, bit: u8) -> bool {
    let a = a.into() as u32;
    let b = b.into() as u32;
    let bit_mask = 1 << bit;
    let input_mask = (1 << bit) - 1;
    // Put a 1 in borrowable position, then borrow occured if it became a 0
    ((bit_mask | (a & input_mask)) - (b & input_mask) - (c as u32)) & bit_mask == 0
}

fn compare(cpu: &mut CPU, a: &Wrapping<u8>, b: &Wrapping<u8>) {
    cpu.registers.znhc(
        *a == *b,
        true,
        sub_borrows(a.0, b.0, false, 4),
        sub_borrows(a.0, b.0, false, 8),
    );
}

fn adc(cpu: &mut CPU, a: &Wrapping<u8>, b: &Wrapping<u8>, c: bool) {
    let res = a + b + Wrapping(c as u8);
    cpu.registers.write_a(res).znhc(
        res.0 == 0,
        false,
        add_produces_carry(a.0, b.0, c, 4),
        add_produces_carry(a.0, b.0, c, 8),
    );
}

fn add(cpu: &mut CPU, a: &Wrapping<u8>, b: &Wrapping<u8>) {
    adc(cpu, a, b, false)
}

fn and(cpu: &mut CPU, a: &Wrapping<u8>, b: &Wrapping<u8>) {
    let res = a & b;
    cpu.registers
        .write_a(res)
        .znhc(res.0 == 0, false, true, false);
}

fn or(cpu: &mut CPU, a: &Wrapping<u8>, b: &Wrapping<u8>) {
    let res = a | b;
    cpu.registers
        .write_a(res)
        .znhc(res.0 == 0, false, false, false);
}

// NOTE: This does not write the result anywhere!
// NOTE: This does not set the flags like SUB.
fn dec(cpu: &mut CPU, a: &Wrapping<u8>) -> Wrapping<u8> {
    let res = a - Wrapping(1);
    cpu.registers
        .write_flag(Flag::Z, res.0 == 0)
        .set_flag(Flag::N)
        .write_flag(Flag::H, sub_borrows(a.0, 1 as u8, false, 4));
    res
}

fn subc(cpu: &mut CPU, a: &Wrapping<u8>, b: &Wrapping<u8>, c: bool) {
    let res = a - b - Wrapping(c as u8);
    cpu.registers.write_a(res).znhc(
        res.0 == 0,
        true,
        sub_borrows(a.0, b.0, c, 4),
        sub_borrows(a.0, b.0, c, 8),
    );
}

fn sub(cpu: &mut CPU, a: &Wrapping<u8>, b: &Wrapping<u8>) {
    subc(cpu, a, b, false)
}

fn xor(cpu: &mut CPU, a: &Wrapping<u8>, b: &Wrapping<u8>) {
    let res = a ^ b;
    cpu.registers
        .write_a(res)
        .znhc(res.0 == 0, false, false, false);
}

fn call(machine: &mut Machine, address: Wrapping<u16>) {
    CPU::push_imm16(machine, Immediate16::from_u16(machine.cpu.registers.pc));
    machine.cpu.registers.pc = address;
}

impl Instruction {
    pub fn execute(self: &Instruction, machine: &mut Machine) -> (u8, u8) {
        // EI effects are delayed by one instruction, we resolve it here
        if machine.cpu.interrupts.interrupt_master_enable_delayed {
            machine.cpu.interrupts.interrupt_master_enable_delayed = false;
            machine.cpu.interrupts.interrupt_master_enable = true;
        }

        match self {
            Instruction::ADC_A_mHL => {
                let a = machine.cpu.registers.read_a();
                let b = machine.read_u8(machine.cpu.registers.hl);
                let c = machine.cpu.registers.read_flag(Flag::C);
                adc(&mut machine.cpu, &a, &b, c);
                (8, 2)
            }

            Instruction::ADC_A_r8(r8) => {
                let a = machine.cpu.registers.read_a();
                let b = machine.cpu.registers.read_r8(r8);
                let c = machine.cpu.registers.read_flag(Flag::C);
                adc(&mut machine.cpu, &a, &b, c);
                (4, 1)
            }

            Instruction::ADC_A_u8(u8) => {
                let a = machine.cpu.registers.read_a();
                let c = machine.cpu.registers.read_flag(Flag::C);
                adc(&mut machine.cpu, &a, u8, c);
                (8, 2)
            }

            Instruction::ADD_A_mHL => {
                let a = machine.cpu.registers.read_a();
                let b = machine.read_u8(machine.cpu.registers.hl);
                add(&mut machine.cpu, &a, &b);
                (8, 2)
            }

            Instruction::ADD_A_r8(r8) => {
                let a = machine.cpu.registers.read_a();
                let b = machine.cpu.registers.read_r8(r8);
                add(&mut machine.cpu, &a, &b);
                (4, 1)
            }

            Instruction::ADD_A_u8(u8) => {
                let a = machine.cpu.registers.read_a();
                add(&mut machine.cpu, &a, u8);
                (8, 2)
            }

            Instruction::ADD_HL_r16(r16) => {
                let a = machine.cpu.registers.hl;
                let b = machine.cpu.registers.read_r16(r16);
                let res = a + b;
                machine
                    .cpu
                    .registers
                    .write_r16(&R16::HL, res)
                    .unset_flag(Flag::N)
                    .write_flag(Flag::H, add_produces_carry(a.0, b.0, false, 12))
                    .write_flag(Flag::C, add_produces_carry(a.0, b.0, false, 16));
                (8, 2)
            }

            Instruction::ADD_SP_i8(i8) => {
                let a = machine.cpu.registers.sp;
                let res = Wrapping(a.0.wrapping_add_signed(i8.0 as i16));
                machine.cpu.registers.write_r16(&R16::SP, res).znhc(
                    false,
                    false,
                    add_produces_carry(a.0, i8.0, false, 4),
                    add_produces_carry(a.0, i8.0, false, 8),
                );
                (16, 4)
            }

            Instruction::AND_A_mHL => {
                let a = machine.cpu.registers.read_a();
                let b = machine.read_u8(machine.cpu.registers.hl);
                and(&mut machine.cpu, &a, &b);
                (8, 2)
            }

            Instruction::AND_A_r8(r8) => {
                let a = machine.cpu.registers.read_a();
                let b = machine.cpu.registers.read_r8(r8);
                and(&mut machine.cpu, &a, &b);
                (4, 1)
            }

            Instruction::AND_u8(u8) => {
                let a = machine.cpu.registers.read_a();
                and(&mut machine.cpu, &a, u8);
                (8, 2)
            }

            Instruction::BIT_u3_mHL(bit_position) => {
                let address = machine.cpu.registers.hl;
                let value = ((machine.read_u8(address).0 >> bit_position) & 0x1) == 0x1;
                bit_complement(&mut machine.cpu, value);
                (12, 3)
            }

            Instruction::BIT_u3_r8(bit_position, reg) => {
                let value = machine.cpu.registers.get_bit(reg, bit_position);
                bit_complement(&mut machine.cpu, value);
                (8, 2)
            }

            Instruction::CALL_a16(imm16) => {
                call(machine, imm16.as_u16());
                (24, 6)
            }

            Instruction::CALL_cc_u16(cc, imm16) => {
                if cc.holds(&machine.cpu) {
                    call(machine, imm16.as_u16());
                    (24, 6)
                } else {
                    (12, 3)
                }
            }

            Instruction::CCF => {
                let c = machine.cpu.registers.read_flag(Flag::C);
                machine
                    .cpu
                    .registers
                    .unset_flag(Flag::N)
                    .unset_flag(Flag::H)
                    .write_flag(Flag::C, !c);
                (4, 1)
            }

            Instruction::CP_A_r8(r8) => {
                let a = machine.cpu.registers.read_a();
                let b = machine.cpu.registers.read_r8(r8);
                compare(&mut machine.cpu, &a, &b);
                (4, 1)
            }

            Instruction::CP_A_u8(u8) => {
                let a = machine.cpu.registers.read_a();
                compare(&mut machine.cpu, &a, u8);
                (8, 2)
            }

            Instruction::CP_A_mHL => {
                let a = machine.cpu.registers.read_a();
                let address = machine.cpu.registers.read_r16(&R16::HL);
                let b = machine.read_u8(address);
                compare(&mut machine.cpu, &a, &b);
                (8, 2)
            }

            Instruction::CPL => {
                let a = machine.cpu.registers.read_a();
                machine
                    .cpu
                    .registers
                    .write_a(Wrapping(!a.0))
                    .set_flag(Flag::N)
                    .set_flag(Flag::H);
                (4, 1)
            }

            Instruction::DAA => {
                let mut data = Wrapping(machine.cpu.registers.read_a().0 as u16);
                let subtraction_flag = machine.cpu.registers.read_flag(Flag::N);
                let mut half_carry = machine.cpu.registers.read_flag(Flag::H);
                let mut carry = machine.cpu.registers.read_flag(Flag::C);
                if subtraction_flag {
                    // post-subtraction
                    if half_carry {
                        data -= Wrapping(0x06);
                    }
                    if carry {
                        data -= Wrapping(0x60);
                    }
                } else {
                    // post-addition
                    if half_carry || ((data.0 & 0x0F) > 0x09) {
                        data += Wrapping(0x06);
                        half_carry = true; // set in case we entered because of the right condition
                    }
                    if carry || ((data.0 & 0x1FF) > 0x9F) {
                        data += Wrapping(0x60);
                        carry = true; // set in case we entered because of the right condition
                    }
                }

                machine
                    .cpu
                    .registers
                    .write_a(Wrapping(data.0 as u8))
                    .write_flag(Flag::Z, data.0 == 0)
                    .write_flag(Flag::H, half_carry)
                    .write_flag(Flag::C, carry);

                (4, 1)
            }

            Instruction::DEC_mHL => {
                let a = machine.read_u8(machine.cpu.registers.hl);
                let res = dec(&mut machine.cpu, &a);
                machine.write_u8(machine.cpu.registers.hl, res);
                (12, 3)
            }

            Instruction::DEC_r8(r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = dec(&mut machine.cpu, &a);
                machine.cpu.registers.write_r8(r8, res);
                (4, 1)
            }

            Instruction::DEC_r16(r16) => {
                let a = machine.cpu.registers.read_r16(r16);
                let res = a - Wrapping(1);
                machine.cpu.registers.write_r16(r16, res);
                (8, 2)
            }

            Instruction::DI => {
                machine.cpu.interrupts.interrupt_master_enable = false;
                (4, 1)
            }

            // NOTE: This sets up IME in one instruction
            Instruction::EI => {
                machine.cpu.interrupts.interrupt_master_enable_delayed = true;
                (4, 1)
            }

            Instruction::HALT => {
                if machine.cpu.interrupts.interrupt_master_enable {
                    machine.cpu.low_power_mode = true;
                } else {
                    if Interrupts::is_interrupt_pending(machine) {
                        // TODO: emulate HALT bug
                        machine.cpu.low_power_mode = true;
                    } else {
                        machine.cpu.low_power_mode = true;
                    }
                }
                (4, 1)
            }

            Instruction::Illegal(opcode) => {
                panic!("Attempted to execute an illegal opcode: 0x{:02X}", opcode)
            }

            Instruction::INC_r8(r8) => {
                // NOTE: Can't use `add` because we don't want to touch Flag::C
                let r8val = machine.cpu.registers.read_r8(r8);
                let res = r8val + Wrapping(1);
                machine
                    .cpu
                    .registers
                    .write_r8(r8, res)
                    .write_flag(Flag::Z, res.0 == 0)
                    .unset_flag(Flag::N)
                    .write_flag(Flag::H, add_produces_carry(r8val.0, 1 as u16, false, 4));
                (4, 1)
            }

            Instruction::INC_r16(r16) => {
                let res = machine.cpu.registers.read_r16(r16) + Wrapping(1);
                machine.cpu.registers.write_r16(r16, res);
                (8, 2)
            }

            Instruction::INC_mHL => {
                let res = machine.read_u8(machine.cpu.registers.hl) + Wrapping(1);
                machine.write_u8(machine.cpu.registers.hl, res);
                (12, 3)
            }

            Instruction::JR_r8(_) => todo!(),

            Instruction::JP_u16(imm16) => {
                machine.cpu.registers.pc = imm16.as_u16();
                (16, 4)
            }

            Instruction::JP_cc_u16(cc, imm16) => {
                if cc.holds(&machine.cpu) {
                    machine.cpu.registers.pc = imm16.as_u16();
                    (16, 4)
                } else {
                    (12, 3)
                }
            }

            Instruction::JP_HL => {
                machine.cpu.registers.pc = machine.cpu.registers.hl;
                (4, 1)
            }

            Instruction::JR_i8(i8) => {
                let pc = machine.cpu.registers.pc.0;
                machine.cpu.registers.pc = Wrapping(pc.wrapping_add_signed((*i8).0 as i16));
                (12, 3)
            }

            Instruction::JR_cc_i8(cc, i8) => {
                let pc = machine.cpu.registers.pc.0;
                if cc.holds(&machine.cpu) {
                    machine.cpu.registers.pc = Wrapping(pc.wrapping_add_signed((*i8).0 as i16));
                    (12, 3)
                } else {
                    (8, 2)
                }
            }

            Instruction::LD_A_mr16(r16) => {
                machine
                    .cpu
                    .registers
                    .write_a(machine.read_u8(machine.cpu.registers.read_r16(r16)));
                (8, 2)
            }

            Instruction::LD_A_mHLdec => {
                machine
                    .cpu
                    .registers
                    .write_a(machine.read_u8(machine.cpu.registers.hl));
                machine.cpu.registers.hl -= 1;
                (8, 2)
            }

            Instruction::LD_A_mHLinc => {
                machine
                    .cpu
                    .registers
                    .write_a(machine.read_u8(machine.cpu.registers.hl));
                machine.cpu.registers.hl += 1;
                (8, 2)
            }

            Instruction::LD_FFu8_A(u8) => {
                machine.write_u8(
                    Wrapping(0xFF00 + (*u8).0 as u16),
                    machine.cpu.registers.read_a(),
                );
                (12, 3)
            }

            Instruction::LD_HL_SP_i8(i8) => {
                let sp = machine.cpu.registers.sp;
                let res = Wrapping(sp.0.wrapping_add_signed(i8.0 as i16));
                machine.cpu.registers.hl = res;
                machine.cpu.registers.znhc(
                    false,
                    false,
                    add_produces_carry(sp.0, i8.0, false, 4),
                    add_produces_carry(sp.0, i8.0, false, 8),
                );
                (12, 3)
            }

            Instruction::LD_mu16_A(imm16) => {
                machine.write_u8(imm16.as_u16(), machine.cpu.registers.read_a());
                (16, 4)
            }

            Instruction::LD_mu16_SP(imm16) => {
                let sp = Immediate16::from_u16(machine.cpu.registers.sp);
                let address = imm16.as_u16();
                machine.write_u8(address, sp.lower_byte);
                machine.write_u8(address + Wrapping(1), sp.higher_byte);
                (20, 5)
            }

            Instruction::LD_H_mHL => todo!(),

            Instruction::LD_L_mHL => todo!(),

            Instruction::LD_FFC_A => {
                machine.write_u8(
                    Wrapping(0xFF00) + Wrapping(machine.cpu.registers.read_c().0 as u16),
                    machine.cpu.registers.read_a(),
                );
                (8, 2)
            }

            Instruction::LD_r8_r8(r8a, r8b) => {
                machine
                    .cpu
                    .registers
                    .write_r8(r8a, machine.cpu.registers.read_r8(r8b));
                (4, 1)
            }

            Instruction::LD_r16_d16(r16, imm16) => {
                machine.cpu.registers.write_r16(r16, imm16.as_u16());
                (12, 3)
            }

            Instruction::LD_mr16_r8(mr16, r8) => {
                machine.write_u8(
                    machine.cpu.registers.read_r16(mr16),
                    machine.cpu.registers.read_r8(r8),
                );
                (8, 2)
            }

            Instruction::LD_mHL_u8(u8) => {
                machine.write_u8(machine.cpu.registers.hl, *u8);
                (12, 3)
            }

            Instruction::LD_mHLdec_A => {
                machine.write_u8(machine.cpu.registers.hl, machine.cpu.registers.read_a());
                machine.cpu.registers.hl -= 1;
                (8, 2)
            }

            Instruction::LD_mHLinc_A => {
                machine.write_u8(machine.cpu.registers.hl, machine.cpu.registers.read_a());
                machine.cpu.registers.hl += 1;
                (8, 2)
            }

            Instruction::LD_A_FFC => {
                let c = machine.cpu.registers.read_c();
                let a = machine.read_u8(Wrapping(0xFF00) + Wrapping(c.0 as u16));
                machine.cpu.registers.write_a(a);
                (8, 2)
            }

            Instruction::LD_A_FFu8(u8) => {
                let a = machine.read_u8(Wrapping(0xFF00) + Wrapping((*u8).0 as u16));
                machine.cpu.registers.write_a(a);
                (12, 3)
            }

            Instruction::LD_A_mu16(imm16) => {
                machine
                    .cpu
                    .registers
                    .write_a(machine.read_u8(imm16.as_u16()));
                (16, 4)
            }

            Instruction::LD_r8_u8(r8, u8) => {
                machine.cpu.registers.write_r8(r8, *u8);
                (8, 2)
            }

            Instruction::LD_r8_mr16(r8, r16) => {
                machine
                    .cpu
                    .registers
                    .write_r8(r8, machine.read_u8(machine.cpu.registers.read_r16(r16)));
                (8, 2)
            }

            Instruction::LD_SP_HL => {
                machine.cpu.registers.sp = machine.cpu.registers.hl;
                (8, 2)
            }

            Instruction::LD_SP_u16(imm16) => {
                machine.cpu.registers.sp = imm16.as_u16();
                (12, 3)
            }

            Instruction::NOP => (4, 1),

            Instruction::OR_A_mHL => {
                let a = machine.cpu.registers.read_a();
                let b = machine.read_u8(machine.cpu.registers.hl);
                or(&mut machine.cpu, &a, &b);
                (8, 2)
            }

            Instruction::OR_A_r8(r8) => {
                let a = machine.cpu.registers.read_a();
                let b = machine.cpu.registers.read_r8(r8);
                or(&mut machine.cpu, &a, &b);
                (4, 1)
            }

            Instruction::OR_A_u8(u8) => {
                let a = machine.cpu.registers.read_a();
                or(&mut machine.cpu, &a, u8);
                (8, 2)
            }

            Instruction::POP_r16(r16) => {
                CPU::pop_r16(machine, r16);
                // Only the flag bits of F are restored
                if *r16 == R16::AF {
                    let masked_af = machine.cpu.registers.read_r16(r16) & Wrapping(0xFFF0);
                    machine.cpu.registers.write_r16(r16, masked_af);
                }
                (12, 3)
            }

            Instruction::PUSH_r16(r16) => {
                let mut byte_to_push = machine.cpu.registers.read_r16(r16);
                // Only the flag bits of F are pushed
                if *r16 == R16::AF {
                    byte_to_push = byte_to_push & Wrapping(0xFFF0);
                }
                CPU::push_imm16(machine, Immediate16::from_u16(byte_to_push));
                (16, 4)
            }

            Instruction::RES_u3_mHL(u8) => {
                let address = machine.cpu.registers.hl;
                let a = machine.read_u8(address);
                let res = bit_reset(&a, u8);
                machine.write_u8(address, res);
                (16, 4)
            }

            Instruction::RES_u3_r8(u8, r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = bit_reset(&a, u8);
                machine.cpu.registers.write_r8(r8, res);
                (8, 2)
            }

            Instruction::RET => {
                CPU::pop_r16(machine, &R16::PC);
                (16, 4)
            }

            Instruction::RET_cc(cc) => {
                if cc.holds(&machine.cpu) {
                    CPU::pop_r16(machine, &R16::PC);
                    (20, 5)
                } else {
                    (8, 2)
                }
            }

            Instruction::RETI => {
                machine.cpu.interrupts.interrupt_master_enable = true;
                CPU::pop_r16(machine, &R16::PC);
                (16, 4)
            }

            Instruction::RL_mHL => {
                let address = machine.cpu.registers.hl;
                let a = machine.read_u8(address);
                let res = rotate_left_through_carry(&mut machine.cpu, &a);
                machine.write_u8(address, res);
                (16, 4)
            }

            Instruction::RL_r8(r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = rotate_left_through_carry(&mut machine.cpu, &a);
                machine.cpu.registers.write_r8(r8, res);
                (8, 2)
            }

            Instruction::RLA => {
                let a = machine.cpu.registers.read_a();
                let res = rotate_left_through_carry(&mut machine.cpu, &a);
                machine.cpu.registers.write_a(res);
                // For some reason, this unsets Z
                machine.cpu.registers.unset_flag(Flag::Z);
                (4, 1)
            }

            Instruction::RLCA => {
                let a = machine.cpu.registers.read_a();
                let res = rotate_left(&mut machine.cpu, &a);
                machine.cpu.registers.write_a(res);
                // For some reason, this unsets Z
                machine.cpu.registers.unset_flag(Flag::Z);
                (4, 1)
            }

            Instruction::RLC_mHL => {
                let address = machine.cpu.registers.hl;
                let a = machine.read_u8(address);
                let res = rotate_left(&mut machine.cpu, &a);
                machine.write_u8(address, res);
                (16, 4)
            }

            Instruction::RLC_r8(r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = rotate_left(&mut machine.cpu, &a);
                machine.cpu.registers.write_r8(r8, res);
                (8, 2)
            }

            Instruction::RRA => {
                let a = machine.cpu.registers.read_a();
                let res = rotate_right_through_carry(&mut machine.cpu, &a);
                machine.cpu.registers.write_a(res);
                // For some reason, this unsets Z
                machine.cpu.registers.unset_flag(Flag::Z);
                (4, 1)
            }

            Instruction::RR_mHL => {
                let address = machine.cpu.registers.hl;
                let a = machine.read_u8(address);
                let res = rotate_right_through_carry(&mut machine.cpu, &a);
                machine.write_u8(address, res);
                (16, 4)
            }

            Instruction::RR_r8(r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = rotate_right_through_carry(&mut machine.cpu, &a);
                machine.cpu.registers.write_r8(r8, res);
                (8, 2)
            }

            Instruction::RRCA => {
                let a = machine.cpu.registers.read_a();
                let res = rotate_right(&mut machine.cpu, &a);
                machine.cpu.registers.write_a(res);
                // For some reason, this unsets Z
                machine.cpu.registers.unset_flag(Flag::Z);
                (4, 1)
            }

            Instruction::RRC_mHL => {
                let address = machine.cpu.registers.hl;
                let a = machine.read_u8(address);
                let res = rotate_right(&mut machine.cpu, &a);
                machine.write_u8(address, res);
                (16, 4)
            }

            Instruction::RRC_r8(r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = rotate_right(&mut machine.cpu, &a);
                machine.cpu.registers.write_r8(r8, res);
                (8, 2)
            }

            Instruction::RST(imm16) => {
                CPU::push_imm16(machine, Immediate16::from_u16(machine.cpu.registers.pc));
                machine.cpu.registers.pc = imm16.as_u16();
                (16, 4)
            }

            Instruction::SBC_A_mHL => {
                let a = machine.cpu.registers.read_a();
                let b = machine.read_u8(machine.cpu.registers.hl);
                let c = machine.cpu.registers.read_flag(Flag::C);
                subc(&mut machine.cpu, &a, &b, c);
                (8, 2)
            }

            Instruction::SBC_A_r8(r8) => {
                let a = machine.cpu.registers.read_a();
                let b = machine.cpu.registers.read_r8(r8);
                let c = machine.cpu.registers.read_flag(Flag::C);
                subc(&mut machine.cpu, &a, &b, c);
                (4, 1)
            }

            Instruction::SBC_A_u8(u8) => {
                let a = machine.cpu.registers.read_a();
                let c = machine.cpu.registers.read_flag(Flag::C);
                subc(&mut machine.cpu, &a, u8, c);
                (8, 2)
            }

            Instruction::SCF => {
                machine
                    .cpu
                    .registers
                    .unset_flag(Flag::N)
                    .unset_flag(Flag::H)
                    .set_flag(Flag::C);
                (4, 1)
            }

            Instruction::SET_u3_mHL(u8) => {
                let address = machine.cpu.registers.hl;
                let a = machine.read_u8(address);
                let res = bit_set(&a, u8);
                machine.write_u8(address, res);
                (16, 4)
            }

            Instruction::SET_u3_r8(u8, r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = bit_set(&a, u8);
                machine.cpu.registers.write_r8(r8, res);
                (8, 2)
            }

            Instruction::SLA_mHL => {
                let address = machine.cpu.registers.hl;
                let a = machine.read_u8(address);
                let res = rotate_left_with(&mut machine.cpu, &a, false);
                machine.write_u8(address, res);
                (16, 4)
            }

            Instruction::SLA_r8(r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = rotate_left_with(&mut machine.cpu, &a, false);
                machine.cpu.registers.write_r8(r8, res);
                (8, 2)
            }

            Instruction::SRA_mHL => {
                let address = machine.cpu.registers.hl;
                let a = machine.read_u8(address);
                let res = shift_right_arithmetically(&mut machine.cpu, &a);
                machine.write_u8(address, res);
                (16, 4)
            }

            Instruction::SRA_r8(r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = shift_right_arithmetically(&mut machine.cpu, &a);
                machine.cpu.registers.write_r8(r8, res);
                (8, 2)
            }

            Instruction::SRL_mHL => {
                let address = machine.cpu.registers.hl;
                let a = machine.read_u8(address);
                let res = shift_right_logically(&mut machine.cpu, &a);
                machine.write_u8(address, res);
                (16, 4)
            }

            Instruction::SRL_r8(r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = shift_right_logically(&mut machine.cpu, &a);
                machine.cpu.registers.write_r8(r8, res);
                (8, 2)
            }

            Instruction::STOP => {
                // TODO
                (4, 1)
            }

            Instruction::SUB_A_mHL => {
                let a = machine.cpu.registers.read_a();
                let b = machine.read_u8(machine.cpu.registers.hl);
                sub(&mut machine.cpu, &a, &b);
                (8, 2)
            }

            Instruction::SUB_A_r8(r8) => {
                let a = machine.cpu.registers.read_a();
                let b = machine.cpu.registers.read_r8(r8);
                sub(&mut machine.cpu, &a, &b);
                (4, 1)
            }

            Instruction::SUB_A_u8(u8) => {
                let a = machine.cpu.registers.read_a();
                sub(&mut machine.cpu, &a, u8);
                (8, 2)
            }

            Instruction::SWAP_mHL => {
                let address = machine.cpu.registers.hl;
                let a = machine.read_u8(address);
                let res = swap(&mut machine.cpu, &a);
                machine.write_u8(address, res);
                (16, 4)
            }

            Instruction::SWAP_r8(r8) => {
                let a = machine.cpu.registers.read_r8(r8);
                let res = swap(&mut machine.cpu, &a);
                machine.cpu.registers.write_r8(r8, res);
                (8, 2)
            }

            Instruction::XOR_A_r8(r8) => {
                let a = machine.cpu.registers.read_a();
                let b = machine.cpu.registers.read_r8(r8);
                xor(&mut machine.cpu, &a, &b);
                (4, 1)
            }

            Instruction::XOR_A_u8(u8) => {
                let a = machine.cpu.registers.read_a();
                xor(&mut machine.cpu, &a, u8);
                (8, 2)
            }

            Instruction::XOR_A_mHL => {
                let a = machine.cpu.registers.read_a();
                let b = machine.read_u8(machine.cpu.registers.hl);
                xor(&mut machine.cpu, &a, &b);
                (8, 2)
            }
        }
    }
}

pub fn rotate_left_with(cpu: &mut CPU, value: &Wrapping<u8>, new_bit: bool) -> Wrapping<u8> {
    let carry = value.0 >> 7;
    let res = Wrapping((value.0 << 1) | (new_bit as u8));
    cpu.registers.znhc(res.0 == 0, false, false, carry == 1);
    res
}

pub fn rotate_left_through_carry(cpu: &mut CPU, value: &Wrapping<u8>) -> Wrapping<u8> {
    let new_bit = cpu.registers.read_flag(Flag::C);
    rotate_left_with(cpu, value, new_bit)
}

pub fn rotate_left(cpu: &mut CPU, value: &Wrapping<u8>) -> Wrapping<u8> {
    let new_bit = (value.0 >> 7) == 1;
    rotate_left_with(cpu, value, new_bit)
}

pub fn rotate_right_with(cpu: &mut CPU, value: &Wrapping<u8>, new_bit: bool) -> Wrapping<u8> {
    let carry = value.0 & 1;
    let res = Wrapping((value.0 >> 1) | ((new_bit as u8) << 7));
    cpu.registers.znhc(res.0 == 0, false, false, carry == 1);
    res
}

pub fn rotate_right_through_carry(cpu: &mut CPU, value: &Wrapping<u8>) -> Wrapping<u8> {
    let new_bit = cpu.registers.read_flag(Flag::C);
    rotate_right_with(cpu, value, new_bit)
}

pub fn rotate_right(cpu: &mut CPU, value: &Wrapping<u8>) -> Wrapping<u8> {
    let new_bit = (value.0 & 1) == 1;
    rotate_right_with(cpu, value, new_bit)
}

pub fn shift_right_arithmetically(cpu: &mut CPU, value: &Wrapping<u8>) -> Wrapping<u8> {
    let carry = value.0 & 1;
    let bit7 = value & Wrapping(0x80);
    let res = (value >> 1) | bit7;
    cpu.registers.znhc(res.0 == 0, false, false, carry == 1);
    res
}

pub fn shift_right_logically(cpu: &mut CPU, value: &Wrapping<u8>) -> Wrapping<u8> {
    let carry = value.0 & 1;
    let res = value >> 1;
    cpu.registers.znhc(res.0 == 0, false, false, carry == 1);
    res
}

pub fn swap(cpu: &mut CPU, value: &Wrapping<u8>) -> Wrapping<u8> {
    let new_low = value >> 4;
    let new_high = (value & Wrapping(0x0F)) << 4;
    let res = new_high | new_low;
    cpu.registers.znhc(res.0 == 0, false, false, false);
    res
}

pub fn bit_complement(cpu: &mut CPU, value: bool) {
    cpu.registers
        .write_flag(Flag::Z, !value)
        .unset_flag(Flag::N)
        .set_flag(Flag::H);
}

pub fn bit_reset(value: &Wrapping<u8>, bit_position: &u8) -> Wrapping<u8> {
    Wrapping(value.0 & !(1 << bit_position))
}

pub fn bit_set(value: &Wrapping<u8>, bit_position: &u8) -> Wrapping<u8> {
    Wrapping(value.0 | (1 << bit_position))
}
