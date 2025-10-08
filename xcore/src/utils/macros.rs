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
