use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::env;
use std::collections::LinkedList;
use std::fmt;


const MEM_SIZE: usize = 52;
const NUM_RS: usize = 4;
const NUM_ALUS: usize = 1;
const NUM_MULTS: usize = 1;

fn main() {
    println!("Hello, world!");

    let args: Vec<String> = env::args().collect();

    let file = File::open(&args[1]).unwrap();

    let buf = BufReader::new(file);
    let assembly: Vec<String> = buf.lines().map(|l| l.expect("Could not parse line")).collect();

    let instructions = assemble(assembly);

    let num_instructions = instructions.len();

    //let mut memory: [u32; MEM_SIZE] = [1; MEM_SIZE];

    let mut cpu = CPU::new(instructions);

    let mut cycles = 0;

    // loop {
    //     let w_res = writeback(&mut cpu);
    //     let e_res = execute(&mut cpu);
    //     let d_res = decode(&mut cpu);
    //     let f_res = fetch(&mut cpu);
    //     println!("CYCLE");
    //     println!("");
    //     cycles += 1;
    //     if (w_res + e_res + d_res + f_res) == 0 {
    //         break;
    //     }
    // }
    //finish off last three cycles
    let mut i = 0;
    loop {
        writeback(&mut cpu);
        println!("HERE: {:?}", cpu.exec_unit.func_units[0].get_result());
        execute(&mut cpu);
        println!("HERE: {:?}", cpu.exec_unit.func_units[0].get_result());
        decode(&mut cpu);
        println!("HERE: {:?}", cpu.exec_unit.func_units[0].get_result());
        fetch(&mut cpu);
        println!("CYCLE");
        println!("");
        cycles += 1;
        println!("{:?}", cpu);
        if i > 40 {
            break;
        }
        i += 1;
    }
    println!("here1");

    cycles += 3;
    println!("{:?}", cpu.registers.gprs);
    println!("Instructions executed: {}", num_instructions);
    println!("Number of cycles: {}", cycles);
    println!("Instructions per cycle: {}", (num_instructions as f32)  / (cycles as f32));
    //println!("End: {:?}", regs);
    //for i in 0..30 {
    //    println!("MEM[{}]: {}", i, memory[i]);
    //}
}

fn fetch(cpu: &mut CPU) -> u32 {

    match cpu.fetch_unit.stalled {
        true => 1,
        false => {
            let inst = cpu.fetch_unit.get_instruction();
            match cpu.fetch_unit.get_exec_pc() {
                Some(addr) => cpu.fetch_unit.pc = addr,
                None => cpu.fetch_unit.pc += 1,
            };
    
            println!("pc: {}", cpu.fetch_unit.pc);
            println!("Fetched instruction: {:?}", inst);
            cpu.decode_unit.add_instruction(inst);
            if let EncodedInstruction::Halt = inst {
                0
            }
            else {
                1
            }
        }
    }
}

