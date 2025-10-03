use axum::response::Response;
use axum::{Router, middleware};
use dashboard::prelude::Result;
use dashboard::web::{routes_login, routes_server};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Server!");
    init_logger()?;
    tracing::info!(target: "-- server", "Logger ready.");

    let pool = dashboard::model::queries::connect_to_db().await?;
    let router = Router::new()
        .merge(routes_login::routes())
        .merge(routes_server::routes())
        .with_state(pool)
        .layer(middleware::map_response(main_response_mapper))
        .layer(CorsLayer::new().allow_origin(Any));

    let address = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::info!(target: "-- server", "Listening on {}\n", address);
    let listener = TcpListener::bind(&address).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

async fn main_response_mapper(res: Response) -> Response {
    println!();
    res
}

fn init_logger() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(true)
        .with_max_level(Level::TRACE)
        .with_span_events(FmtSpan::CLOSE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
