//! Framework-agnostic core for Salvo integration (**no `salvo` dependency**).
//! Salvo 集成的框架无关共享层（**不依赖 `salvo`**）。

pub use sa_token_core::router::{
    AuthFlowResult, AuthResult, PathAuthConfig, create_context, extract_token, process_auth,
    run_auth_flow,
};

pub mod error_response;
pub mod state;

pub use state::{SaTokenState, SaTokenStateBuilder};
