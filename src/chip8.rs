use std::io::prelude::*;
use std::fs::File;
use std::io::BufReader;
use rand::{Rng, thread_rng};
use rand::rngs::ThreadRng;
use rand::distributions::Uniform;

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;

const CHIP8_FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub struct Chip8 {
    // Memory and CPU stuff
    memory: [u8; 4096],
    stack: Vec<usize>,
    v: [u8; 16], // General purpose V registers
    i: usize, // Index register
    pc: usize, // Program Counter
    delay_timer: u8,
    sound_timer: u8,

    // Needed for CXNNN
    rng: ThreadRng,

    // Store pressed key values here
    pub keys: [u8; 16],

    // Display
    pub screen: [usize; WIDTH*HEIGHT],
}

enum PCUpdateFlag {
    Next,
    Skip,
    Block,
    Set(usize),
}

impl Chip8 {
    pub fn new() -> Chip8 {
        Chip8 {
            keys: [0; 16],
            memory: [0; 4096],
            stack: vec![],
            v: [0; 16],
            i: 0,
            pc: 0x200,
            delay_timer: 0,
            sound_timer: 0,
            screen: [0; WIDTH*HEIGHT],
            rng: thread_rng(),
        }
    }

    pub fn init(&mut self, path: &str) {
        self.load_rom(path);
        self.load_font();
    }

    fn load_rom(&mut self, path: &str) {
        let mut f = BufReader::new(File::open(path).expect("File not found"));
        f.read(&mut self.memory[0x200..0xFFF]).expect("Could not read in rom");
    }

    fn load_font(&mut self) {
        for (i, v) in CHIP8_FONTSET.iter().enumerate() {
            self.memory[0x050+i] = *v
        }
    }

