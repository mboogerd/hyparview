use std::fmt::Display;

pub trait Logged {
    fn log_error(&self, msg: &str);
}

impl<V, E: Display> Logged for Result<V, E> {
    fn log_error(&self, msg: &str) {
        match self {
            Ok(_) => (),
            Err(e) => println!("[Error] Description: '{}'. Cause: '{}'", msg, e),
        }
    }
}
