use serde::{Deserialize, Serialize};
use std::env;
use anyhow::{anyhow, Result};
use reqwest::Client as HttpClient;

#[derive(Debug, Clone)]
pub struct ChainlinkService {
    http_client: HttpClient,
    api_key: String,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainlinkWorkflow {
    pub id: String,
    pub name: String,
    pub status: WorkflowStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub trigger: TriggerConfig,
    pub tasks: Vec<TaskConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerConfig {
    #[serde(rename = "cron")]
    Cron { schedule: String },
    #[serde(rename = "webhook")]
    Webhook { endpoint: String },
    #[serde(rename = "event")]
    Event { contract: String, event: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaskConfig {
    #[serde(rename = "http")]
    Http {
        url: String,
        method: String,
        headers: Option<Vec<(String, String)>>,
    },
    #[serde(rename = "consensus")]
    Consensus {
        sources: Vec<String>,
        threshold: u32,
        aggregation: String,
    },
    #[serde(rename = "contract")]
    Contract {
        blockchain: String,
        contract_address: String,
        function: String,
        args: Vec<String>,
    },
    #[serde(rename = "transform")]
    Transform {
        expression: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: String,
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub results: Vec<TaskResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskUpdateRequest {
    pub asset_id: String,
    pub risk_score: u8,
    pub sources: Vec<String>,
    pub confidence: f32,
}

impl ChainlinkService {
    pub fn new() -> Self {
        let api_key = env::var("CHAINLINK_API_KEY")
            .unwrap_or_else(|_| "test_key".to_string());
        
        let base_url = env::var("CHAINLINK_CRE_URL")
            .unwrap_or_else(|_| "https://cre.chainlink.io/api/v1".to_string());
        
        Self {
            http_client: HttpClient::new(),
            api_key,
            base_url,
        }
    }

    // Create a risk monitoring workflow
    pub async fn create_risk_workflow(
        &self,
        asset_id: &str,
        schedule: &str,
    ) -> Result<ChainlinkWorkflow> {
        let workflow_def = WorkflowDefinition {
            name: format!("RWA-Risk-Monitor-{}", asset_id),
            trigger: TriggerConfig::Cron {
                schedule: schedule.to_string(),
            },
            tasks: vec![
                // Task 1: Fetch from AI service
                TaskConfig::Http {
                    url: format!("{}/api/risk/{}", 
                        env::var("AI_SERVICE_URL").unwrap_or_default(), 
                        asset_id
                    ),
                    method: "GET".to_string(),
                    headers: Some(vec![
                        ("X-API-Key".to_string(), env::var("AI_API_KEY").unwrap_or_default())
                    ]),
                },
                // Task 2: Consensus check
                TaskConfig::Consensus {
                    sources: vec!["task_0".to_string()],
                    threshold: 1,
                    aggregation: "median".to_string(),
                },
                // Task 3: Update Solana contract
                TaskConfig::Contract {
                    blockchain: "solana".to_string(),
                    contract_address: env::var("PROGRAM_ID")
                        .unwrap_or_else(|_| "5BsUewMAmMm5PeFCyK5NXgidYFUja87iWhmmxiw9YLzT".to_string()),
                    function: "updateRiskScore".to_string(),
                    args: vec![
                        asset_id.to_string(),
                        "${consensus.result}".to_string(),
                    ],
                },
            ],
        };

        let response = self.http_client
            .post(&format!("{}/workflows", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&workflow_def)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to create workflow: {}", response.status()));
        }

        let workflow = response.json().await?;
        Ok(workflow)
    }

    // Trigger immediate risk update
    pub async fn trigger_risk_update(
        &self,
        asset_id: &str,
        risk_score: u8,
    ) -> Result<String> {
        let update = RiskUpdateRequest {
            asset_id: asset_id.to_string(),
            risk_score,
            sources: vec!["manual".to_string()],
            confidence: 1.0,
        };

        let response = self.http_client
            .post(&format!("{}/oracle/update-risk", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&update)
            .send()
            .await?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result["workflow_id"].as_str().unwrap_or("unknown").to_string())
        } else {
            Err(anyhow!("Trigger failed: {}", response.status()))
        }
    }

    // Get workflow status
    pub async fn get_workflow_status(&self, workflow_id: &str) -> Result<WorkflowExecution> {
        let response = self.http_client
            .get(&format!("{}/workflows/{}/executions/latest", self.base_url, workflow_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get workflow status: {}", response.status()));
        }

        let execution = response.json().await?;
        Ok(execution)
    }

    // Simulate a risk update (for testing)
    pub async fn simulate_risk_update(
        &self,
        asset_id: &str,
        mock_risk_score: u8,
    ) -> Result<serde_json::Value> {
        let simulation = serde_json::json!({
            "workflow": {
                "tasks": [
                    {
                        "type": "http",
                        "config": {
                            "url": format!("{}/api/risk/{}", 
                                env::var("AI_SERVICE_URL").unwrap_or_default(), 
                                asset_id
                            ),
                            "method": "GET",
                            "mockResponse": {
                                "riskScore": mock_risk_score,
                                "confidence": 0.95
                            }
                        }
                    },
                    {
                        "type": "contract",
                        "config": {
                            "blockchain": "solana",
                            "contractAddress": env::var("PROGRAM_ID")
                                .unwrap_or_else(|_| "5BsUewMAmMm5PeFCyK5NXgidYFUja87iWhmmxiw9YLzT".to_string()),
                            "function": "updateRiskScore",
                            "args": [asset_id, mock_risk_score],
                            "mockExecution": true
                        }
                    }
                ]
            }
        });

        let response = self.http_client
            .post(&format!("{}/simulate", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&simulation)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Simulation failed: {}", response.status()));
        }

        let result = response.json().await?;
        Ok(result)
    }

    // Pause workflow
    pub async fn pause_workflow(&self, workflow_id: &str) -> Result<bool> {
        let response = self.http_client
            .post(&format!("{}/workflows/{}/pause", self.base_url, workflow_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    // Resume workflow
    pub async fn resume_workflow(&self, workflow_id: &str) -> Result<bool> {
        let response = self.http_client
            .post(&format!("{}/workflows/{}/resume", self.base_url, workflow_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    // Delete workflow
    pub async fn delete_workflow(&self, workflow_id: &str) -> Result<bool> {
        let response = self.http_client
            .delete(&format!("{}/workflows/{}", self.base_url, workflow_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}
