mod error;
mod worker;
mod writer;

const CLOUDWATCH_MAX_BATCH_EVENTS_LENGTH: usize = 10_000;
const CLOUDWATCH_MAX_BATCH_SIZE: usize = 1024 * 1024;
const CLOUDWATCH_EXTRA_MSG_PAYLOAD_SIZE: usize = 26;

pub use writer::{RusotoLogsWriter, RusotoLogsWriterBuilder};

pub fn writer(
    log_group: &str,
    default_log_stream: &str,
) -> RusotoLogsWriterBuilder<(), Box<dyn Fn(&tracing_core::Metadata) -> String + Send + Sync>> {
    RusotoLogsWriterBuilder::new(log_group, default_log_stream)
}
