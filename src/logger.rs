use clap::ValueEnum;
use colored::{self, *};
use log::{Level, LevelFilter, Metadata, Record};

pub struct Logger;

#[derive(ValueEnum, PartialEq, Debug, Clone)]
pub enum LogType {
    Trace,
    Debug,
    Info,
    Error,
}

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

pub fn change_log_type(t: &LogType) {
    match t {
        LogType::Trace => {
            log::set_max_level(LevelFilter::Trace);
            println!("set up log level trace");
        }
        LogType::Info => {
            log::set_max_level(LevelFilter::Info);
            println!("set up log level info");
        }
        LogType::Debug => {
            log::set_max_level(LevelFilter::Debug);
            println!("set up log level debug");
        }
        LogType::Error => {
            log::set_max_level(LevelFilter::Error);
            println!("set up log level error");
        }
    }
}
