use crate::color::Color;
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::fs::File;
use std::io::Read;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;
const OFF: u32 = 0x000000; // Black
const VF: usize = 0x0f;

pub struct CHIP8 {
    registers: [u8; 16],
    i: u16,
    position_in_memory: usize,
    memory: [u8; 4096],
    stack: [u16; 16],
    stack_pointer: usize,
    keys: [bool; 16],
    delay_timer: u8,
    sound_timer: u8,
    display: [[bool; WIDTH]; HEIGHT],
    window: Window,
    draw_flag: bool,
    pub debug: bool,
    pub color: Color,
}

impl CHIP8 {
    pub fn new() -> CHIP8 {
        CHIP8 {
            registers: [0; 16],
            i: 0,
            memory: [0; 4096],
            position_in_memory: 0x200, // We start reading at 0x200 on the COSMAC VIP though, other variants started at other memory locations apparently
            stack: [0; 16],
            stack_pointer: 0,
            keys: [false; 16],
            delay_timer: 0,
            sound_timer: 0,
            display: [[false; 64]; 32],
            window: Window::new(
                "CHIP8",
                WIDTH,
                HEIGHT,
                WindowOptions {
                    scale: Scale::X32, // Change this value to X16, X8 to make the pixels and window smaller
                    ..WindowOptions::default()
                },
            )
            .unwrap_or_else(|e| {
                panic!("Error creating window: {}", e);
            }),
            draw_flag: false,
            debug: false,
            color: Color::Purple,
        }
    }

    /// The main run loop: Executes instructions, draws if the draw flag is set, and sets the keys on each loop
    pub fn run(&mut self) {
        loop {
            if self.debug {
                self.wait_on_debug_input();
            }
            if self.emulate_cycle() {
                break;
            }
            if self.draw_flag {
                self.draw_graphics();
            }
            self.set_keys();
        }
    }

    /// Loop until a valid key is pressed
    fn wait_on_debug_input(&mut self) {
        let mut key_pressed = false;
        while !key_pressed {
            self.window.update();
            self.window.get_keys_pressed(KeyRepeat::No).iter().for_each(|key|
                match key {
                    Key::Enter => { key_pressed = true },
                    Key::Escape => { std::process::exit(0) },
                    Key::Delete => {
                        key_pressed = true;
                        self.debug = false;
                    },
                    _ => {}
                }
            );
        }
    }

