use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::env;
use std::collections::LinkedList;
use std::fmt;


const MEM_SIZE: usize = 52;
const NUM_RS: usize = 16;
const NUM_ALUS: usize = 2;
const NUM_MULTS: usize = 2;
const MAX_PREDICTIONS: usize = 10;

fn main() {
    println!("Hello, world!");

    let args: Vec<String> = env::args().collect();

    let file = File::open(&args[1]).unwrap();

    let buf = BufReader::new(file);
    let assembly: Vec<String> = buf.lines().map(|l| l.expect("Could not parse line")).collect();

    let instructions = assemble(assembly);

    let mut memory: [u32; MEM_SIZE] = [1; MEM_SIZE];

    // for i in 0..MEM_SIZE {
    //     memory[i] = i as u32;
    // }

    let mut cpu = CPU::new(instructions);

    let mut cycles = 0;

    loop {
        let c_res = commit(&mut cpu);
        writeback(&mut cpu);
        let e_res = execute(&mut cpu, &mut memory);
        let d_res = decode(&mut cpu);
        let f_res = fetch(&mut cpu);
        println!("CYCLE {}", cycles);
        println!("{} : {} : {} : {}", c_res, e_res, d_res, f_res);
        println!("");
        println!("CPU: {:?}", cpu);
        cycles += 1;
        for i in memory.iter() {
            println!("{}", i);
        }
        if (c_res + e_res + d_res + f_res) == 0 {
            break;
        }
    }

    
    println!("Registers Final Values: {:?}", cpu.registers.gprs);
    println!("Instructions executed: {}", cpu.rob.instructions_committed);
    println!("Number of cycles: {}", cycles);
    println!("Instructions per cycle: {:.2}", (cpu.rob.instructions_committed as f32)  / (cycles as f32));
    println!("Branch prediction accuracy: {:.2}", cpu.branch_predictor.accuracy());
    
    for i in memory.iter() {
        println!("{}", i);
    }
    //println!("End: {:?}", regs);
    //for i in 0..30 {
    //    println!("MEM[{}]: {}", i, memory[i]);
    //}
}

fn fetch(cpu: &mut CPU) -> u32 {

    match cpu.fetch_unit.reset {
        true => {
            cpu.fetch_unit.reset = false;

            1
        },
        false => {
            let inst = cpu.fetch_unit.get_instruction();
            cpu.decode_unit.add_instruction(inst, cpu.fetch_unit.pc);
            cpu.fetch_unit.pc += 1;

            println!("pc: {}", cpu.fetch_unit.pc);
            println!("Fetched instruction: {:?}", inst);
            
            if let EncodedInstruction::Halt = inst {
                0
            }
            else {
                1
            }
        }
    }
}

