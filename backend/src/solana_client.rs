use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, read_keypair_file},
    signer::Signer,
    commitment_config::CommitmentConfig,
    system_program,
    instruction::Instruction,
    transaction::Transaction,
};
use std::sync::Arc;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use std::env;
use anyhow::{anyhow, Result};

const PROGRAM_ID: &str = "3ekhJkk57HSt8Rfj44fmgjhix9UXTJVBi6ZQEz7Hs5Po";

// ==================== CORRECT DISCRIMINATORS FROM IDL ====================
const DISCRIMINATOR_INITIALIZE_ASSET: [u8; 8] = [214, 153, 49, 248, 95, 248, 208, 179];
const DISCRIMINATOR_UPDATE_RISK: [u8; 8] = [80, 138, 35, 224, 23, 172, 20, 254];
const DISCRIMINATOR_CREATE_LOAN: [u8; 8] = [166, 131, 118, 219, 138, 218, 206, 140];
const DISCRIMINATOR_REPAY_LOAN: [u8; 8] = [224, 93, 144, 77, 61, 17, 137, 54];
const DISCRIMINATOR_LIQUIDATE_LOAN: [u8; 8] = [111, 249, 185, 54, 161, 147, 178, 24];

// ==================== API Response Types ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetResponse {
    pub asset_id: String,
    pub asset_type: String,
    pub valuation: u64,
    pub metadata_uri: String,
    pub owner: String,
    pub is_active: bool,
    pub risk_score: u8,
    pub last_update: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanResponse {
    pub borrower: String,
    pub asset: String,
    pub principal: u64,
    pub interest_rate: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub is_active: bool,
    pub liquidated: bool,
    pub repaid: bool,
    pub risk_score_at_creation: u8,
}

// ==================== Manual Account Data Structures ====================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAccount {
    pub asset_id: String,
    pub asset_type: String,
    pub valuation: u64,
    pub metadata_uri: String,
    pub owner: Pubkey,
    pub is_active: bool,
    pub risk_score: u8,
    pub last_update: i64,
    pub bump: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanAccount {
    pub borrower: Pubkey,
    pub asset: Pubkey,
    pub principal: u64,
    pub interest_rate: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub is_active: bool,
    pub repaid: bool,
    pub liquidated: bool,
    pub risk_score_at_creation: u8,
    pub bump: u8,
}

// ==================== Borsh-like Serialization/Deserialization ====================
impl AssetAccount {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = 8; // Skip discriminator
        
        let asset_id_len = u32::from_le_bytes(data[cursor..cursor+4].try_into()?) as usize;
        cursor += 4;
        let asset_id = String::from_utf8(data[cursor..cursor+asset_id_len].to_vec())?;
        cursor += asset_id_len;
        
        let asset_type_len = u32::from_le_bytes(data[cursor..cursor+4].try_into()?) as usize;
        cursor += 4;
        let asset_type = String::from_utf8(data[cursor..cursor+asset_type_len].to_vec())?;
        cursor += asset_type_len;
        
        let valuation = u64::from_le_bytes(data[cursor..cursor+8].try_into()?);
        cursor += 8;
        
        let metadata_uri_len = u32::from_le_bytes(data[cursor..cursor+4].try_into()?) as usize;
        cursor += 4;
        let metadata_uri = String::from_utf8(data[cursor..cursor+metadata_uri_len].to_vec())?;
        cursor += metadata_uri_len;
        
        let owner = Pubkey::new_from_array(data[cursor..cursor+32].try_into()?);
        cursor += 32;
        
        let is_active = data[cursor] != 0;
        cursor += 1;
        
        let risk_score = data[cursor];
        cursor += 1;
        
        let last_update = i64::from_le_bytes(data[cursor..cursor+8].try_into()?);
        cursor += 8;
        
        let bump = data[cursor];
        
        Ok(AssetAccount {
            asset_id,
            asset_type,
            valuation,
            metadata_uri,
            owner,
            is_active,
            risk_score,
            last_update,
            bump,
        })
    }
}

impl LoanAccount {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = 8; // Skip discriminator
        
        let borrower = Pubkey::new_from_array(data[cursor..cursor+32].try_into()?);
        cursor += 32;
        
        let asset = Pubkey::new_from_array(data[cursor..cursor+32].try_into()?);
        cursor += 32;
        
        let principal = u64::from_le_bytes(data[cursor..cursor+8].try_into()?);
        cursor += 8;
        
        let interest_rate = u64::from_le_bytes(data[cursor..cursor+8].try_into()?);
        cursor += 8;
        
