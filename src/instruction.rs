#[derive(Clone, Copy, Debug)]
pub enum Dest { Nowhere, Ptr(i32, i32, Size), Here(i32, Size) }

#[derive(Clone, Copy, Debug)]
pub enum Src { Uninitialized, Imm(u64), Ptr(i32, i32, Size), Here(i32, Size) }

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Size { B, H, D, Q }

#[derive(Clone, Copy, Debug)]
pub struct Count(pub u64);

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct Label(pub u64);

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    // NOTE: too-small sources will be zero-extended to u64
    // Likewise, too-small destinations will get the low bits of the u64

    // prologue, allocs space for n_bytes the stack, saves args to destinations
    FFIBegin(u64, [Dest; 6]),   
    FFIRet(Src),

    Copy(Dest, Src, Count),

    JIf(Src, Label),
    Label(Label),
    FFICall(Dest, [Src; 6], extern fn(u64, u64, u64, u64, u64, u64) -> u64),
}

impl Src {
    pub(crate) fn needs_load(&self) -> bool {
        match self {
            Src::Uninitialized => false,
            _ => true,
        }
    }

    pub(crate) fn offset(self, amt: i32) -> Src {
        match self {
            Src::Uninitialized => Src::Uninitialized,
            Src::Imm(x) => Src::Imm(x),
            Src::Ptr(stack, far, sz) => Src::Ptr(stack, far + amt, sz),
            Src::Here(stack, sz) => Src::Here(stack + amt, sz)
        }
    }
}

impl Dest {
    pub(crate) fn needs_store(&self) -> bool {
        match self {
            Dest::Nowhere => false,
            _ => true,
        }
    }

    pub(crate) fn offset(self, amt: i32) -> Dest {
        match self {
            Dest::Nowhere => Dest::Nowhere,
            Dest::Ptr(stack, far, sz) => Dest::Ptr(stack, far + amt, sz),
            Dest::Here(stack, sz) => Dest::Here(stack + amt, sz)
        }
    }
}

pub(crate) fn same_size(dest: Dest, src: Src) -> bool {
    match (dest, src) {
        (Dest::Nowhere, _) => true,
        (_, Src::Uninitialized) => true,
        (_, Src::Imm(_)) => true,
        (Dest::Ptr(_, _, sz1) | Dest::Here(_, sz1), Src::Ptr(_, _, sz2) | Src::Here(_, sz2)) => sz1 == sz2,
        
    }

}