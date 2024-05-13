use bevy::{
    app::{App, Startup, Update},
    math::UVec2,
    DefaultPlugins,
};
use bevy_pixel_buffer::{
    builder::pixel_buffer_setup,
    frame::GetFrame,
    pixel::Pixel,
    pixel_buffer::{PixelBufferPlugin, PixelBufferSize},
    query::QueryPixelBuffer,
};

fn main() {
    let size = PixelBufferSize {
        size: UVec2::new(160, 144),
        pixel_size: UVec2::new(4, 4),
    };

    App::new()
        .add_plugins((DefaultPlugins, PixelBufferPlugin))
        .add_systems(Startup, pixel_buffer_setup(size))
        .add_systems(Update, update)
        .run()
}

fn update(mut pb: QueryPixelBuffer) {
    pb.frame().per_pixel(|_, _| Pixel::random())
}

// use std::fmt;

// use opcodes::{decode_next_instruction, DecodedInstruction};

// pub mod opcodes;

// impl fmt::Display for DecodedInstruction {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "0x{:02X} {:?}", self.address, self.instruction)
//     }
// }

// fn main() {
//     let boot_ROM_bytes = std::fs::read("dmg_boot.bin").unwrap();
//     let mut pc: usize = 0;
//     while (pc < boot_ROM_bytes.len()) {
//         let decoded = decode_next_instruction(pc as u16, &boot_ROM_bytes[pc..]);
//         pc += decoded.instruction_size as usize;
//         println!("{}", decoded)
//     }
// }
