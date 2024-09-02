use std::num::Wrapping;

#[derive(Clone, Debug)]
pub struct Inputs {
    pub inputs_register: Wrapping<u8>,
}

impl Inputs {
    pub fn new() -> Self {
        Inputs {
            inputs_register: Wrapping(0),
        }
    }

    pub fn read(&self) -> Wrapping<u8> {
        self.inputs_register
    }

    pub fn write(&mut self, value: Wrapping<u8>) {
        // Lower nibble is read-only
        self.inputs_register = Wrapping((value.0 & 0xF0) | (self.inputs_register.0 & 0x0F));
    }
}
