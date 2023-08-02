use std::cmp::Ordering;

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
            println!("\treg{idx}:    {:#010} ({:#034b}) ({:#010x})", i, i, i);
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

    fn modded_instr(&mut self, which: u8, mode: u8) {
        let target = self.next_byte();

        assert!(target < 4, "TARGET more than allowed");

        let source: u32 = match mode {
            RR_MODE => {
                let other_register = self.next_byte();

                assert!(other_register < 4, "REGISTER_ID more than allowed");
                self.regs[other_register as usize]
            }
            RB_MODE => {
                let byte = self.next_byte();

                byte as u32
            }
            RW_MODE => {
                let word = self.next_word();

                word as u32
            }
            RD_MODE => self.next_dword(),
            _ => todo!(),
        };

        match which {
            AND => {
                self.regs[target as usize] &= source;
            }
            NAND => {
                self.regs[target as usize] = !(self.regs[target as usize] & source);
            }
            OR => {
                self.regs[target as usize] |= source;
            }
            NOR => {
                self.regs[target as usize] = !(self.regs[target as usize] | source);
            }
            XOR => {
                self.regs[target as usize] ^= source;
            }
            XNOR => {
                self.regs[target as usize] = !(self.regs[target as usize] ^ source);
            }
            MOV => {
                self.regs[target as usize] = source;
            }
            ADD => {
                self.regs[target as usize] += source;
            }
            SUB => {
                self.regs[target as usize] -= source;
            }
            CMP => {
                // for jmz and jlz we will be using 2 and 1 respectively
                self.regs[0] = match self.regs[target as usize].cmp(&source) {
                    Ordering::Less => 1,
                    Ordering::Equal => 0,
                    Ordering::Greater => 2,
                };
            }
            _ => todo!(),
        }
    }

    fn execute(&mut self) {
        while !self.halted {
            if self.counter as usize >= self.code.len() {
                return;
            }

            let byte = self.code[self.counter as usize];

            match byte {
                JMP => {}
                JZ => {}
                JLZ => {}
                JMZ => {}
                LDP => {}
                STP => {}
                AND | NAND | OR | NOR | XOR | XNOR | MOV | ADD | SUB | CMP => {
                    let mode = self.next_byte();

                    self.modded_instr(byte, mode);
                }
                NOT => {
                    let reg = self.next_byte();
                    assert!(reg < 4, "TARGET more than allowed");

                    self.regs[reg as usize] = !self.regs[reg as usize];
                }
                HLT => {}
                NOP => {}
                INT => {}
                _ => todo!(),
            }
            self.step();
        }
    }
}

fn main() {
    let thing: Vec<u8> = vec![MOV, RB_MODE, 0, 12, MOV, RB_MODE, 1, 11, CMP, RR_MODE, 1, 0];

    let mut p1 = Program::new(thing);
    p1.execute();
}