    /// Loads an operation from memory and executes the operation
    /// returns true when it loads a 0x0000 or exit operation
    fn emulate_cycle(&mut self) -> bool {
        let op_byte1 = self.memory[self.position_in_memory] as u16;
        let op_byte2 = self.memory[self.position_in_memory + 1] as u16;
        let opcode = op_byte1 << 8 | op_byte2;

        let x = ((opcode & 0x0F00) >> 8) as u8;
        let y = ((opcode & 0x00F0) >> 4) as u8;
        let nn = (opcode & 0x00FF) as u8;
        let n = (opcode & 0x000F) as u8;
        let nnn = opcode & 0x0FFF;

        self.position_in_memory += 2;

        let mut execution_finished = false;
        match opcode {
            0x0000 => execution_finished = true,
            0x00E0 => self.clear_screen(),
            0x00EE => self.ret(),
            0x1000..=0x1FFF => self.goto(nnn),
            0x2000..=0x2FFF => self.call(nnn),
            0x3000..=0x3FFF => self.skip_if_equal(x, nn),
            0x4000..=0x4FFF => self.skip_if_not_equal(x, nn),
            0x5000..=0x5FF0 => self.skip_xy_equal(x, y),
            0x6000..=0x6FFF => self.set_xnn(x, nn),
            0x7000..=0x7FFF => self.add_xnn(x, nn),
            0x8000..=0x8FFF => match n {
                0 => self.assign_xy(x, y),
                1 => self.or_xy(x, y),
                2 => self.and_xy(x, y),
                3 => self.xor_xy(x, y),
                4 => self.add_xy(x, y),
                5 => self.sub_xy(x, y),
                6 => self.shift_right(x),
                7 => self.sub_yx(x, y),
                14 => self.shift_left(x),
                _ => unimplemented!("opcode {:04x}", opcode),
            },
            0x9000..=0x9FF0 => self.skip_xy_not_equal(x, y),
            0xA000..=0xAFFF => self.set_16bit_register(nnn),
            0xB000..=0xBFFF => self.jump_nnn_plus_v0(nnn),
            0xC000..=0xCFFF => self.rand(x, nn),
            0xD000..=0xDFFF => self.draw(x, y, n),
            0xE000..=0xEFFF => match nn {
                0x9E => self.skip_if_key_pressed(x),
                0xA1 => self.skip_if_key_not_pressed(x),
                _ => unimplemented!("opcode {:04x}", opcode),
            },
            0xF000..=0xFFFF => match nn {
                0x07 => self.set_x_to_delay_timer(x),
                0x0A => self.set_x_to_keypress(x),
                0x15 => self.set_delay_timer_to_x(x),
                0x18 => self.set_sound_timer_to_x(x),
                0x1E => self.add_ix(x),
                0x29 => self.set_i_sprite_addr_x(x),
                0x33 => self.set_bcd(x),
                0x55 => self.reg_dump(x),
                0x65 => self.reg_load(x),
                _ => unimplemented!("opcode {:04x}", opcode),
            },
            _ => unimplemented!("opcode: {:04x}", opcode),
        }
        execution_finished
    }

    /// Update the window
    fn draw_graphics(&mut self) {
        let mut buf = Vec::new();
        for i in 0..self.display.len() {
            for j in 0..self.display[0].len() {
                if self.display[i][j] {
                    buf.push(self.color.hex_color())
                } else {
                    buf.push(OFF)
                }
            }
        }
        self.window.update_with_buffer(&buf, WIDTH, HEIGHT).unwrap();
    }

    /// disp_clear()
    fn clear_screen(&mut self) {
        self.display = [[false; 64]; 32];
    }

    /// goto NNN;
    fn goto(&mut self, addr: u16) {
        self.position_in_memory = addr as usize;
    }

    /// *(0xNNN)()
    fn call(&mut self, addr: u16) {
        let sp = self.stack_pointer;
        let stack = &mut self.stack;

        if sp > stack.len() {
            panic!("Stack overflow!")
        }

        stack[sp] = self.position_in_memory as u16;
        self.stack_pointer += 1;
        self.position_in_memory = addr as usize;
    }

    /// return;
    fn ret(&mut self) {
        if self.stack_pointer == 0 {
            panic!("Stack underflow!");
        }

        self.stack_pointer -= 1;
        self.position_in_memory = self.stack[self.stack_pointer] as usize;
    }

    /// if(Vx==NN)
    fn skip_if_equal(&mut self, x: u8, nn: u8) {
        if self.registers[x as usize] == nn {
            self.position_in_memory += 2;
        }
    }

    /// if(Vx!=NN)
    fn skip_if_not_equal(&mut self, x: u8, nn: u8) {
        if self.registers[x as usize] != nn {
            self.position_in_memory += 2;
        }
    }

    /// if(Vx==Vy)
    fn skip_xy_equal(&mut self, x: u8, y: u8) {
        if self.registers[x as usize] == self.registers[y as usize] {
            self.position_in_memory += 2;
        }
    }

    /// Vx = NN
    fn set_xnn(&mut self, x: u8, nn: u8) {
        self.registers[x as usize] = nn;
    }

    /// Vx += NN
    fn add_xnn(&mut self, x: u8, nn: u8) {
        self.registers[x as usize] = (self.registers[x as usize] as u16 + nn as u16) as u8;
    }

    /// Vx=Vy
    fn assign_xy(&mut self, x: u8, y: u8) {
        self.registers[x as usize] = self.registers[y as usize];
    }

