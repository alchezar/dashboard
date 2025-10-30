use crate::model::queries;
use crate::model::types::{ApiConfigValue, ApiCustomValue, ApiProduct};
use crate::state::AppState;
use crate::web::middleware as mw;
use crate::web::types::{RequiredConfigOption, RequiredCustomField, Response};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router, middleware};
use dashboard_common::prelude::Result;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/products", get(list_products))
        .route("/api/config/cpu", get(list_cpu_options))
        .route("/api/config/ram", get(list_ram_options))
        .route("/api/custom/os", get(list_os_options))
        .route("/api/custom/datacenter", get(list_datacenter_options))
        .route_layer(middleware::from_fn(mw::require_auth))
}

/// Retrieves the product catalog.
///
/// # Arguments
///
/// * `State(app_state)` - The shared application state.
///
/// # Errors
///
/// Returns an `Error` if the database query fails  or if JWT creation fails.
///
#[utoipa::path(
    get,
    path = "/api/products",
    tags = ["Catalog"],
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = Response<Vec<ApiProduct>>, description = "Products found"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 404, body = String, description = "Products not found"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
#[tracing::instrument(level = "trace", target = "handler", skip(app_state))]
async fn list_products(
    State(app_state): State<AppState>,
) -> Result<Json<Response<Vec<ApiProduct>>>> {
    let products = queries::get_products(&app_state.pool).await?;
    tracing::info!(target: "handler", "Found {} products", products.len());

    Ok(Json(Response::new(products)))
}

/// Retrieves CPU options catalog.
///
/// # Arguments
///
/// * `State(app_state)` - The shared application state.
///
/// # Errors
///
/// Returns an `Error` if the database query fails  or if JWT creation fails.
///
#[utoipa::path(
    get,
    path = "/api/config/cpu",
    tags = ["Catalog"],
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = Response<Vec<ApiConfigValue>>, description = "CPU options found"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 404, body = String, description = "CPU options not found"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
async fn list_cpu_options(
    State(app_state): State<AppState>,
) -> Result<Json<Response<Vec<ApiConfigValue>>>> {
    let options =
        queries::get_config_option_value(&app_state.pool, RequiredConfigOption::CPU).await?;
    tracing::info!(target: "handler", count = options.len(), "Found CPU options");

    Ok(Json(Response::new(options)))
}

/// Retrieves RAM options catalog.
///
/// # Arguments
///
/// * `State(app_state)` - The shared application state.
///
/// # Errors
///
/// Returns an `Error` if the database query fails  or if JWT creation fails.
///
#[utoipa::path(
    get,
    path = "/api/config/ram",
    tags = ["Catalog"],
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = Response<Vec<ApiConfigValue>>, description = "RAM options found"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 404, body = String, description = "RAM options not found"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
async fn list_ram_options(
    State(app_state): State<AppState>,
) -> Result<Json<Response<Vec<ApiConfigValue>>>> {
    let options =
        queries::get_config_option_value(&app_state.pool, RequiredConfigOption::RAM).await?;
    tracing::info!(target: "handler", count = options.len(), "Found RAM options");

    Ok(Json(Response::new(options)))
}

/// Retrieves OS Template custom values catalog.
///
/// # Arguments
///
/// * `State(app_state)` - The shared application state.
///
/// # Errors
///
/// Returns an `Error` if the database query fails  or if JWT creation fails.
///
#[utoipa::path(
    get,
    path = "/api/custom/os",
    tags = ["Catalog"],
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = Response<Vec<ApiCustomValue>>, description = "OS Template custom values found"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 404, body = String, description = "OS Template custom values options not found"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
async fn list_os_options(
    State(app_state): State<AppState>,
) -> Result<Json<Response<Vec<ApiCustomValue>>>> {
    let options =
        queries::get_custom_field_value(&app_state.pool, RequiredCustomField::OsTemplate).await?;
    tracing::info!(target: "handler", count = options.len(), "Found OS options");

    Ok(Json(Response::new(options)))
}

/// Retrieves Datacenter location custom values catalog.
///
/// # Arguments
///
/// * `State(app_state)` - The shared application state.
///
/// # Errors
///
/// Returns an `Error` if the database query fails  or if JWT creation fails.
///
#[utoipa::path(
    get,
    path = "/api/custom/datacenter",
    tags = ["Catalog"],
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = Response<Vec<ApiCustomValue>>, description = "Datacenter Location custom values found"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 404, body = String, description = "Datacenter Location custom values options not found"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
async fn list_datacenter_options(
    State(app_state): State<AppState>,
) -> Result<Json<Response<Vec<ApiCustomValue>>>> {
    let options =
        queries::get_custom_field_value(&app_state.pool, RequiredCustomField::Datacenter).await?;
    tracing::info!(target: "handler", count = options.len(), "Found Datacenter options");

    Ok(Json(Response::new(options)))
}
