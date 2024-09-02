use core::fmt;
use std::num::Wrapping;

#[derive(Clone, Debug, Hash)]
pub enum R8 {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
}

impl fmt::Display for R8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug, Hash)]
pub enum R16 {
    AF,
    BC,
    DE,
    HL,
    SP,
    PC,
}

impl fmt::Display for R16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug, Hash)]
pub enum Flag {
    Z,
    N,
    C,
    H,
}

impl Flag {
    pub fn get_bit(&self) -> u8 {
        match self {
            Flag::Z => 7,
            Flag::N => 6,
            Flag::H => 5,
            Flag::C => 4,
        }
    }
}

#[derive(Clone, Debug, Hash)]
pub struct Registers {
    pub af: Wrapping<u16>,
    pub bc: Wrapping<u16>,
    pub de: Wrapping<u16>,
    pub hl: Wrapping<u16>,
    pub sp: Wrapping<u16>,
    pub pc: Wrapping<u16>,
}

pub fn u16_from_u8s(higher: Wrapping<u8>, lower: Wrapping<u8>) -> Wrapping<u16> {
    Wrapping((higher.0 as u16) << 8 | lower.0 as u16)
}

pub fn higher_u8(from: u16) -> u8 {
    (from >> 8) as u8
}

pub fn lower_u8(from: u16) -> u8 {
    from as u8
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            af: Wrapping(0),
            bc: Wrapping(0),
            de: Wrapping(0),
            hl: Wrapping(0),
            sp: Wrapping(0),
            pc: Wrapping(0),
        }
    }

    pub fn write_a(&mut self, a: Wrapping<u8>) -> &mut Self {
        self.af = u16_from_u8s(a, self.read_f());
        self
    }

    fn write_f(&mut self, f: Wrapping<u8>) -> &mut Self {
        self.af = u16_from_u8s(self.read_a(), f);
        self
    }

    pub fn write_b(&mut self, b: Wrapping<u8>) -> &mut Self {
        self.bc = u16_from_u8s(b, self.read_c());
        self
    }

    pub fn write_c(&mut self, c: Wrapping<u8>) -> &mut Self {
        self.bc = u16_from_u8s(self.read_b(), c);
        self
    }

    pub fn write_d(&mut self, d: Wrapping<u8>) -> &mut Self {
        self.de = u16_from_u8s(d, self.read_e());
        self
    }

    pub fn write_e(&mut self, e: Wrapping<u8>) -> &mut Self {
        self.de = u16_from_u8s(self.read_d(), e);
        self
    }

    pub fn write_h(&mut self, h: Wrapping<u8>) -> &mut Self {
        self.hl = u16_from_u8s(h, self.read_l());
        self
    }

    pub fn write_l(&mut self, l: Wrapping<u8>) -> &mut Self {
        self.hl = u16_from_u8s(self.read_h(), l);
        self
    }

    pub fn read_a(&self) -> Wrapping<u8> {
        Wrapping(higher_u8(self.af.0))
    }

    pub fn read_f(&self) -> Wrapping<u8> {
        Wrapping(lower_u8(self.af.0))
    }

    pub fn read_b(&self) -> Wrapping<u8> {
        Wrapping(higher_u8(self.bc.0))
    }

    pub fn read_c(&self) -> Wrapping<u8> {
        Wrapping(lower_u8(self.bc.0))
    }

    pub fn read_d(&self) -> Wrapping<u8> {
        Wrapping(higher_u8(self.de.0))
    }

    pub fn read_e(&self) -> Wrapping<u8> {
        Wrapping(lower_u8(self.de.0))
    }

    pub fn read_h(&self) -> Wrapping<u8> {
        Wrapping(higher_u8(self.hl.0))
    }

    pub fn read_l(&self) -> Wrapping<u8> {
        Wrapping(lower_u8(self.hl.0))
    }

    pub fn read_r8(&self, r8: &R8) -> Wrapping<u8> {
        match r8 {
            R8::A => self.read_a(),
            R8::B => self.read_b(),
            R8::C => self.read_c(),
            R8::D => self.read_d(),
            R8::E => self.read_e(),
            R8::F => self.read_f(),
            R8::H => self.read_h(),
            R8::L => self.read_l(),
        }
    }

    pub fn write_r8(&mut self, r8: &R8, value: Wrapping<u8>) -> &mut Self {
        match r8 {
            R8::A => self.write_a(value),
            R8::B => self.write_b(value),
            R8::C => self.write_c(value),
            R8::D => self.write_d(value),
            R8::E => self.write_e(value),
            R8::F => self.write_f(value),
            R8::H => self.write_h(value),
            R8::L => self.write_l(value),
        }
    }

    pub fn read_r16(&self, r16: &R16) -> Wrapping<u16> {
        match r16 {
            R16::AF => self.af,
            R16::BC => self.bc,
            R16::DE => self.de,
            R16::HL => self.hl,
            R16::SP => self.sp,
            R16::PC => self.pc,
        }
    }

    pub fn write_r16(&mut self, r16: &R16, value: Wrapping<u16>) -> &mut Self {
        match r16 {
            R16::AF => self.af = value,
            R16::BC => self.bc = value,
            R16::DE => self.de = value,
            R16::HL => self.hl = value,
            R16::SP => self.sp = value,
            R16::PC => self.pc = value,
        };
        self
    }

    pub fn get_bit(&self, r8: &R8, bit: &u8) -> bool {
        (self.read_r8(r8).0 & (1 << bit)) != 0
    }

    pub fn read_flag(&self, flag: Flag) -> bool {
        self.read_f().0 & (1 << flag.get_bit()) != 0
    }

    pub fn set_flag(&mut self, flag: Flag) -> &mut Self {
        self.write_flag(flag, true)
    }

    pub fn unset_flag(&mut self, flag: Flag) -> &mut Self {
        self.write_flag(flag, false)
    }

    pub fn write_flag(&mut self, flag: Flag, value: bool) -> &mut Self {
        if value {
            self.write_f(Wrapping(self.read_f().0 | (1 << flag.get_bit())))
        } else {
            self.write_f(Wrapping(self.read_f().0 & !(1 << flag.get_bit())))
        }
    }

    pub fn znhc(&mut self, z: bool, n: bool, h: bool, c: bool) -> &mut Self {
        let clean_f = self.read_f().0 & 0x0F;
        let new_f =
            clean_f | ((z as u8) << 7) | ((n as u8) << 6) | ((h as u8) << 5) | ((c as u8) << 4);
        self.write_f(Wrapping(new_f));
        self
    }
}
