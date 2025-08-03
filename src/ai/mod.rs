pub mod api_client;
pub mod context;
pub mod hermes_integration;
pub mod local_llm;
pub mod model_switcher;

use crate::Result;

pub struct AI {
    model_switcher: model_switcher::ModelSwitcher,
    context: context::Context,
}

impl AI {
    pub fn new() -> Result<Self> {
        let model_switcher = model_switcher::ModelSwitcher::new()?;
        let context = context::Context::new()?;
        
        Ok(Self {
            model_switcher,
            context,
        })
    }
    
    pub async fn process_query(&self, query: &str) -> Result<String> {
        // TODO: Implement AI query processing
        Ok("AI response".to_string())
    }
}
