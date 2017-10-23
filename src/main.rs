use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::env;
use std::thread;
use std::sync::mpsc::channel;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::Receiver;


const MEM_SIZE: usize = 52;

fn main() {
    println!("Hello, world!");

    let args: Vec<String> = env::args().collect();

    let file = File::open(&args[1]).unwrap();

    let buf = BufReader::new(file);
    let assembly: Vec<String> = buf.lines().map(|l| l.expect("Could not parse line")).collect();

    let instructions = assemble(assembly);

    let num_instructions = instructions.len();

    // for i in instructions.iter() {
    //     println!("{:?}", i);
    // }

    let mut memory: [u32; MEM_SIZE] = [1; MEM_SIZE];

    let mut regs = Registers{ gprs: [0; 32]};

    //need a clock channel
    let (tick_sender_f, tick_recv_f) = sync_channel::<i32>(0);
    let (tick_sender_d, tick_recv_d) = sync_channel::<i32>(0);
    let (tick_sender_e, tick_recv_e) = sync_channel::<i32>(0);
    let (tick_sender_w, tick_recv_w) = sync_channel::<i32>(0);

    //Send pc from execute to fetch + control value with Optional None -> 0
    let (pc_sender_e, pc_recv_f) = sync_channel::<Option<usize>>(1);

    //need a channel from fetch -> decode
    let (fetch_sender, fetch_recv) = sync_channel::<EncodedInstruction>(1);

    //need a channel from decode -> execute
    let (decode_sender, decode_recv) = sync_channel::<DecodedInstruction>(1);

    //need a channel from execute -> wb
    let (wb_sender, wb_recv) = sync_channel::<ExecuteResult>(1);

    let (reg_wb_sender, reg_wb_recv) = sync_channel::<ExecuteResult>(1);

    //Need to put initial values into inter stage channels plus signal channels
    reg_wb_sender.send(ExecuteResult::None).unwrap();
    pc_sender_e.send(None).unwrap();
    fetch_sender.send(EncodedInstruction::Noop).unwrap();
    decode_sender.send(DecodedInstruction::Noop).unwrap();
    wb_sender.send(ExecuteResult::None).unwrap();

    thread::Builder::new().name("fetch".to_string()).spawn(move || {
        fetch(instructions, tick_sender_f, pc_recv_f, fetch_sender);
    });
    thread::Builder::new().name("decode".to_string()).spawn(move || {
         decode(&mut regs, tick_sender_d, fetch_recv, decode_sender, reg_wb_recv);
    });
    thread::Builder::new().name("execute".to_string()).spawn(move || {
         execute(&mut memory, tick_sender_e, pc_sender_e, decode_recv, wb_sender);
    });
    thread::Builder::new().name("writeback".to_string()).spawn(|| {
        writeback(tick_sender_w, wb_recv, reg_wb_sender);
    });

    let mut cycles = 0;
    loop {
        println!("MAIN 1");
        let f = tick_recv_f.recv().unwrap();
        println!("MAIN 2");
        let d = tick_recv_d.recv().unwrap();
        println!("MAIN 3");
        let e = tick_recv_e.recv().unwrap();
        println!("MAIN 4");
        let w = tick_recv_w.recv().unwrap();
        println!("tick");
        if f == 0 {
            break;
        }
    }
    let d = tick_recv_d.recv().unwrap();
    let e = tick_recv_e.recv().unwrap();
    let w = tick_recv_w.recv().unwrap();
    let e = tick_recv_e.recv().unwrap();
    let w = tick_recv_w.recv().unwrap();
    let w = tick_recv_w.recv().unwrap();

    cycles += 3;

    println!("Instructions executed: {}", num_instructions);
    println!("Number of cycles: {}", cycles);
    println!("Instructions per cycle: {}", cycles as f32  / num_instructions as f32);
    // loop {
    //     if let Some(instruction) = fetch(&instructions, &mut pc){
    //         let i_decoded = decode(instruction, regs);
    //         if let Some((reg, res)) = execute(i_decoded, &mut memory, &mut pc) {
    //             println!("WB reg: {} res: {}", reg, res);
    //             writeback(reg, res, &mut regs);
    //         }            
    //     } 
    //     else {
    //         break;
    //     }
    //     println!("{:?}", regs);
    // }
    //println!("End: {:?}", regs);
    //for i in 0..30 {
    //    println!("MEM[{}]: {}", i, memory[i]);
    //}
    
}

