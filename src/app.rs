use crate::prelude::{AppState, Result};
use crate::web::middleware as mw;
use crate::web::{routes_login, routes_server};
use axum::serve::Serve;
use axum::{Router, middleware};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

/// Represents the core web application.
///
pub struct App {
    server: Serve<TcpListener, Router, Router>,
}

impl App {
    /// Builds the application, but does not run it.
    ///
    /// This configures the entire Axum router, including routes, state, and
    /// middleware. Binds a `TcpListener` to the provided address and
    /// determines the final URL of the application.
    ///
    /// # Arguments
    ///
    /// * `app_state` - Shared state for the application.
    /// * `address` - Socket address to bind to. If the port is 0, a random
    ///   available port will be used.
    ///
    pub async fn build(app_state: AppState, address: SocketAddr) -> Result<Self> {
        let listener = TcpListener::bind(&address).await?;
        let router = Router::new()
            .merge(routes_login::routes())
            .merge(routes_server::routes())
            .with_state(app_state)
            .layer(middleware::map_response(mw::log_mapper))
            .layer(CorsLayer::new().allow_origin(Any));

        Ok(Self {
            server: axum::serve(listener, router),
        })
    }

    /// Runs the application server.
    ///
    /// This method consumes the `App` instance and starts the server, which
    /// will run until it is shut down or an error occurs.
    ///
    pub async fn run(self) -> Result<()> {
        self.server.await.map_err(Into::into)
    }

    /// Returns the public URL of the application.
    ///
    pub fn get_url(&self) -> Result<String> {
        Ok(format!("http://{}", self.server.local_addr()?))
    }
}
