// source: https://make-a-demo-tool-in-rust.github.io/1-3-jit.html
const PAGE_SIZE: usize = 4096;  // OS X constraint, must be aligned to 0x1000

pub struct JitFn<Arg: Copy, Ret: Copy> {
    addr: *mut u8,

    m_arg: std::marker::PhantomData<*const Arg>,
    m_ret: std::marker::PhantomData<*const Ret>,
}

#[cfg(target_os = "windows")]
impl<Arg: Copy, Ret: Copy> JitFn<Arg, Ret> {
    // NOTE: get_bytes takes a base address
    // and must always produce code of the same length regardless of its argument
    pub fn new(get_bytes: impl Fn(*mut u8) -> Vec<u8>) -> Self {
        use std::mem;
        
        let bytes_1 = get_bytes(std::ptr::null_mut());
        let desired_pages = (bytes_1.len() + PAGE_SIZE - 1) / PAGE_SIZE;

        unsafe {
            let raw_addr: *mut winapi::ctypes::c_void;

            raw_addr = winapi::um::memoryapi::VirtualAlloc(
                std::ptr::null_mut(),
                desired_pages * PAGE_SIZE,
                winapi::um::winnt::MEM_RESERVE | winapi::um::winnt::MEM_COMMIT,
                winapi::um::winnt::PAGE_EXECUTE_READWRITE
            );
            
            let addr: *mut u8 = mem::transmute(raw_addr);
            let bytes_2 = get_bytes(addr);
            assert!(bytes_2.len() == bytes_1.len(), "length should be the same no matter what");
            std::ptr::copy_nonoverlapping(bytes_2.as_ptr(), addr, bytes_2.len());

            Self { addr, m_arg: std::marker::PhantomData, m_ret: std::marker::PhantomData }
        }
    }

    pub unsafe fn run(&self, arg: Arg) -> Ret {
        let ptr: extern fn(Arg) -> Ret = std::mem::transmute(self.addr);
        ptr(arg)
    }
}

#[cfg(target_os = "windows")]
impl<Arg: Copy, Ret: Copy> Drop for JitFn<Arg, Ret> {
    fn drop(&mut self) {
        use std::mem;

        unsafe {
            let result = winapi::um::memoryapi::VirtualFree(
                mem::transmute(self.addr),
                0, 
                winapi::um::winnt::MEM_RELEASE 
            );

            if result == 0 { panic!("VirtualFree returned 0") }
        }
    }
}