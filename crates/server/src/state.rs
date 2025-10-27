use crate::proxmox::Proxmox;
use sqlx::PgPool;
use std::sync::Arc;

/// Holds the application's shared state, like the database connection pool and
/// the Proxmox client across Axum handlers.
///
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub proxmox: Arc<dyn Proxmox + Send + Sync>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("Pool", &self.pool)
            .field("Proxmox", &"Arc<dyn Proxmox>")
            .finish()
    }
}
