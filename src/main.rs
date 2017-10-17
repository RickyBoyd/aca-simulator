use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::env;

const MEM_SIZE: usize = 512;

fn main() {
    println!("Hello, world!");

    let args: Vec<String> = env::args().collect();

    let file = File::open(&args[1]).unwrap();

    let buf = BufReader::new(file);
    let assembly: Vec<String> = buf.lines().map(|l| l.expect("Could not parse line")).collect();

    let instructions = assemble(assembly);

    for i in instructions.iter() {
        println!("{:?}", i);
    }

    let mut memory: [u32; MEM_SIZE] = [0; MEM_SIZE];

    let mut regs = Registers{ gprs: [0; 32], pc: 0, npc: 0};

    let mut pc: usize = 0;

    loop {
        if let Some(instruction) = fetch(&instructions, &mut pc){
            let i_decoded = decode(instruction, regs);
            println!("{:?}", i_decoded);
            if let Some((reg, res)) = execute(i_decoded, &mut memory, &mut pc) {
                writeback(reg, res, &mut regs);
            }            
        } 
        else {
            break;
        }
        println!("{:?}", regs);
    }
    println!("End: {:?}", regs);
}

#[derive(Clone, Copy, Debug)]
struct Registers {
    gprs: [u32; 32], // 32 GPRS
    pc: u32,
    npc: u32,
    //more regs
}

#[derive(Clone, Copy, Debug)]
enum EncodedInstruction {
    Noop,
    Add(usize, usize, usize),
    Addi(usize, usize, u32),
    Beq(usize, usize, usize),
    Bne(usize, usize, usize),
    J(usize),
    Sub(usize, usize, usize),
    Mult(usize, usize, usize),
    Div(usize, usize, usize),
    Or(usize, usize, usize),
    Xor(usize, usize, usize),
    And(usize, usize, usize),
    Sw(usize, usize),
    Lw(usize, usize),
}

#[derive(Clone, Copy, Debug)]
enum DecodedInstruction {
    Noop,
    Add(usize, u32, u32),
    Beq(u32, u32, usize),
    Bne(u32, u32, usize),
    J(usize),
    Sub(usize, u32, u32),
    Mult(usize, u32, u32),
    Div(usize, u32, u32),
    Or(usize, u32, u32),
    Xor(usize, u32, u32),
    And(usize, u32, u32),
    Sw(u32, u32),
    Lw(usize, u32),
}


fn fetch(instructions: &Vec<EncodedInstruction>, pc: &mut usize) -> Option<EncodedInstruction> {

    if (*pc) >= instructions.len() {
        return None;
    }
    let inst = instructions[*pc];
    *pc = *pc + 1;
    Some(inst)
}

fn decode(instruction: EncodedInstruction, registers: Registers) -> DecodedInstruction {
    println!("instruction: {:?}", instruction);

    match instruction {
        EncodedInstruction::Noop            => DecodedInstruction::Noop,
        EncodedInstruction::Addi(d, s, imm) => DecodedInstruction::Add(d, registers.gprs[s], imm),  
        EncodedInstruction::Add(d, s, t)    => DecodedInstruction::Add(d, registers.gprs[s], registers.gprs[t]),
        EncodedInstruction::Beq(s, t, inst) => DecodedInstruction::Beq(registers.gprs[s], registers.gprs[t], inst),
        EncodedInstruction::Bne(s, t, inst) => DecodedInstruction::Bne(registers.gprs[s], registers.gprs[t], inst),
        EncodedInstruction::J(inst)         => DecodedInstruction::J(inst),
        EncodedInstruction::Sub(d, s, t)    => DecodedInstruction::Sub(d, registers.gprs[s], registers.gprs[t]),
        EncodedInstruction::Mult(d, s, t)   => DecodedInstruction::Mult(d, registers.gprs[s], registers.gprs[t]),
        EncodedInstruction::Div(d, s, t)    => DecodedInstruction::Div(d, registers.gprs[s], registers.gprs[t]),
        EncodedInstruction::Or(d, s, t)     => DecodedInstruction::Or(d, registers.gprs[s], registers.gprs[t]),
        EncodedInstruction::Xor(d, s, t)    => DecodedInstruction::Xor(d, registers.gprs[s], registers.gprs[t]),
        EncodedInstruction::And(d, s, t)    => DecodedInstruction::And(d, registers.gprs[s], registers.gprs[t]),
        EncodedInstruction::Sw(t, s)        => DecodedInstruction::Sw(registers.gprs[t], registers.gprs[s]),
        EncodedInstruction::Lw(d, t)        => DecodedInstruction::Lw(d, registers.gprs[t]),
        // _ => {
        //     panic!("{:?} is an unimplemented instruction", instruction);
        //     EncodedInstruction::Noop
        // }
    }
}

