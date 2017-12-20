extern crate clap;
use clap::{Arg, App, SubCommand};
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::collections::LinkedList;
use std::fmt;

const ROB_SIZE: usize = 32;
const MEM_SIZE: usize = 52;
const NUM_RS: usize = 16;
const NUM_ALUS: usize = 4;
const NUM_MULTS: usize = 4;
const MAX_PREDICTIONS: usize = 1024;
const FETCH_WIDTH: usize = 5;
const DECODE_WIDTH: usize = 5;
const COMMIT_WIDTH: usize = 5;

fn main() {
    let matches = App::new("My Super Program")
                          .version("1.0")
                          .author("Richard Boyd rb14427@my.bristol.ac.uk")
                          .about("Superscalar CPU simulator")
                          .arg(Arg::with_name("INPUT")
                               .help("Sets the input file to use")
                               .required(true)
                               .index(1))
                          .arg(Arg::with_name("branch_prediction")
                               .short("p")
                               .long("pred-type")
                               .help("Sets the branch predictor type
                                      \n1 - Static
                                      \n2 - 1 bit history
                                      \n3 - 2 but history
                                      \n4 - 3 bit history and so on")
                               .required(false)
                               .takes_value(true))
                          .arg(Arg::with_name("v")
                               .short("v")
                               .multiple(true)
                               .help("Sets the level of verbosity"))
                          .get_matches();
    
    let pred_type = matches.value_of("branch_prediction").unwrap_or("0").parse::<usize>().unwrap();
    println!("PRED TYPE: {}", pred_type);
    println!("Using input file: {}", matches.value_of("INPUT").unwrap());


    let file = File::open(matches.value_of("INPUT").unwrap()).unwrap();

    let buf = BufReader::new(file);
    let assembly: Vec<String> = buf.lines().map(|l| l.expect("Could not parse line")).collect();

    let instructions = assemble(assembly);

    let mut memory: [u32; MEM_SIZE] = [0; MEM_SIZE];

    for i in 0..MEM_SIZE {
        memory[i] = (MEM_SIZE - i) as u32;
    }

    let mut cpu = CPU::new(instructions, pred_type);

    let mut cycles = 0;

    loop {
        commit(&mut cpu);
        writeback(&mut cpu);
        execute(&mut cpu, &mut memory);
        decode(&mut cpu);
        fetch(&mut cpu);

        println!("CYCLE {}", cycles);
        println!("");
        println!("CPU: {:?}", cpu);

        cycles += 1;
        for i in memory.iter() {
            println!("{}", i);
        }
        println!("Registers: {:?}", cpu.registers.gprs);

        if cpu.finished() {
            break;
        }
    }

    
    println!("Registers Final Values: {:?}", cpu.registers.gprs);
    println!("Instructions executed: {}", cpu.rob.instructions_committed);
    println!("Number of cycles: {}", cycles);
    println!("Instructions per cycle: {:.2}", (cpu.rob.instructions_committed as f32)  / (cycles as f32));
    println!("Branch prediction accuracy: {:.2}", cpu.branch_predictor.accuracy());
    
    for i in 0..30 {
        println!("{}", memory[i]);
    }
    //println!("End: {:?}", regs);
    //for i in 0..30 {
    //    println!("MEM[{}]: {}", i, memory[i]);
    //}
}

fn fetch(cpu: &mut CPU) {

    match cpu.fetch_unit.reset {
        true => {
            cpu.fetch_unit.reset = false;
        },
        false => {
            for _ in 0..FETCH_WIDTH {
                let inst = cpu.fetch_unit.get_instruction();
                match inst {
                    EncodedInstruction::Halt => (),
                    _ => {
                        cpu.decode_unit.add_instruction(inst, cpu.fetch_unit.pc);
                        cpu.fetch_unit.pc = cpu.branch_predictor.predict(inst, cpu.fetch_unit.pc);
                    }
                }
            }
        }
    }
}

fn decode(cpu: &mut CPU) {

    for _ in 0..DECODE_WIDTH {
        let possible_instruction = cpu.decode_unit.get_next_instruction();
        match possible_instruction {
            Some((pc, instruction)) => {
                let reset = cpu.decode_unit.reset;
                match reset {
                    true => {
                        cpu.decode_unit.instruction_q.clear();
                        cpu.decode_unit.reset = false;
                    },
                    false => {
                        match instruction {
                            EncodedInstruction::Noop            => {
                                cpu.decode_unit.pop_instruction();
                            }
                            EncodedInstruction::Halt            => {
                                
                            },
                            EncodedInstruction::Addi(d, s, imm) => {
                                cpu.issue_imm(d, s, imm, Op::Add);
                            },
                            EncodedInstruction::Add(d, s, t)    => {
                                cpu.issue(d, s, t, Op::Add);
                            },
                            EncodedInstruction::And(d, s, t)    => {
                                cpu.issue(d, s, t, Op::And);
                            },
                            EncodedInstruction::Andi(d, s, imm) => {
                                cpu.issue_imm(d, s, imm, Op::And);
                            },
                            EncodedInstruction::Beq(s, t, inst) => {
                                cpu.issue_branch2(s, t, inst, Op::Beq, pc);
                            },
                            EncodedInstruction::Beqz(s, inst) => {
                                cpu.issue_branch1(s, inst, Op::Beqz, pc);
                            }
                            EncodedInstruction::Blt(s, t, inst) => {
                                cpu.issue_branch2(s, t, inst, Op::Blt, pc);
                            },
                            EncodedInstruction::Bgt(s, t, inst) => {
                                cpu.issue_branch2(s, t, inst, Op::Bgt, pc);
                            },
                            EncodedInstruction::Div(d, s, t)    => {
                                cpu.issue(d, s, t, Op::Div);
                            },
                            EncodedInstruction::J(inst)         => {
                                cpu.issue_branch0(inst, Op::J, pc);
                            },
                            EncodedInstruction::Ldc(d, imm)     => {
                                cpu.issue1_imm(d, imm, Op::Mov);
                            },
                            EncodedInstruction::Lw(addr, dest)        => {
                                if let Some(rob_pos) = cpu.rob.commit_to(dest) {
                                    let operand1 = cpu.get_operand(addr);
                                    cpu.registers.set_owner(dest, rob_pos);
                                    cpu.lsq.issue(LSQOp::L, pc, rob_pos, operand1, Operand::None);
                                    cpu.decode_unit.pop_instruction();
                                } else {
                                    println!("NO ROB SPACE");
                                }
                            },
                            EncodedInstruction::Mod(d, s, t)    => {
                                cpu.issue(d, s, t, Op::Mod);
                            },
                            EncodedInstruction::Mov(d, s)       => {
                                cpu.issue1(d, s, Op::Mov);
                            },
                            EncodedInstruction::Mult(d, s, t)   => {
                                cpu.issue(d, s, t, Op::Mult);
                            },
                            EncodedInstruction::Or(d, s, t)     => {
                                cpu.issue(d, s, t, Op::Or);
                            },
                            EncodedInstruction::Sl(d, s, t)     => {
                                cpu.issue_imm(d, s, t, Op::Sl);
                            },
                            EncodedInstruction::Sr(d, s, t)     => {
                                cpu.issue_imm(d, s, t, Op::Sr);
                            },
                            EncodedInstruction::Sub(d, s, t)    => {
                                cpu.issue(d, s, t, Op::Sub);
                            },
                            EncodedInstruction::Subi(d, s, imm) => {
                                cpu.issue_imm(d, s, imm, Op::Sub);
                            },
                            EncodedInstruction::Sw(addr, val)        => {
                                if let Some(rob_pos) = cpu.rob.commit_to_store(val) {
                                    let operand1 = cpu.get_operand(addr);
                                    let operand2 = cpu.get_operand(val);
                                    cpu.registers.set_owner(val, rob_pos);
                                    cpu.lsq.issue(LSQOp::S, pc, rob_pos, operand1, operand2);
                                    cpu.decode_unit.pop_instruction();
                                }
                            },
                            EncodedInstruction::Xor(d, s, t)    => {
                                cpu.issue(d, s, t, Op::Xor);
                            },
                        };
                    },
                };
            },
            None => (),
        };
    }

    //now dispatch
    for fu in &mut cpu.exec_unit.func_units {
        for rs in 0..cpu.exec_unit.rs_sts.len() {
            //try to dipatch to functional unit
            if let Some((x, y)) = cpu.exec_unit.rs_sts[rs].get_operands() {
                if fu.dispatch(x, y, cpu.exec_unit.rs_sts[rs].operation, cpu.exec_unit.rs_sts[rs].rob_entry, cpu.exec_unit.rs_sts[rs].address) {
                    println!("Dispatching {} = {} {:?} {} from {}", cpu.exec_unit.rs_sts[rs].rob_entry, x, cpu.exec_unit.rs_sts[rs].operation, y, rs );
                    cpu.exec_unit.rs_sts[rs].free();
                    break; //found a functional unit to execute this RS 
                }
            }
        }
    }

    //Now check the LSQ if something can be executed
    if cpu.exec_unit.mem_unit.finished() {
        if let Some(i) = cpu.lsq.get_next_instruction() {
            cpu.exec_unit.mem_unit.dispatch(i);
        }
    }
}

fn execute(cpu: &mut CPU, memory: &mut [u32; MEM_SIZE]) {

    for fu in &mut cpu.exec_unit.func_units {
        fu.cycle();
    }

    cpu.exec_unit.mem_unit.cycle(memory);
}

fn writeback(cpu: &mut CPU) {
    for fu in 0..cpu.exec_unit.func_units.len() {
        if let Some((result, rob_entry)) = cpu.exec_unit.func_units[fu].get_result() {
            
                cpu.rob.insert(rob_entry, result);

                //Resolve dependencies if there is any
                println!("CDB BROADCASTING: {:?} to ROB {}", result, rob_entry);
                match result {
                    ExecResult::Value(x) => {
                        
                        // resolve dependencies in the reservation stations
                        for dependent in 0..cpu.exec_unit.rs_sts.len() {
                            cpu.exec_unit.rs_sts[dependent].resolve_dependency(x, rob_entry);
                        }

                        //resolve dependencies in the load store queue
                        cpu.lsq.resolve_dependency(x, rob_entry);
                    },
                    _ => (),
                }
        }
    }

    let mem_res = cpu.exec_unit.mem_unit.get_result();
    if let Some((rob_entry, ExecResult::Value(x))) = mem_res {
        cpu.rob.insert(rob_entry, ExecResult::Value(x));
        // resolve dependencies in the reservation stations
        for dependent in 0..cpu.exec_unit.rs_sts.len() {
            cpu.exec_unit.rs_sts[dependent].resolve_dependency(x, rob_entry);
        }

        //resolve dependencies in the load store queue
        cpu.lsq.resolve_dependency(x, rob_entry);
    }

}

fn commit(cpu: &mut CPU) {
    for _ in 0..COMMIT_WIDTH {
        match cpu.rob.get_commit() {
            ReorderBufferResult::Writeback(res, rob, reg) => {
                println!("Writeback {} {}", res, reg);
                cpu.registers.write_result(res, rob, reg);
            },
            ReorderBufferResult::BranchTaken(inst, pc) => {
                //ROB also beign used to store predicted PC for branches
                //If not equal then a misprediction occurred
                let predicted_correct = cpu.branch_predictor.prediction_correct(inst, pc);
                // IF not correctly predicted
                println!("Prediction correct: {} {}", predicted_correct, inst);
                if !predicted_correct {
                    //Need to clear RSs, FUs, Instruction Queue
                    cpu.reset();
                    //Also need to set the PC correctly
                    cpu.fetch_unit.mispredict(inst);
                    //need to let branch predictor know of incorrect prediction
                    break;
                }
            },
            ReorderBufferResult::BranchNotTaken(pc) => {
                //ROB also beign used to store predicted PC for branches
                //If not equal then a misprediction occurred
                let taken_pc = pc + 1;
                let predicted_correct = cpu.branch_predictor.prediction_correct(taken_pc, pc);
                // IF not correctly predicted
                println!("Prediction correct: {} {}", predicted_correct, taken_pc);
                if !predicted_correct {
                    //Need to clear RSs, FUs, Instruction Queue
                    cpu.reset();
                    //Also need to set the PC correctly
                    cpu.fetch_unit.mispredict(taken_pc);
                    //need to let branch predictor know of incorrect prediction
                    break;
                }
            },
            ReorderBufferResult::Store(r) => {
                cpu.lsq.committed(r);
            }
            ReorderBufferResult::None => (),
        };
    }
}

struct CPU {
    fetch_unit: FetchUnit,
    decode_unit: DecodeUnit,
    exec_unit: ExecUnit,
    registers: Registers,
    rob: ReorderBuffer,
    branch_predictor: BranchPredictor,
    lsq: LSQ,
}

impl fmt::Debug for CPU {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Fetch unit: {:?}\n\nDecode Unit: {:?}\n\nExec Unit: {:?}\n\nRegisters: {:?}\n\nROB: {:?}\n\nLSQ: {:?}", self.fetch_unit, self.decode_unit, self.exec_unit, self.registers, self.rob, self.lsq)
    }
}

