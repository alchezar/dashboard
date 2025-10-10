use crate::prelude::Result;
use tracing::subscriber::set_global_default;
use tracing::{Level, Subscriber};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{EnvFilter, fmt::MakeWriter};

/// Composes and returns a tracing subscriber for application logging.
///
/// # Arguments
///
/// * `max_level`: The default maximum level of logs if the `RUST_LOG`
///   environment variable is not set.
/// * `sink`: Destination where logs will be written to.
///
/// returns: impl Subscriber+Sync+Send
///
/// # Returns
///
/// `Subscriber` instance.
///
pub fn get_subscriber<Sink>(max_level: Level, sink: Sink) -> impl Subscriber + Sync + Send
where
    Sink: for<'a> MakeWriter<'a> + Sync + Send + 'static,
{
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_error| EnvFilter::new(max_level.as_str()));

    // Use compact, pretty-formatted logs in debug builds, and JSON logs in
    // release builds.
    #[cfg(debug_assertions)]
    let subscriber_builder = tracing_subscriber::fmt().compact();
    #[cfg(not(debug_assertions))]
    let subscriber_builder = tracing_subscriber::fmt().json().with_current_span(true);

    subscriber_builder
        .with_env_filter(env_filter)
        .with_max_level(max_level)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_writer(sink)
        .finish()
}

/// Register a subscriber as global default to process span data.
///
/// # Warning
///
/// This function should only be called **once** in the application's lifetime.
///
/// # Arguments
///
/// * `subscriber`: Subscriber to set as the global default for the application.
///
pub fn init_subscriber(subscriber: impl Subscriber + Sync + Send) -> Result<()> {
    // Old loggers support.
    LogTracer::init()?;

    set_global_default(subscriber)?;
    Ok(())
}
