use axum::{
    Router,
    routing::{get, post},
    response::Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::solana_client::SolanaService;
use crate::chainlink_client::ChainlinkService;

#[derive(Clone)]
pub struct AppState {
    pub solana: Arc<SolanaService>,
    pub chainlink: Arc<ChainlinkService>,
}

// Request/Response Types
#[derive(Debug, Deserialize)]
pub struct CreateAssetRequest {
    pub asset_id: String,
    pub asset_type: String,
    pub valuation: u64,
    pub metadata_uri: String,
    pub owner: String,
}

#[derive(Debug, Serialize)]
pub struct CreateAssetResponse {
    pub success: bool,
    pub asset_pda: String,
    pub transaction: String,
    pub asset_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRiskRequest {
    pub risk_score: u8,
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateRiskResponse {
    pub success: bool,
    pub transaction: String,
    pub asset_id: String,
    pub new_risk_score: u8,
}

#[derive(Debug, Serialize)]
pub struct AssetResponse {
    pub success: bool,
    pub asset: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateLoanRequest {
    pub asset_id: String,
    pub borrower: String,
    pub loan_amount: u64,
    pub interest_rate: u64,
    pub duration: i64,
}

#[derive(Debug, Serialize)]
pub struct CreateLoanResponse {
    pub success: bool,
    pub loan_pda: String,
    pub transaction: String,
    pub asset_id: String,
}

#[derive(Debug, Serialize)]
pub struct LoanResponse {
    pub success: bool,
    pub loan: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct RiskHistoryResponse {
    pub success: bool,
    pub asset_id: String,
    pub history: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ChainlinkWebhookRequest {
    pub workflow_id: String,
    pub asset_id: String,
    pub risk_score: u8,
    pub confidence: f32,
    pub sources: Vec<String>,
}

// Route Handlers
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "RWA Backend",
        "timestamp": chrono::Utc::now().timestamp(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub async fn create_asset(
    State(state): State<AppState>,
    Json(req): Json<CreateAssetRequest>,
) -> Result<Json<CreateAssetResponse>, (StatusCode, String)> {
    tracing::info!("📝 Creating asset: {}", req.asset_id);
    
    let owner = Pubkey::from_str(&req.owner)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid owner: {}", e)))?;
    
    match state.solana.initialize_asset(
        &req.asset_id,
        &req.asset_type,
        req.valuation,
        &req.metadata_uri,
        owner,
    ).await {
        Ok(result) => {
            tracing::info!("✅ Asset created: {}", req.asset_id);
            Ok(Json(CreateAssetResponse {
                success: true,
                asset_pda: result.asset_pda,
                transaction: result.transaction,
                asset_id: req.asset_id,
            }))
        },
        Err(e) => {
            tracing::error!("❌ Failed to create asset: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

pub async fn get_asset(
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
) -> Result<Json<AssetResponse>, (StatusCode, String)> {
    tracing::info!("🔍 Fetching asset: {}", asset_id);
    
    match state.solana.get_asset(&asset_id).await {
        Ok(asset) => {
            Ok(Json(AssetResponse {
                success: true,
                asset: serde_json::to_value(asset).unwrap(),
            }))
        },
        Err(e) => {
            tracing::error!("❌ Asset not found: {}", e);
            Err((StatusCode::NOT_FOUND, format!("Asset not found: {}", e)))
        }
    }
}

pub async fn update_risk(
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
    Json(req): Json<UpdateRiskRequest>,
) -> Result<Json<UpdateRiskResponse>, (StatusCode, String)> {
    tracing::info!("🔄 Updating risk for {} to {}", asset_id, req.risk_score);
    
    // Optional: Call Chainlink workflow
    if let Some(source) = &req.source {
        if source == "chainlink" {
            match state.chainlink.trigger_risk_update(&asset_id, req.risk_score).await {
                Ok(workflow_id) => {
                    tracing::info!("⛓️ Chainlink workflow triggered: {}", workflow_id);
                },
                Err(e) => {
                    tracing::warn!("⚠️ Chainlink workflow failed: {}", e);
                    // Continue anyway - we'll update directly
                }
            }
        }
    }
    
    match state.solana.update_risk_score(&asset_id, req.risk_score).await {
        Ok(transaction) => {
            tracing::info!("✅ Risk updated for {}", asset_id);
            Ok(Json(UpdateRiskResponse {
                success: true,
                transaction,
                asset_id,
                new_risk_score: req.risk_score,
            }))
        },
        Err(e) => {
            tracing::error!("❌ Failed to update risk: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

pub async fn get_latest_risk(
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!("📊 Fetching latest risk for: {}", asset_id);
    
    match state.solana.get_asset(&asset_id).await {
        Ok(asset) => {
            Ok(Json(serde_json::json!({
                "success": true,
                "asset_id": asset_id,
                "risk_score": asset.risk_score,
                "last_update": asset.last_update,
                "asset_type": asset.asset_type,
                "valuation": asset.valuation
            })))
        },
        Err(e) => {
            Err((StatusCode::NOT_FOUND, format!("Asset not found: {}", e)))
        }
    }
}

pub async fn create_loan(
    State(state): State<AppState>,
    Json(req): Json<CreateLoanRequest>,
) -> Result<Json<CreateLoanResponse>, (StatusCode, String)> {
    tracing::info!("💰 Creating loan for asset: {}", req.asset_id);
    
    let borrower = Pubkey::from_str(&req.borrower)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid borrower: {}", e)))?;
    
    match state.solana.create_loan(
        &req.asset_id,
        borrower,
        req.loan_amount,
        req.interest_rate,
        req.duration,
    ).await {
        Ok(result) => {
            tracing::info!("✅ Loan created: {}", result.loan_pda);
            Ok(Json(CreateLoanResponse {
                success: true,
                loan_pda: result.loan_pda,
                transaction: result.transaction,
                asset_id: req.asset_id,
            }))
        },
        Err(e) => {
            tracing::error!("❌ Failed to create loan: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

pub async fn get_loan(
    State(state): State<AppState>,
    Path(loan_pda): Path<String>,
) -> Result<Json<LoanResponse>, (StatusCode, String)> {
    tracing::info!("🔍 Fetching loan: {}", loan_pda);
    
    let loan_pubkey = Pubkey::from_str(&loan_pda)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid loan PDA: {}", e)))?;
    
    match state.solana.get_loan(loan_pubkey).await {
        Ok(loan) => {
            Ok(Json(LoanResponse {
                success: true,
                loan: serde_json::to_value(loan).unwrap(),
            }))
        },
        Err(e) => {
            Err((StatusCode::NOT_FOUND, format!("Loan not found: {}", e)))
        }
    }
}

pub async fn chainlink_webhook(
    _state: State<AppState>,  // Prefix with underscore to avoid unused warning
    Json(req): Json<ChainlinkWebhookRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!("⛓️ Chainlink webhook received for asset: {}", req.asset_id);
    
    // Update risk score from Chainlink
    // Note: You'll need to implement the Solana update here
    Ok(Json(serde_json::json!({
        "success": true,
        "workflow_id": req.workflow_id,
        "asset_id": req.asset_id,
        "risk_score": req.risk_score,
        "status": "received"
    })))
}

pub async fn get_risk_history(
    _state: State<AppState>,  // Prefix with underscore to avoid unused warning
    Path(asset_id): Path<String>,
) -> Result<Json<RiskHistoryResponse>, (StatusCode, String)> {
    tracing::info!("📈 Fetching risk history for: {}", asset_id);
    
    // This would normally query a database
    // For now, return mock data
    Ok(Json(RiskHistoryResponse {
        success: true,
        asset_id,
        history: vec![
            serde_json::json!({
                "timestamp": chrono::Utc::now().timestamp() - 86400,
                "risk_score": 45,
                "source": "ai_model_v1"
            }),
            serde_json::json!({
                "timestamp": chrono::Utc::now().timestamp() - 43200,
                "risk_score": 52,
                "source": "chainlink"
            }),
            serde_json::json!({
                "timestamp": chrono::Utc::now().timestamp(),
                "risk_score": 35,
                "source": "manual"
            }),
        ],
    }))
}

// Create router function
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/assets", post(create_asset))
        .route("/assets/:asset_id", get(get_asset))
        .route("/assets/:asset_id/risk", post(update_risk))
        .route("/assets/:asset_id/risk/latest", get(get_latest_risk))
        .route("/assets/:asset_id/risk/history", get(get_risk_history))
        .route("/loans", post(create_loan))
        .route("/loans/:loan_pda", get(get_loan))
        .route("/chainlink/webhook", post(chainlink_webhook))
        .with_state(state)
}
