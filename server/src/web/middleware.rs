use crate::web::auth::token;
use axum::body::Body;
use axum::http::Request;
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use common::error::{AuthError, Error, Result};

/// A middleware to print a blank line after each response.
///
/// This serves as a simple visual separator between requests in the development
/// console logs.
pub async fn log_mapper(res: Response) -> Response {
    #[cfg(debug_assertions)]
    println!();

    res
}

/// Axum middleware to require authentication.
/// Extracts the Bearer token from the `Authorization` header,
/// validates it, and stores the resulting claims in the request extensions.
///
/// # Arguments
///
/// * `request`: Body of the incoming request.
/// * `next`: `Next` middleware in the chain.
///
/// # Returns
///
/// Response from the next middleware if authentication is successful.
///
pub async fn require_auth(mut request: Request<Body>, next: Next) -> Result<Response> {
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|slice| slice.strip_prefix("Bearer "))
        .ok_or(Error::Auth(AuthError::Token))?;

    let claims = token::validate(token)?;
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}
