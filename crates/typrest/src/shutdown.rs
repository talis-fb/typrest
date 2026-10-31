use std::fmt;
use std::io;

use tokio::sync::watch;
use tracing::warn;

const FORCED_EXIT_STATUS: i32 = 1;

/// The operating-system signal that requested the shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Signal {
    Interrupt,
    Terminate,
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Interrupt => "SIGINT",
            Self::Terminate => "SIGTERM",
        })
    }
}

/// Cloneable handle that resolves once the process receives its first shutdown signal.
#[derive(Debug, Clone)]
pub(crate) struct ShutdownSignal {
    receiver: watch::Receiver<Option<Signal>>,
}

/// Fires a [`ShutdownSignal`] without an operating-system signal.
#[derive(Debug)]
pub(crate) struct ShutdownTrigger {
    sender: watch::Sender<Option<Signal>>,
}

impl ShutdownTrigger {
    pub(crate) fn fire(&self, signal: Signal) {
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
                signal = %second,
                "received {second} during shutdown, exiting immediately"
            );
            std::process::exit(FORCED_EXIT_STATUS);
        });

        Ok(signal)
    }

    /// Resolves with the first signal. Never resolves if the trigger is dropped without firing.
    pub(crate) async fn wait(&self) -> Signal {
        let mut receiver = self.receiver.clone();
        loop {
            if let Some(signal) = *receiver.borrow_and_update() {
                return signal;
            }
            if receiver.changed().await.is_err() {
                std::future::pending().await
            }
        }
    }
}

struct OsSignals {
    sigint: tokio::signal::unix::Signal,
    sigterm: tokio::signal::unix::Signal,
}

impl OsSignals {
    fn new() -> io::Result<Self> {
        use tokio::signal::unix::{SignalKind, signal};

        Ok(Self {
            sigint: signal(SignalKind::interrupt())?,
            sigterm: signal(SignalKind::terminate())?,
        })
    }

    async fn next(&mut self) -> Signal {
        tokio::select! {
            _ = self.sigint.recv() => Signal::Interrupt,
            _ = self.sigterm.recv() => Signal::Terminate,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{ShutdownSignal, Signal};

    #[tokio::test]
    async fn wait_resolves_with_the_fired_signal() {
        let (trigger, signal) = ShutdownSignal::channel();
        let clone = signal.clone();

        trigger.fire(Signal::Terminate);

        assert_eq!(signal.wait().await, Signal::Terminate);
        assert_eq!(clone.wait().await, Signal::Terminate);
    }

    #[tokio::test]
    async fn wait_stays_pending_until_fired() {
        let (_trigger, signal) = ShutdownSignal::channel();

        let result = tokio::time::timeout(Duration::from_millis(50), signal.wait()).await;

        assert!(result.is_err());
    }
}
