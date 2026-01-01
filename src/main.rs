pub mod application_state;
pub mod command_line_arguments;
pub mod conditions;
pub mod cpu;
pub mod inputs;
pub mod instructions;
pub mod machine;
pub mod memory;
pub mod message;
pub mod pixel_fetcher;
pub mod ppu;
pub mod registers;
pub mod utils;
pub mod view;

use application_state::ApplicationState;
use clap::Parser;
use command_line_arguments::CommandLineArguments;
use iced::{
    self,
    advanced::graphics::core::font,
    window::{self, settings::PlatformSpecific},
    Point, Settings, Size, Task,
};
use message::Message;
use tracing_subscriber::{fmt, EnvFilter};

use crate::message::StepBeforeCheckingBreakpoint;

const BREAKPOINTS: &[u16] = &[
    // 0x0000,
    // 0x00F1, // passed logo check
    // 0x00FC, // passed header checksum check
    // 0x0100, // made it out of the boot ROM
    // 0x026C,
    // 0x0272,
    // 0xC355,
    // 0xC738,
    // 0xC662,
    // 0xDEF8,

    // Debugging dmg-acid2
    // 0x02D2, // scx ← 0
    // 0x02B3, // scx ← 243
];

fn main() -> Result<(), iced::Error> {
    tracing_subscriber::fmt()
        .event_format(fmt::format().without_time().compact())
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let args = CommandLineArguments::parse();
    let mut settings = Settings::default();
    settings.default_font = font::Font::MONOSPACE;
    iced::application(
        move || {
            return (
                ApplicationState::new(&args, BREAKPOINTS),
                Task::done(Message::BeginRunUntilBreakpoint(
                    StepBeforeCheckingBreakpoint::No,
                )),
            );
        },
        ApplicationState::update,
        ApplicationState::view,
    )
    .subscription(ApplicationState::subscription)
    .settings(settings)
    .window(window::Settings {
        blur: false,
        closeable: true,
        decorations: false,
        exit_on_close_request: true,
        fullscreen: false,
        icon: None,
        level: window::Level::Normal,
        max_size: None,
        maximized: false,
        min_size: None,
        minimizable: true,
        platform_specific: PlatformSpecific::default(),
        position: window::Position::SpecificWith(|window_size, monitor_size| Point {
            x: monitor_size.width - window_size.width,
            y: 0.0,
        }),
        resizable: true,
        size: Size::new(1600.0, 1200.0),
        transparent: false,
        visible: true,
    })
    .run()
}