    pub fn run_cycle(&mut self) {
        let op = self.fetch_opcode();
        match self.execute_opcode(op) {
            PCUpdateFlag::Next => self.pc += 2,
            PCUpdateFlag::Skip => self.pc += 4,
            PCUpdateFlag::Set(addr) => self.pc = addr,
            PCUpdateFlag::Block => (),

        }
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        // TODO: Implement actual buzzer
        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                println!("BEEP");
            }
            self.sound_timer -= 1;
        }
    }

    fn fetch_opcode(&self) -> u16 {
        (self.memory[self.pc] as u16) << 8 | self.memory[self.pc + 1] as u16
    }

    fn execute_opcode(&mut self, op: u16) -> PCUpdateFlag {
        match op & 0xF000 {
            // 00E0 and 00EE
            0x0000 => match op & 0x00FF {
                // 00E0: Clears the screen
                0xE0 => {
                    self.screen = [0; WIDTH*HEIGHT];
                    PCUpdateFlag::Next
                }
                // 00EE: Returns from subroutine
                0xEE => {
                    self.pc = self.stack.pop().expect("0x0EE opcode ran with an empty stack!");
                    PCUpdateFlag::Next
                },
                _ => panic!("Unknown 0x00E opcode: {:x}", op)
            }
            // 1NNN: Jump to address NNN
            0x1000 => PCUpdateFlag::Set(get_addr(op)),
            // 2NNN: Call subroutine at NNN
            0x2000 => {
                let nnn = get_addr(op);
                self.stack.push(self.pc);
                PCUpdateFlag::Set(nnn)
            }
            // 3XNN: Skips the next instruction if VX == NN
            0x3000 => {
                self.cond_skip_n(op, |x, y| { x == y })
            },
            // 4XNN: Skips next instruction if VX != NN
            0x4000 => {
                self.cond_skip_n(op, |x, y| { x != y })
            },
            // 5XY0: Skips next instruction if VX == VY
            0x5000 => {
                self.cond_skip_v(op, |x, y| { x == y })
            },
            // 6XNN: Sets VX to NN
            0x6000 => {
                self.v[get_opx(op)] = (op & 0x00FF) as u8;
                PCUpdateFlag::Next
            },
            // 7XNN: Adds NN to VX (Carry flag is not changed)
            0x7000 => {
                let vx = get_opx(op);
                let nn = (op & 0x00FF) as u8;
                let res = self.v[vx].wrapping_add(nn);
                self.v[vx] = res;
                PCUpdateFlag::Next
            },
            // Multiple 0x8000 opcodes
            0x8000 => match op & 0x000F {
                // 8XY0: Sets VX = VY
                0x0 => {
                    let (pvx, pvy) = get_opxy(op);
                    self.v[pvx] = self.v[pvy];
                    PCUpdateFlag::Next
                },
                // 8XY1: Sets VX = VX | VY
                0x1 => {
                    self.set_vx(op, |x, y| { x | y })
                },
                // 8XY2: Sets VX = VX & VY
                0x2 => {
                    self.set_vx(op, |x, y| { x & y })
                },
                // 8XY3: Sets VX = VX ^ VY
                0x3 => {
                    self.set_vx(op, |x, y| { x ^ y })
                },
                // 8XY4: Sets VX = VX + VY (Sets carry flag)
                0x4 => {
                    let (pvx, pvy) = get_opxy(op);
                    let (res, flag) = self.v[pvx].overflowing_add(self.v[pvy]);
                    if flag {
                        self.v[0xF] = 1;
                    } else {
                        self.v[0xF] = 0;
                    }
                    self.v[pvx] = res;
                    PCUpdateFlag::Next
                },
                // 8XY5: Sets VX = VX - VY (Sets carry flag)
                0x5 => {
                    let (pvx, pvy) = get_opxy(op);
                    let (res, flag) = self.v[pvx].overflowing_sub(self.v[pvy]);
                    if flag {
                        self.v[0xF] = 1;
                    } else {
                        self.v[0xF] = 0;
                    }
                    self.v[pvx] = res;
                    PCUpdateFlag::Next
                },
                // 8XY6: Stores the least significant bit of VX in VF and then shifts VX right by 1
                0x6 => {
                    let pvx = get_opx(op);
                    self.v[0xF] = self.v[pvx] & 0x1;
                    self.v[pvx] >>= 1;
                    PCUpdateFlag::Next
                },
                // 8XY7: Sets VX = VY - VX (Set carry flag)
                0x7 => {
                    let (pvx, pvy) = get_opxy(op);
                    let (res, flag) = self.v[pvy].overflowing_sub(self.v[pvx]);
                    if flag {
                        self.v[0xF] = 1;
                    } else {
                        self.v[0xF] = 0;
                    }
                    self.v[pvx] = res;
                    PCUpdateFlag::Next
                },
                // 8XYE: Stores the least significant bit of VX in VF and then shifts VX left by 1
                0xE => {
                    let pvx = get_opx(op);
                    self.v[0xF] = self.v[pvx] & 0x1;
                    self.v[pvx] <<= 1;
                    PCUpdateFlag::Next
                },
                _ => panic!("Unknown 0x8000 opcode: {:x}", op)
            }
            // 9XY0: Skips next instruction if VX != VY
            0x9000 => {
                self.cond_skip_v(op, &|x, y| { x != y })
            },
            // ANNN: Sets I to the address NNNN
            0xA000 => {
                self.i = get_addr(op);
                PCUpdateFlag::Next
            },
            // BNNN: Jumps to the address NNN plus V0
            0xB000 => {
                let nnn = get_addr(op);
                let v0 = self.v[0] as usize;
                PCUpdateFlag::Set(nnn+v0)
            }
            // CXNN: Sets VX to the result of a bitwise and operation on a random number (Typically: 0 to 255) and NN.
            0xC000 => {
                let range = Uniform::new(0, 255);
                let num = self.rng.sample(range);
                let pvx = get_opx(op);
                let nn = op & 0x00FF;
                self.v[pvx] = (num & nn) as u8;
                PCUpdateFlag::Next
            }
            // DXYN: Draws a sprite at VX,VY, 8px wide, height of N+1px. Each row read from
            // memory[I]. Set VF to 1 if any pixel goes from 1 to 0, set to 0 if that doesn't
            // happen.
            0xD000 => {
                let vx = self.v[get_opx(op)] as usize;
                let vy = self.v[get_opy(op)] as usize;
                let n = (op & 0x000F) as usize;

                self.v[0xF] = 0;
                for y in 0..n {
                    let px = self.memory[self.i+y];
                    for x in 0..8 {
                        let location = vx + x + ((vy + y) * WIDTH);
                        if px & (0x80 >> x) != 0 {
                            if self.screen[location] == 1 {
                                self.v[0xF] = 1;
                            }
                            self.screen[location] ^= 1;
                        }
                    }
                }

                PCUpdateFlag::Next
            },
            // Multiple 0xE000 opcodes
            0xE000 => match op & 0x00FF {
                // EX9E: Skips next instruction if key in VX is pressed
                0x9E => {
                    let vx = self.v[get_opx(op)] as usize;
                    if self.keys[vx] == 1 {
                        PCUpdateFlag::Skip
                    }
                    else {
                        PCUpdateFlag::Next
                    }
                },
                // EXA1: Skips next instruction if key in VX ISN'T pressed
                0xA1 => {
                    let vx = self.v[get_opx(op)] as usize;
                    if self.keys[vx] != 1 {
                        PCUpdateFlag::Skip
                    }
                    else {
                        PCUpdateFlag::Next
                    }
                },
                _ => panic!("Unkown 0xE000 opcode: {:x}", op)
            }
            // Multiple 0xF000 opcodes
            0xF000 => match op & 0x00FF {
                // FX07: Sets VX to the value of the delay timer
                0x07 => {
                    self.v[get_opx(op)] = self.delay_timer;
                    PCUpdateFlag::Next
                }
                // FX0A: A key press is awaited and then stored in VX (blocking)
                0x0A => {
                    let vx = get_opx(op);
                    for i in 0..16 {
                        if self.keys[i] == 1 {
                            self.v[vx] = i as u8;
                            return PCUpdateFlag::Next
                        }
                    }
                    PCUpdateFlag::Block
                },
                // FX15: Set delay timer to VX
                0x15 => {
                    self.delay_timer = self.v[get_opx(op)];
                    PCUpdateFlag::Next
                },
                // FX18: Set sound timer to VX
                0x18 => {
                    self.sound_timer = self.v[get_opx(op)];
                    PCUpdateFlag::Next
                },
                // FX1E: Adds VX to I (carry flag not set)
                0x1E => {
                    self.i = self.i.wrapping_add(self.v[get_opx(op)] as usize);
                    PCUpdateFlag::Next
                },
                // FX29: Sets I to the location of the sprite for the caracter in VX
                0x29 => {
                    self.i = (self.v[get_opx(op)]+0x050) as usize;
                    PCUpdateFlag::Next
                },
                // FX33: Stores the BCD representatin of VX, with the most significant of three
                // digits at the address in I, the middle digit at I+1, and the least significat
                // digit at I+2
                0x33 => {
                    let mut vx = self.v[get_opx(op)];
                    for i in (0..3).rev() {
                        self.memory[self.i + i] = vx % 10;
                        vx /= 10;
                    }
                    PCUpdateFlag::Next
                },
                // FX55: Stores V0 to VX (inclusive) in memory addr starting at I
                0x55 => {
                    let vx = get_opx(op);
                    for i in 0..=vx {
                        self.memory[self.i+i] = self.v[i]
                    }
                    PCUpdateFlag::Next
                },
                // FX55: Loads V0 to VX (inclusive) in memory addr starting at I
                0x65 => {
                    let vx = get_opx(op);
                    for i in 0..=vx {
                        self.v[i] = self.memory[self.i+i]
                    }
                    PCUpdateFlag::Next
                },
                _ => panic!("Unknown 0xF000 opcode: {:x}", op)
            }
            _ => panic!("Unknown opcode: {:x}", op)
        }
    }

    fn cond_skip_v(&self, op: u16, f: impl Fn(u8, u8) -> bool) -> PCUpdateFlag {
        let vx = self.v[get_opx(op)];
        let vy = self.v[get_opy(op)];

        if f(vx, vy) {
            PCUpdateFlag::Skip
        } else {
            PCUpdateFlag::Next
        }
    }

    fn cond_skip_n(&self, op: u16, f: impl Fn(u8, u8) -> bool) -> PCUpdateFlag {
        let vx = self.v[get_opx(op)];
        let nn = (op & 0x00FF) as u8;

        if f(vx, nn) {
            PCUpdateFlag::Skip
        } else {
            PCUpdateFlag::Next
        }
    }

    fn set_vx(&mut self, op: u16, f: impl Fn(u8, u8) -> u8) -> PCUpdateFlag {
        let (pvx, pvy) = get_opxy(op);
        self.v[pvx] = f(self.v[pvx], self.v[pvy]);
        PCUpdateFlag::Next
    }
}

fn get_addr(op: u16) -> usize {
    (op & 0x0FFF) as usize
}

fn get_opx(op: u16) -> usize {
    ((op & 0x0F00) >> 8) as usize
}

fn get_opy(op: u16) -> usize {
    ((op & 0x00F0) >> 4) as usize
}

fn get_opxy(op: u16) -> (usize, usize) {
    (get_opx(op), get_opy(op))
}

