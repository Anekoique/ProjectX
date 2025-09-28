#[macro_export]
macro_rules! import_modules {
    ($($arch:ident => $mod:ident),* $(,)?) => {
        $(
            #[cfg($arch)]
            mod $mod;
        )*
    };
    ($($arch:ident),* $(,)?) => {
        $(
            #[cfg($arch)]
            mod $arch;
        )*
    };
}

#[macro_export]
macro_rules! define_cpu {
    ($($arch:ident => $core_type:ty),* $(,)?) => {
        $(
            #[cfg($arch)]
            pub static XCPU: std::sync::LazyLock<std::sync::Mutex<CPU<$core_type>>> =
                std::sync::LazyLock::new(|| {
                    std::sync::Mutex::new(CPU::new(<$core_type>::new()))
                });
        )*
    };
}

#[macro_export]
macro_rules! define_decoder {
    ($($cfg_flag:ident => $toml_path:literal),* $(,)?) => {
        $(
            #[cfg($cfg_flag)]
            pub static DECODER: std::sync::LazyLock<decoder::Decoder> = std::sync::LazyLock::new(|| {
                match decoder::Decoder::new(&[include_str!($toml_path).to_string()]) {
                    Ok(decoder) => decoder,
                    Err(errors) => {
                        eprintln!("Error parsing decoder definition file: {}", $toml_path);
                        for error in &errors[0] {
                            eprintln!("\t{error}");
                        }
                        panic!("Fatal errors in TOML file: {}", $toml_path);
                    }
                }
            });
        )*
    };
}
