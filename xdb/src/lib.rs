mod log;

pub use log::{debug, error, info, trace, warn};

pub fn init_sdb() {
    crate::log::init();
    crate::log::set_max_level(option_env!("X_LOG").unwrap_or(""));
}
