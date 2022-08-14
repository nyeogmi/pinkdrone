use std::collections::HashMap;

use crate::instruction::{Instruction, Dest, Src, Size, Label, same_size};


struct LabelReference {
    at: usize,
    label: Label,

    // if relative, then this is 32 bit and well, relative
    // otherwise, it's absolute (64 bit)
    relative_to: Option<usize>
}


pub struct Codegen {
    base_address: u64,
    code: Vec<u8>,

    label_references: Vec<LabelReference>,
    label_locations: HashMap<Label, usize>,
}

impl Codegen {
    pub fn new(base_address: u64) -> Self {
        Codegen { base_address, code: vec![], label_references: vec![], label_locations: HashMap::new() }
    }

    pub fn finalize(mut self) -> Vec<u8> {
        for i in self.label_references {
            let location = *self.label_locations.get(&i.label).expect("label not defined");

            if let Some(rel) = i.relative_to {
                let offset = ((location as isize) - (rel as isize)) as i32;
                let bytes: [u8; 4] = offset.to_le_bytes();
                self.code[i.at..i.at + 4].clone_from_slice(&bytes)
            } else {
                let bytes = location.to_le_bytes();
                self.code[i.at..i.at + 8].clone_from_slice(&bytes);
            }
        }
        return self.code;
    }

    pub fn write(&mut self, instruction: Instruction) {
        match instruction {
            Instruction::Begin(n_bytes, args) => {
                // prologue
                self.code.extend([
                    // push rbp,
                    0x55,
                    // mov rbp, rsp
                    0x48, 0x89, 0xe5,
                ]);

                // alloc bytes needed
                if n_bytes > 0 {
                    // subtract the number of bytes needed from rsp
                    self.code.extend([0x48, 0x81, 0xec]);
                    self.code.extend((n_bytes as u32).to_le_bytes());
                }

                // mov rax, {rdi, rsi, rdx, rcx, r8, r9}
                let arg0_impl: &[u8] = b"\x48\x89\xf8";
                let arg1_impl: &[u8] = b"\x48\x89\xf0";
                let arg2_impl: &[u8] = b"\x48\x89\xd0";
                let arg3_impl: &[u8] = b"\x48\x89\xc8";
                let arg4_impl: &[u8] = b"\x4c\x89\xc0";
                let arg5_impl: &[u8] = b"\x4c\x89\xc8";

                for (arg, bytecode) in [
                    (args[0], arg0_impl),
                    (args[1], arg1_impl),
                    (args[2], arg2_impl),
                    (args[3], arg3_impl),
                    (args[4], arg4_impl),
                    (args[5], arg5_impl),
                ] {
                    if arg.needs_store() {
                        self.code.extend(bytecode);
                        self.store_rax(arg);
                    }
                }
            }
            Instruction::Ret(src) => {
                // return value is a u64
                self.load_rax(src);

                // epilogue
                self.code.extend([
                    // mov rsp, rbp
                    0x48, 0x89, 0xec,
                    // pop rbp
                    0x5d,
                    // ret
                    0xc3,
                ])
            }

            // TODO: String ops?
            Instruction::Copy(dest, src, count) => {
                if count.0 != 1 { assert!(same_size(dest, src)); }
                if dest.needs_store() {
                    for i in 0..count.0 {
                        self.load_rax(src.offset(i as i32));
                        self.store_rax(dest.offset(i as i32))
                    }
                }
            }
            Instruction::JIf(Src::Imm(0), _) => { /* generate nothing -- label can't be reached */ },
            Instruction::JIf(Src::Imm(_), label) => { 
                // jmp
                self.code.extend([0xe9]);
                let at = self.code.len();
                self.code.extend([0x00, 0x00, 0x00, 0x00]);
                let relative_to = Some(self.code.len());
                self.label_references.push(LabelReference {at, label, relative_to})
            }
            Instruction::JIf(src, label) => {
                self.load_rax(src);

                // test rax, rax
                self.code.extend([0x48, 0x85, 0xc0]);

                // jnz
                self.code.extend([0x0f, 0x85]);
                let at = self.code.len();
                self.code.extend([0x00, 0x00, 0x00, 0x00]);
                let relative_to = Some(self.code.len());
                self.label_references.push(LabelReference {at, label, relative_to})
            }

            Instruction::Label(label) => {
                let existing = self.label_locations.insert(label, self.code.len());
                if  let Some(_) = existing {
                    panic!("label defined twice: {:?}", label) // TODO: More graceful way to save the error for later
                } 
            }

            Instruction::FFICall(dest, args, function) => {
                self.write_fficall(dest, args, function as u64);
            }
        }
    }

