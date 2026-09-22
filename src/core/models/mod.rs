use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub task_id: Option<String>,
    pub workflow_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CampaignBrowser {
    pub browser_id: Option<String>,
    pub tasks: Option<Vec<Task>>,
}

// Backward compatibility type alias
pub type Member = CampaignBrowser;

#[derive(Debug, Serialize, Deserialize)]
pub struct Campaign {
    pub campaign_id: Option<String>,
    #[serde(alias = "members")]
    pub browsers: Option<Vec<CampaignBrowser>>,
}

pub mod id;
pub use id::*;
pub mod workflow;
pub mod settings;
pub mod storage;
pub use storage::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_campaign_parsing() {
        let json_data = r#"{
            "campaign_id": "camp_123",
            "browsers": [
                {
                    "browser_id": "browser_abc",
                    "tasks": [
                        {
                            "task_id": "task_1",
                            "workflow_id": "wf_abc"
                        }
                    ]
                }
            ]
        }"#;

        let campaign: Campaign = serde_json::from_str(json_data).unwrap();
        assert_eq!(campaign.campaign_id.unwrap(), "camp_123");
        assert_eq!(campaign.browsers.unwrap()[0].browser_id.as_deref().unwrap(), "browser_abc");

        // Backward compatibility with "members"
        let legacy_json = r#"{
            "campaign_id": "camp_legacy",
            "members": [
                {
                    "browser_id": "browser_legacy",
                    "tasks": []
                }
            ]
        }"#;
        let legacy_camp: Campaign = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(legacy_camp.browsers.unwrap()[0].browser_id.as_deref().unwrap(), "browser_legacy");
    }
}
