pub mod config;
pub mod error;
pub mod model;
pub mod proxmox;
pub mod state;
pub mod web;

pub mod prelude {
    pub use crate::error::{AuthError, Error, Result};
    pub use crate::state::AppState;
}