    /// Vx=Vx|Vy
    fn or_xy(&mut self, x: u8, y: u8) {
        self.registers[x as usize] |= self.registers[y as usize];
    }

    /// Vx=Vx&Vy
    fn and_xy(&mut self, x: u8, y: u8) {
        self.registers[x as usize] &= self.registers[y as usize];
    }

    /// Vx=Vx^Vy
    fn xor_xy(&mut self, x: u8, y: u8) {
        self.registers[x as usize] ^= self.registers[y as usize];
    }

    /// Vx += Vy
    fn add_xy(&mut self, x: u8, y: u8) {
        let vx = self.registers[x as usize] as u16;
        let vy: u16 = self.registers[y as usize] as u16;
        let result = vx + vy;
        // Set the carry
        self.memory[VF] = if result > 0xFF { 1 } else { 0 };
        self.registers[x as usize] = result as u8;
    }

    /// Vx -= Vy
    fn sub_xy(&mut self, x: u8, y: u8) {
        self.memory[VF] = if self.memory[x as usize] > self.memory[y as usize] { 1 } else { 0 };
        self.registers[x as usize] -= self.registers[y as usize];
    }

    /// Vx>>=1
    fn shift_right(&mut self, x: u8) {
        self.memory[VF] = self.memory[x as usize] & 1;
        self.registers[x as usize] >>= 1;
    }

    /// Vx=Vy-Vx
    fn sub_yx(&mut self, x: u8, y: u8) {
        // Set the carry
        self.memory[VF] = if self.registers[y as usize] > self.registers[x as usize] { 1 } else { 0 };
        self.registers[x as usize] = self.registers[y as usize].wrapping_sub(self.registers[x as usize]);
    }

    /// Vx<<=1
    fn shift_left(&mut self, x: u8) {
        self.memory[VF] = (self.memory[x as usize] & 0b10000000) >> 7;
        self.registers[x as usize] <<= 1;
    }

    /// if(Vx==Vy)
    fn skip_xy_not_equal(&mut self, x: u8, y: u8) {
        if self.registers[x as usize] != self.registers[y as usize] {
            self.position_in_memory += 2;
        }
    }

    /// I = NNN
    fn set_16bit_register(&mut self, addr: u16) {
        self.i = addr;
    }

    /// PC=V0+NNN
    fn jump_nnn_plus_v0(&mut self, addr: u16) {
        self.position_in_memory = (self.registers[0] as u16 + addr) as usize;
    }

    /// Vx=rand()&NN
    fn rand(&mut self, x: u8, nn: u8) {
        self.registers[x as usize] = rand::random::<u8>() & nn;
    }

    /// draw(Vx,Vy,N)
    fn draw(&mut self, x: u8, y: u8, n: u8) {
        let vx = self.registers[x as usize];
        let vy = self.registers[y as usize];
        self.memory[VF] = 0;
        for r in 0..n {
            let row = self.memory[(self.i + r as u16) as usize];
            let screen_y = ((vy + r) % 32) as usize;
            for col in 0..8 {
                let val = (row & 0x80 >> col) > 0;
                let screen_x = ((vx + col) % 64) as usize;
                if val & self.display[screen_y][screen_x] != self.display[screen_y][screen_x] {
                    self.memory[VF] = 1;
                }
                self.display[screen_y][screen_x] ^= val;
            }
        }
        self.draw_flag = true;
    }

    /// if(key()==Vx)
    fn skip_if_key_pressed(&mut self, x: u8) {
        if self.keys[self.registers[x as usize] as usize] {
            self.position_in_memory += 2;
        }
    }

    /// if(key()!=Vx)
    fn skip_if_key_not_pressed(&mut self, x: u8) {
        if !self.keys[self.registers[x as usize] as usize] {
            self.position_in_memory += 2;
        }
    }

