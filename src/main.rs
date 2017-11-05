use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::env;
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

    let num_instructions = instructions.instructions.len();

    // for i in instructions.iter() {
    //     println!("{:?}", i);
    // }

    let mut memory: [u32; MEM_SIZE] = [1; MEM_SIZE];

    let mut regs = Registers{ gprs: [0; 32]};

    //Send pc from execute to fetch + control value with Optional None -> 0
    let (pc_sender_e, pc_recv_f) = sync_channel::<Option<usize>>(1);

    //need a channel from fetch -> decode
    let (fetch_sender, fetch_recv) = sync_channel::<EncodedInstruction>(1);

    //need a channel from decode -> execute
    let (decode_sender, decode_recv) = sync_channel::<DecodedInstruction>(1);

    //need a channel from execute -> wb
    let (wb_sender, wb_recv) = sync_channel::<ExecuteResult>(1);

    //Channel to signal stalled or not to fetch
    let (stalled_f_sender, stalled_f_recv) = sync_channel::<bool>(1);
    let (stalled_d_sender, stalled_d_recv) = sync_channel::<bool>(1);

    //Channel for setting reset signal for decode stage
    let (reset_d_sender, reset_d_recv) = sync_channel::<bool>(2);

    //Need to put initial values into inter stage channels plus signal channels
    fetch_sender.send(EncodedInstruction::Noop).unwrap();
    decode_sender.send(DecodedInstruction::Noop).unwrap();
    wb_sender.send(ExecuteResult::None).unwrap();
    reset_d_sender.send(false).unwrap();

    let mut cycles = 0;
    let mut fetch_unit = FetchUnit::new();
    let mut exec_unit = ExecutionUnit::new();

    loop {
        let w_res = writeback(&mut regs, &wb_recv);
        let e_res = execute(&mut exec_unit,&mut memory, &pc_sender_e, &decode_recv, 
                            &wb_sender, &stalled_f_sender, &stalled_d_sender, &reset_d_sender);

        let d_res = decode(regs, &fetch_recv, &decode_sender, &stalled_d_recv, &reset_d_recv);
        let f_res = fetch(&mut fetch_unit, &instructions, &pc_recv_f, &fetch_sender, &stalled_f_recv);
        println!("CYCLE");
        println!("");
        cycles += 1;
        if (w_res + e_res + d_res + f_res) == 0 {
            break;
        }
    }
    //finish off last three cycles
    println!("here1");

    cycles += 3;
    println!("{:?}", regs);
    println!("Instructions executed: {}", num_instructions);
    println!("Number of cycles: {}", cycles);
    println!("Instructions per cycle: {}", num_instructions as f32  / cycles as f32);
    //println!("End: {:?}", regs);
    //for i in 0..30 {
    //    println!("MEM[{}]: {}", i, memory[i]);
    //}
}

fn fetch(fetch_unit: &mut FetchUnit, instructions: &Instructions, pc_receiver: &Receiver<Option<usize>>, 
         fetch_sender: &SyncSender<EncodedInstruction>, stalled: &Receiver<bool>) -> u32 {

    match stalled.recv().unwrap() {
        true => 1,
        false => {
            let inst = instructions.get_instruction(fetch_unit.pc);
            match pc_receiver.recv().unwrap() {
                Some(addr) => fetch_unit.pc = addr,
                None => fetch_unit.pc += 1,
            };
    
            println!("pc: {}", fetch_unit.pc);
            println!("Fetched instruction: {:?}", inst);
            fetch_sender.send(inst).unwrap();
            if let EncodedInstruction::Halt = inst {
                0
            }
            else {
                1
            }
        }
    }

    
}

fn decode(registers: Registers, fetch_recv: &Receiver<EncodedInstruction>, 
          decoded_sender: &SyncSender<DecodedInstruction>, stalled: &Receiver<bool>,
          reset: &Receiver<bool>) -> u32 {
    
    match stalled.recv().unwrap() {
        true => 1,
        false => {
            let instruction = fetch_recv.recv().unwrap();
            let decoded = match reset.recv().unwrap() {
                true => {
                    DecodedInstruction::Noop
                },
                false => {
                    match instruction {
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
                    }
                },
            };
            println!("Decoded instruction: {:?}", decoded);
            decoded_sender.send(decoded).unwrap();
            if let DecodedInstruction::Halt = decoded {
                0
            } else {
                1
            }
        },
    }
}

