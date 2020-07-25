mod chip8;

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Command {
    #[structopt(
        about = "Loads and runs a program",
        help = "USAGE: load myChip8Binary.chip8"
    )]
    Load { filename: String },
}

fn load(filename: String) {
    let mut chip8 = chip8::CHIP8::new();
    chip8.load_and_run(&filename);
}

fn main() {
    let args = Command::from_args();
    match args {
        Command::Load { filename } => load(filename),
    }
}