    /// Vx = get_delay()
    fn set_x_to_delay_timer(&mut self, x: u8) {
        self.registers[x as usize] = self.delay_timer;
    }

    /// Vx = get_key()
    fn set_x_to_keypress(&mut self, x: u8) {
        self.wait_for_keypress_and_set_keys();
        for (pos, &key) in self.keys.iter().enumerate() {
            if key {
                self.registers[x as usize] = pos as u8;
            }
        }
    }

    /// Reads raw stdin and records key presses
    /// Only the first key pressed is read. i.e. if '1' and '2' are both pressed, only '1' is set
    /// Blocking operation that waits on a VALID key press
    fn wait_for_keypress_and_set_keys(&mut self) {
        let mut key_pressed = false;
        self.window.update(); // Get current state before we check
        while !key_pressed {
            key_pressed = self.set_keys();
        }
    }

    fn set_keys(&mut self) -> bool {
        let mut key_pressed = false;
        self.window.get_keys_pressed(KeyRepeat::No).iter().for_each(|key|
            match key {
                Key::Key1 => {
                    self.keys[1] = true;
                    key_pressed = true;
                }
                Key::Key2 => {
                    self.keys[2] = true;
                    key_pressed = true;
                }
                Key::Key3 => {
                    self.keys[3] = true;
                    key_pressed = true;
                }
                Key::Key4 => {
                    self.keys[12] = true;
                    key_pressed = true;
                }
                Key::Q => {
                    self.keys[4] = true;
                    key_pressed = true;
                }
                Key::W => {
                    self.keys[5] = true;
                    key_pressed = true;
                }
                Key::E => {
                    self.keys[6] = true;
                    key_pressed = true;
                }
                Key::R => {
                    self.keys[13] = true;
                    key_pressed = true;
                }
                Key::A => {
                    self.keys[7] = true;
                    key_pressed = true;
                }
                Key::S => {
                    self.keys[8] = true;
                    key_pressed = true;
                }
                Key::D => {
                    self.keys[9] = true;
                    key_pressed = true;
                }
                Key::F => {
                    self.keys[14] = true;
                    key_pressed = true;
                }
                Key::Z => {
                    self.keys[10] = true;
                    key_pressed = true;
                }
                Key::X => {
                    self.keys[0] = true;
                    key_pressed = true;
                }
                Key::C => {
                    self.keys[11] = true;
                    key_pressed = true;
                }
                Key::V => {
                    self.keys[15] = true;
                    key_pressed = true;
                }
                _ => {},
            }
        );
        self.window.update(); // Update the window each time otherwise the state is static
        key_pressed
    }

    /// delay_timer(Vx)
    fn set_delay_timer_to_x(&mut self, x: u8) {
        self.delay_timer = self.registers[x as usize];
    }

    /// sound_timer(Vx)
    fn set_sound_timer_to_x(&mut self, x: u8) {
        self.sound_timer = self.registers[x as usize];
    }

    /// I +=Vx
    fn add_ix(&mut self, x: u8) {
        self.i += self.registers[x as usize] as u16;
    }

    /// I=sprite_addr[Vx]
    fn set_i_sprite_addr_x(&mut self, x: u8) {
        self.i = 0x50 + 5 * (self.registers[x as usize] as u16);
    }

    /// set_BCD(Vx);
    /// *(I+0)=BCD(3);
    /// *(I+1)=BCD(2);
    /// *(I+2)=BCD(1);
    fn set_bcd(&mut self, x: u8) {
        let vx = self.registers[x as usize];
        self.memory[self.i as usize] = vx / 100;
        self.memory[self.i as usize + 1] = (vx / 10) % 10;
        self.memory[self.i as usize + 2] = (vx % 100) % 10;
    }

    /// reg_dump(Vx,&I)
    fn reg_dump(&mut self, x: u8) {
        self.memory[(self.i as usize)..(self.i + x as u16 + 1) as usize]
            .copy_from_slice(&self.registers[0..(x as usize + 1)])
    }

