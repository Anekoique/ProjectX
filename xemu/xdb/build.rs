fn main() {
    // Build Spike FFI wrapper when difftest is enabled.
    // SPIKE_DIR env var points to Spike install prefix (default: /opt/homebrew).
    if std::env::var("CARGO_FEATURE_DIFFTEST").is_ok() {
        let spike_dir = std::env::var("SPIKE_DIR").unwrap_or_else(|_| "/opt/homebrew".to_string());
        let wrapper_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../tools/difftest/spike");

        cc::Build::new()
            .cpp(true)
            .std("c++17")
            .file(format!("{wrapper_dir}/spike_wrapper.cc"))
            .include(format!("{spike_dir}/include"))
            .include(wrapper_dir)
            .compile("spike_wrapper");

        println!("cargo:rustc-link-search=native={spike_dir}/lib");
        println!("cargo:rustc-link-lib=dylib=riscv");
        println!("cargo:rustc-link-lib=dylib=softfloat");
        println!("cargo:rustc-link-lib=static=fesvr");
        println!("cargo:rustc-link-lib=static=disasm");
        println!("cargo:rustc-link-lib=dylib=c++");
    }
}
