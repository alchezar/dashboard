pub mod config;
pub mod database;
pub mod error;
pub mod services;
pub mod routes;
pub mod models {
    pub mod user;
}

pub mod prelude {
    pub use crate::error::DashboardError;
}