impl CPU {
    fn new(instructions: Vec<EncodedInstruction>, pred_type: usize) -> CPU {
        CPU {
            fetch_unit: FetchUnit::new(instructions),
            decode_unit: DecodeUnit::new(),
            exec_unit: ExecUnit::new(),
            registers: Registers::new(),
            rob: ReorderBuffer::new(),
            branch_predictor: BranchPredictor::new(pred_type),
            lsq: LSQ::new(),
        }
    }

    fn issue(&mut self, d: usize, s: usize, t: usize, op: Op) {
        if let Some(r) = self.exec_unit.get_free_rs() {
            if let Some(rob_pos) = self.rob.commit_to(d) {
                let operand1 = self.get_operand(s);
                let operand2 = self.get_operand(t);
                self.registers.set_owner(d, rob_pos);
                self.exec_unit.issue(operand1, operand2, op, r, rob_pos);
                self.decode_unit.pop_instruction();
            }
        }
    }

    fn issue1(&mut self, d: usize, s: usize, op: Op) {
        if let Some(r) = self.exec_unit.get_free_rs() {
            if let Some(rob_pos) = self.rob.commit_to(d) {
                let operand1 = self.get_operand(s);
                self.registers.set_owner(d, rob_pos);
                self.exec_unit.issue(operand1, Operand::None, op, r, rob_pos);
                self.decode_unit.pop_instruction();
            }
        }
    }

