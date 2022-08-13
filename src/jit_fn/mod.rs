#[cfg(target_os = "windows")]
mod impl_windows;

#[cfg(target_os = "windows")]
use impl_windows as implementation;

pub use implementation::JitFn;