use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::env;
use std::path::Path;

const MEM_SIZE: usize = 512;

const ADD_FUNC: u32 = 0b000_0010_0000;
const ADDI_OPCODE: u32 = 0b00_1000;
const AND_FUNC: u32 = 0b000_0010_0100;
const BEQ_OPCODE: u32 = 0b00_0100;
const BNE_OPCODE: u32 = 0b00_0101;
const DIV_FUNC: u32 = 0b0000_0000_0001_1010;
const J_OPCODE: u32 = 0b00_0010;
const LW_OPCODE: u32 = 0b1000_11;
const MULT_FUNC: u32 = 0b0000_0000_0001_1000;
const OR_FUNC: u32 = 0b000_0010_0101;
const SUB_FUNC: u32 = 0b000_0010_0010;
const SW_OPCODE: u32 = 0b10_1011;
const XOR_FUNC: u32 = 0b000_0010_0110;

fn main() {
    println!("Hello, world!");

    let args: Vec<String> = env::args().collect();

    let file = File::open(&args[1]).unwrap();

    let buf = BufReader::new(file);
    let assembly: Vec<String> = buf.lines().map(|l| l.expect("Could not parse line")).collect();

    let instructions = assemble(assembly);
    for i in instructions.iter() {
        println!("{:032b}", i);
    }
    //let instructions: Vec<u32> = vec![0b0000_0000_10_0001_0000_1000_0010_0000, 0, 0];
    let mut memory: [u32; MEM_SIZE] = [0; MEM_SIZE];

    let mut regs = Registers{ gprs: [0; 32], pc: 0, npc: 0};

    loop {
        let i = fetch(&instructions);
        let i_decoded = decode(i, regs);
        println!("{:?}", i_decoded);
        if let Some((reg, res)) = execute(i_decoded, &mut memory) {
            writeback(reg, res, &mut regs);
        }
        println!("{:?}", regs);
        break;
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
    Add(usize, u32, u32),
    Sub(usize, u32, u32),
    Mult(usize, u32, u32),
    Div(usize, u32, u32),
    Or(usize, u32, u32),
    Xor(usize, u32, u32),
    And(usize, u32, u32),
    Sw(u32, usize, usize),
    Lw(usize, usize, usize),
}

fn fetch(instructions: &Vec<u32>) -> u32 {
    0
}

fn decode(i: u32, registers: Registers) -> Instruction {
    println!("instruction: {}", i);
    Instruction::Nop
}

fn execute(i: Instruction, memory: &mut [u32; MEM_SIZE]) -> Option<(usize, u32)> {
    match i {
        Instruction::Add(r, x, y) => Some((r, x + y)),
        Instruction::Sub(r, x ,y) => Some((r, x - y)),
        Instruction::Mult(r, x, y) => Some((r, x * y)),
        Instruction::Div(r, x, y) => Some((r, x / y)),
        Instruction::Or(r, x, y) => Some((r, x | y)),
        Instruction::Xor(r, x, y) => Some((r, x ^ y)),
        Instruction::And(r, x, y) => Some((r, x & y)),
        Instruction::Lw(r, s, offset) => Some((r, memory[s + offset])),
        Instruction::Sw(t, s, offset) => {
            memory[s + offset] = t;
            None
        }
        Instruction::Nop => None
    }
}

fn writeback(register: usize, result: u32, regs: &mut Registers ) {
    regs.gprs[register] = result;
}

fn assemble(assembly: Vec<String>) -> Vec<u32> {
    let mut instructions: Vec<u32> = Vec::new();

    for line in assembly {
        let split_inst: Vec<&str> = line.split_whitespace().collect();
        match split_inst[0] {
            "ADD" => {
                //0000 00ss ssst tttt dddd d000 0010 0000
                let instruction = opcode_zero(split_inst, ADD_FUNC);
                instructions.push(instruction);
            }
            "ADDI" => { //ADDI
                //0010 00ss ssst tttt iiii iiii iiii iiii
                let instruction = two_regs_imm(split_inst, ADDI_OPCODE);
                instructions.push(instruction);
            }
            "AND" => { 
                //0000 00ss ssst tttt dddd d000 0010 0100
                let instruction = opcode_zero(split_inst, AND_FUNC);
                instructions.push(instruction);
            }
            "BEQ" => {
                //0001 00ss ssst tttt iiii iiii iiii iiii
                let instruction = two_regs_imm(split_inst, BEQ_OPCODE);
                instructions.push(instruction);
            }
            "BNE" => {
                //0001 01ss ssst tttt iiii iiii iiii iiii
                let instruction = two_regs_imm(split_inst, BNE_OPCODE);
                instructions.push(instruction);
            }
            "DIV" => {
                //0000 00ss ssst tttt 0000 0000 0001 1010
                let s = split_inst[1].parse::<u32>().unwrap();
                let t = split_inst[2].parse::<u32>().unwrap();
                let mut instruction: u32 = DIV_FUNC;
                instruction |= t << 16;
                instruction |= s << 21;
                instructions.push(instruction);
            }
            "J" => {
                //0000 10ii iiii iiii iiii iiii iiii iiii
                let imm = split_inst[1].parse::<u32>().unwrap();
                let mut instruction: u32 = J_OPCODE << 26;
                instruction |= imm;
                instructions.push(instruction);
            }
            "LW" => { // LW
                //1000 11ss ssst tttt iiii iiii iiii iiii
                let instruction = two_regs_imm(split_inst, LW_OPCODE);
                instructions.push(instruction);
            }
            "MULT" => {
                //0000 00ss ssst tttt 0000 0000 0001 1000
                let s = split_inst[1].parse::<u32>().unwrap();
                let t = split_inst[2].parse::<u32>().unwrap();
                let mut instruction = MULT_FUNC;
                instruction |= t << 16;
                instruction |= s << 21;
                instructions.push(instruction);
            }
            "NOOP" => {
                instructions.push(0);
            }
            "OR" => {
                //0000 00ss ssst tttt dddd d000 0010 0101
                let instruction = opcode_zero(split_inst, OR_FUNC);
                instructions.push(instruction);
            }
            "SUB" => {
                //0000 00ss ssst tttt dddd d000 0010 0010
                let instruction = opcode_zero(split_inst, SUB_FUNC);
                instructions.push(instruction);
            }
            "SW" => { // SW
                //1010 11ss ssst tttt iiii iiii iiii iiii
                let instruction = two_regs_imm(split_inst, SW_OPCODE);
                instructions.push(instruction);       
            }
            "XOR" => {
                //0000 00ss ssst tttt dddd d--- --10 0110
                let instruction = opcode_zero(split_inst, XOR_FUNC);
                instructions.push(instruction);
            }
            _ => {
                panic!("Unimplemented opcode {}", split_inst[0]);
            }
        };
    }
    return instructions;
}

fn opcode_zero(inst: Vec<&str>, func: u32) -> u32 {
    let d = inst[1].parse::<u32>().unwrap();
    let s = inst[2].parse::<u32>().unwrap();
    let t = inst[3].parse::<u32>().unwrap();
    let mut instruction: u32 = 0;
    instruction |= func;
    instruction |= d << 11;
    instruction |= t << 16;
    instruction |= s << 21;
    return instruction;
}

fn two_regs_imm(split_inst: Vec<&str>, opcode: u32) -> u32{
    let t = split_inst[1].parse::<u32>().unwrap();
    let s = split_inst[2].parse::<u32>().unwrap();
    let imm = split_inst[3].parse::<u32>().unwrap();
    let mut instruction = opcode << 26;
    instruction |= imm;
    instruction |= t << 16;
    instruction |= s << 21;
    return instruction;
}