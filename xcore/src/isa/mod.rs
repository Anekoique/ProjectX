#[cfg(riscv64)]
mod riscv64;
#[cfg(loongarch64)]
mod loongarch64;
#[cfg(riscv32)]
mod riscv32;
#[cfg(loongarch32)]
mod loongarch32;

pub fn hello() {
    #[cfg(riscv64)]
    riscv64::hello();
    #[cfg(loongarch64)]
    loongarch64::hello();
    #[cfg(riscv32)]
    riscv32::hello();
    #[cfg(loongarch32)]
    loongarch32::hello();
}