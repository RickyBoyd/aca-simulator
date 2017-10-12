const  MEM_SIZE: usize = 512;


fn main() {
    println!("Hello, world!");

    //let instructions = read_program(args[0]);

    let instructions: Vec<u32> = vec![0b0000_0000_10_0001_0000_1000_0010_0000, 0, 0];
    let mut memory: [u32; MEM_SIZE] = [0; MEM_SIZE];

    let mut regs = Registers{ gprs: [1; 32], pc: 0, npc: 0};

    loop {
        let i = fetch(&instructions);
        let i_decoded = decode(i, regs);
        println!("{:?}", i_decoded);
        if let Some((reg, res)) = execute(i_decoded, &mut memory) {
            writeback(reg, res, &mut regs);
        }
        println!("{:?}", regs);
    }
}

#[derive(Clone, Copy, Debug)]
struct Registers {
    gprs: [u32; 32], // 32 GPRS
    pc: u32,
    npc: u32,
    //more regs
}

#[derive(Debug)]
enum Instruction {
    Nop,
    Add(u8, u32, u32),
    Sub(u8, u32, u32),
    Mult(u8, u32, u32),
    Div(u8, u32, u32),
    Or(u8, u32, u32),
    Xor(u8, u32, u32),
    And(u8, u32, u32),
    Sw(u32, u32, u32),
    Lw(u8, u32, u32),
}

// impl regs {
//     fn advance_pc(&self) {
    
//     }
// }

// fn read_program(filepath: &str) -> Vec<u32> {
//     //read in binary file 32 bits at a time into a vector
// }

fn fetch(instructions: &Vec<u32>) -> u32 {
    0b0000_0000_10_0001_0000_1000_0010_0000
}

fn decode(i: u32, registers: Registers) -> Instruction {
    let opcode = (i >> 26) as u8;
    println!("instruction: {:032b}", i);
    println!("opcode: {:b}", opcode);
    match opcode {
        0b0000_0000 => {
            let func = (i & 0b11_1111) as u8;
            let shamt = ((i >> 6) & 0b1_1111) as u8;
            let rd = ((i >> 11) & 0b1_1111) as u8;
            let rt = registers.gprs[((i >> 16) & 0b1_1111) as usize];
            let rs = registers.gprs[((i >> 21) & 0b1_1111) as usize];
            println!("func: {:b}", func);
            match func {
                0b10_0000 => Instruction::Add(rd, rs, rt),
                0b10_0010 => Instruction::Sub(rd, rs, rt),
                0b10_0101 => Instruction::Or(rd, rs, rt),
                0b10_0110 => Instruction::Xor(rd, rs, rt),
                0b10_0100 => Instruction::And(rd, rs, rt),
                _ => {
                    panic!("Unimplemented func {:b}", func); 
                    Instruction::Nop
                }
            }
        }
        0b0000_1000 => { //ADDI
            let imm = i & 0xFFFF;
            let t = ((i >> 16) & 0b11111) as u8;
            let s = ((i >> 21) & 0b11111) as usize;
            Instruction::Add(t, registers.gprs[s], imm)
        }
        0b0010_0000 => { // LB
            Instruction::Nop
        }
        0b0010_0011 => { // LW
            Instruction::Nop
        }
        0b0010_1000 => { // SB
            Instruction::Nop
        }
        0b0010_1011 => { // SW
            Instruction::Nop
        }
        _ => {
            panic!("Unimplemented opcode {:b}", opcode);
            Instruction::Nop
        }
    }
}

fn execute(i: Instruction, memory: &mut [u32; MEM_SIZE]) -> Option<(u8, u32)> {
    match i {
        Instruction::Add(r, x, y) => Some((r, x + y)),
        Instruction::Sub(r, x ,y) => Some((r, x - y)),
        Instruction::Mult(r, x, y) => Some((r, x * y)),
        Instruction::Div(r, x, y) => Some((r, x / y)),
        Instruction::Or(r, x, y) => Some((r, x | y)),
        Instruction::Xor(r, x, y) => Some((r, x ^ y)),
        Instruction::And(r, x, y) => Some((r, x & y)),
        Instruction::Lw(r, s, offset) => Some((r, memory[(s + offset) as usize])),
        Instruction::Sw(t, s, offset) => {
            memory[(s + offset) as usize] = t;
            None
        }
        Instruction::Nop => None
    }
}

fn writeback(register: u8, result: u32, regs: &mut Registers ) {
    regs.gprs[register as usize] = result;
}
