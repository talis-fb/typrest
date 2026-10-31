use std::io;

use tokio::sync::watch;
use tracing::warn;

pub(crate) const SIGNAL_SIGINT: &str = "SIGINT";
pub(crate) const SIGNAL_SIGTERM: &str = "SIGTERM";
const FORCED_EXIT_STATUS: i32 = 1;

/// Cloneable handle that resolves once the process receives its first shutdown signal.
#[derive(Debug, Clone)]
pub(crate) struct ShutdownSignal {
    receiver: watch::Receiver<Option<&'static str>>,
}

/// Fires a [`ShutdownSignal`] without an operating-system signal.
#[derive(Debug)]
pub(crate) struct ShutdownTrigger {
    sender: watch::Sender<Option<&'static str>>,
}

impl ShutdownTrigger {
    pub(crate) fn fire(&self, signal: &'static str) {
        self.sender.send_replace(Some(signal));
    }
}

impl ShutdownSignal {
    pub(crate) fn channel() -> (ShutdownTrigger, Self) {
        let (sender, receiver) = watch::channel(None);
        (ShutdownTrigger { sender }, Self { receiver })
    }

    /// Installs the `SIGTERM` and `SIGINT` handlers. A second signal exits the process with
    /// status `1` immediately. Must be called inside a tokio runtime.
    pub(crate) fn install() -> io::Result<Self> {
        let mut signals = OsSignals::new()?;
        let (trigger, signal) = Self::channel();

        tokio::spawn(async move {
            trigger.fire(signals.next().await);
            let second = signals.next().await;
            warn!(
                event = "server.shutdown.forced",
                signal = second,
                "received {second} during shutdown, exiting immediately"
            );
            std::process::exit(FORCED_EXIT_STATUS);
        });

        Ok(signal)
    }

    /// Resolves with the name of the first signal. Never resolves if the trigger is dropped
    /// without firing.
    pub(crate) async fn wait(&self) -> &'static str {
        let mut receiver = self.receiver.clone();
        let signal = receiver
            .wait_for(Option::is_some)
            .await
            .map(|signal| *signal);
        match signal {
            Ok(signal) => signal.unwrap_or(SIGNAL_SIGTERM),
            Err(_) => std::future::pending().await,
        }
    }
}

#[cfg(unix)]
struct OsSignals {
    sigint: tokio::signal::unix::Signal,
    sigterm: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl OsSignals {
    fn new() -> io::Result<Self> {
        use tokio::signal::unix::{SignalKind, signal};

        Ok(Self {
            sigint: signal(SignalKind::interrupt())?,
            sigterm: signal(SignalKind::terminate())?,
        })
    }

    async fn next(&mut self) -> &'static str {
        tokio::select! {
            _ = self.sigint.recv() => SIGNAL_SIGINT,
            _ = self.sigterm.recv() => SIGNAL_SIGTERM,
        }
    }
}

#[cfg(not(unix))]
struct OsSignals;

#[cfg(not(unix))]
impl OsSignals {
    fn new() -> io::Result<Self> {
        Ok(Self)
    }

    async fn next(&mut self) -> &'static str {
        match tokio::signal::ctrl_c().await {
            Ok(()) => SIGNAL_SIGINT,
            Err(_) => std::future::pending().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{SIGNAL_SIGTERM, ShutdownSignal};

    #[tokio::test]
    async fn wait_resolves_with_the_fired_signal() {
        let (trigger, signal) = ShutdownSignal::channel();
        let clone = signal.clone();

        trigger.fire(SIGNAL_SIGTERM);

        assert_eq!(signal.wait().await, SIGNAL_SIGTERM);
        assert_eq!(clone.wait().await, SIGNAL_SIGTERM);
    }

    #[tokio::test]
    async fn wait_stays_pending_until_fired() {
        let (_trigger, signal) = ShutdownSignal::channel();

        let result = tokio::time::timeout(Duration::from_millis(50), signal.wait()).await;

        assert!(result.is_err());
    }
}
