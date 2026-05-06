//! M0-008: shared `tracing-subscriber` init + panic hook for every binary.

use std::sync::Mutex;

use tracing_subscriber::EnvFilter;

type PanicReporter = Box<dyn Fn(&str) + Send + Sync>;
static PANIC_REPORTER: Mutex<Option<PanicReporter>> = Mutex::new(None);

/// Initialize tracing-subscriber and install a panic hook that emits a
/// `system.panic` message via the subscriber. Binaries can additionally
/// register a `panic_reporter` that flushes whatever recorder/run-bundle
/// state is in scope before the process exits.
pub fn init(target: &'static str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("{info}");
        tracing::error!(target = target, payload = "system.panic", panic = %msg);
        if let Ok(reporter) = PANIC_REPORTER.lock() {
            if let Some(callback) = reporter.as_ref() {
                callback(&msg);
            }
        }
        prev_hook(info);
    }));
}

/// Optionally register a panic reporter that the install hook will call before
/// re-invoking the previous hook. Used by `cf-app` to flush an in-flight run
/// bundle before exit.
pub fn set_panic_reporter<F: Fn(&str) + Send + Sync + 'static>(callback: F) {
    if let Ok(mut slot) = PANIC_REPORTER.lock() {
        *slot = Some(Box::new(callback));
    }
}
