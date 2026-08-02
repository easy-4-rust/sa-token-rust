# OAuth2 Authorization Code Flow Guide

[中文](/zh/guide/oauth2.md) | English

---

## Overview

sa-token-rust implements the complete OAuth2 Authorization Code Grant flow, compliant with RFC 6749, supporting third-party application authorization, single sign-on, API access control, and more.

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Core Components](#core-components)
- [Authorization Flow](#authorization-flow)
- [API Reference](#api-reference)
- [Security Best Practices](#security-best-practices)

## Features

- ✅ OAuth2 RFC 6749 compliant
- ✅ Authorization Code Grant flow
- ✅ Client management (registration, verification)
- ✅ Authorization code generation and validation
- ✅ Access token management
- ✅ Refresh token mechanism
- ✅ Strict redirect URI validation
- ✅ Scope permission control
- ✅ Token revocation
- ✅ Automatic expiration cleanup
- ✅ Enhanced error handling with detailed validation steps
- ✅ Comprehensive code documentation with security considerations
- ✅ Atomic code consumption (one-time use enforcement)

## Quick Start

### 1. Create OAuth2 Manager

```rust
use sa_token_core::OAuth2Manager;
use std::sync::Arc;

let storage = Arc::new(MemoryStorage::new());
let oauth2 = OAuth2Manager::new(storage)
    .with_ttl(
        600,      // Authorization code 10 minutes
        3600,     // Access token 1 hour
        2592000   // Refresh token 30 days
    );
```

### 2. Register Client

```rust
use sa_token_core::OAuth2Client;

let client = OAuth2Client {
    client_id: "web_app_001".to_string(),
    client_secret: "secret_abc123xyz".to_string(),
    redirect_uris: vec![
        "http://localhost:3000/callback".to_string(),
    ],
    grant_types: vec![
        "authorization_code".to_string(),
        "refresh_token".to_string(),
    ],
    scope: vec![
        "read".to_string(),
        "write".to_string(),
        "profile".to_string(),
    ],
};

oauth2.register_client(&client).await?;
```

### 3. Complete Authorization Flow

```rust
// Step 1: Generate authorization code (after user consent)
let auth_code = oauth2.generate_authorization_code(
    "web_app_001".to_string(),
    "user_123".to_string(),
    "http://localhost:3000/callback".to_string(),
    vec!["read".to_string(), "profile".to_string()],
);

oauth2.store_authorization_code(&auth_code).await?;

// Step 2: Exchange code for token
let token = oauth2.exchange_code_for_token(
    &auth_code.code,
    "web_app_001",
    "secret_abc123xyz",
    "http://localhost:3000/callback",
).await?;

// Step 3: Use access token
let token_info = oauth2.verify_access_token(&token.access_token).await?;

// Step 4: Refresh token
let new_token = oauth2.refresh_access_token(
    token.refresh_token.as_ref().unwrap(),
    "web_app_001",
    "secret_abc123xyz",
).await?;
```

## Core Components

### OAuth2Manager

OAuth2 manager responsible for the entire authorization flow.

```rust
pub struct OAuth2Manager {
    storage: Arc<dyn SaStorage>,
    code_ttl: i64,
    token_ttl: i64,
    refresh_token_ttl: i64,
}
```

**Methods**:
- `new(storage)` - Create manager
- `with_ttl(code_ttl, token_ttl, refresh_ttl)` - Set expiration times
- `register_client(&client)` - Register client
- `get_client(client_id)` - Get client information
- `verify_client(client_id, client_secret)` - Verify client credentials
- `generate_authorization_code(...)` - Generate authorization code
- `store_authorization_code(&code)` - Store authorization code
- `exchange_code_for_token(...)` - Exchange code for token
- `verify_access_token(&token)` - Verify access token
- `refresh_access_token(...)` - Refresh access token
- `revoke_token(&token)` - Revoke token

### OAuth2Client

Client information.

```rust
pub struct OAuth2Client {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub scope: Vec<String>,
}
```

### AuthorizationCode

Authorization code information.

```rust
pub struct AuthorizationCode {
    pub code: String,
    pub client_id: String,
    pub user_id: String,
    pub redirect_uri: String,
    pub scope: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

### AccessToken

Access token response.

```rust
pub struct AccessToken {
    pub access_token: String,
    pub token_type: String,        // "Bearer"
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: Vec<String>,
}
```

## Authorization Flow

### Complete Flow Diagram

```
┌─────────┐                               ┌─────────────┐
│  User   │                               │   Client    │
└────┬────┘                               └──────┬──────┘
     │                                            │
     │  1. Access third-party app                 │
     │───────────────────────────────────────────▶│
     │                                            │
     │  2. Redirect to authorization page         │
     │◀───────────────────────────────────────────│
     │                                            │
┌────▼────┐                               ┌──────┴──────┐
│Auth     │                               │  Resource   │
│Server   │                               │  Server     │
└────┬────┘                               └──────┬──────┘
     │                                            │
     │  3. User grants permission                 │
     │                                            │
     │  4. Generate authorization code            │
     │  oauth2.generate_authorization_code()      │
     │                                            │
     │  5. Redirect back with code                │
     │───────────────────────────────────────────▶│
     │                                            │
     │  6. Exchange code for token                │
     │  oauth2.exchange_code_for_token()          │
     │◀───────────────────────────────────────────│
     │                                            │
     │  7. Return access & refresh tokens         │
     │───────────────────────────────────────────▶│
     │                                            │
     │  8. Access resources with token            │
     │                                            │───────▶
     │                                            │
     │  9. Return resources                       │
     │                                            │◀───────
     │                                            │
     │  10. Refresh token when expired            │
     │  oauth2.refresh_access_token()             │
     │◀───────────────────────────────────────────│
     │                                            │
     │  11. Return new access token               │
     │───────────────────────────────────────────▶│
     │                                            │
```

## API Reference

### Client Management

#### register_client

Register an OAuth2 client.

```rust
pub async fn register_client(&self, client: &OAuth2Client) -> SaTokenResult<()>
```

#### get_client

Get client information.

```rust
pub async fn get_client(&self, client_id: &str) -> SaTokenResult<OAuth2Client>
```

#### verify_client

Verify client credentials.

```rust
pub async fn verify_client(&self, client_id: &str, client_secret: &str) -> SaTokenResult<bool>
```

### Authorization Code Management

#### generate_authorization_code

Generate authorization code.

```rust
pub fn generate_authorization_code(
    &self,
    client_id: String,
    user_id: String,
    redirect_uri: String,
    scope: Vec<String>,
) -> AuthorizationCode
```

#### store_authorization_code

Store authorization code.

```rust
pub async fn store_authorization_code(&self, auth_code: &AuthorizationCode) -> SaTokenResult<()>
```

#### exchange_code_for_token

Exchange authorization code for access token.

```rust
pub async fn exchange_code_for_token(
    &self,
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> SaTokenResult<AccessToken>
```

### Token Management

#### verify_access_token

Verify access token.

```rust
pub async fn verify_access_token(&self, access_token: &str) -> SaTokenResult<OAuth2TokenInfo>
```

#### refresh_access_token

Refresh access token.

```rust
pub async fn refresh_access_token(
    &self,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> SaTokenResult<AccessToken>
```

#### revoke_token

Revoke token.

```rust
pub async fn revoke_token(&self, token: &str) -> SaTokenResult<()>
```

## Security Best Practices

### 1. Client Credentials

- ✅ Use strong keys for client_secret (at least 32 characters)
- ✅ Rotate client keys regularly
- ✅ Store credentials securely, never hardcode
- ✅ Use HTTPS to transmit credentials

### 2. Authorization Code

- ✅ Code can only be used once (implemented)
- ✅ Short validity period (default 10 minutes)
- ✅ Strict redirect_uri validation (implemented)
- ✅ Use state parameter to prevent CSRF

### 3. Access Token

- ✅ Short validity period (recommended 1-2 hours)
- ✅ Use Bearer Token format
- ✅ Validate signature and expiration
- ✅ Implement token revocation (implemented)

### 4. Transmission Security

- ✅ **Must use HTTPS**
- ✅ Enable HSTS
- ✅ Use secure TLS versions (1.2+)
- ✅ Verify SSL certificates

## Examples

Run the complete example:

```bash
cargo run --example oauth2_example
```

View example code: `sa-token-examples/oauth2_example.rs`

## References

- [OAuth 2.0 RFC 6749](https://tools.ietf.org/html/rfc6749)
- [OAuth 2.0 Security Best Practices](https://tools.ietf.org/html/draft-ietf-oauth-security-topics)
- [Example Code](https://github.com/sa-tokens/sa-token-rust/blob/main/sa-token-examples/oauth2_example.rs)

## Next Steps

- [JWT Guide](/guide/jwt.md)
- [Event Listeners](/guide/event-listener.md)
- [Permission Matching](/guide/permission-matching.md)

