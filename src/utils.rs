use std::num::Wrapping;

pub fn is_bit_set(value: &Wrapping<u8>, bit_position: u8) -> bool {
    (value.0 & (1 << bit_position)) != 0
}

pub fn write_bit(value: &Wrapping<u8>, bit_position: u8, bit_value: bool) -> Wrapping<u8> {
    if bit_value {
        set_bit(value, bit_position)
    } else {
        unset_bit(value, bit_position)
    }
}

pub fn set_bit(value: &Wrapping<u8>, bit_position: u8) -> Wrapping<u8> {
    Wrapping(value.0 | (1 << bit_position))
}

pub fn unset_bit(value: &Wrapping<u8>, bit_position: u8) -> Wrapping<u8> {
    Wrapping(value.0 & !(1 << bit_position))
}

pub fn set_bit_mut(value: &mut Wrapping<u8>, bit_position: u8) {
    *value = set_bit(value, bit_position)
}

pub fn unset_bit_mut(value: &mut Wrapping<u8>, bit_position: u8) {
    *value = unset_bit(value, bit_position)
}