    /// reg_load(Vx,&I)
    fn reg_load(&mut self, x: u8) {
        self.registers[0..x as usize + 1]
            .copy_from_slice(&self.memory[(self.i as usize)..(self.i + x as u16 + 1) as usize]);
    }

    /// Loads the specified chip8 program into memory
    pub fn load_into_memory(&mut self, file: &str) {
        self.load_fonts();
        let mut buffer = [0u8; 3584];
        match File::open(file) {
            Ok(mut file) => match file.read(&mut buffer[..]) {
                Ok(_bytes) => {
                    self.memory[0x200..].copy_from_slice(&buffer);
                }
                Err(err) => eprintln!("Error reading file: {}", err),
            },
            Err(err) => eprintln!("Error opening file: {}", err),
        }
    }

    fn load_fonts(&mut self) {
        let fonts: [u8; 80] = [
            0xf0, 0x90, 0x90, 0x90, 0xf0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xf0, 0x10, 0xf0, 0x80,
            0xf0, 0xf0, 0x10, 0xf0, 0x10, 0xf0, 0x90, 0x90, 0xf0, 0x10, 0x10, 0xf0, 0x80, 0xf0,
            0x10, 0xf0, 0xf0, 0x80, 0xf0, 0x90, 0xf0, 0xf0, 0x10, 0x20, 0x40, 0x40, 0xf0, 0x90,
            0xf0, 0x90, 0xf0, 0xf0, 0x90, 0xf0, 0x10, 0xf0, 0xf0, 0x90, 0xf0, 0x90, 0x90, 0xe0,
            0x90, 0xe0, 0x90, 0xe0, 0xf0, 0x80, 0x80, 0x80, 0xf0, 0xe0, 0x90, 0x90, 0x90, 0xe0,
            0xf0, 0x80, 0xf0, 0x80, 0xf0, 0xf0, 0x80, 0xf0, 0x80, 0x80,
        ];

        // 0x50 is the font offset
        // http://www.multigesture.net/articles/how-to-write-an-emulator-chip-8-interpreter/
        self.memory[0x50..0xA0].copy_from_slice(&fonts);
    }

    /// Loads a specified Chip8 program into memory and then runs
    pub fn load_and_run(&mut self, file: &str) {
        self.load_into_memory(file);
        self.run();
    }
}

#[test]
fn test_clear_screen() {
    let mut chip8 = CHIP8::new();
    chip8.display[0][0] = true;
    chip8.load_and_run("testbin/clear_screen.chip8");
    assert_eq!(chip8.display[0][0], false);
}

#[test]
fn test_skip_if_equal_iseq() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/is_eq.chip8");
    assert_eq!(chip8.registers[0], 5);
}

#[test]
fn test_skip_if_equal_noteq() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/not_eq.chip8");
    assert_eq!(chip8.registers[0], 6);
}

#[test]
fn test_skip_if_not_equal_iseq() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/skip_not_eq_iseq.chip8");
    assert_eq!(chip8.registers[0], 6);
}

#[test]
fn test_skip_if_not_equal_neq() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/skip_not_eq_neq.chip8");
    assert_eq!(chip8.registers[0], 5);
}

#[test]
fn test_skip_xy_equal_eq() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/xy_eq.chip8");
    assert_eq!(chip8.registers[0], 5);
}

#[test]
fn test_skip_xy_equal_neq() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/xy_neq.chip8");
    assert_eq!(chip8.registers[0], 6);
}

#[test]
fn test_skip_xy_not_equal_eq() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/xy_neq_eq.chip8");
    assert_eq!(chip8.registers[0], 6);
}

#[test]
fn test_skip_xy_not_equal_neq() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/xy_neq_neq.chip8");
    assert_eq!(chip8.registers[0], 5);
}

#[test]
fn test_set_xnn() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/set_registers.chip8");
    assert_eq!(chip8.registers[0], 5);
    assert_eq!(chip8.registers[1], 10);
}