        let start_time = i64::from_le_bytes(data[cursor..cursor+8].try_into()?);
        cursor += 8;
        
        let end_time = i64::from_le_bytes(data[cursor..cursor+8].try_into()?);
        cursor += 8;
        
        let is_active = data[cursor] != 0;
        cursor += 1;
        
        let repaid = data[cursor] != 0;
        cursor += 1;
        
        let liquidated = data[cursor] != 0;
        cursor += 1;
        
        let risk_score_at_creation = data[cursor];
        cursor += 1;
        
        let bump = data[cursor];
        
        Ok(LoanAccount {
            borrower,
            asset,
            principal,
            interest_rate,
            start_time,
            end_time,
            is_active,
            repaid,
            liquidated,
            risk_score_at_creation,
            bump,
        })
    }
}

// ==================== Solana Service ====================
pub struct SolanaService {
    client: Arc<RpcClient>,
    program_id: Pubkey,
    payer: Keypair,
}

pub struct InitializeAssetResult {
    pub asset_pda: String,
    pub transaction: String,
}

pub struct CreateLoanResult {
    pub loan_pda: String,
    pub transaction: String,
}

impl SolanaService {
    pub async fn new() -> Result<Self> {
        let rpc_url = env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
        
        let program_id = Pubkey::from_str(PROGRAM_ID)
            .map_err(|e| anyhow!("Invalid program ID: {}", e))?;
        
        let payer = if let Ok(private_key) = env::var("WALLET_PRIVATE_KEY") {
            let bytes: Vec<u8> = serde_json::from_str(&private_key)
                .map_err(|e| anyhow!("Invalid private key format: {}", e))?;
            Keypair::from_bytes(&bytes)
                .map_err(|e| anyhow!("Failed to create keypair: {}", e))?
        } else {
            let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home dir"))?;
            let keypath = home.join(".config/solana/id.json");
            read_keypair_file(&keypath)
                .map_err(|e| anyhow!("Failed to read keypair: {}", e))?
        };
        
        let client = Arc::new(RpcClient::new_with_commitment(
            rpc_url,
            CommitmentConfig::confirmed(),
        ));
        
        let _ = client.get_latest_blockhash()
            .map_err(|e| anyhow!("Failed to connect to Solana: {}", e))?;
        
        Ok(Self {
            client,
            program_id,
            payer,
        })
    }

    pub async fn initialize_asset(
        &self,
        asset_id: &str,
        asset_type: &str,
        valuation: u64,
        metadata_uri: &str,
        owner: Pubkey,
    ) -> Result<InitializeAssetResult> {
        let (asset_pda, bump) = Pubkey::find_program_address(
            &[b"asset", asset_id.as_bytes()],
            &self.program_id,
        );

        tracing::info!("Asset PDA: {} with bump: {}", asset_pda, bump);

        let mut instruction_data = DISCRIMINATOR_INITIALIZE_ASSET.to_vec();
        
        // Serialize parameters (simplified string encoding)
        let asset_id_bytes = asset_id.as_bytes();
        instruction_data.extend_from_slice(&(asset_id_bytes.len() as u32).to_le_bytes());
        instruction_data.extend_from_slice(asset_id_bytes);
        
        let asset_type_bytes = asset_type.as_bytes();
        instruction_data.extend_from_slice(&(asset_type_bytes.len() as u32).to_le_bytes());
        instruction_data.extend_from_slice(asset_type_bytes);
        
        instruction_data.extend_from_slice(&valuation.to_le_bytes());
        
        let metadata_bytes = metadata_uri.as_bytes();
        instruction_data.extend_from_slice(&(metadata_bytes.len() as u32).to_le_bytes());
        instruction_data.extend_from_slice(metadata_bytes);

        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(asset_pda, false),
            solana_sdk::instruction::AccountMeta::new(owner, true),
            solana_sdk::instruction::AccountMeta::new_readonly(system_program::id(), false),
        ];

        let instruction = Instruction {
            program_id: self.program_id,
            accounts,
            data: instruction_data,
        };