    fn issue1_imm(&mut self, d: usize, imm: u32, op: Op) {
        if let Some(r) = self.exec_unit.get_free_rs() {
            if let Some(rob_pos) = self.rob.commit_to(d) {
                self.registers.set_owner(d, rob_pos);
                self.exec_unit.issue(Operand::Value(imm), Operand::None, op, r, rob_pos);
                self.decode_unit.pop_instruction();
            }
        }
    }

    fn issue_imm(&mut self, d: usize, s: usize, imm: u32, op: Op) {
        if let Some(r) = self.exec_unit.get_free_rs() {
            if let Some(rob_pos) = self.rob.commit_to(d) {
                let operand1 = self.get_operand(s);
                self.registers.set_owner(d, rob_pos);
                self.exec_unit.issue(operand1, Operand::Value(imm), op, r, rob_pos);
                self.decode_unit.pop_instruction();
            }
        }
    }

    fn issue_branch0(&mut self, inst: usize, op: Op, pc: usize) {
        if let Some(r) = self.exec_unit.get_free_rs() {
            if let Some(rob_pos) = self.rob.commit_to(pc) {
                self.exec_unit.issue_branch(Operand::None, Operand::None, op, r, rob_pos, inst);
                self.decode_unit.pop_instruction();
            }
        }
    }

    fn issue_branch1(&mut self, s: usize, inst: usize, op: Op, pc: usize) {
        if let Some(r) = self.exec_unit.get_free_rs() {
            if let Some(rob_pos) = self.rob.commit_to(pc) {
                let operand1 = self.get_operand(s);
                self.exec_unit.issue_branch(operand1, Operand::None, op, r, rob_pos, inst);
                self.decode_unit.pop_instruction();
            }
        }
    }

    fn issue_branch2(&mut self, s: usize, t: usize, inst: usize, op: Op, pc: usize) {
        if let Some(r) = self.exec_unit.get_free_rs() {
            if let Some(rob_pos) = self.rob.commit_to(pc) {
                let operand1 = self.get_operand(s);
                let operand2 = self.get_operand(t);
                self.exec_unit.issue_branch(operand1, operand2, op, r, rob_pos, inst);
                self.decode_unit.pop_instruction();
                
            }
        }
    }

    fn finished(&self) -> bool {
        self.fetch_unit.finished() &&
        self.decode_unit.finished() &&
        self.exec_unit.finished() &&
        self.rob.is_empty() &&
        self.lsq.finished()
    }

    fn get_operand(&self, reg: usize) -> Operand {
        let o = self.read_reg(reg);
        match o {
            Operand::Rob(r) => {
                if let Some(result) = self.rob.buffer[r].result {
                    if let ExecResult::Value(x) = result {
                        Operand::Value(x)
                    } else { o }
                } else { o }
            },
            _ => {
                o
            }
        }
    }

    fn read_reg(&self, reg: usize) -> Operand {
        match self.registers.rat[reg] {
            None => {
                Operand::Value(self.registers.gprs[reg])
            },
            Some(rob_entry) => {
                if let Some(ExecResult::Value(x)) = self.rob.buffer[rob_entry].result {
                    Operand::Value(x)
                } else {
                    Operand::Rob(rob_entry)
                }
            },
        }
    }

