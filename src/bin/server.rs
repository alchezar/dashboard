use dashboard::prelude::DashboardError;
use dashboard::{database, routes};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), DashboardError> {
    println!("Server!");

    let pool = database::connect().await?;
    let router = routes::create_routes(pool);
    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(&address).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
