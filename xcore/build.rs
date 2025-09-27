// build.rs (简洁版)
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

    let arch = detect_arch();
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

fn detect_arch() -> String {
    println!("cargo:rerun-if-env-changed=X_ARCH");
    if let Ok(arch) = env::var("X_ARCH") {
        return arch;
    }

    const FEATURES: &[(&str, &str)] = &[
        ("CARGO_FEATURE_LOONGARCH64", "loongarch64"),
        ("CARGO_FEATURE_LOONGARCH32", "loongarch32"),
        ("CARGO_FEATURE_RISCV64", "riscv64"),
        ("CARGO_FEATURE_RISCV32", "riscv32"),
    ];
    for (env_var, arch_name) in FEATURES {
        if env::var(env_var).is_ok() {
            return arch_name.to_string();
        }
    }

    "riscv32".to_string()
}
