mod config;
mod error;
mod otel;
pub mod telemetry;

pub use config::ConfigExt;
pub use error::log_error;
pub use otel::{accept_trace, SendTrace};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