    fn reset(&mut self) {
        self.registers.clear_rat();
        self.exec_unit.reset();
        self.decode_unit.reset();
        self.rob.empty();
        self.lsq.clear();
    }
}

#[derive(Debug)]
struct FetchUnit {
    pc: usize,
    exec_pc: Option<usize>,
    instructions: Vec<EncodedInstruction>,
    stalled: bool,
    reset: bool,
}

impl FetchUnit {
    fn new(encoded_instructions: Vec<EncodedInstruction>) -> FetchUnit {
        FetchUnit {
            pc: 0,
            exec_pc: None,
            instructions: encoded_instructions,
            stalled: false,
            reset: false,
        }
    }

    fn finished(&self) -> bool {
        if self.pc < self.instructions.len() {
            false
        } else {
            true
        }
    }

    fn mispredict(&mut self, new_pc: usize) {
        self.reset = true;
        self.pc = new_pc;
    }

    fn get_instruction(&self) -> EncodedInstruction {
        if self.pc < self.instructions.len() {
            self.instructions[self.pc]
        } else {
            EncodedInstruction::Halt
        }
    }
}

#[derive(Debug)]
struct DecodeUnit {
    instruction_q: LinkedList<(usize, EncodedInstruction)>,
    stalled: bool,
    reset: bool,
}

impl DecodeUnit {
    fn new() -> DecodeUnit {
        DecodeUnit {
            instruction_q: LinkedList::new(),
            stalled: false,
            reset: false,
        }
    }

    fn finished(&self) -> bool {
        self.instruction_q.is_empty()
    }

    fn reset(&mut self) {
        self.clear_instructions();
    }

    fn clear_instructions(&mut self) {
        self.instruction_q.clear();
    }

    fn add_instruction(&mut self, instruction: EncodedInstruction, pc: usize) {
        self.instruction_q.push_back((pc, instruction));
    }

    fn get_next_instruction(&self) -> Option<(usize, EncodedInstruction)> {
        match self.instruction_q.front() {
            Some(x) => Some((*x).clone()),
            None => None,
        }
    }

    fn pop_instruction(&mut self) {
        self.instruction_q.pop_front();
    }
}

struct ExecUnit {
    func_units: Vec<FunctionalUnit>,
    rs_sts: Vec<ReservationStation>,
    mem_unit: MemoryUnit,
}

impl fmt::Debug for ExecUnit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Reservation Stations {:?}\nFunctional Units: {:?}\n\nMemory Unit: {:?}", self.rs_sts, self.func_units, self.mem_unit)
    }
}

impl ExecUnit {
    fn new() -> ExecUnit {
        let mut fus: Vec<FunctionalUnit> = Vec::new();

        //ALUs
        for _ in 0..NUM_ALUS {
            fus.push(FunctionalUnit::new(FUType::ALU));
        }
        for _ in 0..NUM_MULTS {
            fus.push(FunctionalUnit::new(FUType::Multiplier));
        }

        fus.push(FunctionalUnit::new(FUType::Branch));
        fus.push(FunctionalUnit::new(FUType::Branch));
        

        let mut rs_sts: Vec<ReservationStation> = Vec::new();
        for _ in 0..NUM_RS {
            rs_sts.push(ReservationStation::new());
        }
        ExecUnit {
            func_units: fus,
            rs_sts: rs_sts,
            mem_unit: MemoryUnit::new(),
        }
    }

    fn reset(&mut self) {
        for rs in &mut self.rs_sts {
            rs.free();
        }
        for fu in &mut self.func_units {
            fu.reset();
        }
    }

    fn finished(&self) -> bool {
        self.func_units.iter().all(|ref x| x.finished()) && self.rs_sts.iter().all(|ref x| x.finished() && self.mem_unit.finished())
    }

    fn get_free_rs(&self) -> Option<usize> {
        for rs in 0..self.rs_sts.len() {
            if !self.rs_sts[rs].busy {
                return Some(rs);
            }
        }
        return None;
    }

    fn issue(&mut self, o1: Operand, o2: Operand, operation: Op, rs: usize, rob_entry: usize) {
        self.rs_sts[rs].issue(o1, o2, operation, rob_entry);
    }

    fn issue_branch(&mut self, o1: Operand, o2: Operand, operation: Op, rs: usize, rob_entry: usize, addr: usize) {
        self.rs_sts[rs].issue_branch(o1, o2, operation, rob_entry, addr);
    }
}

#[derive(Debug, Copy, Clone)]
enum LSQOp {
    L,
    S,
}

#[derive(Debug, Copy, Clone)]
struct LSQEntry {
    op: LSQOp,
    pc: usize,
    rob_entry: usize,
    addr: Operand,
    value: Operand,
    committed: bool,
}

impl LSQEntry {
    fn new(op: LSQOp, pc: usize, rob_entry: usize, addr: Operand, value: Operand) -> LSQEntry {
        LSQEntry {
            op: op,
            pc: pc,
            rob_entry: rob_entry,
            addr: addr,
            value: value,
            committed: false,
        }
    }
}

#[derive(Debug)]
struct LSQ {
    lsq: LinkedList<LSQEntry>,
}

impl LSQ {
    fn new() -> LSQ {
        LSQ {
             lsq: LinkedList::new(),
        }
    }

    fn finished(&self) -> bool {
        self.lsq.len() == 0
    }

    fn clear(&mut self) {
        
        while let Some(back) = self.lsq.pop_back() {
            if back.committed == true {
                self.lsq.push_back(back);
                break;
            }
        }
    }

    fn issue(&mut self, op: LSQOp, pc: usize, rob_entry: usize, addr: Operand, value: Operand) {
        self.lsq.push_back(LSQEntry::new(op, pc, rob_entry, addr, value));
    }

