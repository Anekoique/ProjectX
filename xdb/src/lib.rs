mod log;

pub fn init_sdb() {
    crate::log::init();
    crate::log::set_max_level(option_env!("X_LOG").unwrap_or(""));
}
