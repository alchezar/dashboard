use axum::response::Response;
use axum::{Router, middleware};
use dashboard::prelude::{Controller, Result};
use dashboard::web::{routes_login, routes_server};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Server!");

    let controller = Controller::new().await?;
    let router = Router::new()
        .with_state(controller)
        .merge(routes_login::routes())
        .merge(routes_server::routes())
        .layer(middleware::map_response(main_response_mapper))
        .layer(CookieManagerLayer::new());

    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(&address).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

async fn main_response_mapper(res: Response) -> Response {
    println!();
    res
}
