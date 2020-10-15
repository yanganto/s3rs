use colored::{self, *};
use log::{Level, LevelFilter, Metadata, Record};

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            match record.level() {
                log::Level::Error => println!("{} - {}", "ERROR".red().bold(), record.args()),
                log::Level::Warn => println!("{} - {}", "WARN".red(), record.args()),
                log::Level::Info => println!("{} - {}", "INFO".cyan(), record.args()),
                log::Level::Debug => println!("{} - {}", "DEBUG".blue().bold(), record.args()),
                log::Level::Trace => println!("{} - {}", "TRACE".blue(), record.args()),
            }
        }
    }
    fn flush(&self) {}
}

pub fn change_log_type(command: &str) {
    if command.ends_with("trace") {
        log::set_max_level(LevelFilter::Trace);
        println!("set up log level trace");
    } else if command.ends_with("debug") {
        log::set_max_level(LevelFilter::Debug);
        println!("set up log level debug");
    } else if command.ends_with("info") {
        log::set_max_level(LevelFilter::Info);
        println!("set up log level info");
    } else if command.ends_with("error") {
        log::set_max_level(LevelFilter::Error);
        println!("set up log level error");
    } else {
        println!("usage: log [trace/debug/info/error]");
    }
}
