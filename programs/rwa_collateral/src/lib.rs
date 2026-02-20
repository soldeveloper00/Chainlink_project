use anchor_lang::prelude::*;

declare_id!("3ekhJkk57HSt8Rfj44fmgjhix9UXTJVBi6ZQEz7Hs5Po");

#[program]
pub mod rwa_collateral {
    use super::*;

    // Initialize a new RWA asset
    pub fn initialize_asset(
        ctx: Context<InitializeAsset>,
        asset_id: String,
        asset_type: String,
        valuation: u64,
        metadata_uri: String,
    ) -> Result<()> {
        let asset = &mut ctx.accounts.asset;
        
        asset.asset_id = asset_id;
        asset.asset_type = asset_type;
        asset.valuation = valuation;
        asset.metadata_uri = metadata_uri;
        asset.owner = *ctx.accounts.owner.key;
        asset.is_active = true;
        asset.risk_score = 50; // Default medium risk
        asset.bump = ctx.bumps.asset;
        
        msg!("Asset created: {}", asset.asset_id);
        Ok(())
    }

    // Update risk score (called by AI oracle)
    pub fn update_risk_score(
        ctx: Context<UpdateRiskScore>,
        new_risk_score: u8,
    ) -> Result<()> {
        let asset = &mut ctx.accounts.asset;
        
        require!(asset.is_active, ErrorCode::AssetInactive);
        require!(new_risk_score <= 100, ErrorCode::InvalidRiskScore);
        
        asset.risk_score = new_risk_score;
        
        msg!("Risk score updated to: {}", new_risk_score);
        Ok(())
    }

    // Create loan against RWA
    pub fn create_loan(
        ctx: Context<CreateLoan>,
        loan_amount: u64,
        interest_rate: u64, // basis points (1% = 100)
        duration: i64,      // in seconds
    ) -> Result<()> {
        let loan = &mut ctx.accounts.loan;
        let asset = &ctx.accounts.asset;
        
        // Calculate max loan based on risk score
        let max_ltv = match asset.risk_score {
            0..=20 => 70,  // Low risk: 70% LTV
            21..=40 => 60, // Medium-low: 60% LTV
            41..=60 => 50, // Medium: 50% LTV
            61..=80 => 35, // Medium-high: 35% LTV
            81..=100 => 20, // High risk: 20% LTV
            _ => 0,
        };
        
        let max_loan = (asset.valuation as u128 * max_ltv as u128 / 100) as u64;
        require!(loan_amount <= max_loan, ErrorCode::LoanTooHigh);
        
        loan.borrower = *ctx.accounts.borrower.key;
        loan.asset = asset.key();
        loan.principal = loan_amount;
        loan.interest_rate = interest_rate;
        loan.start_time = Clock::get()?.unix_timestamp;
        loan.end_time = loan.start_time + duration;
        loan.is_active = true;
        loan.risk_score_at_creation = asset.risk_score;
        loan.bump = ctx.bumps.loan;
        
        msg!("Loan created: {} for asset {}", loan_amount, asset.asset_id);
        Ok(())
    }

    // Repay loan
    pub fn repay_loan(ctx: Context<RepayLoan>) -> Result<()> {
        let loan = &mut ctx.accounts.loan;
        
        require!(loan.is_active, ErrorCode::LoanInactive);
        
        loan.is_active = false;
        loan.repaid = true;
        
        msg!("Loan repaid");
        Ok(())
    }

    // Liquidate loan if risk too high
    pub fn liquidate_loan(ctx: Context<LiquidateLoan>) -> Result<()> {
        let loan = &mut ctx.accounts.loan;
        let asset = &ctx.accounts.asset;
        
        require!(loan.is_active, ErrorCode::LoanInactive);
        require!(asset.risk_score > 80, ErrorCode::NotEligibleForLiquidation);
        
        loan.is_active = false;
        loan.liquidated = true;
        
        msg!("Loan liquidated due to high risk: {}", asset.risk_score);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(asset_id: String)]
pub struct InitializeAsset<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + 32 + 32 + 8 + 200 + 32 + 1 + 1 + 1,
        seeds = [b"asset", asset_id.as_bytes()],
        bump
    )]
    pub asset: Account<'info, Asset>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRiskScore<'info> {
    #[account(
        mut,
        seeds = [b"asset", asset.asset_id.as_bytes()],
        bump = asset.bump
    )]
    pub asset: Account<'info, Asset>,
    
    pub authority: Signer<'info>, // Oracle authority
}

#[derive(Accounts)]
pub struct CreateLoan<'info> {
    #[account(
        init,
        payer = borrower,
        space = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1 + 1 + 1 + 8,
        seeds = [b"loan", asset.key().as_ref(), borrower.key().as_ref()],
        bump
    )]
    pub loan: Account<'info, Loan>,
    
    #[account(
        mut,
        seeds = [b"asset", asset.asset_id.as_bytes()],
        bump = asset.bump
    )]
    pub asset: Account<'info, Asset>,
    
    #[account(mut)]
    pub borrower: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RepayLoan<'info> {
    #[account(
        mut,
        seeds = [b"loan", loan.asset.as_ref(), loan.borrower.as_ref()],
        bump = loan.bump
    )]
    pub loan: Account<'info, Loan>,
    
    #[account(mut)]
    pub borrower: Signer<'info>,
}

#[derive(Accounts)]
pub struct LiquidateLoan<'info> {
    #[account(
        mut,
        seeds = [b"loan", loan.asset.as_ref(), loan.borrower.as_ref()],
        bump = loan.bump
    )]
    pub loan: Account<'info, Loan>,
    
    #[account(
        seeds = [b"asset", asset.asset_id.as_bytes()],
        bump = asset.bump
    )]
    pub asset: Account<'info, Asset>,
    
    pub liquidator: Signer<'info>,
}

#[account]
pub struct Asset {
    pub asset_id: String,        // 32 bytes
    pub asset_type: String,      // 32 bytes
    pub valuation: u64,          // 8 bytes
    pub metadata_uri: String,    // 200 bytes
    pub owner: Pubkey,           // 32 bytes
    pub is_active: bool,         // 1 byte
    pub risk_score: u8,          // 1 byte
    pub bump: u8,                // 1 byte
}

#[account]
pub struct Loan {
    pub borrower: Pubkey,        // 32 bytes
    pub asset: Pubkey,           // 32 bytes
    pub principal: u64,          // 8 bytes
    pub interest_rate: u64,      // 8 bytes
    pub start_time: i64,         // 8 bytes
    pub end_time: i64,           // 8 bytes
    pub is_active: bool,         // 1 byte
    pub repaid: bool,            // 1 byte
    pub liquidated: bool,        // 1 byte
    pub risk_score_at_creation: u8, // 1 byte
    pub bump: u8,                // 1 byte
}

#[error_code]
pub enum ErrorCode {
    #[msg("Asset is not active")]
    AssetInactive,
    #[msg("Invalid risk score")]
    InvalidRiskScore,
    #[msg("Loan amount exceeds maximum LTV")]
    LoanTooHigh,
    #[msg("Loan is not active")]
    LoanInactive,
    #[msg("Not eligible for liquidation")]
    NotEligibleForLiquidation,
}