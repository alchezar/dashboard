use crate::error::{AuthError, Error, Result};
use crate::web::auth::{validate_token};
use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn mw_require_auth(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response> {
    let token = request
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(Error::Auth(AuthError::TokenNotFound))?;

    let claims = validate_token(token)?;
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}