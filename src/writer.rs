use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use parking_lot::RwLock;
use rusoto_logs::{CloudWatchLogs, CloudWatchLogsClient, InputLogEvent};
use tokio::runtime::{Builder, Handle};
use tokio::sync::mpsc::{channel, Sender};
use tracing_core::Metadata;
use tracing_subscriber::fmt::MakeWriter;

use crate::worker::rusoto_worker_loop;
use crate::{
    CLOUDWATCH_EXTRA_MSG_PAYLOAD_SIZE, CLOUDWATCH_MAX_BATCH_EVENTS_LENGTH,
    CLOUDWATCH_MAX_BATCH_SIZE,
};

pub struct RusotoLogsWriterBuilder<C, F> {
    client: Arc<C>,
    log_group: String,
    default_log_stream: String,
    make_log_stream: F,
    channels: Arc<RwLock<HashMap<String, Sender<InputLogEvent>>>>,
    runtime_handle: Handle,
}

impl RusotoLogsWriterBuilder<(), Box<dyn Fn(&Metadata) -> String + Send + Sync>> {
    pub fn new(log_group: &str, default_log_stream: &str) -> Self {
        let default_log_stream = default_log_stream.to_string();
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime");
        let runtime_handle = runtime.handle().clone();
        std::thread::spawn(move || {
            runtime.block_on(std::future::pending::<()>());
        });
        Self {
            client: Arc::new(()),
            log_group: log_group.to_string(),
            default_log_stream: default_log_stream.clone(),
            make_log_stream: Box::new(move |_| default_log_stream.clone()),
            channels: Arc::new(RwLock::new(HashMap::new())),
            runtime_handle,
        }
    }
}

impl<C, F> RusotoLogsWriterBuilder<C, F> {
    pub fn with_client<C2>(self, client: C2) -> RusotoLogsWriterBuilder<C2, F>
    where
        C2: CloudWatchLogs + Send + Sync + 'static,
    {
        RusotoLogsWriterBuilder {
            client: Arc::new(client),
            log_group: self.log_group,
            default_log_stream: self.default_log_stream,
            make_log_stream: self.make_log_stream,
            channels: self.channels,
            runtime_handle: self.runtime_handle,
        }
    }

    pub fn default_client(self) -> RusotoLogsWriterBuilder<CloudWatchLogsClient, F> {
        RusotoLogsWriterBuilder {
            client: Arc::new(CloudWatchLogsClient::new(Default::default())),
            log_group: self.log_group,
            default_log_stream: self.default_log_stream,
            make_log_stream: self.make_log_stream,
            channels: self.channels,
            runtime_handle: self.runtime_handle,
        }
    }

    pub fn make_log_stream<F2>(self, make_log_stream: F2) -> RusotoLogsWriterBuilder<C, F2>
    where
        F2: Fn(&Metadata) -> String + Send + Sync,
    {
        RusotoLogsWriterBuilder {
            client: self.client,
            log_group: self.log_group,
            default_log_stream: self.default_log_stream,
            make_log_stream,
            channels: self.channels,
            runtime_handle: self.runtime_handle,
        }
    }
}

impl<C, F> RusotoLogsWriterBuilder<C, F>
where
    C: CloudWatchLogs + Send + Sync + 'static,
    F: Fn(&Metadata) -> String + Send + Sync,
{
    fn get_or_spawn_worker(&self, log_stream: String) -> Sender<InputLogEvent> {
        let read_guard = self.channels.read();
        if let Some(sender) = read_guard.get(&log_stream) {
            if !sender.is_closed() {
                return sender.clone();
            }
        }
        drop(read_guard);
        let mut write_guard = self.channels.write();
        let (sender, receiver) = channel(CLOUDWATCH_MAX_BATCH_EVENTS_LENGTH);
        self.runtime_handle.spawn(rusoto_worker_loop(
            self.client.clone(),
            receiver,
            self.log_group.clone(),
            log_stream.clone(),
        ));
        write_guard.insert(log_stream, sender.clone());
        sender
    }
}

impl<C, F> MakeWriter for RusotoLogsWriterBuilder<C, F>
where
    C: CloudWatchLogs + Send + Sync + 'static,
    F: Fn(&Metadata) -> String + Send + Sync,
{
    type Writer = RusotoLogsWriter;

    fn make_writer(&self) -> Self::Writer {
        let channel = self.get_or_spawn_worker(self.default_log_stream.clone());
        RusotoLogsWriter::new(channel, self.runtime_handle.clone())
    }

    fn make_writer_for(&self, metadata: &Metadata<'_>) -> Self::Writer {
        let channel = self.get_or_spawn_worker((self.make_log_stream)(metadata));
        RusotoLogsWriter::new(channel, self.runtime_handle.clone())
    }
}

pub struct RusotoLogsWriter {
    line_writer: io::LineWriter<Inner>,
}

impl RusotoLogsWriter {
    fn new(channel: Sender<InputLogEvent>, runtime_handle: Handle) -> Self {
        let inner = Inner {
            channel,
            runtime_handle,
        };
        let line_writer = io::LineWriter::new(inner);
        Self { line_writer }
    }
}

impl io::Write for RusotoLogsWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.line_writer.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.line_writer.flush()
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.line_writer.write_vectored(bufs)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.line_writer.write_all(buf)
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> io::Result<()> {
        self.line_writer.write_fmt(fmt)
    }
}

#[derive(Clone)]
struct Inner {
    channel: Sender<InputLogEvent>,
    runtime_handle: Handle,
}

impl io::Write for Inner {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf_len = buf.len();
        let timestamp = timestamp();

        let buf = buf.strip_suffix(b"\n").unwrap_or(buf);

        let buf = if buf.len() > CLOUDWATCH_MAX_BATCH_SIZE - CLOUDWATCH_EXTRA_MSG_PAYLOAD_SIZE {
            eprintln!("Message size exceeds max payload size, truncated");
            buf.split_at(CLOUDWATCH_MAX_BATCH_SIZE - CLOUDWATCH_EXTRA_MSG_PAYLOAD_SIZE)
                .0
        } else {
            buf
        };

        let message = String::from_utf8(buf.to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let channel = self.channel.clone();
        self.runtime_handle.spawn(async move {
            let _ = channel.send(InputLogEvent { message, timestamp }).await;
        });

        Ok(buf_len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write(buf).map(|_| ())
    }
}

/// Returns current unix timestamp in milliseconds
fn timestamp() -> i64 {
    use std::convert::TryFrom;
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).unwrap(),
        Err(err) => -i64::try_from(err.duration().as_millis()).unwrap(),
    }
}
