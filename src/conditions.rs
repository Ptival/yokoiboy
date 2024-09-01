use core::fmt;

use crate::{cpu::CPU, registers::Flag};

#[derive(Clone, Debug, Hash)]
pub enum Condition {
    C,
    Z,
    NC,
    NZ,
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Condition {
    pub fn holds(&self, cpu: &CPU) -> bool {
        match self {
            Condition::C => cpu.registers.read_flag(Flag::C),
            Condition::Z => cpu.registers.read_flag(Flag::Z),
            Condition::NC => !cpu.registers.read_flag(Flag::C),
            Condition::NZ => !cpu.registers.read_flag(Flag::Z),
        }
    }
}
