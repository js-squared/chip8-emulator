pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const START_ADDRESS: u16 = 0x200;

const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;

const DIGIT_SPRITES_SIZE: usize = 16 * 5; // 16 characters of 5 bytes each
const DIGIT_SPRITES: [u8; DIGIT_SPRITES_SIZE] = [
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
    0xF0, 0x80, 0xF0, 0x80, 0x80 // F
];

// TODO add flags for when sound should play and for runtime errors caused
//      by bugs in the input ROM (should be similar to how screen is used)
pub struct Processor {
    pc: u16, // program counter
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
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
            v_reg: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            dt: 0,
            st: 0,
        };
        new_processor.ram[..DIGIT_SPRITES_SIZE]
                     .copy_from_slice(&DIGIT_SPRITES);
        new_processor
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDRESS;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_reg = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.dt = 0;
        self.st = 0;
        self.ram[..DIGIT_SPRITES_SIZE]
            .copy_from_slice(&DIGIT_SPRITES);
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

    fn execute(&mut self, opcode: u16) {
        let digit1 = (opcode & 0xF000) >> (3*4);
        let digit2 = (opcode & 0x0F00) >> (2*4);
        let digit3 = (opcode & 0x00F0) >> 4;
        let digit4 = opcode & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            // Nop
            (0, 0, 0, 0) => return,

            // Clear screen
            (0, 0, 0xE, 0) => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            },

            // Return from subroutine
            (0, 0, 0xE, 0xE) => {
                self.pc = self.pop();
            },

            // (1NNN) Jump to address 0xNNN
            (1, _, _, _) => {
                self.pc = opcode & 0xFFF;
            },

            // (2NNN) Call 0xNNN
            //        Enter subroutine at 0xNNN, adding current PC to stack
            //        so we can return here
            (2, _, _, _) => {
                self.push(self.pc);
                self.pc = opcode & 0xFFF;
            },

            // (3XNN) Skip if VX == 0xNN
            (3, _, _, _) => {
                let x = digit2 as usize;
                let nn = (opcode & 0xFF) as u8;
                if self.v_reg[x] == nn {
                    self.pc += 2;
                }
            },

            // (4XNN) Skip if VX != 0xNN
            (4, _, _, _) => {
                let x = digit2 as usize;
                let nn = (opcode & 0xFF) as u8;
                if self.v_reg[x] != nn {
                    self.pc += 2
                }
            },

            // (5XY0) Skip if VX == VY
            (5, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                if self.v_reg[x] == self.v_reg[y] {
                    self.pc += 2
                }
            },

            // (6XNN) VX = 0xNN
            (6, _, _, _) => {
                let x = digit2 as usize;
                let nn = (opcode & 0xFF) as u8;
                self.v_reg[x] = nn;
            },

            // (7XNN) VX += 0xNN
            //        Doesn't affect carry flag
            (7, _, _, _) => {
                let x = digit2 as usize;
                let nn = (opcode & 0xFF) as u8;
                self.v_reg[x] = self.v_reg[x].wrapping_add(nn);
            },

            // (8XY0) VX = VY
            (8, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] = self.v_reg[y];
            },

            // (8XY1) VX |= VY
            (8, _, _, 1) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] |= self.v_reg[y];
            },

            // (8XY2) VX &= VY
            (8, _, _, 2) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] &= self.v_reg[y];
            },

            // (8XY3) VX ^= VY
            (8, _, _, 3) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] ^= self.v_reg[y];
            },

            // (8XY4) VX += VY
            //        Sets VF if carry
            (8, _, _, 4) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (new_vx, carry) = self.v_reg[x]
                                          .overflowing_add(self.v_reg[y]);
                let new_vf = if carry { 1 } else { 0 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            },

            // (8XY5) VX -= VY
            //        Clears VF if borrow
            (8, _, _, 5) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (new_vx, carry) = self.v_reg[x]
                                          .overflowing_sub(self.v_reg[y]);
                let new_vf = if carry { 0 } else { 1 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            },

            // (8XY6) VX >>= 1
            //        Stores dropped bit in VF
            (8, _, _, 6) => {
                let x = digit2 as usize;

                let dropped_bit = self.v_reg[x] & 1;

                self.v_reg[x] >>= 1;
                self.v_reg[0xF] = dropped_bit;
            },

            // (8XY7) VX = VY - VX
            //        Clears VF if borrow
            (8, _, _, 7) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (new_vx, carry) = self.v_reg[y]
                                          .overflowing_sub(self.v_reg[x]);
                let new_vf = if carry { 1 } else { 0 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            },

            // (8XYE) VX <<= VY
            //        Store dropped bit in VF
            (8, _, _, 0xE) => {
                let x = digit2 as usize;

                let dropped_bit = (self.v_reg[x] >> 7) & 1;

                self.v_reg[x] <<= 1;
                self.v_reg[0xF] = dropped_bit;
            },

            // (9XY0) Skip if VX != VY
            (9, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                if self.v_reg[x] != self.v_reg[y] {
                    self.pc += 2
                }
            },

            // (ANNN) I = 0xNNN
            (0xA, _, _, _) => {
                let nnn = opcode & 0xFFF;

                self.i_reg = nnn;
            },

            // (BNNN) Jump to V0 + 0xNNN
            (0xB, _, _, _) => {
                let nnn = opcode & 0xFFF;

                self.pc = (self.v_reg[0] as u16) + nnn;
            },

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

        if self.st > 0 {
            if self.st == 1 {
                // TODO make BEEP sound
            }
            self.st -= 1;
        }
    }
}