    fn write_fficall(&mut self, dest: Dest, args: [Src; 6], function: u64) {
        // push rdi;   mov rdi, rax
        let arg0_impl: &[u8] = b"\x57\x48\x89\xc7";

        // push rsi;   mov rsi, rax
        let arg1_impl = b"\x56\x48\x89\xc6";

        // push rdx;   mov rdx, rax
        let arg2_impl = b"\x52\x48\x89\xc2";

        // push rcx;   mov rcx, rax
        let arg3_impl = b"\x51\x48\x89\xc1";

        // push r8;  mov r8, rax
        let arg4_impl = b"\x41\x50\x49\x89\xc0";

        // push r9;  mov r9, rax
        let arg5_impl = b"\x41\x51\x49\x89\xc1";

        for (arg, bytecode) in [
            (args[0], arg0_impl),
            (args[1], arg1_impl),
            (args[2], arg2_impl),
            (args[3], arg3_impl),
            (args[4], arg4_impl),
            (args[5], arg5_impl),
        ] {
            if arg.needs_load() {
                self.load_rax(arg);
                self.code.extend(bytecode);
            }
        }

        // mov rax, <address of function>
        self.code.extend([0x48, 0xb8]);
        self.code.extend(function.to_le_bytes());

        // call rax
        self.code.extend([0xff, 0xd0]);

        // mov dest, rax
        if dest.needs_store() {
            self.store_rax(dest)
        }

        // pop rdi
        let arg0_cleanup: &[u8] = b"\x5f";

        // pop rsi
        let arg1_cleanup: &[u8] = b"\x5e";

        // pop rdx
        let arg2_cleanup: &[u8] = b"\x5a";

        // pop rcx
        let arg3_cleanup: &[u8] = b"\x59";

        // pop r8
        let arg4_cleanup: &[u8] = b"\x41\x58";

        // pop r9
        let arg5_cleanup: &[u8] = b"\x41\x59";

        for (arg, bytecode) in [
            (args[5], arg5_cleanup),
            (args[4], arg4_cleanup),
            (args[3], arg3_cleanup),
            (args[2], arg2_cleanup),
            (args[1], arg1_cleanup),
            (args[0], arg0_cleanup)
        ] {
            if arg.needs_load() {
                self.code.extend(bytecode);
            }
        }
    }

    fn load_rax(&mut self, src: Src) {
        match src {
            Src::Uninitialized => {}
            Src::Imm(0) => {
                // xor eax, eax
                // also clears the top 32 bits
                self.code.extend([0x31, 0xc0])
            }
            Src::Imm(x) => {
                // TODO: Special case < u32 max value as mov eax instead

                // mov rax, ...
                self.code.extend([0x48, 0xb8]);
                self.code.extend((x as u64).to_le_bytes());
            }
            Src::Ptr(offset_to_ptr, offset_after_ptr, sz) => {
                // mov rax, rbp
                self.code.extend([0x48, 0x89, 0xe8]);
                self.load_relative_to_rax(offset_to_ptr, Size::Q);
                // now the pointer is in rax
                self.load_relative_to_rax(offset_after_ptr, sz)
            }
            Src::Here(stack_offset, sz) => {
                // mov rax, rbp
                self.code.extend([0x48, 0x89, 0xe8]);
                self.load_relative_to_rax(stack_offset, sz)
            }
        }
    }

    fn load_relative_to_rax(&mut self, offset: i32, sz: Size) {
        if offset == 0 {
            match sz {
                // movzx eax, BYTE PTR [rax]
                Size::B => self.code.extend([0x0f, 0xb6, 0x00]),
                // movzx eax, WORD PTR [rax]
                Size::H => self.code.extend([0x0f, 0xb7, 0x00]),
                // mov eax, DWORD PTR [rax]
                Size::D => self.code.extend([0x8b, 0x00]),
                // mov rax, QWORD PTR [rax]
                Size::Q => self.code.extend([0x48, 0x8b, 0x00]),
            }
        }
        match sz {
            // movzx eax, BYTE PTR [rax + ?]
            Size::B => self.code.extend([0x0f, 0xb6, 0x80]),
            // movzx eax, WORD PTR [rax + ?]
            Size::H => self.code.extend([0x0f, 0xb7, 0x80]),
            // mov eax, DWORD PTR [rax + ?]
            Size::D => self.code.extend([0x8b, 0x80]),
            // mov rax, QWORD PTR [rax + ?]
            Size::Q => self.code.extend([0x48, 0x8b, 0x80]),
        }
        self.code.extend(offset.to_le_bytes());
    }
    

    fn store_rax(&mut self, dest: Dest) {
        match dest {
            Dest::Nowhere => { /* do nothing! */ }
            Dest::Ptr(offset_to_ptr, offset_after_ptr, sz) => {
                // mov rax, rbp
                self.code.extend([0x48, 0x89, 0xe8]);
                self.load_relative_to_rax(offset_to_ptr, Size::Q);
                // mov rcx, rax
                self.code.extend([0x48, 0x89, 0xc1]);
                // mov [rcx + sz], rax
                self.store_relative_to_rcx(offset_after_ptr, sz)
            }
            Dest::Here(stack_offset, sz) => {
                // move rcx, rbp
                self.code.extend([0x48, 0x89, 0xe9]);
                // mov [rcx + sz], rax
                self.store_relative_to_rcx(stack_offset, sz)
            }
        }
    }

    fn store_relative_to_rcx(&mut self, offset: i32, sz: Size) {
        match sz {
            // mov BYTE [rcx - ?], al
            Size::B => self.code.extend([0x88, 0x81]),
            // mov WORD [rcx - ?], ax
            Size::H => self.code.extend([0x66, 0x89, 0x81]),
            // mov DWORD [rcx - ?], eax
            Size::D => self.code.extend([0x89, 0x81]),
            // mov QWORD [rcx - ?], eax
            Size::Q => self.code.extend([0x48, 0x89, 0x81]),
        }
        self.code.extend(offset.to_le_bytes())
    }
}