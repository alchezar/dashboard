pub mod model {
    pub mod queries;
    pub mod types;
}
pub mod web {
    pub mod auth;
    pub mod mw_auth;
    pub mod routes_login;
    pub mod routes_server;
    pub mod types;
}
pub mod config;
pub mod error;

pub mod prelude {
    pub use crate::error::{AuthError, Error, Result};
}
