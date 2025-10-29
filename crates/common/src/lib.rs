pub mod error;
pub mod telemetry;

pub mod prelude {
    pub use crate::error::{AuthError, Error, ProxmoxError, Result};
}