fn decode(cpu: &mut CPU) {
    match cpu.decode_unit.stalled {
        true => {
            // do nothing for now
        },
        false => {
            let possible_instruction = cpu.decode_unit.get_next_instruction();
            match possible_instruction {
                Some(instruction) => {
                    let reset = cpu.decode_unit.reset;
                    match reset {
                        true => {
                            cpu.decode_unit.reset = false;
                            //TODO properly implement reset
                        },
                        false => {
                            match instruction {
                                EncodedInstruction::Noop            => {
                                    cpu.decode_unit.pop_instruction();
                                }
                                EncodedInstruction::Halt            => {
                                    //set decode to finished
                                },
                                EncodedInstruction::Addi(d, s, imm) => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        let operand1 = cpu.read_reg(s, r);
                                        let operand2 = Operand::Value(imm);
                                        cpu.registers.set_owner(d, r);
                                        cpu.exec_unit.issue(operand1, operand2, Op::Add, r, d);
                                        cpu.decode_unit.pop_instruction();
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                },
                                EncodedInstruction::Add(d, s, t)    => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        let operand1 = cpu.read_reg(s, r);
                                        let operand2 = cpu.read_reg(t, r);
                                        cpu.registers.set_owner(d, r);
                                        cpu.exec_unit.issue(operand1, operand2, Op::Add, r, d);
                                        cpu.decode_unit.pop_instruction();
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                },
                                EncodedInstruction::And(d, s, t)    => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        let operand1 = cpu.read_reg(s, r);
                                        let operand2 = cpu.read_reg(t, r);
                                        cpu.registers.set_owner(d, r);
                                        cpu.exec_unit.issue(operand1, operand2, Op::And, r, d);
                                        cpu.decode_unit.pop_instruction();
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                },
                                EncodedInstruction::Andi(d, s, imm) => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        let operand1 = cpu.read_reg(s, r);
                                        let operand2 = Operand::Value(imm);
                                        cpu.registers.set_owner(d, r);
                                        cpu.exec_unit.issue(operand1, operand2, Op::And, r, d);
                                        cpu.decode_unit.pop_instruction();
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                },
                                EncodedInstruction::Beq(s, t, inst) => {
                                    //DecodedInstruction::Beq(registers.read_reg(s), registers.read_reg(t), inst)
                                },
                                EncodedInstruction::Blt(s, t, inst) => {
                                    //DecodedInstruction::Blt(registers.read_reg(s), registers.read_reg(t), inst)
                                },
                                EncodedInstruction::Div(d, s, t)    => {
                                    //DecodedInstruction::Div(d, registers.read_reg(s), registers.read_reg(t))
                                },
                                EncodedInstruction::J(inst)         => {
                                    //DecodedInstruction::J(inst)
                                },
                                EncodedInstruction::Ldc(d, imm)     => {
                                    //DecodedInstruction::Mov(d, imm)
                                },
                                EncodedInstruction::Li(d, imm)      => {
                                    //DecodedInstruction::Load(d, imm)
                                },
                                EncodedInstruction::Lw(d, t)        => {
                                    //DecodedInstruction::Load(d, registers.read_reg(t))
                                },
                                EncodedInstruction::Mod(d, s, t)    => {
                                    //DecodedInstruction::Mod(d, registers.read_reg(s), registers.read_reg(t))
                                },
                                EncodedInstruction::Mov(d, s)       => {
                                    //DecodedInstruction::Mov(d, registers.read_reg(s))
                                },
                                EncodedInstruction::Mult(d, s, t)   => {
                                    //DecodedInstruction::Mult(d, registers.read_reg(s), registers.read_reg(t))
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        let operand1 = cpu.read_reg(s, r);
                                        let operand2 = cpu.read_reg(t, r);
                                        cpu.registers.set_owner(d, r);
                                        cpu.exec_unit.issue(operand1, operand2, Op::Mult, r, d);
                                        cpu.decode_unit.pop_instruction();
                                    }
                                },
                                EncodedInstruction::Or(d, s, t)     => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        let operand1 = cpu.read_reg(s, r);
                                        let operand2 = cpu.read_reg(t, r);
                                        cpu.registers.set_owner(d, r);
                                        cpu.exec_unit.issue(operand1, operand2, Op::Or, r, d);
                                        cpu.decode_unit.pop_instruction();
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                },
                                EncodedInstruction::Sl(d, s, t)     => {
                                    //DecodedInstruction::Sl(d, registers.read_reg(s), t)
                                },
                                EncodedInstruction::Sr(d, s, t)     => {
                                    //DecodedInstruction::Sr(d, registers.read_reg(s), t)
                                },
                                EncodedInstruction::Sub(d, s, t)    => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        let operand1 = cpu.read_reg(s, r);
                                        let operand2 = cpu.read_reg(t, r);
                                        cpu.registers.set_owner(d, r);
                                        cpu.exec_unit.issue(operand1, operand2, Op::Sub, r, d);
                                        cpu.decode_unit.pop_instruction();
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                },
                                EncodedInstruction::Subi(d, s, imm) => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        let operand1 = cpu.read_reg(s,r);
                                        let operand2 = Operand::Value(imm);
                                        cpu.registers.set_owner(d, r);
                                        cpu.exec_unit.issue(operand1, operand2, Op::Sub, r, d);
                                        cpu.decode_unit.pop_instruction();
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                },
                                EncodedInstruction::Si(t, imm)      => {
                                    //DecodedInstruction::Store(registers.read_reg(t), imm)
                                },
                                EncodedInstruction::Sw(s, d)        => {
                                    //DecodedInstruction::Store(registers.read_reg(s), registers.read_reg(d))
                                },
                                EncodedInstruction::Xor(d, s, t)    => {
                                    if let Some(r) = cpu.exec_unit.get_free_rs() {
                                        let operand1 = cpu.read_reg(s, r);
                                        let operand2 = cpu.read_reg(t, r);
                                        cpu.registers.set_owner(d, r);
                                        cpu.exec_unit.issue(operand1, operand2, Op::Xor, r, d);
                                        cpu.decode_unit.pop_instruction();
                                    }
                                    else {
                                        // do nothing until next cycle
                                    }
                                },
                                // _ => {
                                //     panic!("{:?} is an unimplemented instruction", instruction);
                                //     EncodedInstruction::Noop
                                // }
                            }
                        },
                    };
                },
                None => (),
            };
        },
    };
}

