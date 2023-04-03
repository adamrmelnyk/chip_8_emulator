mod chip8;
mod color;

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Command {
    #[structopt(
        about = "Loads and runs a program",
        help = "USAGE: load myChip8Binary.chip8 <optional-color>"
    )]
    Load {
        filename: String,
        color: Option<color::Color>,
    },
    #[structopt(
        about = "Loads and runs a program in debug mode.
        Waits on a key to be pressed before proceeding
        at each operation.
        ENTER -> Proceeds to next instruction
        ESC -> Exits the emulator
        DELETE -> Resumes normal execution",
        help = "USAGE: debug myChip8Binary.chip8"
    )]
    Debug { filename: String },
}

fn load(filename: String, color: color::Color) {
    let mut chip8 = chip8::CHIP8::new();
    chip8.color = color;
    chip8.load_and_run(&filename);
}

fn debug(filename: String) {
    let mut chip8 = chip8::CHIP8::new();
    chip8.debug = true;
    chip8.load_and_run(&filename);
}

fn main() {
    let args = Command::from_args();
    match args {
        Command::Load { filename, color } => match color {
            Some(color) => load(filename, color),
            None => load(filename, color::Color::Purple),
        },
        Command::Debug { filename } => debug(filename),
    }
}
