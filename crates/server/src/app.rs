use crate::model;
use crate::state::AppState;
use crate::web::middleware as mw;
use crate::web::routes::{catalog, login, server};
use crate::web::{self};
use axum::serve::Serve;
use axum::{Router, middleware};
use dashboard_common::prelude::Result;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;

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
            .merge(login::routes())
            .merge(server::routes(app_state.clone()))
            .merge(catalog::routes(app_state.clone()))
            .merge(SwaggerUi::new("/openapi").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .with_state(app_state.clone())
            .layer(middleware::map_response(mw::log_mapper))
            .layer(mw::allow_cors(&app_state.config.cors));

        Ok(Self {
            server: axum::serve(listener, router),
        })
    }

    /// Runs the application server with graceful shutdown support.
    ///
    /// This method consumes the `App` instance and starts the server, which
    /// will run until it is shut down or an error occurs.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    pub async fn run(self) -> Result<()> {
        // Future that completes when the server should begin graceful shutdown.
        // When the shutdown signal is received, the server stops accepting new
        // connections and waits for active requests to complete before shutting
        // down.
        let shutdown_signal = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for Ctrl+C");
            tracing::info!("Shutting signal received.");
        };

        self.server
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(Into::into)
    }

    /// Returns the public URL of the application.
    ///
    pub fn get_url(&self) -> Result<String> {
        Ok(format!("http://{}", self.server.local_addr()?))
    }
}

/// API documentation for the application.
///
/// This struct defines the OpenAPI specification for the entire application,
/// including all paths, components (schemas), and security schemes.
/// It is used by `utoipa` to generate the OpenAPI JSON and Swagger UI.
///
#[derive(utoipa::OpenApi)]
#[openapi(
    tags(
        (name = "Login", description = "User authentication endpoints"),
        (name = "Server", description = "Server management endpoints"),
        (name = "Catalog", description = "Frontend helper endpoints")
    ),
    paths(
        login::login,
        login::register,
        server::get_user,
        server::list_servers,
        server::create_server,
        server::get_server,
        server::delete_server,
        server::server_action,
        catalog::list_products,
        catalog::list_cpu_options,
        catalog::list_ram_options,
        catalog::list_os_options,
        catalog::list_datacenter_options,
    ),
    components(schemas(
        model::types::NewUser,
        model::types::LoginPayload,
        model::types::ServerStatus,
        model::types::ApiUser,
        web::types::ServerActionPayload,
        web::types::TokenResponse,
        web::types::UserResponse,
    )),
    modifiers(&JwtSecurity)
)]
struct ApiDoc;

/// Modifier to add JWT Bearer authentication scheme to the OpenAPI documentation.
///
/// This struct implements the `utoipa::Modify` trait to programmatically add
/// a security scheme named "bearer_auth" to the generated OpenAPI specification.
/// This scheme uses HTTP Bearer authentication.
///
struct JwtSecurity;
impl utoipa::Modify for JwtSecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            )
        }
    }
}