#[derive(Clone, Copy, Debug)]
struct Registers {
    gprs: [u32; 32], // 32 GPRS
}

#[derive(Clone, Copy, Debug)]
enum EncodedInstruction {
    Noop,
    Halt,
    Add(usize, usize, usize),
    Addi(usize, usize, u32),
    And(usize, usize, usize),
    Andi(usize, usize, u32),
    Beq(usize, usize, usize),
    Blt(usize, usize, usize),
    Div(usize, usize, usize),
    J(usize),
    Ldc(usize, u32),
    Li(usize, u32),
    Lw(usize, usize),
    Mod(usize, usize, usize),
    Mov(usize, usize),
    Mult(usize, usize, usize),
    Or(usize, usize, usize),
    Si(usize, u32),
    Sl(usize, usize, u32),
    Sr(usize, usize, u32),
    Sw(usize, usize),
    Sub(usize, usize, usize),
    Subi(usize, usize, u32),
    Xor(usize, usize, usize),
}

#[derive(Clone, Copy, Debug)]
enum DecodedInstruction {
    Noop,
    Halt,
    Add(usize, u32, u32),
    And(usize, u32, u32),
    Beq(u32, u32, usize),
    Blt(u32, u32, usize),
    Div(usize, u32, u32),
    J(usize),
    Load(usize, u32),
    Mod(usize, u32, u32),
    Mov(usize, u32),
    Mult(usize, u32, u32),
    Or(usize, u32, u32),
    Sl(usize, u32, u32),
    Sr(usize, u32, u32),
    Sub(usize, u32, u32),
    Store(u32, u32),
    Xor(usize, u32, u32),
}

#[derive(Clone, Copy, Debug)]
enum ExecuteResult {
    Halt,
    None,
    Writeback(usize, u32),
}


fn fetch(instructions: Vec<EncodedInstruction>, clock: SyncSender<i32>, pc_receiver: Receiver<Option<usize>>, 
         fetch_sender: SyncSender<EncodedInstruction>) {
    let mut pc = 0;
    loop {
        if pc >= instructions.len() {
            println!("Need to finish here!");
            fetch_sender.send(EncodedInstruction::Halt);
            clock.send(0).unwrap();
            break;
        }
        let inst = instructions[pc];
        match pc_receiver.recv().unwrap() {
            Some(addr) => pc = addr,
            None => pc += 1,
        };
        //let instruction = Some(inst);
        println!("pc: {}", pc);
        println!("Fetched instruction: {:?}", inst);
        fetch_sender.send(inst).unwrap();
        clock.send(1).unwrap();
    }
    pc_receiver.recv().unwrap();
    pc_receiver.recv().unwrap();
    pc_receiver.recv().unwrap();
}

