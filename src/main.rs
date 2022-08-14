use chumsky::Parser;
use instruction::Size;
use object::Object;
use pretty_hex::*;

use crate::{jit_fn::JitFn, instruction::{Instruction, Dest, Src, Count}, interpreter_fn::InterpreterFn};

mod codegen;
mod instruction;
mod interpreter_fn;
mod jit_fn;
mod object;
mod parser;

fn main() {
    // TODO: Support hex literals again
    let proc = crate::parser::parse("
        ffinyeh 0x10 ().
    ").map(|(o, s)| o);
    /*
    let proc = crate::parser::parse("
        nyeh 16 ().
        bp-4 D = 305453895.
        bp-2 H = 3562. 
        ret bp-4 D.
    ").map(|(o, s)| o);
    */

    /*
    let proc = Object {
        instructions: vec![
            Begin(16, [Dest::Nowhere, Dest::Nowhere, Dest::Nowhere, Dest::Nowhere, Dest::Nowhere, Dest::Nowhere]),
            Copy(Dest::Here(-4, Size::D), Src::Imm(0x1234db47), Count(1)),
            Copy(Dest::Here(-2, Size::H), Src::Imm(0x0dea), Count(1)),
            Ret(Src::Here(-4, Size::D)),
        ]
    };
    */

    println!("Parsed: {:#?}", proc);
    return;

    let proc = proc.unwrap();
    println!("code:\n{:?}", proc.codegen(0).hex_dump());

    let jit_bat: JitFn<(), u64> = JitFn::new(|addr| proc.codegen(addr as u64));

    let bat = unsafe { jit_bat.run(()) };
    println!("QUAKE, MORTAL. IT IS I, {:#010X}", bat);

    let interpret_bat: InterpreterFn = InterpreterFn::new(proc.instructions, 1024);

    let bat = interpret_bat.run(0, 0, 0, 0, 0, 0);
    println!("QUAKE, MORTAL. IT IS I, {:#010X}", bat);

    
    /*
    let dead_bat: JitFn<(), u32> = JitFn::new(|_| vec![
        0x55,
        0x48, 0x89, 0xe5,
        0x89, 0x7d, 0xfc,
        0xb8, 0x47, 0xdb, 0xea, 0x0d,
        0x5d,
        0xc3,
    ]);

    let bat = unsafe { dead_bat.run(()) };
    println!("QUAKE, MORTAL. IT IS I, {:#010X}", bat);
    */
}
