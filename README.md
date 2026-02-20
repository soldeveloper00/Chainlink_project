markdown
# 🏦 RWA Collateral Risk Engine

A DeFi lending protocol where Real World Assets (RWAs) are tokenized and used as collateral with AI-driven dynamic risk scoring on Solana blockchain.

## 📋 Table of Contents
- [Overview](#overview)
- [Architecture](#architecture)
- [Smart Contract](#smart-contract)
- [Backend API](#backend-api)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [API Endpoints](#api-endpoints)
- [Testing](#testing)
- [Deployment](#deployment)
- [Project Structure](#project-structure)
- [Team](#team)

## 🔍 Overview
This project implements a **RWA Collateral Risk Engine** on Solana with:
- Tokenization of Real World Assets (real estate, invoices, commodities)
- Dynamic risk scoring using AI/LLM
- Chainlink CRE integration for orchestration
- Automated LTV calculation based on risk scores

## 🏗️ Architecture
┌─────────────────┐ ┌──────────────────┐ ┌─────────────────┐
│ Smart Contract│ │ Rust Backend │ │ Chainlink CRE │
│ (Solana) │◄────│ (Axum Server) │◄────│ (Orchestration)│
└─────────────────┘ └──────────────────┘ └─────────────────┘
▲ ▲ ▲
│ │ │
▼ ▼ ▼
[Tokenization] [REST API Layer] [AI Risk Scoring]

text

## 📦 Smart Contract

### Features
- **Asset Management**: Initialize and manage RWA assets
- **Risk Scoring**: Update risk scores from AI/Chainlink
- **Lending**: Create loans with risk-based LTV
- **Liquidation**: Automatic liquidation for high-risk assets

### Program ID (DevNet)
CGSxN3xi6yrGmc4N1129A521VC2ZPFJ6j9sJoxvv2y7t

text

### Account Structures
```rust
// Asset Account
pub struct Asset {
    asset_id: String,
    asset_type: String,
    valuation: u64,
    metadata_uri: String,
    owner: Pubkey,
    is_active: bool,
    risk_score: u8,
    bump: u8,
}

// Loan Account
pub struct Loan {
    borrower: Pubkey,
    asset: Pubkey,
    principal: u64,
    interest_rate: u64,
    start_time: i64,
    end_time: i64,
    is_active: bool,
    repaid: bool,
    liquidated: bool,
    risk_score_at_creation: u8,
    bump: u8,
}
🚀 Backend API
Technology Stack
Rust with Axum framework

Solana RPC Client for blockchain interaction

Borsh for serialization/deserialization

📋 Prerequisites
bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.18.0/install)"

# Install Anchor
cargo install --git https://github.com/coral-xyz/anchor --tag v0.29.0 anchor-cli --locked

# Install Node.js (for testing)
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Install Yarn
npm install -g yarn
🔧 Installation
1. Clone Repository
bash
git clone https://github.com/yourusername/rwa-collateral.git
cd rwa-collateral
2. Build Smart Contract
bash
anchor build
3. Build Backend
bash
cd backend
cargo build --release
4. Environment Configuration
Create .env file in backend directory:

env
SOLANA_RPC_URL=https://api.devnet.solana.com
PORT=3001
WALLET_PRIVATE_KEY=[YOUR_PRIVATE_KEY_ARRAY]
CHAINLINK_API_KEY=your_chainlink_key
AI_SERVICE_URL=http://localhost:5000
🎮 Usage
Start Backend Server
bash
cd backend
cargo run
# Server runs on http://localhost:3001
Deploy Smart Contract
bash
# Set Solana config to DevNet
solana config set --url devnet

# Deploy
anchor deploy
📡 API Endpoints
Method	Endpoint	Description
GET	/health	Health check
POST	/assets	Create new asset
GET	/assets/:asset_id	Get asset details
POST	/assets/:asset_id/risk	Update risk score
GET	/assets/:asset_id/risk/latest	Get latest risk
GET	/assets/:asset_id/risk/history	Get risk history
POST	/loans	Create loan
GET	/loans/:loan_pda	Get loan details
POST	/chainlink/webhook	Chainlink webhook
API Examples
Health Check
bash
curl http://localhost:3001/health
Create Asset
bash
curl -X POST http://localhost:3001/assets \
  -H "Content-Type: application/json" \
  -d '{
    "asset_id": "asset-001",
    "asset_type": "real_estate",
    "valuation": 50000000,
    "metadata_uri": "ipfs://QmTest123",
    "owner": "AQ68XzKR3fjGypbKi6Ai23vUBTTbEhuKg6EY4uBqAfVY"
  }'
Get Asset
bash
curl http://localhost:3001/assets/asset-001
Update Risk Score
bash
curl -X POST http://localhost:3001/assets/asset-001/risk \
  -H "Content-Type: application/json" \
  -d '{"risk_score": 35}'
Get Latest Risk
bash
curl http://localhost:3001/assets/asset-001/risk/latest
Create Loan
bash
curl -X POST http://localhost:3001/loans \
  -H "Content-Type: application/json" \
  -d '{
    "asset_id": "asset-001",
    "borrower": "AQ68XzKR3fjGypbKi6Ai23vUBTTbEhuKg6EY4uBqAfVY",
    "loan_amount": 17500000,
    "interest_rate": 500,
    "duration": 2592000
  }'
Chainlink Webhook
bash
curl -X POST http://localhost:3001/chainlink/webhook \
  -H "Content-Type: application/json" \
  -d '{
    "workflow_id": "chainlink-workflow-1",
    "asset_id": "asset-001",
    "risk_score": 42,
    "confidence": 0.95,
    "sources": ["chainlink", "ai-model"]
  }'
🧪 Testing
Smart Contract Tests
bash
# Run all tests (with local validator)
anchor test

# Run tests on DevNet (skip deploy)
anchor test --skip-deploy --provider.cluster devnet

# Run specific test file
anchor test -- --test test_file_name
Backend Tests
Using Bash Script
bash
cd backend
chmod +x test_api.sh
./test_api.sh
Using Node.js
bash
cd backend
npm install axios
node test_api.js
Manual Testing with Curl
Save this as manual_test.sh:

bash
#!/bin/bash

BASE_URL="http://localhost:3001"
WALLET="AQ68XzKR3fjGypbKi6Ai23vUBTTbEhuKg6EY4uBqAfVY"
ASSET_ID="test-asset-$(date +%s)"

echo "🔍 Testing RWA Backend API"
echo "=========================="

# 1. Health Check
echo -n "1. Health Check: "
curl -s $BASE_URL/health | grep -q "healthy" && echo "✅" || echo "❌"

# 2. Create Asset
echo -n "2. Create Asset: "
curl -s -X POST $BASE_URL/assets \
  -H "Content-Type: application/json" \
  -d '{
    "asset_id": "'$ASSET_ID'",
    "asset_type": "real_estate",
    "valuation": 50000000,
    "metadata_uri": "ipfs://QmTest123",
    "owner": "'$WALLET'"
  }' | grep -q "success" && echo "✅" || echo "❌"

# 3. Get Asset
echo -n "3. Get Asset: "
curl -s $BASE_URL/assets/$ASSET_ID | grep -q "$ASSET_ID" && echo "✅" || echo "❌"

# 4. Update Risk
echo -n "4. Update Risk: "
curl -s -X POST $BASE_URL/assets/$ASSET_ID/risk \
  -H "Content-Type: application/json" \
  -d '{"risk_score": 35}' | grep -q "success" && echo "✅" || echo "❌"

# 5. Get Latest Risk
echo -n "5. Get Latest Risk: "
curl -s $BASE_URL/assets/$ASSET_ID/risk/latest | grep -q "35" && echo "✅" || echo "❌"

echo "=========================="
echo "✅ Tests complete! Asset ID: $ASSET_ID"
Run it:

bash
chmod +x manual_test.sh
./manual_test.sh
Expected Test Output
text
========================================
   RWA Backend API Test Suite
========================================
▶ Test 1: Health Check ✅ PASSED
▶ Test 2: Create Asset ✅ PASSED
▶ Test 3: Get Asset ✅ PASSED
▶ Test 4: Update Risk ✅ PASSED
▶ Test 5: Get Latest Risk ✅ PASSED
▶ Test 6: Create Loan ✅ PASSED
▶ Test 7: Get Risk History ✅ PASSED
▶ Test 8: Chainlink Webhook ✅ PASSED
========================================
✅ ALL TESTS PASSED! (8/8)
========================================
🚀 Deployment
Deploy Smart Contract to DevNet
bash
# Set Solana config to DevNet
solana config set --url devnet

# Get some DevNet SOL (if needed)
solana airdrop 2

# Deploy
anchor deploy
Deploy Backend (Production)
bash
cd backend
cargo build --release
./target/release/backend
Using Docker
bash
# Build Docker image
docker build -t rwa-backend .

# Run container
docker run -p 3001:3001 --env-file .env rwa-backend
📁 Project Structure
text
rwa_collateral/
├── programs/
│   └── rwa_collateral/          # Smart Contract
│       ├── src/
│       │   └── lib.rs
│       └── Cargo.toml
├── backend/                       # Rust Backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── routes.rs
│   │   ├── solana_client.rs
│   │   └── chainlink_client.rs
│   ├── tests/
│   │   └── api_tests.rs
│   ├── Cargo.toml
│   └── .env
├── tests/                         # Integration tests
│   └── rwa_collateral.ts
├── Anchor.toml
└── README.md
🔗 Integration Points
Smart Contract ↔ Backend
Program ID: 3ekhJkk57HSt8Rfj44fmgjhix9UXTJVBi6ZQEz7Hs5Po

RPC URL: https://api.devnet.solana.com

Functions: initialize_asset, update_risk_score, create_loan, repay_loan, liquidate_loan

Backend ↔ Chainlink CRE
Webhook endpoint: POST /chainlink/webhook

Workflow triggers: Risk score updates

Data format: JSON with asset_id, risk_score, confidence, sources

Backend ↔ AI Service
Endpoint: Configured via AI_SERVICE_URL env var

Expected: Risk scores with confidence metrics

🧪 Troubleshooting
Common Issues & Solutions
Issue	Solution
429 Too Many Requests	Use local validator or Helius RPC
Program not found	Deploy program first: anchor deploy
InstructionFallbackNotFound	Check discriminators in solana_client.rs
Backend not starting	Check .env file and port availability
Tests failing	Ensure backend is running on port 3001
Quick Fix Commands
bash
# Reset everything
cd ~/projects/rwa_collateral
rm -rf target node_modules
cargo clean
anchor build

# Deploy fresh
anchor deploy

# Run backend
cd backend && cargo run
