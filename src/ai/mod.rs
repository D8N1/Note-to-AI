pub mod api_client;
pub mod context;
pub mod hermes_integration;
pub mod local_llm;
pub mod model_switcher;

use crate::Result;

pub struct AI {
    // Simplified without complex context
}

impl AI {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
    
    pub async fn process_query(&self, _query: &str) -> Result<String> {
        Ok("AI response".to_string())
    }
}
