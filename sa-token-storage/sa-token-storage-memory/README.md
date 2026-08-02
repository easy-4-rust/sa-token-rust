# sa-token-storage-memory

In-memory storage implementation for sa-token-rust.

## Features

- ⚡ **High Performance**: Ultra-fast in-memory storage
- 🎯 **Zero Configuration**: Works out of the box
- 🧪 **Perfect for Development**: Quick setup for testing
- 💾 **TTL Support**: Automatic expiration handling

## Installation

```toml
[dependencies]
sa-token-storage-memory = "0.1.2"
```

## Usage

```rust
use sa_token_storage_memory::MemoryStorage;
use sa_token_plugin_axum::SaTokenState;
use std::sync::Arc;

let storage = Arc::new(MemoryStorage::new());

let state = SaTokenState::builder()
    .storage(storage)
    .timeout(7200)
    .build();
```

## ⚠️ Important Notes

- **Not for Production**: Data is lost on restart
- **Single Instance**: Does not work across multiple servers
- **Memory Limited**: Suitable for development and testing only

For production environments, use [sa-token-storage-redis](../sa-token-storage-redis) instead.

## Author

**金书记**

## License

Licensed under either of Apache-2.0 or MIT.