fn execute(cpu: &mut CPU)  {

    //execute 
    println!("HERE: {:?}", cpu.exec_unit.func_units[0].get_result());
    for fu in &mut cpu.exec_unit.func_units {
        fu.cycle();
    }
    println!("HERE: {:?}", cpu.exec_unit.func_units[0].get_result());
    //now dispatch
    for rs in 0..cpu.exec_unit.rs_sts.len() {
        if cpu.exec_unit.rs_sts[rs].ready && !cpu.exec_unit.rs_sts[rs].finished && !cpu.exec_unit.rs_sts[rs].executing {
            //try to dipatch to functional unit
            for fu in &mut cpu.exec_unit.func_units {
                if !fu.is_busy() {
                    if let Operand::Value(x) = cpu.exec_unit.rs_sts[rs].o1 {
                        if let Operand::Value(y) = cpu.exec_unit.rs_sts[rs].o2 {
                            println!("Dispatching {} = {} {:?} {} from {}", cpu.exec_unit.rs_sts[rs].wb_reg, x, cpu.exec_unit.rs_sts[rs].operation,y, rs );
                            if fu.dispatch(x, y, cpu.exec_unit.rs_sts[rs].operation, rs, cpu.exec_unit.rs_sts[rs].wb_reg) {
                                cpu.exec_unit.rs_sts[rs].executing = true;
                                break; //found a functional unit to execute this RS 
                            }
                           
                        }
                    }
                    
                }
            }
        }
    }

}

fn writeback(cpu: &mut CPU) {
    cpu.cdb_busy = false;

    for rs in 0..cpu.exec_unit.rs_sts.len() {
        if cpu.exec_unit.rs_sts[rs].finished {
            cpu.exec_unit.rs_sts[rs].free();
        }
        cpu.exec_unit.rs_sts[rs].update_ready()
    }

    for fu in 0..cpu.exec_unit.func_units.len() {
        cpu.exec_unit.func_units[fu].update_busy();
        println!("FU BUSY: {:?}", cpu.exec_unit.func_units[fu].get_result());
    }


    for fu in 0..cpu.exec_unit.func_units.len() {
        if let Some((result, register, rs)) = cpu.exec_unit.func_units[fu].get_result() {
            if !cpu.cdb_busy {
                println!("WRITING BACK: {}, {}, {}", result, register, rs);
                cpu.cdb_busy = true;
                let deps = cpu.exec_unit.rs_sts[rs].dependents.clone();
                for dependent in deps {
                    if let Operand::ReservationStation(r) = cpu.exec_unit.rs_sts[dependent].o1 {
                        if rs == r {
                            cpu.exec_unit.rs_sts[dependent].o1 = Operand::Value(result);
                            cpu.exec_unit.rs_sts[dependent].ready = false;
                        }
                    }
                    if let Operand::ReservationStation(r) = cpu.exec_unit.rs_sts[dependent].o2 {
                        if rs == r {
                            cpu.exec_unit.rs_sts[dependent].o2 = Operand::Value(result);
                            cpu.exec_unit.rs_sts[dependent].ready = false;
                        }
                    }
                }
                cpu.registers.write_result(result, register, rs);
                cpu.exec_unit.func_units[fu].free();
                println!("HERE: {:?}", cpu.exec_unit.func_units[0].get_result());
                cpu.exec_unit.rs_sts[rs].finished = true;
            }
        }
    }
    println!("HERE: {:?}", cpu.exec_unit.func_units[0].get_result());
}