fn execute(exec_unit: &mut ExecutionUnit, memory: &mut [u32; MEM_SIZE], pc_sender: &SyncSender<Option<usize>>, 
           decode_recv: &Receiver<DecodedInstruction>, wb_sender: &SyncSender<ExecuteResult>,
           stalled_fetch: &SyncSender<bool>, stalled_decode: & SyncSender<bool>,
           reset_decode: &SyncSender<bool>) -> u32 {
    
    let instruction = decode_recv.recv().unwrap();    
    let reorder_buffer_pos = exec_unit.reorder_buffer.get_new_pos();
    let mut pc: Option<usize> = None;
    let mut stalled = false;
    let mut reset = false;
    let wb = match exec_unit.branch_reset {
        true => {
            ExecuteResult::None
        },
        false => {
            match instruction {
                DecodedInstruction::Noop => ExecuteResult::None,
                DecodedInstruction::Halt => ExecuteResult::Halt,
                DecodedInstruction::Add(r, x, y) => ExecuteResult::Writeback(r, x + y),
                DecodedInstruction::And(r, x, y) => ExecuteResult::Writeback(r, x & y),
                DecodedInstruction::Blt(s, t, inst) => {
                    if s < t {
                        pc = Some(inst);
                        reset = true;
                    }
                    ExecuteResult::None
                },
                DecodedInstruction::Beq(s, t, inst) => {
                    if s == t {
                        pc = Some(inst);
                        reset = true;
                    }
                    ExecuteResult::None
                }
                DecodedInstruction::Div(r, x, y) => ExecuteResult::Writeback(r, x / y),
                DecodedInstruction::J(inst) => {
                    pc = Some(inst);
                    reset = true;
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
            }
        },
    };
    
    println!("Execute result {:?}", wb);
    exec_unit.reorder_buffer.insert(reorder_buffer_pos, wb);
    println!("{:?}", exec_unit.reorder_buffer);
    wb_sender.send(exec_unit.reorder_buffer.get_writeback()).unwrap();
    println!("{:?}", exec_unit.reorder_buffer);
    pc_sender.send(pc).unwrap();

    //set the stalled signal for execute and decode
    reset_decode.send(reset).unwrap();
    exec_unit.branch_reset = reset;

    //set the stalled signal for fetch and decode
    stalled_fetch.send(stalled).unwrap();
    stalled_decode.send(stalled).unwrap();
    if let DecodedInstruction::Halt = instruction {
        0
    }
    else {
        1
    }
}

fn writeback(registers: &mut Registers, wb_recv: &Receiver<ExecuteResult>) -> u32 {
    let wb_res = wb_recv.recv().unwrap();
    println!("WB RES: {:?}", wb_res);
    match wb_res {
        ExecuteResult::None => 1,
        ExecuteResult::Halt => 0,
        ExecuteResult::Writeback(register, value) => {
            registers.gprs[register] = value;
            1
        },
    }
}

struct ExecutionUnit {
    reorder_buffer: ReorderBuffer,
    branch_reset: bool,
}

impl ExecutionUnit {
    fn new() -> ExecutionUnit {
        ExecutionUnit {
            reorder_buffer: ReorderBuffer::new(),
            branch_reset: false,
        }
    }
}

struct Instructions {
    instructions: Vec<EncodedInstruction>,
}

impl Instructions {
    fn get_instruction(&self, position: usize) -> EncodedInstruction {
        if position < self.instructions.len() {
            self.instructions[position]
        } else {
            EncodedInstruction::Halt
        }
    }
}

#[derive(Debug)]
struct ReorderBuffer {
    oldest: usize,
    newest: usize,
    buffer: [Option<ExecuteResult>; 16],
}

impl ReorderBuffer {
    fn new() -> ReorderBuffer {
        ReorderBuffer {
            oldest: 0,
            newest: 0,
            buffer: [None ; 16],
        }
    }

    fn get_new_pos(&mut self) -> usize {
        let ret = self.newest;
        self.newest = (self.newest + 1) % self.buffer.len();
        ret
    }

    fn insert(&mut self, pos: usize, result: ExecuteResult) -> () {
        self.buffer[pos] = Some(result);
    }

    fn get_writeback(&mut self) -> ExecuteResult {
        if let Some(result) = self.buffer[self.oldest] {
            self.buffer[self.oldest] = None;
            self.oldest = (self.oldest + 1) % self.buffer.len();
            result
        } else {
            ExecuteResult::None
        }
    }
}

struct FetchUnit {
    pc: usize,
}

impl FetchUnit {
    fn new() -> FetchUnit {
        FetchUnit {
            pc: 0,
        }
    }
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

fn assemble(assembly: Vec<String>) -> Instructions {
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
    Instructions{ instructions: instructions }
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