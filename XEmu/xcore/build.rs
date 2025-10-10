// build.rs
use std::env;

fn main() {
    const EXPECTED_CFGS: &[&str] = &[
        "isa32",
        "isa64",
        "riscv",
        "loongarch",
        "riscv32",
        "riscv64",
        "loongarch32",
        "loongarch64",
    ];
    for cfg in EXPECTED_CFGS {
        println!("cargo:rustc-check-cfg=cfg({})", cfg);
    }

    println!("cargo:rerun-if-env-changed=X_ARCH");
    let arch = env::var("X_ARCH").unwrap_or_else(|_| "riscv32".to_string());
    println!("cargo:rustc-cfg={}", arch);

    if arch.ends_with("64") {
        println!("cargo:rustc-cfg=isa64");
    } else if arch.ends_with("32") {
        println!("cargo:rustc-cfg=isa32");
    }

    if arch.starts_with("riscv") {
        println!("cargo:rustc-cfg=riscv");
    } else if arch.starts_with("loongarch") {
        println!("cargo:rustc-cfg=loongarch");
    }
}
