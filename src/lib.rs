//! Convenience facade for sa-token-rust.

pub use sa_token_core::*;
pub use sa_token_storage_memory as storage_memory;

#[cfg(feature = "plugin-apikey")]
pub use sa_token_plugin_apikey as plugin_apikey;
#[cfg(feature = "plugin-jwt")]
pub use sa_token_plugin_jwt as plugin_jwt;
#[cfg(feature = "plugin-oauth2")]
pub use sa_token_plugin_oauth2 as plugin_oauth2;
#[cfg(feature = "plugin-observability")]
pub use sa_token_plugin_observability as plugin_observability;
#[cfg(feature = "plugin-sso")]
pub use sa_token_plugin_sso as plugin_sso;

/// Compatibility helper retained from the initial crate scaffold.
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
