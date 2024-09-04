pub mod application_state;
pub mod command_line_arguments;
pub mod conditions;
pub mod cpu;
pub mod inputs;
pub mod instructions;
pub mod machine;
pub mod memory;
pub mod message;
pub mod ppu;
pub mod registers;
pub mod utils;
pub mod view;

use application_state::ApplicationState;
use clap::Parser;
use command_line_arguments::CommandLineArguments;
use iced::{self, advanced::graphics::core::font, Settings, Size, Task};
use message::Message;

const BREAKPOINTS: &[u16] = &[
    // 0x00F1, // passed logo check
    // 0x00FC, // passed header checksum check
    // 0x0100, // made it out of the boot ROM
    // 0x026C,
    // 0x0272,
    // 0xC355,
    // 0xC738,
    // 0xC662,
    // 0xDEF8,
];

fn main() -> Result<(), iced::Error> {
    let args = CommandLineArguments::parse();

    let mut settings = Settings::default();
    settings.default_font = font::Font::MONOSPACE;
    iced::application("YokoiBoy", ApplicationState::update, ApplicationState::view)
        .subscription(ApplicationState::subscription)
        .settings(settings)
        .window_size(Size::new(1600.0, 1100.0))
        .run_with(move || {
            (
                ApplicationState::new(&args, BREAKPOINTS),
                Task::done(Message::BeginRunUntilBreakpoint),
            )
        })
}