    fn resolve_dependency(&mut self, result: u32, rob_entry: usize) {
        for entry in self.lsq.iter_mut() {
            // If the address of a load or store is dependign on an execution result
            if let Operand::Rob(r) = entry.addr {
                if r == rob_entry {
                    entry.addr = Operand::Value(result);
                }
            }
            //If a store is depending on a register result
            if let Operand::Rob(r) = entry.value {
                if r == rob_entry {
                    entry.value = Operand::Value(result);
                }
            }
        }
    }

    fn get_next_instruction(&mut self) -> Option<LSQEntry> {
        match self.lsq.pop_front() {
            Some(instruction) => {
                self.lsq.push_front(instruction);
                match instruction.op {
                    LSQOp::S => {
                        if instruction.committed {
                            if let Operand::Value(_) = instruction.addr {
                                if let Operand::Value(_) = instruction.value {
                                    self.lsq.pop_front();
                                    Some((instruction).clone())
                                } else { None }
                            } else { None }
                        } else { None }
                    },
                    LSQOp::L => {
                        if let Operand::Value(_) = instruction.addr {
                            self.lsq.pop_front();
                            Some((instruction).clone())
                        } else { None }
                    }
                }
            },
            None => None,
        }
    }

    fn committed(&mut self, rob_entry: usize) {
        for entry in self.lsq.iter_mut() {
            if rob_entry == entry.rob_entry {
                entry.committed = true;
            }
        }
    }
}

#[derive(Debug)]
struct MemoryUnit {
    instruction: LSQEntry,
    cycles: u32,
    result: Option<u32>,
}

impl MemoryUnit {

    fn new() -> MemoryUnit {
        MemoryUnit {
            instruction: LSQEntry::new(LSQOp::S, 0, 0, Operand::None, Operand::None),
            cycles: 0,
            result: None,
        }
    }

    fn reset(&mut self) {
        self.cycles = 0;
        self.result = None;
    }

    fn finished(&self) -> bool {
        if let None = self.result {
            if self.cycles == 0{
                true
            } else { false }
            
        } else { false }
    }

    fn dispatch(&mut self, next_instruction: LSQEntry) -> bool {
        if let None = self.result {
            if self.cycles == 0 {
                println!("Mem dispatch: {:?}", next_instruction);
                self.instruction = next_instruction;
                self.result = None;
                self.cycles = 2;
                true
            } else { false }
        } else { false }
    }

    fn cycle(&mut self, memory: &mut [u32; MEM_SIZE]) {
        if self.cycles > 0 {
            self.cycles -= 1;
            if self.cycles == 0 {
                match self.instruction.op {
                    LSQOp::S => {
                        if let Operand::Value(value) = self.instruction.value {
                            if let Operand::Value(addr) = self.instruction.addr {
                                memory[addr as usize] = value;
                            } else { panic!("Dispatched store without knowing the address {:?}", self.instruction.addr); }
                            
                        } else { panic!("Dispatched store without knowing the value {:?}", self.instruction.value); }
                    },
                    LSQOp::L => {
                        if let Operand::Value(addr) = self.instruction.addr {
                            self.result = Some(memory[addr as usize]);
                        }
                    }
                }
            }
        }
    }