#[test]
fn test_add_xnn() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/add_xnn.chip8");
    assert_eq!(chip8.registers[0], 10);
}

#[test]
fn test_assign_xy() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/assign_xy.chip8");
    assert_eq!(chip8.registers[0], 6);
}

#[test]
fn test_or_xy() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/or_xy.chip8");
    assert_eq!(chip8.registers[0], 255);
}

#[test]
fn test_and_xy() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/and_xy.chip8");
    assert_eq!(chip8.registers[0], 0);
}

#[test]
fn test_xor_xy() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/xor_xy.chip8");
    assert_eq!(chip8.registers[0], 255);
}

#[test]
fn test_add_xy() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/add_xy.chip8");
    assert_eq!(chip8.registers[0], 15);
}

#[test]
fn test_sub_xy() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/sub_xy.chip8");
    assert_eq!(chip8.registers[0], 5);
}

#[test]
fn test_shift_right() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/shift_right.chip8");
    assert_eq!(chip8.registers[0], 2);
}

#[test]
fn test_sub_yx() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/sub_yx.chip8");
    assert_eq!(chip8.registers[0], 5);
}

#[test]
fn test_shift_left() {
    let mut chip8 = CHIP8::new();
    chip8.load_into_memory("testbin/shift_left.chip8");
    chip8.run();
    assert_eq!(chip8.registers[0], 10);
}

#[test]
fn test_set_16bit_register() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/set_16bit_register.chip8");
    assert_eq!(chip8.i, 10);
}

#[test]
fn test_jump_nnn_plus_v0() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/jump_nnn_plus_v0.chip8");
    assert_eq!(chip8.registers[1], 5); // We skipped 0x610A
}

#[test]
fn test_rand() {
    let mut chip8 = CHIP8::new();
    assert_eq!(chip8.registers[0], 0);
    chip8.load_and_run("testbin/rand.chip8");
    assert_ne!(chip8.registers[0], 0);
}

#[test]
fn test_draw() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/draw.chip8");

    // Checking if we drew this:
    //   ****
    // **    **
    // ********
    assert_eq!(chip8.memory[chip8.i as usize], 0x3C);
    assert_eq!(chip8.memory[(chip8.i + 1) as usize], 0xC3);
    assert_eq!(chip8.memory[(chip8.i + 2) as usize], 0xFF);
    assert_eq!(chip8.display[0][0], false);
    assert_eq!(chip8.display[0][1], false);
    assert_eq!(chip8.display[0][2], true);
    assert_eq!(chip8.display[0][3], true);
    assert_eq!(chip8.display[0][4], true);
    assert_eq!(chip8.display[0][5], true);
    assert_eq!(chip8.display[1][0], true);
    assert_eq!(chip8.display[1][1], true);
    assert_eq!(chip8.display[1][6], true);
    assert_eq!(chip8.display[1][7], true);
    assert_eq!(chip8.display[0][5], true);
    assert_eq!(chip8.display[2][0], true);
    assert_eq!(chip8.display[2][1], true);
    assert_eq!(chip8.display[2][2], true);
    assert_eq!(chip8.display[2][3], true);
    assert_eq!(chip8.display[2][4], true);
    assert_eq!(chip8.display[2][5], true);
    assert_eq!(chip8.display[2][6], true);
    assert_eq!(chip8.display[2][7], true);
}

#[test]
fn test_skip_if_key_pressed() {
    let mut chip8 = CHIP8::new();
    chip8.keys[0] = true;
    chip8.load_and_run("testbin/skip_if_key_pressed.chip8");
    assert_eq!(chip8.registers[1], 1); // Skips last operation
}

#[test]
fn test_skip_if_key_pressed_not_pressed() {
    let mut chip8 = CHIP8::new();
    chip8.keys[0] = false;
    chip8.load_and_run("testbin/skip_if_key_pressed.chip8");
    assert_eq!(chip8.registers[1], 2); // Does not skip last operation
}

