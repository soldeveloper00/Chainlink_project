mod routes;
mod solana_client;
mod chainlink_client;

use std::sync::Arc;
use dotenv::dotenv;
use tracing_subscriber;
use std::env;

use routes::{AppState, create_router};
use solana_client::SolanaService;
use chainlink_client::ChainlinkService;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tracing::info!("🚀 Starting RWA Backend Service");

    // Initialize services
    let solana = match SolanaService::new().await {
        Ok(service) => {
            tracing::info!("✅ Solana service initialized");
            Arc::new(service)
        },
        Err(e) => {
            tracing::error!("❌ Failed to initialize Solana service: {}", e);
            std::process::exit(1);
        }
    };
    
    let chainlink = Arc::new(ChainlinkService::new());
    tracing::info!("✅ Chainlink service initialized");
    
    let state = AppState { solana, chainlink };

    // Build router
    let app = create_router(state);

    // Start server
    let port = env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("📡 Server listening on http://{}", addr);
    
    axum::serve(listener, app).await.unwrap();
}
