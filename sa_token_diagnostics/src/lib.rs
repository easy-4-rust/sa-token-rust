//! Controlled, opt-in diagnostics for Sa-Token applications.
//!
//! Transport authentication is intentionally application-owned. Do not expose
//! profile capture or Tokio console ports directly to an untrusted network.

#[cfg(feature = "pprof")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "pprof")]
use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiagnosticError {
    #[error("profile duration must be greater than zero")]
    ZeroDuration,
    #[error("requested profile duration exceeds the configured maximum")]
    DurationLimit,
    #[error("a profile capture is already active")]
    Busy,
    #[error("profile capture failed: {0}")]
    Capture(String),
    #[error("profile task failed: {0}")]
    Task(String),
}

/// Returns the official Tokio console builder without installing a global
/// subscriber. Call `.spawn()` after configuring it. Requires
/// `RUSTFLAGS="--cfg tokio_unstable"` at build time.
#[cfg(feature = "tokio-console")]
pub fn console_builder() -> console_subscriber::Builder {
    console_subscriber::ConsoleLayer::builder()
}

#[cfg(feature = "tokio-console")]
pub use console_subscriber;

/// Re-export DHAT so the application binary can opt into the required global
/// allocator and profiler guard. Libraries must not choose a global allocator.
#[cfg(feature = "dhat-heap")]
pub use dhat;

/// Single-flight, duration-capped CPU profile capture.
///
/// An HTTP/gRPC adapter may return the generated SVG, but it must enforce
/// administrator authentication, rate limits and audit logging first.
#[cfg(feature = "pprof")]
#[derive(Clone)]
pub struct PprofController {
    frequency: i32,
    max_duration: Duration,
    active: Arc<Mutex<()>>,
}

#[cfg(feature = "pprof")]
impl PprofController {
    pub fn new(frequency: i32, max_duration: Duration) -> Result<Self, DiagnosticError> {
        if frequency <= 0 {
            return Err(DiagnosticError::Capture(
                "sampling frequency must be greater than zero".to_string(),
            ));
        }
        if max_duration.is_zero() {
            return Err(DiagnosticError::ZeroDuration);
        }
        Ok(Self {
            frequency,
            max_duration,
            active: Arc::new(Mutex::new(())),
        })
    }

    pub fn max_duration(&self) -> Duration {
        self.max_duration
    }

    pub async fn capture_flamegraph(&self, duration: Duration) -> Result<Vec<u8>, DiagnosticError> {
        if duration.is_zero() {
            return Err(DiagnosticError::ZeroDuration);
        }
        if duration > self.max_duration {
            return Err(DiagnosticError::DurationLimit);
        }

        let frequency = self.frequency;
        let active = Arc::clone(&self.active);
        tokio::task::spawn_blocking(move || {
            let _single_flight = active.try_lock().map_err(|_| DiagnosticError::Busy)?;
            let guard = pprof::ProfilerGuardBuilder::default()
                .frequency(frequency)
                .build()
                .map_err(|error| DiagnosticError::Capture(error.to_string()))?;
            std::thread::sleep(duration);
            let report = guard
                .report()
                .build()
                .map_err(|error| DiagnosticError::Capture(error.to_string()))?;
            let mut svg = Vec::new();
            report
                .flamegraph(&mut svg)
                .map_err(|error| DiagnosticError::Capture(error.to_string()))?;
            if svg.is_empty() {
                return Err(DiagnosticError::Capture(
                    "profiler returned no samples on this platform".to_string(),
                ));
            }
            Ok(svg)
        })
        .await
        .map_err(|error| DiagnosticError::Task(error.to_string()))?
    }
}

#[cfg(all(test, feature = "pprof"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn enforces_duration_cap_before_starting_profiler() {
        let controller = PprofController::new(99, Duration::from_millis(300)).unwrap();
        assert!(matches!(
            controller
                .capture_flamegraph(Duration::from_millis(301))
                .await,
            Err(DiagnosticError::DurationLimit)
        ));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn captures_a_real_svg_profile() {
        let controller = PprofController::new(99, Duration::from_millis(300)).unwrap();
        let load = std::thread::spawn(|| {
            let started = std::time::Instant::now();
            let mut value = 1_u64;
            while started.elapsed() < Duration::from_millis(250) {
                value = value.wrapping_mul(6364136223846793005).wrapping_add(1);
                std::hint::black_box(value);
            }
        });
        let svg = controller
            .capture_flamegraph(Duration::from_millis(150))
            .await
            .unwrap();
        load.join().unwrap();
        assert!(svg.windows(4).any(|window| window == b"<svg"));
    }
}
