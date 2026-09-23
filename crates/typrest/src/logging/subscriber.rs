use std::fmt;
use std::io::IsTerminal;

use tracing::field::{Field, Visit};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::field::{RecordFields, VisitOutput};
use tracing_subscriber::fmt::format::{FormatFields, Writer};
use tracing_subscriber::util::SubscriberInitExt;

use crate::args::{ColorChoice, LogFormat};

pub(crate) fn init(format: LogFormat, color: ColorChoice) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("typrest=info"));
    let ansi = match color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => std::io::stdout().is_terminal(),
    };

    let result = match format {
        LogFormat::Pretty => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stdout)
            .with_target(false)
            .with_ansi(ansi)
            .fmt_fields(MessageOnlyFields)
            .compact()
            .finish()
            .try_init(),
        LogFormat::Json => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stdout)
            .with_ansi(false)
            .json()
            .flatten_event(true)
            .finish()
            .try_init(),
    };

    if let Err(error) = result {
        tracing::warn!(
            event = "logging.subscriber.install.failed",
            error = %error,
            "logging subscriber was not installed: {error}"
        );
    }
}

#[derive(Clone, Copy)]
struct MessageOnlyFields;

impl<'writer> FormatFields<'writer> for MessageOnlyFields {
    fn format_fields<R: RecordFields>(&self, writer: Writer<'writer>, fields: R) -> fmt::Result {
        let mut visitor = MessageVisitor {
            writer,
            result: Ok(()),
        };
        fields.record(&mut visitor);
        visitor.finish()
    }
}

struct MessageVisitor<'writer> {
    writer: Writer<'writer>,
    result: fmt::Result,
}

impl Visit for MessageVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" && self.result.is_ok() {
            self.result = write!(self.writer, "{value:?}");
        }
    }
}

impl VisitOutput<fmt::Result> for MessageVisitor<'_> {
    fn finish(self) -> fmt::Result {
        self.result
    }
}
