use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use sa_token_core::event::LoggingListener;
use sa_token_core::{SaTokenConfig, SaTokenManager};
use sa_token_storage_memory::MemoryStorage;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

struct SharedGuard(Arc<Mutex<Vec<u8>>>);

impl Write for SharedGuard {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'writer> MakeWriter<'writer> for SharedWriter {
    type Writer = SharedGuard;

    fn make_writer(&'writer self) -> Self::Writer {
        SharedGuard(Arc::clone(&self.0))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn authentication_logs_never_contain_token_or_login_id() {
    let writer = SharedWriter::default();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(writer.clone())
        .finish();
    let _default = tracing::subscriber::set_default(subscriber);

    let manager = SaTokenManager::new(Arc::new(MemoryStorage::new()), SaTokenConfig::default());
    manager.event_bus().register(Arc::new(LoggingListener));

    let secret_login_id = "user-pii-never-log-01837";
    let token = manager.login(secret_login_id).await.unwrap();
    manager.logout(&token).await.unwrap();

    let output = String::from_utf8(writer.0.lock().unwrap().clone()).unwrap();
    assert!(!output.contains(secret_login_id), "{output}");
    assert!(!output.contains(token.as_str()), "{output}");
    assert!(output.contains("event=\"login\""), "{output}");
    assert!(output.contains("event=\"logout\""), "{output}");
}
