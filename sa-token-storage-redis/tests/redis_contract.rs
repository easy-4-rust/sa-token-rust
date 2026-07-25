use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sa_token_adapter::{Expiration, SaStorage, TtlState};
use sa_token_storage_redis::RedisStorage;

async fn storage() -> Option<RedisStorage> {
    let url = std::env::var("SA_TOKEN_TEST_REDIS_URL").ok()?;
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    RedisStorage::new(&url, format!("sa-token-test:{unique}:"))
        .await
        .ok()
}

#[tokio::test]
async fn redis_atomic_and_scan_contract() {
    let Some(storage) = storage().await else {
        eprintln!("skipped: SA_TOKEN_TEST_REDIS_URL is not configured");
        return;
    };
    let storage = Arc::new(storage);

    let mut tasks = Vec::new();
    for index in 0..32 {
        let storage = storage.clone();
        tasks.push(tokio::spawn(async move {
            storage
                .set_if_absent(
                    "nonce",
                    &index.to_string(),
                    Expiration::After(Duration::from_secs(30)),
                )
                .await
                .expect("set_if_absent")
        }));
    }
    let mut inserted = 0;
    for task in tasks {
        inserted += usize::from(task.await.expect("join"));
    }
    assert_eq!(inserted, 1);

    storage
        .set("ttl-key", "before", Some(Duration::from_secs(30)))
        .await
        .expect("set ttl key");
    storage
        .set_with_expiration("ttl-key", "after", Expiration::Keep)
        .await
        .expect("preserve ttl");
    assert_eq!(
        storage.get("ttl-key").await.expect("get").as_deref(),
        Some("after")
    );
    assert!(matches!(
        storage.ttl_state("ttl-key").await.expect("ttl state"),
        TtlState::ExpiresIn(_)
    ));

    for key in ["scan:1", "scan:2", "scan:3"] {
        storage.set(key, "value", None).await.expect("set scan key");
    }
    let mut cursor = None;
    let mut keys = Vec::new();
    loop {
        let page = storage
            .scan("scan:*", cursor.as_deref(), 1)
            .await
            .expect("scan");
        keys.extend(page.keys);
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    keys.sort();
    assert_eq!(keys, vec!["scan:1", "scan:2", "scan:3"]);

    storage.clear().await.expect("clear test namespace");
}
