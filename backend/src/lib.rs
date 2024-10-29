mod constants;

use crate::constants::{
    DIGIT_SPRITES, DIGIT_SPRITES_SIZE, NUM_KEYS, NUM_REGS, RAM_SIZE, SCREEN_HEIGHT, SCREEN_WIDTH,
    STACK_SIZE, START_ADDRESS,
};
use rand::random;

// TODO add flags for runtime errors caused
//      by bugs in the input ROM (should be similar to how screen is used)
pub struct Processor {
    pc: u16, // program counter
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    sound: bool,
    v_reg: [u8; NUM_REGS],
    i_reg: u16,
    sp: u16, // stack pointer
    stack: [u16; STACK_SIZE],
    keys: [bool; NUM_KEYS],
    dt: u8, // delay timer
    st: u8, // sound timer
}

impl Processor {
    pub fn new() -> Self {
        let mut new_processor = Self {
            pc: START_ADDRESS,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            sound: false,
            v_reg: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            dt: 0,
            st: 0,
        };
        new_processor.ram[..DIGIT_SPRITES_SIZE].copy_from_slice(&DIGIT_SPRITES);
        new_processor
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDRESS;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.sound = false;
        self.v_reg = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.dt = 0;
        self.st = 0;
        self.ram[..DIGIT_SPRITES_SIZE].copy_from_slice(&DIGIT_SPRITES);
    }

    // TODO: behavior for overflow?
    fn push(&mut self, value: u16) {
        self.stack[self.sp as usize] = value;
        self.sp += 1;
    }

