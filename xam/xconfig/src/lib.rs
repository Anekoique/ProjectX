//! Platform-specific constants and parameters for [xhal].
//! This crate is migrated from [ArceOS] 's [axconfig].

#![no_std]

axconfig_macros::include_configs!(
    path_env = "X_CONFIG_PATH",
    fallback = "../configs/dummy.toml"
);
