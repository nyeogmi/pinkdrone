use crate::{instruction::Instruction, codegen::Codegen};

#[derive(Debug)]
pub struct Object {
    pub instructions: Vec<Instruction>,
}

impl Object {
    pub fn codegen(&self, base_address: u64) -> Vec<u8> {
        let mut codegen = Codegen::new(base_address);
        for inst in self.instructions.iter() {
            codegen.write(*inst);
        }
        codegen.finalize()
    }
}