fn decode(cpu: &mut CPU) -> u32 {
    let ret = match cpu.decode_unit.stalled {
        true => {
            0
        },
        false => {
            let possible_instruction = cpu.decode_unit.get_next_instruction();
            match possible_instruction {
                Some((pc, instruction)) => {
                    let reset = cpu.decode_unit.reset;
                    match reset {
                        true => {
                            cpu.decode_unit.reset = false;
                            1
                        },
                        false => {
                            match instruction {
                                EncodedInstruction::Noop            => {
                                    cpu.decode_unit.pop_instruction();
                                    1
                                }
                                EncodedInstruction::Halt            => {
                                    cpu.decode_unit.pop_instruction();
                                    0
                                },
                                EncodedInstruction::Addi(d, s, imm) => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = Operand::Value(imm);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Add, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Add(d, s, t)    => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = cpu.get_operand(t);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Add, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::And(d, s, t)    => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = cpu.get_operand(t);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::And, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Andi(d, s, imm) => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = Operand::Value(imm);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::And, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Beq(s, t, inst) => {
                                    //DecodedInstruction::Beq(registers.get_operand(s), registers.get_operand(t), inst)
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(0) {
                                            let predict_taken = cpu.branch_predictor.predict(Op::Beq, rob_pos, inst, pc + 1);
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = cpu.get_operand(t);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Beq, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                            //TODO SET THE PC IF PREDICTION IS TRUE
                                            if predict_taken {
                                                cpu.fetch_unit.speculate(inst);
                                            }
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Beqz(s, inst) => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(0) {
                                            let predict_taken = cpu.branch_predictor.predict(Op::Blt, rob_pos, inst, pc + 1);
                                            let operand1 = cpu.get_operand(s);
                                            cpu.exec_unit.issue(operand1, Operand::None, Op::Beqz, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                            //TODO SET THE PC IF PREDICTION IS TRUE
                                            if predict_taken {
                                                cpu.fetch_unit.speculate(inst);
                                            }
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                }
                                EncodedInstruction::Blt(s, t, inst) => {
                                    //DecodedInstruction::Blt(registers.get_operand(s), registers.get_operand(t), inst)
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(0) {
                                            let predict_taken = cpu.branch_predictor.predict(Op::Blt, rob_pos, inst, pc + 1);
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = cpu.get_operand(t);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Blt, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                            //TODO SET THE PC IF PREDICTION IS TRUE
                                            if predict_taken {
                                                cpu.fetch_unit.speculate(inst);
                                            }
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Div(d, s, t)    => {
                                    //DecodedInstruction::Div(d, registers.get_operand(s), registers.get_operand(t))
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = cpu.get_operand(t);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Div, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    1
                                },
                                EncodedInstruction::J(inst)         => {
                                    //DecodedInstruction::J(inst)
                                     if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(0) {
                                            let predict_taken = cpu.branch_predictor.predict(Op::J, rob_pos, inst, pc + 1);
                                            cpu.exec_unit.issue(Operand::None, Operand::None, Op::J, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                            //TODO SET THE PC IF PREDICTION IS TRUE
                                            if predict_taken {
                                                cpu.fetch_unit.speculate(inst);
                                            }
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Ldc(d, imm)     => {
                                    //DecodedInstruction::Mov(d, imm)
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(Operand::Value(imm), Operand::None, Op::Mov, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    1
                                },
                                EncodedInstruction::Lw(addr, val)        => {
                                    //DecodedInstruction::Load(d, registers.get_operand(t))
                                    println!("LW DECODE");
                                    if let Some(rob_pos) = cpu.rob.commit_to(val) {
                                        let operand1 = cpu.get_operand(addr);
                                        cpu.registers.set_owner(val, rob_pos);
                                        cpu.lsq.issue(LSQOp::L, pc, rob_pos, operand1, Operand::None);
                                        cpu.decode_unit.pop_instruction();
                                    } else {
                                        println!("NO ROB SPACE");
                                    }
                                    1
                                },
                                EncodedInstruction::Mod(d, s, t)    => {
                                    //DecodedInstruction::Mod(d, registers.get_operand(s), registers.get_operand(t))
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = cpu.get_operand(t);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Mod, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    1
                                },
                                EncodedInstruction::Mov(d, s)       => {
                                    //DecodedInstruction::Mov(d, registers.get_operand(s))
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, Operand::None, Op::Mov, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    1
                                },
                                EncodedInstruction::Mult(d, s, t)   => {
                                    //DecodedInstruction::Mult(d, registers.get_operand(s), registers.get_operand(t))
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = cpu.get_operand(t);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Mult, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    1
                                },
                                EncodedInstruction::Or(d, s, t)     => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = cpu.get_operand(t);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Or, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Sl(d, s, t)     => {
                                    //DecodedInstruction::Sl(d, registers.get_operand(s), t)
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, Operand::Value(t), Op::Sl, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Sr(d, s, t)     => {
                                    //DecodedInstruction::Sr(d, registers.get_operand(s), t)
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, Operand::Value(t), Op::Sr, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Sub(d, s, t)    => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = cpu.get_operand(t);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Sub, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Subi(d, s, imm) => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = Operand::Value(imm);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Sub, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                                EncodedInstruction::Sw(addr, dest)        => {
                                    if let Some(rob_pos) = cpu.rob.commit_to_store(dest) {
                                        let operand1 = cpu.get_operand(addr);
                                        let operand2 = cpu.get_operand(dest);
                                        cpu.registers.set_owner(dest, rob_pos);
                                        cpu.lsq.issue(LSQOp::S, pc, rob_pos, operand1, operand2);
                                        cpu.decode_unit.pop_instruction();
                                    }
                                    1
                                },
                                EncodedInstruction::Xor(d, s, t)    => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        if let Some(rob_pos) = cpu.rob.commit_to(d) {
                                            let operand1 = cpu.get_operand(s);
                                            let operand2 = cpu.get_operand(t);
                                            cpu.registers.set_owner(d, rob_pos);
                                            cpu.exec_unit.issue(operand1, operand2, Op::Xor, r, rob_pos);
                                            cpu.decode_unit.pop_instruction();
                                        }
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                    1
                                },
                            }
                        },
                    }
                },
                None => 0,
            }
        },
    };

    //now dispatch
    for rs in 0..cpu.exec_unit.rs_sts.len() {
        if cpu.exec_unit.rs_sts[rs].ready {
            //try to dipatch to functional unit
            for fu in &mut cpu.exec_unit.func_units {
                if !fu.is_busy() {
                    match cpu.exec_unit.rs_sts[rs].o1 {
                        Operand::Value(x) => {
                            match cpu.exec_unit.rs_sts[rs].o2 {
                                Operand::Value(y) => {
                                    println!("Dispatching {} = {} {:?} {} from {}", cpu.exec_unit.rs_sts[rs].rob_entry, x, cpu.exec_unit.rs_sts[rs].operation, y, rs );
                                    if fu.dispatch(x, y, cpu.exec_unit.rs_sts[rs].operation, cpu.exec_unit.rs_sts[rs].rob_entry) {
                                        cpu.exec_unit.rs_sts[rs].free();
                                        break; //found a functional unit to execute this RS 
                                    }
                                },
                                Operand::None => {
                                    println!("Dispatching {} = {:?} {} from {}", cpu.exec_unit.rs_sts[rs].rob_entry, cpu.exec_unit.rs_sts[rs].operation, x, rs );
                                    if fu.dispatch(x, 0, cpu.exec_unit.rs_sts[rs].operation, cpu.exec_unit.rs_sts[rs].rob_entry) {
                                        cpu.exec_unit.rs_sts[rs].free();
                                        break; //found a functional unit to execute this RS 
                                    }
                                }
                                _ => {
                                    panic!("OPERANDS INCORRECT1");
                                    //break;
                                },
                            };
                        },
                        Operand::None => {
                            match cpu.exec_unit.rs_sts[rs].o2 {
                                Operand::None => {
                                    if fu.dispatch(0, 0, cpu.exec_unit.rs_sts[rs].operation, cpu.exec_unit.rs_sts[rs].rob_entry) {
                                        cpu.exec_unit.rs_sts[rs].free();
                                        break; //found a functional unit to execute this RS 
                                    }
                                },
                                _ => {
                                    panic!("OPERANDS INCORRECT2"); 
                                },
                            };
                        },
                        _ => (),
                    }; 
                }
            }
        }
    }

    //Now check the LSQ if something can be executed

    if !cpu.exec_unit.mem_unit.busy {
        if let Some(i) = cpu.lsq.get_next_instruction() {
            cpu.exec_unit.mem_unit.dispatch(i);
        }
    }
    
    return ret;
}

fn execute(cpu: &mut CPU, memory: &mut [u32; MEM_SIZE]) -> u32 {

    for fu in &mut cpu.exec_unit.func_units {
        fu.cycle();
    }

    cpu.exec_unit.mem_unit.cycle(memory);

    if cpu.exec_unit.finished() && cpu.lsq.finished() {
        0
    }
    else {
        1
    }

}

fn writeback(cpu: &mut CPU) {
    cpu.cdb_busy = false;

    for fu in 0..cpu.exec_unit.func_units.len() {
        if let Some((result, rob_entry)) = cpu.exec_unit.func_units[fu].get_result() {
            if !cpu.cdb_busy {
                cpu.cdb_busy = true;
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

                //cpu.registers.write_result(result, rob_entry);
                cpu.exec_unit.func_units[fu].free();
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

fn commit(cpu: &mut CPU) -> u32 {

    for fu in 0..cpu.exec_unit.func_units.len() {
        cpu.exec_unit.func_units[fu].update_busy();
    }

    match cpu.rob.get_commit() {
        ReorderBufferResult::Writeback(res, rob, reg) => {
            cpu.registers.write_result(res, rob, reg);
        },
        ReorderBufferResult::Branch(branch_taken, rob) => {
            //ROB also beign used to store predicted PC for branches
            //If not equal then a misprediction occurred
            let (correctly_predicted, pc) = cpu.branch_predictor.resolve_prediction(branch_taken, rob);
            // IF not correctly predicted
            println!("Prediction correct: {} {}", correctly_predicted, pc);
            if !correctly_predicted {
                //Need to clear RSs, FUs, Instruction Queue
                cpu.reset();
                //Also need to set the PC correctly
                cpu.fetch_unit.mispredict(pc);
                //need to let branch predictor know of incorrect prediction
                
            }
        },
        ReorderBufferResult::Store(r) => {
            cpu.lsq.committed(r);
        }
        ReorderBufferResult::None => (),
    };
    
    if cpu.rob.is_empty() {
        0
    } else {
        1
    }
}

struct CPU {
    fetch_unit: FetchUnit,
    decode_unit: DecodeUnit,
    exec_unit: ExecUnit,
    cdb_busy: bool,
    registers: Registers,
    rob: ReorderBuffer,
    branch_predictor: BranchPredictor,
    lsq: LSQ,
}

impl fmt::Debug for CPU {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "cdb_busy {:}\n\nFetch unit: {:?}\n\nDecode Unit: {:?}\n\nExec Unit: {:?}\n\nRegisters: {:?}\n\nROB: {:?}\n\nPrediction Unit: {:?}\n\nLSQ: {:?}", self.cdb_busy, self.fetch_unit, self.decode_unit, self.exec_unit, self.registers, self.rob, self.branch_predictor, self.lsq)
    }
}

impl CPU {
    fn new(instructions: Vec<EncodedInstruction>) -> CPU {
        CPU {
            fetch_unit: FetchUnit::new(instructions),
            decode_unit: DecodeUnit::new(),
            exec_unit: ExecUnit::new(),
            cdb_busy: false,
            registers: Registers::new(),
            rob: ReorderBuffer::new(),
            branch_predictor: BranchPredictor::new(),
            lsq: LSQ::new(),
        }
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
        println!("Should defs happen");
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

    fn speculate(&mut self, spec_pc: usize) {
        self.reset = true;
        self.pc = spec_pc;
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
        self.mem_unit.reset();
    }

    fn finished(&mut self) -> bool {
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
        self.lsq.clear();
    }

    fn issue(&mut self, op: LSQOp, pc: usize, rob_entry: usize, addr: Operand, value: Operand) {
        self.lsq.push_back(LSQEntry::new(op, pc, rob_entry, addr, value));
    }

    fn resolve_dependency(&mut self, result: u32, rob_entry: usize) {
        println!("MEM DEP RES: {} {}", result, rob_entry);
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
    busy: bool,
}

impl MemoryUnit {

    fn new() -> MemoryUnit {
        MemoryUnit {
            instruction: LSQEntry::new(LSQOp::S, 0, 0, Operand::None, Operand::None),
            cycles: 0,
            result: None,
            busy: false,
        }
    }

    fn reset(&mut self) {
        println!("MEM UNIT RESET");
        self.cycles = 0;
        self.result = None;
        self.busy = false;
    }

    fn finished(&self) -> bool {
        !self.busy
    }

    fn dispatch(&mut self, next_instruction: LSQEntry) {
        println!("Mem dispatch: {:?}", next_instruction);
        self.instruction = next_instruction;
        self.result = None;
        self.busy = true;
        self.cycles = 3;
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
                                //NOW need to set busy to false
                                self.busy = false;
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
                self.busy = false;
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
    is_busy: bool,
    op1: u32,
    op2: u32,
    operation: Op,
    cycles: i32,
    result: Option<ExecResult>,
    rob_entry: usize,
}

impl FunctionalUnit {
    fn new(fu_type: FUType) -> FunctionalUnit {
        FunctionalUnit {
            fu_type: fu_type,
            is_busy: false,
            op1: 0,
            op2: 0,
            operation: Op::None,
            cycles: 0,
            result: None,
            rob_entry: 0,
        }
    }

    fn dispatch(&mut self, o1: u32, o2: u32, operation: Op, rob_entry: usize) -> bool {
        let correct_type = match self.fu_type {
            FUType::ALU => {
                match operation {
                    Op::Add => {
                        self.cycles = 1;
                        true
                    },
                    Op::And => {
                        self.cycles = 1;
                        true
                    },
                    Op::Or => {
                        self.cycles = 1;
                        true
                    },
                    Op::Sub => {
                        self.cycles = 1;
                        true
                    }
                    Op::Xor => {
                        self.cycles = 1;
                        true
                    }
                    Op::Mov => {
                        self.cycles = 1;
                        true
                    }
                    Op::Sl => {
                        self.cycles = 1;
                        true
                    }
                    Op::Sr => {
                        self.cycles = 1;
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
                        self.cycles = 2;
                        true
                    },
                    Op::Div => {
                        self.cycles = 3;
                        true
                    },
                    Op::Mod => {
                        self.cycles = 3;
                        true
                    }
                    _ => false,
                }
            },
            FUType::Branch => {
                match operation {
                    Op::J => {
                        println!("I ACCEPT DEAR SIR");
                        self.cycles = 1;
                        true
                    },
                    Op::Beq => {
                        println!("I ACCEPT DEAR SIR");
                        self.cycles = 1;
                        true
                    }
                    Op::Beqz => {
                        println!("I ACCEPT DEAR SIR");
                        self.cycles = 1;
                        true
                    }
                    Op::Blt => {
                        self.cycles = 1;
                        true
                    }
                    _ => false,
                }
            },
        };
        if correct_type {
            println!("DISPATCHING {:?}", operation);
            self.op1 = o1;
            self.op2 = o2;
            self.is_busy = true;
            self.operation = operation;
            self.rob_entry = rob_entry;
        }
        return correct_type;
    }

    fn cycle(&mut self) {
        if self.cycles > 0 {
            self.cycles -= 1;
            if self.cycles == 0 {
                self.result = match self.fu_type {
                    FUType::ALU => {
                        match self.operation {
                            Op::Add => {
                                Some(ExecResult::Value(self.op1 + self.op2))
                            },
                            Op::And => {
                                Some(ExecResult::Value(self.op1 & self.op2))
                            },
                            Op::Or => {
                                Some(ExecResult::Value(self.op1 | self.op2))
                            },
                            Op::Sub => {
                                Some(ExecResult::Value(self.op1 - self.op2))
                            },
                            Op::Xor => {
                                Some(ExecResult::Value(self.op1 ^ self.op2))
                            },
                            Op::Mov => {
                                Some(ExecResult::Value(self.op1))
                            }
                            Op::Sr => {
                                Some(ExecResult::Value(self.op1 >> self.op2))
                            }
                            Op::Sl => {
                                Some(ExecResult::Value(self.op1 << self.op2))
                            }
                            _ => {
                                panic!("Not an ALU operation {:?}", self.operation);
                            },
                        }
                    },
                    FUType::Multiplier => {
                        match self.operation {
                            Op::Div => {
                                Some(ExecResult::Value(self.op1 / self.op2))
                            },
                            Op::Mult => {
                                Some(ExecResult::Value(self.op1 * self.op2))
                            },
                            Op::Mod => {
                                Some(ExecResult::Value(self.op1 % self.op2))
                            }
                            _ => {
                                panic!("Not a MULTIPLIER operation {:?}", self.operation);
                            },
                        }
                    },
                    FUType::Branch => {
                        match self.operation {
                            Op::J => {
                                println!("HEREALJBDHGACHAHIUD:UHDWOH:OWHJD"); 
                                Some(ExecResult::Branch(true))
                            },
                            Op::Beq => {
                                println!("BEQ {} =? {}", self.op1, self.op2);
                                Some(ExecResult::Branch(self.op1 == self.op2))
                            },
                            Op::Beqz => {
                                Some(ExecResult::Branch(self.op1 == 0))
                            }
                            Op::Blt => {
                                Some(ExecResult::Branch(self.op1 < self.op2))
                            }
                            _ => {
                               panic!("Not a BRANCH operation {:?}", self.operation); 
                            }
                        }
                    },
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

    fn get_result(&self) -> Option<(ExecResult, usize)> {
        if let Some(x) = self.result {
            Some((x, self.rob_entry))
        }
        else {
            None
        }
    }
    
    fn is_busy(&self) -> bool {
        self.is_busy
    }

    fn update_busy(&mut self) {
        if let None = self.result {
            if self.cycles == 0{
                self.is_busy = false;
            }
            else{
                self.is_busy = true;
            } 
        }
        else {
            self.is_busy = true;
        }
    }

    fn free(&mut self) {
        self.result = None;
        self.is_busy = false; 
    }

    fn reset(&mut self) {
        self.op1 = 0;
        self.op2 = 0;
        self.operation = Op::None;
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
            operation: Op::None,
            busy: false,
            ready: false,
        }
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

#[derive(Debug, Copy, Clone)]
struct Prediction {
    predict_taken: bool,
    taken_pc: usize,
    not_taken_pc: usize,
}

impl Prediction {
    fn new(prediction: bool, taken: usize, not_taken: usize) -> Prediction {
        Prediction {
            predict_taken: prediction,
            taken_pc: taken,
            not_taken_pc: not_taken,
        }
    }
}

#[derive(Debug)]
struct BranchPredictor {
    table: [Option<(usize, Prediction)>; 10],
    total_predictions: u32,
    total_correct: u32,
}

impl BranchPredictor {
    fn new() -> BranchPredictor {
        BranchPredictor {
            table: [None; 10],
            total_predictions: 0,
            total_correct: 0,
        }
    }

    fn accuracy(&self) -> f32 {
        self.total_correct as f32 / self.total_predictions as f32
    }

    fn insert(&mut self, rob_entry: usize, prediction: Prediction) {
        for i in 0..self.table.len() {
            if let None = self.table[i] {
                self.table[i] = Some((rob_entry, prediction));
                break;
            }
        }
    }

    fn predict(&mut self, operation: Op, rob_pos: usize, taken_pc: usize, not_taken_pc: usize) -> bool {
        match operation {
            Op::J => {
                self.insert(rob_pos, Prediction::new(true, taken_pc, not_taken_pc));
                true
            },
            Op::Beq => {
                self.insert(rob_pos, Prediction::new(false, taken_pc, not_taken_pc));
                false 
            }
            Op::Beqz => {
                self.insert(rob_pos, Prediction::new(false, taken_pc, not_taken_pc));
                false 
            }
            Op::Blt => {
                self.insert(rob_pos, Prediction::new(false, taken_pc, not_taken_pc));
                false
            }
            _ => {
                panic!("Not implemented yet {:?}", operation);
                //false
            }
        }
    }

    fn resolve_prediction(&mut self, taken: bool, rob_pos: usize) -> (bool, usize) {
        //TODO Need to remove from table also
        self.total_predictions += 1;
        for entry in 0..self.table.len() {
            if let Some((rob, prediction)) = self.table[entry] {
                if rob == rob_pos {
                    if prediction.predict_taken == taken {
                        self.total_correct += 1;
                        self.table[entry] = None;
                        return (true, 0);
                    } else {
                        if prediction.predict_taken {
                            //predicted true but actually came to be false
                            println!("BRANCH MISPREDICTED! Guessed taken.");
                            self.table[entry] = None;
                            return (false, prediction.not_taken_pc);
                        }
                        else {
                            //predict false but was true
                            println!("BRANCH MISPREDICTED! Guessed not taken.");
                            self.table[entry] = None;
                            return (false, prediction.taken_pc)
                        }
                    }
                }
            }
        }
        panic!("No entry for this {} {}", rob_pos, taken);
    }

}

#[derive(Debug, Copy, Clone)]
enum ExecResult {
    Value(u32),
    Branch(bool),
    Store,
}

#[derive(Debug, Copy, Clone)]
enum ReorderBufferResult {
    Writeback(u32, usize, usize),
    Branch(bool, usize),
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
    buffer: [ReorderBufferEntry; 32],
}

impl ReorderBuffer {
    fn new() -> ReorderBuffer {
        ReorderBuffer {
            instructions_committed: 0,
            commit: 0,
            issue: 0,
            buffer: [ReorderBufferEntry::new() ; 32],
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
                ExecResult::Branch(branch) => {
                    ReorderBufferResult::Branch(branch, rob_ret)
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

    fn read_reg(&self, reg: usize) -> Operand {
        match self.rat[reg] {
            None => {
                Operand::Value(self.gprs[reg])
            }
            Some(rob) => {
                Operand::Rob(rob)
            }
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