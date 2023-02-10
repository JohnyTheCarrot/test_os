use crate::SCREEN;
use conquer_once::spin::OnceCell;
use core::fmt::Write;
use log::{Metadata, Record};

pub struct Logger {}

pub static LOGGER: OnceCell<Logger> = OnceCell::uninit();

impl Logger {
    pub fn init() {
        let logger = LOGGER.get_or_init(move || Self {});

        log::set_logger(logger).expect("logger already initialized");
        log::set_max_level(log::LevelFilter::Trace);
        log::info!("Logger enabled!");
    }
}

impl log::Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let mut screen = SCREEN.get().unwrap().lock();

        writeln!(screen, "{:5}: {}", record.level(), record.args()).unwrap()
    }

    fn flush(&self) {}
}
