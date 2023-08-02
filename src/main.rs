const ADD: u8 = 0x01;
const SUB: u8 = 0x02;
const JMP: u8 = 0x03;
const JZ: u8 = 0x04;
const JLZ: u8 = 0x05;
const JMZ: u8 = 0x06;
const MOV: u8 = 0x07;
const LDP: u8 = 0x08;
const STP: u8 = 0x09;
const AND: u8 = 0x0A;
const NOT: u8 = 0x0B;
const OR: u8 = 0x0C;
const NOR: u8 = 0x0D;
const NAND: u8 = 0x0E;
const XOR: u8 = 0x0F;
const XNOR: u8 = 0x10;
const HLT: u8 = 0x11;
const NOP: u8 = 0x12;
const INT: u8 = 0x13;
const CMP: u8 = 0x14;

const RR_MODE: u8 = 0x01;
const RB_MODE: u8 = 0x02;
const RW_MODE: u8 = 0x03;
const RD_MODE: u8 = 0x04;

struct Program {
    regs: [u32; 4],
    code: Vec<u8>,
    counter: u32,
    halted: bool,
}

impl Program {
    fn new(code: Vec<u8>) -> Self {
        Program {
            regs: [0u32; 4],
            code,
            counter: 0,
            halted: false,
        }
    }

    fn step(&mut self) {
        println!("Advancing to next byte\n\tcounter: {}", self.counter);

        for (idx, i) in self.regs.into_iter().enumerate() {
            println!("\treg{idx}:    {}", i);
        }

        self.counter += 1;
    }

    fn next_byte(&mut self) -> u8 {
        // dbg!(self.code[self.counter as usize]);
        self.step();

        if self.counter as usize >= self.code.len() {
            std::process::exit(0);
        }

        self.code[self.counter as usize]
    }

    fn next_word(&mut self) -> u16 {
        let first = self.next_byte();
        let second = self.next_byte();

        let array: [u8; 2] = [second, first];

        ((array[0] as u16) << 8) | array[1] as u16
    }

    fn next_dword(&mut self) -> u32 {
        let first = self.next_word();
        let second = self.next_word();

        let array: [u16; 2] = [second, first];

        ((array[0] as u32) << 16) | array[1] as u32
    }

    fn execute(&mut self) {
        while !self.halted {
            if self.counter as usize >= self.code.len() {
                return;
            }

            let byte = self.code[self.counter as usize];

            match byte {
                ADD => {}
                SUB => {}
                JMP => {}
                JZ => {}
                JLZ => {}
                JMZ => {}
                MOV => {
                    let mode = self.next_byte();
                    let register_id = self.next_byte();
                    assert!(register_id < 4, "REGISTER_ID more than allowed");

                    match mode {
                        RR_MODE => {
                            let other_register = self.next_byte();

                            assert!(register_id < 4, "REGISTER_ID more than allowed");
                            self.regs[register_id as usize] = self.regs[other_register as usize];
                            println!(
                                "Set register-{register_id} to value of register-{other_register}"
                            );
                        }
                        RB_MODE => {
                            let byte = self.next_byte();

                            self.regs[register_id as usize] = byte as u32;

                            println!(
                                "Set register-{register_id} to the casted value of byte {byte}"
                            );
                        }
                        RW_MODE => {
                            let word = self.next_word();

                            self.regs[register_id as usize] = word as u32;

                            println!(
                                "Set register-{register_id} to the casted value of wword {word}"
                            );
                        }
                        RD_MODE => {
                            let dword = self.next_dword();

                            self.regs[register_id as usize] = dword;

                            println!(
                                "Set register-{register_id} to the casted value of dword {dword}"
                            );
                        }
                        _ => todo!(),
                    }
                }
                LDP => {}
                STP => {}
                AND => {}
                NOT => {}
                OR => {}
                NOR => {}
                NAND => {}
                XOR => {}
                XNOR => {}
                HLT => {}
                NOP => {}
                INT => {}
                CMP => {}
                _ => todo!(),
            }
            self.step();
        }
    }
}

fn main() {
    let thing: Vec<u8> = vec![7, 4, 1, 0x01, 0x01, 0x00, 0x00, 7, 1, 0, 1];
    let mut p1 = Program::new(thing);
    p1.execute();
}