#[test]
fn test_skip_if_key_not_pressed_np() {
    let mut chip8 = CHIP8::new();
    chip8.keys[0] = false;
    chip8.load_and_run("testbin/skip_if_key_not_pressed.chip8");
    assert_eq!(chip8.registers[1], 1); // Skips last operation
}

#[test]
fn test_skip_if_key_not_pressed_p() {
    let mut chip8 = CHIP8::new();
    chip8.keys[0] = true;
    chip8.load_and_run("testbin/skip_if_key_not_pressed.chip8");
    assert_eq!(chip8.registers[1], 2); // Does not skip last operation
}

#[test]
fn test_set_timers() {
    let mut chip8 = CHIP8::new();
    assert_eq!(chip8.sound_timer, 0);
    assert_eq!(chip8.delay_timer, 0);
    chip8.load_and_run("testbin/timers.chip8");
    assert_eq!(chip8.registers[0], 5);
    assert_eq!(chip8.delay_timer, 5);
    assert_eq!(chip8.sound_timer, 10);
}

#[test]
#[ignore] // Ignoring because this test waits for a keyboardinterrupt, pressing 'w' will make the test pass
fn test_set_x_to_keypress() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/set_x_to_key_press.chip8");
    assert_eq!(chip8.registers[0], 5);
}

#[test]
fn test_add_ix() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/add_ix.chip8");
    assert_eq!(chip8.registers[0], 5);
    assert_eq!(chip8.i, 5);
}

#[test]
fn test_set_i_sprite_addr_x() {
    let mut chip8 = CHIP8::new();
    chip8.load_and_run("testbin/set_i_to_sprite.chip8");
    assert_eq!(chip8.i, 130);
}

#[test]
fn test_set_bcd() {
    let mut chip8 = CHIP8::new();
    chip8.i = 0x600;
    chip8.load_and_run("testbin/set_bcd.chip8");

    assert_eq!(chip8.memory[1536], 2);
    assert_eq!(chip8.memory[1537], 3);
    assert_eq!(chip8.memory[1538], 8);
}

#[test]
fn test_reg_dump() {}

#[test]
fn test_reg_load() {}

#[test]
fn test_load_into_memory() {
    let mut chip8 = CHIP8::new();
    chip8.load_into_memory("testbin/stack_math.chip8");

    // Check that everything is in place
    assert_eq!(chip8.registers[0], 0);
    assert_eq!(chip8.registers[1], 0);

    // Check the fonts

    // Check for zero
    assert_eq!(chip8.memory[0x50], 0xf0);
    assert_eq!(chip8.memory[0x51], 0x90);
    assert_eq!(chip8.memory[0x52], 0x90);
    assert_eq!(chip8.memory[0x53], 0x90);
    assert_eq!(chip8.memory[0x54], 0xf0);

    // Check loaded program memory
    assert_eq!(chip8.memory[0x200], 0x60);
    assert_eq!(chip8.memory[0x201], 0x05);
    assert_eq!(chip8.memory[0x202], 0x61);
    assert_eq!(chip8.memory[0x203], 0x0A);
    assert_eq!(chip8.memory[0x204], 0x23);
    assert_eq!(chip8.memory[0x205], 0x00);
    assert_eq!(chip8.memory[0x206], 0x23);
    assert_eq!(chip8.memory[0x207], 0x00);
    assert_eq!(chip8.memory[0x300], 0x80);
    assert_eq!(chip8.memory[0x301], 0x14);
    assert_eq!(chip8.memory[0x302], 0x80);
    assert_eq!(chip8.memory[0x303], 0x14);
    assert_eq!(chip8.memory[0x304], 0x00);
    assert_eq!(chip8.memory[0x305], 0xEE);

    chip8.run();

    // Check the results in the registers
    assert_eq!(chip8.registers[1], 10);
    assert_eq!(chip8.registers[0], 45);
}
