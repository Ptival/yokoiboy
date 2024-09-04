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
        let registers = cpu.registers();
        match self {
            Condition::C => registers.read_flag(Flag::C),
            Condition::Z => registers.read_flag(Flag::Z),
            Condition::NC => !registers.read_flag(Flag::C),
            Condition::NZ => !registers.read_flag(Flag::Z),
        }
    }
}