struct CPU {
    fetch_unit: FetchUnit,
    decode_unit: DecodeUnit,
    exec_unit: ExecUnit,
    cdb_busy: bool,
    registers: Registers,
}

impl fmt::Debug for CPU {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "cdb_busy {:}\n Fetch unit: {:?}\nDecode Unit: {:?}\nExec Unit: {:?}\nRegisters: {:?}\n", self.cdb_busy, self.fetch_unit, self.decode_unit, self.exec_unit, self.registers)
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
        }
    }

    fn read_reg(&mut self, reg: usize, rs: usize) -> Operand {
        match self.registers.rat[reg] {
            None => {
                Operand::Value(self.registers.gprs[reg])
            }
            Some(reservation) => {
                self.exec_unit.rs_sts[reservation].add_dependent(rs);
                Operand::ReservationStation(reservation)
            }
        }
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

    fn get_exec_pc(&mut self) -> Option<usize> {
        let ret = self.exec_pc;
        self.exec_pc = None;
        ret
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
    instruction_q: LinkedList<EncodedInstruction>,
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

    fn add_instruction(&mut self, instruction: EncodedInstruction) {
        self.instruction_q.push_back(instruction);
    }

    fn get_next_instruction(&self) -> Option<EncodedInstruction> {
        match self.instruction_q.front() {
            Some(x) => Some((*x).clone()),
            None => None,
        }
    }

    fn pop_instruction(&mut self) {
        self.instruction_q.pop_front();
    }
}

// trait FunctionalUnit {
//     fn can_dispatch(&self, operation: Op) -> bool;
//     fn dispatch(&mut self, o1: u32, o2: u32, operation: Op, rs: usize, reg: usize);
//     fn cycle(&mut self);
//     fn is_finished(&self) -> bool;
//     fn get_result(&self) -> Option<(u32, usize, usize)>;
//     fn is_busy(&self) -> bool;
//     fn update_busy(&mut self);
//     fn free(&mut self);
// }

struct ExecUnit {
    func_units: Vec<FunctionalUnit>,
    rs_sts: Vec<ReservationStation>,
}

impl fmt::Debug for ExecUnit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Reservation Stations {:?}\nFunctional Units: {:?}", self.rs_sts, self.func_units)
    }
}

impl ExecUnit {
    fn new() -> ExecUnit {
        let mut fus: Vec<FunctionalUnit> = Vec::new();
        for i in 0..NUM_ALUS {
            fus.push(FunctionalUnit::new(FUType::ALU));
        }
        for i in 0..NUM_MULTS {
            fus.push(FunctionalUnit::new(FUType::Multiplier));
        }

        let mut rs_sts: Vec<ReservationStation> = Vec::new();
        for i in 0..NUM_RS {
            rs_sts.push(ReservationStation::new());
        }
        ExecUnit {
            func_units: fus,
            rs_sts: rs_sts,
        }
    }

    fn get_free_rs(&self) -> Option<usize> {
        for rs in 0..self.rs_sts.len() {
            if !self.rs_sts[rs].busy {
                return Some(rs);
            }
        }
        return None;
    }

    fn issue(&mut self, o1: Operand, o2: Operand, operation: Op, rs: usize, writereg: usize) {
        self.rs_sts[rs].issue(o1, o2, operation, writereg);
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
    Load,
    Mult,
    Div,
}

#[derive(Debug, Copy, Clone)]
enum FUType {
    Multiplier,
    ALU,
}

#[derive(Debug)]
struct FunctionalUnit {
    fu_type: FUType,
    is_busy: bool,
    op1: u32,
    op2: u32,
    operation: Op,
    cycles: i32,
    result: Option<u32>,
    rs_executing: usize,
    reg_wb: usize,
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
            rs_executing: 0,
            reg_wb: 0,
        }
    }

    fn dispatch(&mut self, o1: u32, o2: u32, operation: Op, rs: usize, reg: usize) -> bool {
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
                    _ => {
                        self.cycles = 1;
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
                    _ => false,
                }
            }
        };
        if correct_type {
            self.op1 = o1;
            self.op2 = o2;
            self.operation = operation;
            self.reg_wb = reg;
            self.rs_executing = rs;
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
                                Some(self.op1 + self.op2)  
                            },
                            Op::And => {
                                Some(self.op1 & self.op2)
                            },
                            Op::Or => {
                                Some(self.op1 | self.op2)
                            },
                            Op::Sub => {
                                Some(self.op1 - self.op2)
                            },
                            Op::Xor => {
                                Some(self.op1 ^ self.op2)
                            },
                            _ => {
                                panic!("Not an ALU operation {:?}", self.operation);
                            },
                        }
                    },
                    FUType::Multiplier => {
                        match self.operation {
                            Op::Div => {
                                Some(self.op1 / self.op2)
                            },
                            Op::Mult => {
                                Some(self.op1 * self.op2)
                            },
                            _ => {
                                panic!("Not a MULTIPLIER operation {:?}", self.operation);
                            },
                        }
                    },
                };
                
            }
        }
    }

    fn is_finished(&self) -> bool {
        if let Some(_) = self.result {
            true
        }
        else {
            false
        }
    }

    fn get_result(&self) -> Option<(u32, usize, usize)> {
        if let Some(x) = self.result {
            Some((x, self.reg_wb, self.rs_executing))
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
    }
}

