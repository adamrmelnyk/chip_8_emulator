struct CPU {
    registers: [u8; 16],
    i: u16,
    position_in_memory: usize,
    memory: [u8; 4096],
    stack: [u16; 16],
    stack_pointer: usize,
}

impl CPU {
    fn run(&mut self) {
        loop {
            let op_byte1 = self.memory[self.position_in_memory] as u16;
            let op_byte2 = self.memory[self.position_in_memory + 1] as u16;
            let opcode = op_byte1 << 8 | op_byte2;

            let x = ((opcode & 0x0F00) >> 8) as u8;
            let y = ((opcode & 0x00F0) >> 4) as u8;
            let nn = (opcode & 0x00FF) as u8;
            let op_minor = (opcode & 0x000F) as u8;
            let addr = opcode & 0x0FFF;

            self.position_in_memory += 2;

            match opcode {
                0x0000 => { return },
                0x00EE => { self.ret() },
                0x1000..=0x1FFF => { self.goto(addr) },
                0x2000..=0x2FFF => { self.call(addr) }
                0x3000..=0x3FFF => { self.skip_if_equal(x, nn) }
                0x4000..=0x4FFF => { self.skip_if_not_equal(x, nn) }
                0x5000..=0x5FF0 => { self.skip_xy_equal(x, y) }
                0x6000..=0x6FFF => { self.set_xnn(x, nn) }
                0x7000..=0x7FFF => { self.add_xnn(x, nn) }
                0x8000..=0x8FFF => {
                    match op_minor {
                        0 => self.assign_xy(x,y),
                        1 => self.or_xy(x,y),
                        2 => self.and_xy(x,y),
                        3 => self.xor_xy(x,y),
                        4 => self.add_xy(x,y),
                        5 => self.sub_xy(x,y),
                        6 => self.shift_right(x),
                        7 => self.sub_yx(x,y),
                        14 => self.shift_left(x),
                        _ => unimplemented!("opcode {:04x}", opcode),
                    }
                },
                0x9000..=0x9FFF => { self.skip_xy_not_equal(x, y) }
                0xA000..=0xAFFF => { self.set_16bit_register(addr) }
                0xB000..=0xBFFF => { self.jump_nnn_plus_v0(addr) }
                0xC000..=0xCFFF => { self.rand(x, nn) }
                0xD000..=0xDFFF => { self.draw(x, y, op_minor) }
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
        self.registers[x as usize] += self.registers[y as usize];
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
        // TODO: Implement
        unimplemented!("Function is not implemented yet")
    }
}

fn main() {
    let mut cpu = CPU {
        registers: [0; 16],
        i: 0,
        memory: [0; 4096],
        position_in_memory: 0,
        stack: [0; 16],
        stack_pointer: 0,
    };

    cpu.registers[0] = 5;
    cpu.registers[1] = 10;

    cpu.memory[0x000] = 0x21; cpu.memory[0x001] = 0x00;
    cpu.memory[0x002] = 0x21; cpu.memory[0x003] = 0x00;
    cpu.memory[0x100] = 0x80; cpu.memory[0x101] = 0x14;
    cpu.memory[0x102] = 0x80; cpu.memory[0x103] = 0x14;
    cpu.memory[0x104] = 0x00; cpu.memory[0x105] = 0xEE;

    cpu.run();

    assert_eq!(cpu.registers[0], 45);
    println!("5 + (10 * 2) + (10 * 2) = {}", cpu.registers[0]);
}