    // TODO: behavior for underflow?
    fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }

    pub fn tick(&mut self) {
        // Fetch
        let opcode = self.fetch();
        // Decode and Execute
        self.execute(opcode);
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn get_sound(&self) -> bool {
        self.sound
    }

    pub fn keypress(&mut self, index: usize, pressed: bool) {
        self.keys[index] = pressed;
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDRESS as usize;
        let end = (START_ADDRESS as usize) + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    fn execute(&mut self, opcode: u16) {
        let digit1 = (opcode & 0xF000) >> (3 * 4);
        let digit2 = (opcode & 0x0F00) >> (2 * 4);
        let digit3 = (opcode & 0x00F0) >> 4;
        let digit4 = opcode & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            // Nop
            (0, 0, 0, 0) => return,

            // Clear screen
            (0, 0, 0xE, 0) => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            }

            // Return from subroutine
            (0, 0, 0xE, 0xE) => {
                self.pc = self.pop();
            }

            // (1NNN) Jump to address 0xNNN
            (1, _, _, _) => {
                self.pc = opcode & 0xFFF;
            }

            // (2NNN) Call 0xNNN
            //        Enter subroutine at 0xNNN, adding current PC to stack
            //        so we can return here
            (2, _, _, _) => {
                self.push(self.pc);
                self.pc = opcode & 0xFFF;
            }

            // (3XNN) Skip if VX == 0xNN
            (3, _, _, _) => {
                let x = digit2 as usize;
                let nn = (opcode & 0xFF) as u8;
                if self.v_reg[x] == nn {
                    self.pc += 2;
                }
            }

            // (4XNN) Skip if VX != 0xNN
            (4, _, _, _) => {
                let x = digit2 as usize;
                let nn = (opcode & 0xFF) as u8;
                if self.v_reg[x] != nn {
                    self.pc += 2
                }
            }

            // (5XY0) Skip if VX == VY
            (5, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                if self.v_reg[x] == self.v_reg[y] {
                    self.pc += 2
                }
            }

            // (6XNN) VX = 0xNN
            (6, _, _, _) => {
                let x = digit2 as usize;
                let nn = (opcode & 0xFF) as u8;
                self.v_reg[x] = nn;
            }

            // (7XNN) VX += 0xNN
            //        Doesn't affect carry flag
            (7, _, _, _) => {
                let x = digit2 as usize;
                let nn = (opcode & 0xFF) as u8;
                self.v_reg[x] = self.v_reg[x].wrapping_add(nn);
            }

            // (8XY0) VX = VY
            (8, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] = self.v_reg[y];
            }

            // (8XY1) VX |= VY
            (8, _, _, 1) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] |= self.v_reg[y];
            }

            // (8XY2) VX &= VY
            (8, _, _, 2) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] &= self.v_reg[y];
            }

            // (8XY3) VX ^= VY
            (8, _, _, 3) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] ^= self.v_reg[y];
            }

            // (8XY4) VX += VY
            //        Sets VF if carry
            (8, _, _, 4) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (new_vx, carry) = self.v_reg[x].overflowing_add(self.v_reg[y]);
                let new_vf = if carry { 1 } else { 0 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            }

            // (8XY5) VX -= VY
            //        Clears VF if borrow
            (8, _, _, 5) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (new_vx, carry) = self.v_reg[x].overflowing_sub(self.v_reg[y]);
                let new_vf = if carry { 0 } else { 1 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            }

            // (8XY6) VX >>= 1
            //        Stores dropped bit in VF
            (8, _, _, 6) => {
                let x = digit2 as usize;

                let dropped_bit = self.v_reg[x] & 1;

                self.v_reg[x] >>= 1;
                self.v_reg[0xF] = dropped_bit;
            }

            // (8XY7) VX = VY - VX
            //        Clears VF if borrow
            (8, _, _, 7) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (new_vx, carry) = self.v_reg[y].overflowing_sub(self.v_reg[x]);
                let new_vf = if carry { 1 } else { 0 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            }

            // (8XYE) VX <<= VY
            //        Store dropped bit in VF
            (8, _, _, 0xE) => {
                let x = digit2 as usize;

                let dropped_bit = (self.v_reg[x] >> 7) & 1;

                self.v_reg[x] <<= 1;
                self.v_reg[0xF] = dropped_bit;
            }

            // (9XY0) Skip if VX != VY
            (9, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                if self.v_reg[x] != self.v_reg[y] {
                    self.pc += 2
                }
            }

            // (ANNN) I = 0xNNN
            (0xA, _, _, _) => {
                let nnn = opcode & 0xFFF;

                self.i_reg = nnn;
            }

            // (BNNN) Jump to V0 + 0xNNN
            (0xB, _, _, _) => {
                let nnn = opcode & 0xFFF;

                self.pc = (self.v_reg[0] as u16) + nnn;
            }

            // (CXNN) VX = rand() & 0xNN
            (0xC, _, _, _) => {
                let x = digit2 as usize;
                let nn = (opcode & 0xFF) as u8;

                let random_integer: u8 = random();

                self.v_reg[x] = random_integer & nn;
            }

            // (DXYN) Draw sprite at (VX, VY)
            //        Sprite is 0xN pixels tall, on/off based on value in I,
            //        VF set if any pixels flipped (from on to off)
            (0xD, _, _, _) => {
                // get coords where sprite will be drawn
                let x_coord = self.v_reg[digit2 as usize] as u16;
                let y_coord = self.v_reg[digit3 as usize] as u16;
                let num_rows = digit4;

                let mut flipped = false;

                for y_line in 0..num_rows {
                    let address = self.i_reg + y_line as u16;
                    let pixels = self.ram[address as usize];

                    for x_line in 0..8 {
                        // use mask to get current pixel's bit
                        if (pixels & (0b1000_0000 >> x_line)) != 0 {
                            // sprites wrap around screen
                            let x = (x_coord + x_line) as usize % SCREEN_WIDTH;
                            let y = (y_coord + y_line) as usize % SCREEN_HEIGHT;

                            // pixel's index in 1D array
                            let pixel_index = x + SCREEN_WIDTH * y;

                            if self.screen[pixel_index] {
                                flipped = true;
                            }

                            self.screen[pixel_index] ^= true;
                        }
                    }
                }

                if flipped {
                    self.v_reg[0xF] = 1;
                } else {
                    self.v_reg[0xF] = 0;
                }
            }

            // (EX9E) Skip if key index in VX is pressed
            (0xE, _, 9, 0xE) => {
                let vx = self.v_reg[digit2 as usize];

                if self.keys[vx as usize] {
                    self.pc += 2;
                }
            }

            // (EXA1) Skip if key index in VX isn't pressed
            (0xE, _, 0xA, 1) => {
                let vx = self.v_reg[digit2 as usize];

                if !self.keys[vx as usize] {
                    self.pc += 2;
                }
            }

            // (FX07) VX = Delay Timer
            (0xF, _, 0, 7) => {
                let x = digit2 as usize;

                self.v_reg[x] = self.dt;
            }

            // (FX0A) Waits for keypress, stores index in VX
            //        Blocking operation
            (0xF, _, 0, 0xA) => {
                let x = digit2 as usize;

                let mut pressed = false;

                for i in 0..self.keys.len() {
                    if self.keys[i] {
                        self.v_reg[x] = i as u8;
                        pressed = true;
                        break;
                    }
                }

                // redo if no button pressed
                if !pressed {
                    self.pc -= 2;
                }
            }

            // (FX15) Delay Timer = VX
            (0xF, _, 1, 5) => {
                let x = digit2 as usize;

                self.dt = self.v_reg[x];
            }

            // (FX18) Sound Timer = VX
            (0xF, _, 1, 8) => {
                let x = digit2 as usize;

                self.st = self.v_reg[x];
            }

            // (FX1E) I += VX
            (0xF, _, 1, 0xE) => {
                let x = digit2 as usize;

                self.i_reg = self.i_reg.wrapping_add(self.v_reg[x] as u16);
            }

            // (FX29) Set I to address of font character in VX
            (0xF, _, 2, 9) => {
                let x = digit2 as usize;

                self.i_reg = (self.v_reg[x] as u16) * 5;
            }

            // (FX33) Stores BCD encoding of VX into I
            (0xF, _, 3, 3) => {
                let vx = self.v_reg[digit2 as usize];

                let hundreds = (vx - vx % 100) / 100;
                let ones = vx % 10;
                let tens = (vx - 100 * hundreds - ones) / 10;

                self.ram[self.i_reg as usize] = hundreds;
                self.ram[(self.i_reg + 1) as usize] = tens;
                self.ram[(self.i_reg + 2) as usize] = ones;
            }

            // (FX55) Stores V0 thru VX into RAM address starting at I
            //        Inclusive range
            (0xF, _, 5, 5) => {
                let x = digit2 as usize;
                let i_reg_value = self.i_reg as usize;

                for i in 0..=x {
                    self.ram[i_reg_value + i] = self.v_reg[i];
                }
            }

            // (FX65) Fills V0 thru VX with RAM values starting at address in I
            //        Inclusive
            (0xF, _, 6, 5) => {
                let x = digit2 as usize;
                let i_reg_value = self.i_reg as usize;

                for i in 0..=x {
                    self.v_reg[i] = self.ram[i_reg_value + i];
                }
            }

            // TODO behavior for invalid opcode? interpreter will only reach
            //      the bottom catch-all pattern if there is a bug in the ROM
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", opcode),
        }
    }

    fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.pc as usize] as u16;
        let lower_byte = self.ram[(self.pc + 1) as usize] as u16;
        let opcode = (higher_byte << 8) | lower_byte;
        self.pc += 2;
        opcode
    }

    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }

        self.sound = false;
        if self.st > 0 {
            if self.st == 1 {
                // make BEEP sound
                self.sound = true;
            }
            self.st -= 1;
        }
    }
}
