use clap::Parser;

#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct CommandLineArguments {
    #[arg(short, long)]
    pub boot_rom: String,
    #[arg(short, long)]
    pub game_rom: String,
    #[arg(short, long, default_value_t = false)]
    pub log_for_doctor: bool,
}
