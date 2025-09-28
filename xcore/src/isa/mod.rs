#[cfg(loongarch)]
mod loongarch;
#[cfg(riscv)]
mod riscv;

mod decoder;

use std::sync::LazyLock;

static DECODER: LazyLock<decoder::Decoder> = LazyLock::new(|| {
    match decoder::Decoder::new(&[include_str!("./riscv/rv32.toml").to_string()]) {
        Ok(decoder) => decoder,
        Err(errors) => {
            for error in &errors[0] {
                println!("\t{error}");
            }
            panic!("Errors in ./riscv/rv32.toml:");
        }
    }
});

pub fn init_decoder() {
    trace!("hello xcore");
}
