use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }

    pub fn from_u8(v: u8) -> LogLevel {
        match v {
            0 => LogLevel::Error,
            1 => LogLevel::Warn,
            2 => LogLevel::Info,
            3 => LogLevel::Debug,
            4 => LogLevel::Trace,
            _ => LogLevel::Info,
        }
    }
}

static CURRENT_LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);

pub fn set_log_level(level: LogLevel) {
    CURRENT_LOG_LEVEL.store(level as u8, Ordering::Relaxed);
}

fn should_log(level: LogLevel) -> bool {
    level as u8 <= CURRENT_LOG_LEVEL.load(Ordering::Relaxed)
}

fn log(level: LogLevel, module: &str, msg: &str, fields: &[(&str, String)]) {
    if !should_log(level) {
        return;
    }

    let mut output = format!(r#""l"="{}"|"mod"="{}""#, level.as_str(), module,);
    for (k, v) in fields {
        output.push_str(&format!(r#"|"{}"="{}""#, k, v));
    }
    output.push_str(&format!(r#"|"m"="{}""#, msg));

    println!("{output}");
}

pub fn log_banner(msg: &str) {
    let pattern = "-".repeat(msg.len() + 2);
    println!("{pattern}");
    println!("{msg}");
    println!("{pattern}");
}

pub fn log_info(module: &str, msg: &str, fields: &[(&str, String)]) {
    log(LogLevel::Info, module, msg, fields);
}

pub fn log_warn(module: &str, msg: &str, fields: &[(&str, String)]) {
    log(LogLevel::Warn, module, msg, fields);
}

pub fn log_error(module: &str, msg: &str, fields: &[(&str, String)]) {
    log(LogLevel::Error, module, msg, fields);
}

pub fn log_debug(module: &str, msg: &str, fields: &[(&str, String)]) {
    log(LogLevel::Debug, module, msg, fields);
}