fn decode(registers: &mut Registers, clock: SyncSender<i32>, fetch_recv: Receiver<EncodedInstruction>, 
          decoded_sender: SyncSender<DecodedInstruction>, reg_wb_recv: Receiver<ExecuteResult>) {
    loop {
        let instruction = fetch_recv.recv().unwrap();
        if let ExecuteResult::Writeback(reg, res) = reg_wb_recv.recv().unwrap() {
            registers.gprs[reg] = res;
        }

        if let EncodedInstruction::Halt = instruction {
            decoded_sender.send(DecodedInstruction::Halt).unwrap();
            clock.send(1).unwrap();
            break;
        }
        
        let decoded = match instruction {
            EncodedInstruction::Noop            => DecodedInstruction::Noop,
            EncodedInstruction::Halt            => DecodedInstruction::Halt,
            EncodedInstruction::Addi(d, s, imm) => DecodedInstruction::Add(d, registers.gprs[s], imm),  
            EncodedInstruction::Add(d, s, t)    => DecodedInstruction::Add(d, registers.gprs[s], registers.gprs[t]),
            EncodedInstruction::And(d, s, t)    => DecodedInstruction::And(d, registers.gprs[s], registers.gprs[t]),
            EncodedInstruction::Andi(d, s, imm) => DecodedInstruction::And(d, registers.gprs[s], imm),
            EncodedInstruction::Beq(s, t, inst) => DecodedInstruction::Beq(registers.gprs[s], registers.gprs[t], inst),
            EncodedInstruction::Blt(s, t, inst) => DecodedInstruction::Blt(registers.gprs[s], registers.gprs[t], inst),
            EncodedInstruction::Div(d, s, t)    => DecodedInstruction::Div(d, registers.gprs[s], registers.gprs[t]),
            EncodedInstruction::J(inst)         => DecodedInstruction::J(inst),
            EncodedInstruction::Ldc(d, imm)     => DecodedInstruction::Mov(d, imm),
            EncodedInstruction::Li(d, imm)      => DecodedInstruction::Load(d, imm),
            EncodedInstruction::Lw(d, t)        => DecodedInstruction::Load(d, registers.gprs[t]),
            EncodedInstruction::Mod(d, s, t)    => DecodedInstruction::Mod(d, registers.gprs[s], registers.gprs[t]),
            EncodedInstruction::Mov(d, s)       => DecodedInstruction::Mov(d, registers.gprs[s]),
            EncodedInstruction::Mult(d, s, t)   => DecodedInstruction::Mult(d, registers.gprs[s], registers.gprs[t]),
            EncodedInstruction::Or(d, s, t)     => DecodedInstruction::Or(d, registers.gprs[s], registers.gprs[t]),
            EncodedInstruction::Sl(d, s, t)     => DecodedInstruction::Sl(d, registers.gprs[s], t),
            EncodedInstruction::Sr(d, s, t)     => DecodedInstruction::Sr(d, registers.gprs[s], t),
            EncodedInstruction::Sub(d, s, t)    => DecodedInstruction::Sub(d, registers.gprs[s], registers.gprs[t]),
            EncodedInstruction::Subi(d, s, imm) => DecodedInstruction::Sub(d, registers.gprs[s], imm),
            EncodedInstruction::Si(t, imm)      => DecodedInstruction::Store(registers.gprs[t], imm),
            EncodedInstruction::Sw(s, d)        => DecodedInstruction::Store(registers.gprs[s], registers.gprs[d]),
            EncodedInstruction::Xor(d, s, t)    => DecodedInstruction::Xor(d, registers.gprs[s], registers.gprs[t]),
            // _ => {
            //     panic!("{:?} is an unimplemented instruction", instruction);
            //     EncodedInstruction::Noop
            // }
        };
        println!("Decoded instruction: {:?}", decoded);
        decoded_sender.send(decoded).unwrap();
        clock.send(1).unwrap();
    }

    //Do some stuff here toc ollect the last two register results
    loop {
        let wb = reg_wb_recv.recv().unwrap();
        if let ExecuteResult::Halt = wb {
            break;
        }
        if let ExecuteResult::Writeback(reg, res) = reg_wb_recv.recv().unwrap() {
            registers.gprs[reg] = res;
        }
    }
    println!("END: Registers: {:?}", registers);
}