        let recent_blockhash = self.client.get_latest_blockhash()
            .map_err(|e| anyhow!("Failed to get blockhash: {}", e))?;
            
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&owner),
            &[&self.payer],
            recent_blockhash,
        );

        let signature = self.client.send_and_confirm_transaction(&transaction)
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        Ok(InitializeAssetResult {
            asset_pda: asset_pda.to_string(),
            transaction: signature.to_string(),
        })
    }

    pub async fn update_risk_score(
        &self,
        asset_id: &str,
        risk_score: u8,
    ) -> Result<String> {
        let (asset_pda, _) = Pubkey::find_program_address(
            &[b"asset", asset_id.as_bytes()],
            &self.program_id,
        );

        let mut instruction_data = DISCRIMINATOR_UPDATE_RISK.to_vec();
        instruction_data.push(risk_score);

        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(asset_pda, false),
            solana_sdk::instruction::AccountMeta::new_readonly(self.payer.pubkey(), true),
        ];

        let instruction = Instruction {
            program_id: self.program_id,
            accounts,
            data: instruction_data,
        };

        let recent_blockhash = self.client.get_latest_blockhash()
            .map_err(|e| anyhow!("Failed to get blockhash: {}", e))?;
            
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_blockhash,
        );

        let signature = self.client.send_and_confirm_transaction(&transaction)
            .map_err(|e| anyhow!("Update failed: {}", e))?;

        Ok(signature.to_string())
    }

    pub async fn get_asset(&self, asset_id: &str) -> Result<AssetResponse> {
        let (asset_pda, _) = Pubkey::find_program_address(
            &[b"asset", asset_id.as_bytes()],
            &self.program_id,
        );

        tracing::info!("Fetching asset from PDA: {}", asset_pda);

        let account = self.client.get_account(&asset_pda)
            .map_err(|e| anyhow!("Asset not found: {}", e))?;
        
        let asset_account = AssetAccount::from_bytes(&account.data)?;
        
        Ok(AssetResponse {
            asset_id: asset_account.asset_id,
            asset_type: asset_account.asset_type,
            valuation: asset_account.valuation,
            metadata_uri: asset_account.metadata_uri,
            owner: asset_account.owner.to_string(),
            is_active: asset_account.is_active,
            risk_score: asset_account.risk_score,
            last_update: asset_account.last_update,
        })
    }

    pub async fn create_loan(
        &self,
        asset_id: &str,
        borrower: Pubkey,
        loan_amount: u64,
        interest_rate: u64,
        duration: i64,
    ) -> Result<CreateLoanResult> {
        let (asset_pda, _) = Pubkey::find_program_address(
            &[b"asset", asset_id.as_bytes()],
            &self.program_id,
        );

        let (loan_pda, _) = Pubkey::find_program_address(
            &[b"loan", asset_pda.as_ref(), borrower.as_ref()],
            &self.program_id,
        );

        tracing::info!("Loan PDA: {}", loan_pda);

        let mut instruction_data = DISCRIMINATOR_CREATE_LOAN.to_vec();
        instruction_data.extend_from_slice(&loan_amount.to_le_bytes());
        instruction_data.extend_from_slice(&interest_rate.to_le_bytes());
        instruction_data.extend_from_slice(&duration.to_le_bytes());

        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(loan_pda, false),
            solana_sdk::instruction::AccountMeta::new(asset_pda, false),
            solana_sdk::instruction::AccountMeta::new(borrower, true),
            solana_sdk::instruction::AccountMeta::new_readonly(system_program::id(), false),
        ];

        let instruction = Instruction {
            program_id: self.program_id,
            accounts,
            data: instruction_data,
        };

        let recent_blockhash = self.client.get_latest_blockhash()
            .map_err(|e| anyhow!("Failed to get blockhash: {}", e))?;
            
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&borrower),
            &[&self.payer],
            recent_blockhash,
        );

        let signature = self.client.send_and_confirm_transaction(&transaction)
            .map_err(|e| anyhow!("Loan creation failed: {}", e))?;

        Ok(CreateLoanResult {
            loan_pda: loan_pda.to_string(),
            transaction: signature.to_string(),
        })
    }

    pub async fn get_loan(&self, loan_pda: Pubkey) -> Result<LoanResponse> {
        tracing::info!("Fetching loan from PDA: {}", loan_pda);

        let account = self.client.get_account(&loan_pda)
            .map_err(|e| anyhow!("Loan not found: {}", e))?;
        
        let loan_account = LoanAccount::from_bytes(&account.data)?;
        
        Ok(LoanResponse {
            borrower: loan_account.borrower.to_string(),
            asset: loan_account.asset.to_string(),
            principal: loan_account.principal,
            interest_rate: loan_account.interest_rate,
            start_time: loan_account.start_time,
            end_time: loan_account.end_time,
            is_active: loan_account.is_active,
            liquidated: loan_account.liquidated,
            repaid: loan_account.repaid,
            risk_score_at_creation: loan_account.risk_score_at_creation,
        })
    }

    pub fn get_payer_pubkey(&self) -> Pubkey {
        self.payer.pubkey()
    }
}
