# CHIP8 Emulator

A simple CHIP8 emulator written in rust. This project is currently incomplete. Though it will run a chip8 program it's missing reading keyboard instructions at each iteration as well as the display.

## Building

Make sure you have the latest version of Rust.

```sh
cargo build
```

## Running

This emulator reads binary files written with chip8 operations. A program might look something like this:

```sh
hexdump testbin/xy_neq_neq.chip8
0000000 0560 0661 1090 0170
0000008
```

The testbin folder has several examples of chip8 programs for each instruction.

To run a program, use the load operation followed by the file you wish to load:

```sh
chip_8_emulator load myChip8Prog.chip8
```

### Keyboard

The CHIP8 keyboard:

| 1 | 2 | 3 | c |
|---|---|---|---|
| 4 | 5 | 6 | d |
| 7 | 8 | 9 | e |
| a | 0 | b | f |

Is mapped to:

| 1 | 2 | 3 | 4 |
|---|---|---|---|
| q | w | e | r |
| a | s | d | f |
| z | x | c | v |

## Testing

```sh
cargo test
```

## TODO

* Draw on the display
* Keyboard input on each loop
* More tests
* Rewind
* Window size CLI param
