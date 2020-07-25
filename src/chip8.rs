use std::fs::File;
use std::io::Read;
use std::io::{stdin, stdout, Write};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

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
    display: [[bool; 64]; 32],
}

impl CHIP8 {
    pub fn new() -> CHIP8 {
        CHIP8 {
            registers: [0; 16],
            i: 0,
            memory: [0; 4096],
            position_in_memory: 0,
            stack: [0; 16],
            stack_pointer: 0,
            keys: [false; 16],
            delay_timer: 0,
            sound_timer: 0,
            display: [[false; 64]; 32],
        }
    }

    pub fn run(&mut self) {
        loop {
            let op_byte1 = self.memory[self.position_in_memory] as u16;
            let op_byte2 = self.memory[self.position_in_memory + 1] as u16;
            let opcode = op_byte1 << 8 | op_byte2;

            let x = ((opcode & 0x0F00) >> 8) as u8;
            let y = ((opcode & 0x00F0) >> 4) as u8;
            let nn = (opcode & 0x00FF) as u8;
            let n = (opcode & 0x000F) as u8;
            let nnn = opcode & 0x0FFF;

            self.position_in_memory += 2;

            match opcode {
                0x0000 => return,
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
        }
    }

    fn goto(&mut self, addr: u16) {
        self.position_in_memory = addr as usize;
    }

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
        self.registers[x as usize] += nn;
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
        self.registers[x as usize] += self.registers[y as usize];
    }

    /// Vx -= Vy
    fn sub_xy(&mut self, x: u8, y: u8) {
        self.registers[x as usize] -= self.registers[y as usize];
    }

    /// Vx>>=1
    fn shift_right(&mut self, x: u8) {
        self.registers[x as usize] >>= 1;
    }

    /// Vx=Vy-Vx
    fn sub_yx(&mut self, x: u8, y: u8) {
        self.registers[x as usize] = self.registers[y as usize] - self.registers[x as usize];
    }

    /// Vx<<=1
    fn shift_left(&mut self, x: u8) {
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
        // From Wikipedia
        // Draws a sprite at coordinate (VX, VY) that has a width of 8 pixels and a height of N pixels.
        // Each row of 8 pixels is read as bit-coded starting from memory location I;
        // I value doesn’t change after the execution of this instruction. As described above,
        // VF is set to 1 if any screen pixels are flipped from set to unset when the sprite is drawn,
        // and to 0 if that doesn’t happen
        let sprite = &self.memory[self.i as usize..(self.i + n as u16) as usize] as &[u8];
        for (r, row) in sprite.iter().enumerate() {
            for c in 0..8 {
                let new = row >> (7 - c) & 0x01;
                if new == 1 {
                    let xc = (x as usize + c) % 64;
                    let yr = (y as usize + r) % 32;
                    let old = self.display[xc][yr];
                    if old {
                        self.registers[15] = 1;
                    }
                    self.display[xc][yr] = (new == 1) ^ old;
                }
            }
        }
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
        self.set_keys(); // This operations waits for input
        for (pos, &key) in self.keys.iter().enumerate() {
            if key {
                self.registers[x as usize] = pos as u8;
            }
        }
    }

    /// Reads raw stdin and records key presses
    /// Only the first key pressed is read. i.e. if '1' and '2' are both pressed, only '1' is set
    fn set_keys(&mut self) {
        let stdin = stdin();
        let mut stdout = stdout().into_raw_mode().unwrap();
        stdout.flush().unwrap();
        for c in stdin.keys() {
            let event = c.unwrap();
            match event {
                Key::Char('1') => {
                    self.keys[0] = true;
                    break;
                }
                Key::Char('2') => {
                    self.keys[1] = true;
                    break;
                }
                Key::Char('3') => {
                    self.keys[2] = true;
                    break;
                }
                Key::Char('4') => {
                    self.keys[3] = true;
                    break;
                }
                Key::Char('q') => {
                    self.keys[4] = true;
                    break;
                }
                Key::Char('w') => {
                    self.keys[5] = true;
                    break;
                }
                Key::Char('e') => {
                    self.keys[6] = true;
                    break;
                }
                Key::Char('r') => {
                    self.keys[7] = true;
                    break;
                }
                Key::Char('a') => {
                    self.keys[8] = true;
                    break;
                }
                Key::Char('s') => {
                    self.keys[9] = true;
                    break;
                }
                Key::Char('d') => {
                    self.keys[10] = true;
                    break;
                }
                Key::Char('f') => {
                    self.keys[11] = true;
                    break;
                }
                Key::Char('z') => {
                    self.keys[12] = true;
                    break;
                }
                Key::Char('x') => {
                    self.keys[13] = true;
                    break;
                }
                Key::Char('c') => {
                    self.keys[14] = true;
                    break;
                }
                Key::Char('v') => {
                    self.keys[15] = true;
                    break;
                }
                _ => {}
            }
        }
        stdout.flush().unwrap();
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
        self.i = self.registers[x as usize] as u16 * 5;
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
        let mut buffer = [0u8; 4096];
        match File::open(file) {
            Ok(mut file) => match file.read(&mut buffer[..]) {
                Ok(_bytes) => {
                    self.memory[0..].copy_from_slice(&buffer);
                }
                Err(err) => eprintln!("Error reading file: {}", err),
            },
            Err(err) => eprintln!("Error opening file: {}", err),
        }
    }

    /// Loads a specified Chip8 program into memory and then runs
    pub fn load_and_run(&mut self, file: &str) {
        self.load_into_memory(file);
        self.run();
    }
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
    // TODO
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
#[ignore] // Ignoring because this test waits for a keyboardinterrupt
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
fn test_set_i_sprite_addr_x() {}

#[test]
fn test_set_bcd() {}

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
    assert_eq!(chip8.memory[0x000], 0x60);
    assert_eq!(chip8.memory[0x001], 0x05);
    assert_eq!(chip8.memory[0x002], 0x61);
    assert_eq!(chip8.memory[0x003], 0x0A);
    assert_eq!(chip8.memory[0x004], 0x21);
    assert_eq!(chip8.memory[0x005], 0x00);
    assert_eq!(chip8.memory[0x006], 0x21);
    assert_eq!(chip8.memory[0x007], 0x00);
    assert_eq!(chip8.memory[0x100], 0x80);
    assert_eq!(chip8.memory[0x101], 0x14);
    assert_eq!(chip8.memory[0x102], 0x80);
    assert_eq!(chip8.memory[0x103], 0x14);
    assert_eq!(chip8.memory[0x104], 0x00);
    assert_eq!(chip8.memory[0x105], 0xEE);

    chip8.run();

    // Check the results in the registers
    assert_eq!(chip8.registers[1], 10);
    assert_eq!(chip8.registers[0], 45);
}