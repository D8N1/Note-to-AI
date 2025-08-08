use std::path::PathBuf;
use anyhow::Result;

// Temporary stub while ML dependencies are disabled
#[derive(Debug, Clone)]
pub struct LocalLLM;

impl LocalLLM {
    pub async fn new(_model_path: PathBuf) -> Result<Self> {
        Ok(Self)
    }
    
    pub async fn generate(&self, prompt: &str, _max_tokens: usize) -> Result<String> {
        Ok(format!("🤖 AI Response to: {}", prompt))
    }
}