fn execute(i: DecodedInstruction, memory: &mut [u32; MEM_SIZE], pc: &mut usize) -> Option<(usize, u32)> {
    match i {
        DecodedInstruction::Add(r, x, y) => Some((r, x + y)),
        DecodedInstruction::Sub(r, x ,y) => Some((r, x - y)),
        DecodedInstruction::Mult(r, x, y) => Some((r, x * y)),
        DecodedInstruction::Div(r, x, y) => Some((r, x / y)),
        DecodedInstruction::Or(r, x, y) => Some((r, x | y)),
        DecodedInstruction::Xor(r, x, y) => Some((r, x ^ y)),
        DecodedInstruction::And(r, x, y) => Some((r, x & y)),
        DecodedInstruction::Lw(r, s) => Some((r, memory[s as usize])),
        DecodedInstruction::Sw(t, s) => {
            memory[s as usize] = t;
            None
        }
        DecodedInstruction::J(inst) => {
            *pc = inst;
            None
        }
        DecodedInstruction::Bne(s, t, inst) => {
            if s != t {
                *pc = inst
            }
            None
        },
        DecodedInstruction::Beq(s, t, inst) => {
            if s == t {
                *pc = inst
            }
            None
        }
        DecodedInstruction::Noop => None
    }
}

fn writeback(register: usize, result: u32, regs: &mut Registers ) {
    regs.gprs[register] = result;
}

fn assemble(assembly: Vec<String>) -> Vec<EncodedInstruction> {
    let mut instructions: Vec<EncodedInstruction> = Vec::new();

    for line in assembly {
        let split_inst: Vec<&str> = line.split_whitespace().collect();
        match split_inst[0] {
            "ADD" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Add(d, s, t));
            }
            "ADDI" => { //ADDI
                let (d, s, imm) = three_args(split_inst);
                instructions.push(EncodedInstruction::Addi(d, s, imm as u32));
            }
            "AND" => { 
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::And(d, s, t));
            }
            "BEQ" => {
                let (s, t, addr) = three_args(split_inst);
                instructions.push(EncodedInstruction::Beq(s, t, addr));
            }
            "BNE" => {
                let (s, t, addr) = three_args(split_inst);
                instructions.push(EncodedInstruction::Bne(s, t, addr));
            }
            "DIV" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Div(d, s, t));
            }
            "J" => {
                let imm = split_inst[1].parse::<usize>().unwrap();
                instructions.push(EncodedInstruction::J(imm));
            }
            "LW" => { // LW
                let (s, t) = two_args(split_inst);
                instructions.push(EncodedInstruction::Lw(s, t));
            }
            "MULT" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Mult(d, s, t));
            }
            "NOOP" => {
                instructions.push(EncodedInstruction::Noop);
            }
            "OR" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Or(d, s, t));
            }
            "SUB" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Sub(d, s, t));
            }
            "SW" => {
                let (d, s) = two_args(split_inst);
                instructions.push(EncodedInstruction::Sw(d, s));       
            }
            "XOR" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Xor(d, s, t));
            }
            _ => {
                panic!("Unimplemented opcode {}", split_inst[0]);
            }
        };
    }
    return instructions;
}

fn three_args(inst: Vec<&str>) -> (usize, usize, usize) {
    let d = inst[1].parse::<usize>().unwrap();
    let s = inst[2].parse::<usize>().unwrap();
    let t = inst[3].parse::<usize>().unwrap();
    (d, s, t)
}

fn two_args(split_inst: Vec<&str>) -> (usize, usize) {
    let t = split_inst[1].parse::<usize>().unwrap();
    let s = split_inst[2].parse::<usize>().unwrap();
    (t, s)
}