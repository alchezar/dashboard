use crate::model;
use crate::state::AppState;
use crate::web::middleware as mw;
use crate::web::{self, routes_login, routes_server};
use axum::serve::Serve;
use axum::{Router, middleware};
use dashboard_common::error::Result;
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
            .merge(routes_login::routes())
            .merge(routes_server::routes())
            .merge(SwaggerUi::new("/openapi").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .with_state(app_state)
            .layer(middleware::map_response(mw::log_mapper))
            .layer(mw::allow_cors());

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

/// API documentation for the application.
///
/// This struct defines the OpenAPI specification for the entire application,
/// including all paths, components (schemas), and security schemes.
/// It is used by `utoipa` to generate the OpenAPI JSON and Swagger UI.
///
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        routes_login::login,
        routes_login::register,
        routes_server::get_user,
        routes_server::list_servers,
        routes_server::create_server,
        routes_server::get_server,
        routes_server::delete_server,
        routes_server::server_action,
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
