#[cfg(any(target_arch = "riscv64", all(test, feature = "riscv64")))]
pub mod riscv64;

#[cfg(any(target_arch = "riscv64", all(test, feature = "riscv64")))]
pub use riscv64::*;