    fn get_result(&mut self) -> Option<(usize, ExecResult)> {
        if let Some(result) = self.result {
            if self.cycles == 0 {
                self.result = None;
                Some((self.instruction.rob_entry, ExecResult::Value(result)))
            } else { None }
            
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Op {
    None,
    Add,
    And,
    Or,
    Sub,
    Xor,
    Mov,
    Sr,
    Sl,
    Mult,
    Div,
    Mod,
    J,
    Beq,
    Beqz,
    Blt,
    Bgt,
}

#[derive(Debug, Copy, Clone)]
enum FUType {
    Multiplier,
    ALU,
    Branch,
}

#[derive(Debug)]
struct FunctionalUnit {
    fu_type: FUType,
    op1: u32,
    op2: u32,
    addr: usize,
    operation: Op,
    cycles: i32,
    result: Option<(usize, ExecResult)>,
    rob_entry: usize,
    op1_next: u32,
    op2_next: u32,
    addr_next: usize,
    operation_next: Op,
    rob_entry_next: usize,
}

impl FunctionalUnit {
    fn new(fu_type: FUType) -> FunctionalUnit {
        FunctionalUnit {
            fu_type: fu_type,
            op1: 0,
            op2: 0,
            addr: 0,
            operation: Op::None,
            cycles: 0,
            result: None,
            rob_entry: 0,
            op1_next: 0,
            op2_next: 0,
            addr_next: 0,
            operation_next: Op::None,
            rob_entry_next: 0,
        }
    }

    fn dispatch(&mut self, o1: u32, o2: u32, operation: Op, rob_entry: usize, addr: usize) -> bool {
        if self.cycles > 1 {
            if let Op::None = self.operation_next {
                return false;
            }
        }
        let correct_type = match self.fu_type {
            FUType::ALU => {
                match operation {
                    Op::Add => {
                        true
                    },
                    Op::And => {
                        true
                    },
                    Op::Or => {
                        true
                    },
                    Op::Sub => {
                        true
                    }
                    Op::Xor => {
                        true
                    }
                    Op::Mov => {
                        true
                    }
                    Op::Sl => {
                        true
                    }
                    Op::Sr => {
                        true
                    }
                    _ => {
                        false
                    }
                }
            },
            FUType::Multiplier => {
                match operation {
                    Op::Mult => {
                        true
                    },
                    Op::Div => {
                        true
                    },
                    Op::Mod => {
                        true
                    }
                    _ => false,
                }
            },
            FUType::Branch => {
                match operation {
                    Op::J => {
                        true
                    },
                    Op::Beq => {
                        true
                    },
                    Op::Beqz => {
                        true
                    },
                    Op::Blt => {
                        true
                    },
                    Op::Bgt => {
                        true
                    }
                    _ => false,
                }
            },
        };
        if correct_type {
            if self.cycles == 0 {
                self.op1 = o1;
                self.op2 = o2;
                self.addr = addr;
                self.operation = operation;
                self.rob_entry = rob_entry;
                self.set_cycles();
            }  else {
                self.op1_next = o1;
                self.op2_next = o2;
                self.addr_next = addr;
                self.operation_next = operation;
                self.rob_entry_next = rob_entry;
            }
        }
        return correct_type;
    }

    fn set_cycles(&mut self) {
        match self.fu_type {
            FUType::ALU => {
                match self.operation {
                    Op::Add => {
                        self.cycles = 1;
                    },
                    Op::And => {
                        self.cycles = 1;
                    },
                    Op::Or => {
                        self.cycles = 1;
                    },
                    Op::Sub => {
                        self.cycles = 1;
                    }
                    Op::Xor => {
                        self.cycles = 1;
                    }
                    Op::Mov => {
                        self.cycles = 1;
                    }
                    Op::Sl => {
                        self.cycles = 1;
                    }
                    Op::Sr => {
                        self.cycles = 1;
                    }
                    _ => (),
                }
            },
            FUType::Multiplier => {
                match self.operation {
                    Op::Mult => {
                        self.cycles = 2;
                    },
                    Op::Div => {
                        self.cycles = 3;
                    },
                    Op::Mod => {
                        self.cycles = 3;
                    }
                        _ => (),
                    }
            },
            FUType::Branch => {
                match self.operation {
                    Op::J => {
                        self.cycles = 1;
                    },
                    Op::Beq => {
                        self.cycles = 1;
                    },
                    Op::Beqz => {
                        self.cycles = 1;
                    },
                    Op::Blt => {
                        self.cycles = 1;
                    },
                    Op::Bgt => {
                        self.cycles = 1;
                    }
                    _ => (),
                }
            },
        };
    }

    fn cycle(&mut self) {
        if self.cycles > 0 {
            self.cycles -= 1;
            if self.cycles == 0 {
                self.result = match self.fu_type {
                    FUType::ALU => {
                        match self.operation {
                            Op::Add => {
                                Some((self.rob_entry,ExecResult::Value(self.op1 + self.op2)))
                            },
                            Op::And => {
                                Some((self.rob_entry,ExecResult::Value(self.op1 & self.op2)))
                            },
                            Op::Or => {
                                Some((self.rob_entry,ExecResult::Value(self.op1 | self.op2)))
                            },
                            Op::Sub => {
                                Some((self.rob_entry,ExecResult::Value(self.op1 - self.op2)))
                            },
                            Op::Xor => {
                                Some((self.rob_entry,ExecResult::Value(self.op1 ^ self.op2)))
                            },
                            Op::Mov => {
                                Some((self.rob_entry,ExecResult::Value(self.op1)))
                            }
                            Op::Sr => {
                                Some((self.rob_entry,ExecResult::Value(self.op1 >> self.op2)))
                            }
                            Op::Sl => {
                                Some((self.rob_entry,ExecResult::Value(self.op1 << self.op2)))
                            }
                            _ => {
                                panic!("Not an ALU operation {:?}", self.operation);
                            },
                        }
                    },
                    FUType::Multiplier => {
                        match self.operation {
                            Op::Div => {
                                if self.op2 == 0{
                                    Some((self.rob_entry,ExecResult::Value(0)))
                                } else {
                                    Some((self.rob_entry,ExecResult::Value(self.op1 / self.op2)))
                                }
                            },
                            Op::Mult => {
                                Some((self.rob_entry,ExecResult::Value(self.op1 * self.op2)))
                            },
                            Op::Mod => {
                                if self.op2 == 0 {
                                    Some((self.rob_entry,ExecResult::Value(0)))
                                } else {
                                    Some((self.rob_entry,ExecResult::Value(self.op1 % self.op2)))
                                }
                            }
                            _ => {
                                panic!("Not a MULTIPLIER operation {:?}", self.operation);
                            },
                        }
                    },
                    FUType::Branch => {
                        match self.operation {
                            Op::J => {
                                Some((self.rob_entry, ExecResult::BranchTaken(self.addr)))
                            },
                            Op::Beq => {
                                if self.op1 == self.op2 {
                                    Some((self.rob_entry, ExecResult::BranchTaken(self.addr)))
                                } else {
                                    Some((self.rob_entry, ExecResult::BranchNotTaken()))
                                }
                                
                            },
                            Op::Beqz => {
                                if self.op1 == 0 {
                                    Some((self.rob_entry, ExecResult::BranchTaken(self.addr)))
                                } else {
                                    Some((self.rob_entry, ExecResult::BranchNotTaken())) 
                                }
                                
                            },
                            Op::Blt => {
                                if self.op1 < self.op2 {
                                    Some((self.rob_entry, ExecResult::BranchTaken(self.addr)))
                                } else {
                                    Some((self.rob_entry, ExecResult::BranchNotTaken()))
                                }
                                
                            },
                            Op::Bgt => {
                                if self.op1 > self.op2 {
                                    Some((self.rob_entry, ExecResult::BranchTaken(self.addr)))
                                } else {
                                    Some((self.rob_entry, ExecResult::BranchNotTaken()))
                                }
                                
                            },
                            _ => {
                               panic!("Not a BRANCH operation {:?}", self.operation); 
                            }
                        }
                    },
                };
                
                match self.operation_next {
                    Op::None => (),
                    _ => {
                        self.operation = self.operation_next;
                        self.op1 = self.op1_next;
                        self.op2 = self.op2_next;
                        self.addr = self.addr_next;
                        self.rob_entry = self.rob_entry_next;
                        self.set_cycles();
                        self.operation_next = Op::None;
                    }
                };
            }
        }
        
    }

    fn finished(&self) -> bool {
        if let None = self.result {
            if self.cycles == 0  {
                true
            }
            else {
                false
            }
        }
        else {
            false
        }
    }

    fn get_result(&mut self) -> Option<(ExecResult, usize)> {
        if let Some((r, x)) = self.result {
            self.result = None;
            Some((x, r))
        }
        else {
            None
        }
    }

    fn reset(&mut self) {
        self.op1 = 0;
        self.op2 = 0;
        self.operation = Op::None;
        self.addr = 0;
        self.operation_next = Op::None;
        self.cycles = 0;
        self.result = None;
        self.rob_entry = 0; 
    }
}

#[derive(Debug, Copy, Clone)]
enum Operand {
    Value(u32),
    Rob(usize),
    None,
}

#[derive(Debug)]
struct ReservationStation {
    rob_entry: usize,
    o1: Operand,
    o2: Operand,
    address: usize,
    operation: Op,
    busy: bool,
    ready: bool,
}

impl ReservationStation {
    fn new() -> ReservationStation {
        ReservationStation {
            rob_entry: 0,
            o1: Operand::None,
            o2: Operand::None,
            address: 0,
            operation: Op::None,
            busy: false,
            ready: false,
        }
    }

    fn get_operands(&self) -> Option<(u32, u32)> {
        match self.o1 {
            Operand::Value(x) => {
                match self.o2 {
                    Operand::Value(y) => {
                        Some((x, y))
                    },
                    Operand::None => {
                        Some((x, 0))
                    },
                    _ => {
                        None
                    }
                }
            }
            Operand::None => {
                Some((0 , 0))
            }
            _ => {
                None
            }
        }
    }

    fn issue_branch(&mut self, operand1: Operand, operand2: Operand, operation: Op, rob_entry: usize, addr: usize) {
        self.rob_entry = rob_entry;
        self.o1 = operand1;
        self.o2 = operand2;
        self.address = addr;
        self.operation = operation;
        self.ready = self.dependencies_resolved();
        self.busy = true;
    }

    fn issue(&mut self, operand1: Operand, operand2: Operand, operation: Op, rob_entry: usize) {
        self.rob_entry = rob_entry;
        self.o1 = operand1;
        self.o2 = operand2;
        self.operation = operation;
        self.ready = self.dependencies_resolved();
        self.busy = true;
    }

    fn free(&mut self) {
        self.o1 = Operand::None;
        self.o2 = Operand::None;
        self.address = 0;
        self.operation = Op::None;
        self.busy = false;
        self.ready = false;
    }

    fn resolve_dependency(&mut self, x: u32, rob_entry: usize) {
        if let Operand::Rob(r) = self.o1 {
            if rob_entry == r {
                self.o1 = Operand::Value(x);
            }
        }
        if let Operand::Rob(r) = self.o2 {
            if rob_entry == r {
                self.o2 = Operand::Value(x);
            }
        }
        
        self.ready = self.dependencies_resolved();
    }

    fn dependencies_resolved(&self) -> bool {
        match self.o1 {
            Operand::Rob(_) => {
                false
            },
            _ => {
                match self.o2 {
                    Operand::Rob(_) => {
                        false
                    }
                    _ => {
                        true
                    }
                }
            },
        }
    }

    fn finished(&self) -> bool {
        !self.busy
    }
}

struct BranchPredictor {
    bht: [ u32 ; MAX_PREDICTIONS],
    btb: [ (usize, bool) ; MAX_PREDICTIONS],
    total_predictions: u32,
    total_correct: u32,
    pred_type: usize,
}

impl BranchPredictor {
    fn new(pred_type: usize) -> BranchPredictor {
        let bht = if pred_type == 1 {
            [0; MAX_PREDICTIONS]
        } else if pred_type == 2 {
            [1; MAX_PREDICTIONS]
        } else {
            [0; MAX_PREDICTIONS]
        };
        BranchPredictor {
            bht: bht,
            btb: [ (0, false) ; MAX_PREDICTIONS],
            total_predictions: 0,
            total_correct: 0,
            pred_type: pred_type
        }
    }
    fn accuracy(&self) -> f32 {
        self.total_correct as f32 / self.total_predictions as f32
    }

    fn predict(&mut self, instruction: EncodedInstruction, pc: usize) -> usize {
        let index = pc & 0b1111111111; //Use least significant 10 bits for index

        if self.pred_type == 0 {
            //static prediction
            match instruction {
                EncodedInstruction::J(address) => {
                    self.btb[index] = (address, true);
                    println!("Predict taken {:?} {} {}", instruction, address, pc);
                    address
                },
                _ => {
                    self.btb[index] = (pc + 1, false);
                    println!("Predict not taken {:?} {} {}", instruction, pc, pc + 1);
                    pc + 1
                }
            }
        } else { 
            match instruction {
                EncodedInstruction::J(address) => {
                    self.btb[index] = (address, true);
                    println!("Predict taken {:?} {} {}", instruction, address, pc);
                    address
                },
                EncodedInstruction::Beq(_, _, inst) => {
                    self.make_prediction(index, inst, pc)
                },
                EncodedInstruction::Beqz(_, inst) => {
                    self.make_prediction(index, inst, pc)
                },
                EncodedInstruction::Blt(_, _, inst) => {
                    self.make_prediction(index, inst, pc)
                }
                _ => {
                    self.btb[index] = (pc + 1, false);
                    println!("Predict not taken {:?} {} {}", instruction, pc, pc + 1);
                    pc + 1
                }
            }
        }
    }

    fn make_prediction(&mut self, entry: usize, inst: usize, pc: usize) -> usize {
        if self.pred_type == 0 {
            self.btb[entry] = (pc + 1, false);
            pc + 1
        } else {
            if self.bht[entry] <= ((1 << self.pred_type) - 1) / 2 {
                self.btb[entry] = (pc + 1, false);
                pc + 1
            } else {
                self.btb[entry] = (inst, true);
                inst
            }
        }
    }

    fn prediction_correct(&mut self, taken_pc: usize, pc: usize) -> bool {
        //TODO Need to remove from table also
        println!("RESOLVING PREDICTION. {} ", pc);
        self.total_predictions += 1;

        let entry = pc & 0b1111111111;

        let (predicted_pc, predicted_branch_taken) = self.btb[entry];

        if predicted_pc == taken_pc {
            if self.pred_type >= 1 {
                if predicted_branch_taken {
                    if !(self.bht[entry] == (1 << self.pred_type) - 1) {
                        self.bht[entry] += 1;
                    }
                } else {
                    if !(self.bht[entry] == 0) {
                        self.bht[entry] -= 1;
                    }
                }
            }
            println!("BRANCH CORRECTLY PREDICTED {} {} {}", predicted_pc, taken_pc, pc);
            self.total_correct += 1;
            true
        } else {
            if self.pred_type >= 1 {
                if predicted_branch_taken {
                    if !(self.bht[entry] == 0) {
                        self.bht[entry] -= 1;
                    }
                } else {
                    if !(self.bht[entry] == (1 << self.pred_type) - 1) {
                        self.bht[entry] += 1;
                    }
                }
            }
            println!("BRANCH MISPREDICTED! {} {} {}", predicted_pc, taken_pc, pc);
            false
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum ExecResult {
    Value(u32),
    BranchTaken(usize),
    BranchNotTaken(),
    Store,
}

#[derive(Debug, Copy, Clone)]
enum ReorderBufferResult {
    Writeback(u32, usize, usize),
    BranchTaken(usize, usize),
    BranchNotTaken(usize),
    Store(usize),
    None,
}

#[derive(Debug, Copy, Clone)]
struct ReorderBufferEntry {
    register: usize,
    result: Option<ExecResult>,
}

impl ReorderBufferEntry {
    fn new() -> ReorderBufferEntry {
        ReorderBufferEntry {
            register: 0,
            result: None,
        }
    }

    fn clear(&mut self) {
        self.result = None;
    }
}

#[derive(Debug)]
struct ReorderBuffer {
    instructions_committed: usize,
    commit: usize,
    issue: usize,
    buffer: [ReorderBufferEntry; ROB_SIZE],
}

impl ReorderBuffer {
    fn new() -> ReorderBuffer {
        ReorderBuffer {
            instructions_committed: 0,
            commit: 0,
            issue: 0,
            buffer: [ReorderBufferEntry::new() ; ROB_SIZE],
        }
    }

    fn is_empty(&self) -> bool {
        self.commit == self.issue
    }

    fn empty(&mut self) {
        for entry in &mut self.buffer {
            entry.clear();
        }
        self.commit = 0;
        self.issue = 0;
        self.issue = self.commit;
    }

    fn commit_to_store(&mut self,  register: usize) -> Option<usize> {
        if self.inc(self.issue) == self.commit {
            None
        } else {
            let ret = self.issue;
            self.buffer[ret].result = Some(ExecResult::Store);
            self.buffer[ret].register = register;
            self.issue = (self.issue + 1) % self.buffer.len();
            Some(ret)
        }
    }

    fn inc(&self, x: usize) -> usize {
        (x + 1) % self.buffer.len()
    }

    fn commit_to(&mut self,  register: usize) -> Option<usize> {
        if self.inc(self.issue) == self.commit {
            None
        } else {
            let ret = self.issue;
            self.buffer[ret].result = None;
            self.buffer[ret].register = register;
            self.issue = self.inc(self.issue);
            Some(ret)
        }
    }

    fn insert(&mut self, pos: usize, result: ExecResult) {
        self.buffer[pos].result = Some((result));
    }

    fn get_commit(&mut self) -> ReorderBufferResult {
        if let Some(result) = self.buffer[self.commit].result {
            self.instructions_committed += 1;
            let rob_ret = self.commit;
            let reg_ret = self.buffer[self.commit].register;
            self.buffer[self.commit].clear();
            self.commit = (self.commit + 1) % self.buffer.len();
            match result {
                ExecResult::Value(val) => {
                    ReorderBufferResult::Writeback(val, rob_ret, reg_ret)
                }
                ExecResult::BranchTaken(inst) => {
                    ReorderBufferResult::BranchTaken(inst, reg_ret)
                }
                ExecResult::BranchNotTaken() => {
                    ReorderBufferResult::BranchNotTaken(reg_ret)
                }
                ExecResult::Store => {
                    ReorderBufferResult::Store(rob_ret)
                }
            }
        } else {
            ReorderBufferResult::None
        }
    }
}

#[derive(Debug)]
struct Registers {
    gprs: [u32; 32], // 32 GPRS
    rat: [Option<usize>; 32], 
}

impl Registers {
    fn new() -> Registers {
        Registers{
            gprs: [0u32; 32],
            rat: [None; 32],
        }
    }

    fn clear_rat(&mut self) {
        for i in 0..self.rat.len() {
            self.rat[i] = None;
        }
    }

    fn set_owner(&mut self, reg: usize, new_owner: usize) {
        self.rat[reg] = Some(new_owner);
    }

    fn write_result(&mut self, value: u32, rob: usize, register: usize) {
        self.gprs[register] = value;
        if let Some(rat_entry) = self.rat[register] {
            if rat_entry == rob {
                self.rat[register] = None;
            }
        }
    }
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
    Beqz(usize, usize),
    Bgt(usize, usize, usize),
    Blt(usize, usize, usize),
    Div(usize, usize, usize),
    J(usize),
    Ldc(usize, u32),
    Lw(usize, usize),
    Mod(usize, usize, usize),
    Mov(usize, usize),
    Mult(usize, usize, usize),
    Or(usize, usize, usize),
    Sl(usize, usize, u32),
    Sr(usize, usize, u32),
    Sw(usize, usize),
    Sub(usize, usize, usize),
    Subi(usize, usize, u32),
    Xor(usize, usize, usize),
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
            "BEQZ" => {
                let (s, addr) = two_args(split_inst);
                instructions.push(EncodedInstruction::Beqz(s, addr));
            }
            "BGT" => {
                let (s, t, addr) = three_args(split_inst);
                instructions.push(EncodedInstruction::Bgt(s, t, addr));
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
    instructions
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