fn execute(memory: &mut [u32; MEM_SIZE], clock: SyncSender<i32>, pc_sender: SyncSender<Option<usize>>, 
           decode_recv: Receiver<DecodedInstruction>, wb_sender: SyncSender<ExecuteResult>) {
    loop {
        println!("1");
        let instruction = decode_recv.recv().unwrap();
        println!("2");
        if let DecodedInstruction::Halt = instruction {
            println!("3");
            wb_sender.send(ExecuteResult::Halt).unwrap();
            println!("4");
            clock.send(0).unwrap();
            println!("5");
            break;
        }
        println!("-1");
        let mut pc: Option<usize> = None;
        let wb = match instruction {
            DecodedInstruction::Noop => ExecuteResult::None,
            DecodedInstruction::Halt => ExecuteResult::Halt,
            DecodedInstruction::Add(r, x, y) => ExecuteResult::Writeback(r, x + y),
            DecodedInstruction::And(r, x, y) => ExecuteResult::Writeback(r, x & y),
            DecodedInstruction::Blt(s, t, inst) => {
                if s < t {
                    pc = Some(inst);
                }
                ExecuteResult::None
            },
            DecodedInstruction::Beq(s, t, inst) => {
                if s == t {
                    pc = Some(inst);
                }
                ExecuteResult::None
            }
            DecodedInstruction::Div(r, x, y) => ExecuteResult::Writeback(r, x / y),
            DecodedInstruction::J(inst) => {
                pc = Some(inst);
                ExecuteResult::None
            }
            DecodedInstruction::Load(r, s) => ExecuteResult::Writeback(r, memory[s as usize]),
            DecodedInstruction::Mod(d, s, t) => ExecuteResult::Writeback(d, s % t),
            DecodedInstruction::Mov(d, s) => ExecuteResult::Writeback(d, s),
            DecodedInstruction::Mult(r, x, y) => ExecuteResult::Writeback(r, x * y),
            DecodedInstruction::Or(r, x, y) => ExecuteResult::Writeback(r, x | y),
            DecodedInstruction::Sl(r, x, y) => ExecuteResult::Writeback(r, x << y),
            DecodedInstruction::Sr(r, x, y) => ExecuteResult::Writeback(r, x >> y),
            DecodedInstruction::Sub(r, x ,y) => ExecuteResult::Writeback(r, x - y),
            DecodedInstruction::Store(t, s) => {
                memory[s as usize] = t;
                ExecuteResult::None
            }
            DecodedInstruction::Xor(r, x, y) => ExecuteResult::Writeback(r, x ^ y),
        };
        println!("Execute result {:?}", wb);
        wb_sender.send(wb).unwrap();
        println!("-2");
        pc_sender.send(pc).unwrap();
        println!("-3");
        clock.send(1).unwrap();
        println!("-4");
    }
}

fn writeback(clock: SyncSender<i32>, wb_recv: Receiver<ExecuteResult>, reg_writer: SyncSender<ExecuteResult>) {
    loop {
        let wb_res = wb_recv.recv().unwrap();
        println!("WB RES: {:?}", wb_res);
        reg_writer.send(wb_res).unwrap();
        if let ExecuteResult::Halt = wb_res {
            clock.send(0).unwrap();
            break;
        }
        clock.send(1).unwrap();
    }
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
            "ANDI" => { 
                let (d, s, imm) = three_args(split_inst);
                instructions.push(EncodedInstruction::Andi(d, s, imm as u32));
            }
            "BEQ" => {
                let (s, t, addr) = three_args(split_inst);
                instructions.push(EncodedInstruction::Beq(s, t, addr));
            }
            "BLT" => {
                let (s, t, addr) = three_args(split_inst);
                instructions.push(EncodedInstruction::Blt(s, t, addr));
            }
            "DIV" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Div(d, s, t));
            }
            "J" => {
                let imm = split_inst[1].parse::<usize>().unwrap();
                instructions.push(EncodedInstruction::J(imm));
            }
            "LDC" => {
                let (s, imm) = two_args(split_inst);
                instructions.push(EncodedInstruction::Ldc(s, imm as u32));
            }
            "LI" => {
                let (s, imm) = two_args(split_inst);
                instructions.push(EncodedInstruction::Li(s, imm as u32));
            }
            "LW" => { // LW
                let (s, t) = two_args(split_inst);
                instructions.push(EncodedInstruction::Lw(s, t));
            }
            "MOD" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Mod(d, s, t));
            }
            "MOV" =>{
                let (d, s) = two_args(split_inst);
                instructions.push(EncodedInstruction::Mov(d, s));
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
            "SL" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Sl(d, s, t as u32));
            }
            "SR" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Sr(d, s, t as u32));
            }
            "SUB" => {
                let (d, s, t) = three_args(split_inst);
                instructions.push(EncodedInstruction::Sub(d, s, t));
            }
            "SUBI" => {
                let (d, s, imm) = three_args(split_inst);
                instructions.push(EncodedInstruction::Subi(d, s, imm as u32));
            }
            "SI" => {
                let (s, imm) = two_args(split_inst);
                instructions.push(EncodedInstruction::Si(s, imm as u32));
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