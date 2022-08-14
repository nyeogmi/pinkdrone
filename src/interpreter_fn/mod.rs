use std::{collections::HashMap};

use crate::instruction::{Label, Instruction, same_size, Dest, Src, Size};

pub struct InterpreterFn {  // note: always takes `u64` x 6 and returns u64
    code: Vec<Instruction>,
    stack_size: usize,

    label_locations: HashMap<Label, usize>, // index of instruction in `code`
}

impl InterpreterFn {
    pub fn new(code: Vec<Instruction>, stack_size: usize) -> Self {
        let mut label_locations =   HashMap::new();
        for (i, c) in code.iter().enumerate() {
            if let Instruction::Label(l) = c {
                label_locations.insert(*l, i);
            }
        }
        InterpreterFn {
            code, stack_size, label_locations,
        }
    }
    pub fn run(&self, arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
        let mut stack = vec![0; self.stack_size];

        let mut ip: usize = 0;
        let mut bp: usize = self.stack_size; 
        let mut sp: usize = self.stack_size; 

        fn load(stack: &Vec<u8>, bp: usize, src: Src) -> u64 {
            match src {
                Src::Uninitialized => 0x123456789abcdef0,
                Src::Imm(i) => i,
                Src::Ptr(offset_to_ptr, offset_after_ptr, sz) => 
                    load_relative(stack, load_relative(stack, bp, offset_to_ptr, Size::Q) as usize, offset_after_ptr, sz),
                Src::Here(stack_offset, sz) => load_relative(stack, bp, stack_offset, sz)
            }
        }

        fn store(stack: &mut Vec<u8>, bp: usize, dest: Dest, value: u64) {
            match dest {
                Dest::Nowhere => {}
                Dest::Ptr(offset_to_ptr, offset_after_ptr, sz) => {
                    store_relative(stack, load_relative(stack, bp, offset_to_ptr, Size::Q) as usize, offset_after_ptr, sz, value)
                }
                Dest::Here(stack_offset, sz) => store_relative(stack, bp, stack_offset, sz, value)
            }
        }

        fn load_relative(stack: &Vec<u8>, base: usize, offset: i32, size: Size) -> u64  {
            let location = (base as i32 + offset) as usize;
            match size {
                Size::B => stack[location] as u64,
                Size::H => u16::from_le_bytes(stack[location..location + 2].try_into().expect("should have room for 2 bytes")) as u64,
                Size::D => u32::from_le_bytes(stack[location..location + 4].try_into().expect("should have room for 4 bytes")) as u64,
                Size::Q => u64::from_le_bytes(stack[location..location + 8].try_into().expect("should have room for 8 bytes")) as u64,
            }
        }

        fn store_relative(stack: &mut Vec<u8>, base: usize, offset: i32, size: Size, value: u64) {
            let location = (base as i32 + offset) as usize;
            match size {
                Size::B => stack[location..location + 1].clone_from_slice(&value.to_le_bytes()[..1]),
                Size::H => stack[location..location + 2].clone_from_slice(&value.to_le_bytes()[..2]),
                Size::D => stack[location..location + 4].clone_from_slice(&value.to_le_bytes()[..4]),
                Size::Q => stack[location..location + 8].clone_from_slice(&value.to_le_bytes()[..8]),
            }
        }

        loop {
            if !(0..self.code.len()).contains(&ip) { 
                panic!("instruction pointer escaped"); 
            }

            match self.code[ip] {
                Instruction::Begin(n_bytes, args) => {
                    sp = bp - n_bytes as usize;
                    store(&mut stack, bp, args[0], arg0);
                    store(&mut stack, bp, args[1], arg1);
                    store(&mut stack, bp, args[2], arg2);
                    store(&mut stack, bp, args[3], arg3);
                    store(&mut stack, bp, args[4], arg4);
                    store(&mut stack, bp, args[5], arg5);
                }
                Instruction::Ret(src) => {
                    return load(&mut stack, bp, src);
                }
                Instruction::Copy(dest, src, count) => {
                    if count.0 != 1 { assert!(same_size(dest, src)); }
                    for i in 0..count.0 {
                        let val = load(&stack, bp, src);
                        store(&mut stack, bp, dest, val);
                    }
                }
                Instruction::JIf(src, label) => {
                    if load(&stack, bp, src) != 0 {
                        ip = *self.label_locations.get(&label).expect("label must be defined");
                        continue;
                    }
                }
                Instruction::Label(_) => {}
                Instruction::FFICall(dest, args, func) => {
                    let result = func(
                        load(&stack, bp, args[0]), load(&stack, bp, args[1]), load(&stack, bp, args[2]),
                        load(&stack, bp, args[3]), load(&stack, bp, args[4]), load(&stack, bp, args[5])
                    );
                    store(&mut stack, bp, dest, result)
                }
            }

            ip += 1;
        }
    }
}