#[derive(Debug)]
enum Operand {
    Value(u32),
    ReservationStation(usize),
    None,
}

#[derive(Debug)]
struct ReservationStation {
    wb_reg: usize,
    o1: Operand,
    o2: Operand,
    operation: Op,
    dependents: Vec<usize>,
    busy: bool,
    ready: bool,
    finished: bool,
    executing: bool,
}

impl ReservationStation {
    fn new() -> ReservationStation {
        ReservationStation {
            wb_reg: 0,
            o1: Operand::None,
            o2: Operand::None,
            operation: Op::None,
            dependents: Vec::new(),
            busy: false,
            ready: false,
            finished: false,
            executing: false,
        }
    }

    fn issue(&mut self, operand1: Operand, operand2: Operand, operation: Op, wb_reg: usize) {
        self.wb_reg = wb_reg;
        self.o1 = operand1;
        self.o2 = operand2;
        self.operation = operation;
        self.busy = true;
    }

    fn add_dependent(&mut self, dependent: usize) {
        self.dependents.push(dependent);
    }

    fn free(&mut self) {
        self.o1 = Operand::None;
        self.o2 = Operand::None;
        self.operation = Op::None;
        self.dependents.clear();
        self.busy = false;
        self.finished = false;
        self.ready = false;
        self.executing = false;
    }

    fn update_ready(&mut self) {
        self.ready = self.dependencies_resolved();
    }

    fn dependencies_resolved(&self) -> bool {
        match self.o1 {
            Operand::Value(_) => {
                match self.o2 {
                    Operand::Value(_) => true,
                    _ => false,
                }
            },
            _ => false,
        }
    }
}

// #[derive(Debug)]
// struct ReorderBuffer {
//     oldest: usize,
//     newest: usize,
//     buffer: [Option<ExecuteResult>; 16],
// }

// impl ReorderBuffer {
//     fn new() -> ReorderBuffer {
//         ReorderBuffer {
//             oldest: 0,
//             newest: 0,
//             buffer: [None ; 16],
//         }
//     }

//     fn get_new_pos(&mut self) -> usize {
//         let ret = self.newest;
//         self.newest = (self.newest + 1) % self.buffer.len();
//         ret
//     }

//     fn insert(&mut self, pos: usize, result: ExecuteResult) -> () {
//         self.buffer[pos] = Some(result);
//     }

//     fn get_writeback(&mut self) -> ExecuteResult {
//         if let Some(result) = self.buffer[self.oldest] {
//             self.buffer[self.oldest] = None;
//             self.oldest = (self.oldest + 1) % self.buffer.len();
//             result
//         } else {
//             ExecuteResult::None
//         }
//     }
// }

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

    fn read_reg(&self, reg: usize) -> Operand {
        match self.rat[reg] {
            None => {
                Operand::Value(self.gprs[reg])
            }
            Some(reservation) => {
                Operand::ReservationStation(reservation)
            }
        }
    }

    fn set_owner(&mut self, reg: usize, new_owner: usize) {
        self.rat[reg] = Some(new_owner);
    }

    fn write_result(&mut self, value: u32, register: usize, rs: usize) {
        if self.rat[register] == Some(rs) {
            self.gprs[register] = value;
            self.rat[register] = None;
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