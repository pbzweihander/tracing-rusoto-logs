# tracing-rusoto-logs

[`MakeWriter`](https://docs.rs/tracing-subscriber/0.2.19/tracing_subscriber/fmt/trait.MakeWriter.html) implementation
that writes tracing events to CloudWatch Logs.

## Usage

```rust
tracing_subscriber::fmt()
    .json()
    .writer(tracing_rusoto_logs::writer("log_group_name", "log_stream_name").default_client())
    .init();
```

### With dynamic log stream name

```rust
fn log_stream_name_maker(metadata: &Metadata<'_>) -> String {
    format!("some_log_stream/{}", metadata.target())
}

tracing_subscriber::fmt()
    .json()
    .writer(
        tracing_rusoto_logs::writer("log_group_name", "some_log_stream")
            .default_client()
            .make_log_stream(log_stream_name_maker),
    )
    .init();
```
