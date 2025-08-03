use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub endpoint: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub cost_per_token: f64,
    pub latency_ms: u64,
    pub capabilities: Vec<String>,
    pub context_window: usize,
    pub is_available: bool,
}

#[derive(Debug, Clone)]
pub enum SwitchingStrategy {
    CostOptimized,
    LatencyOptimized,
    CapabilityBased(Vec<String>),
    LoadBalanced,
    Manual(String),
}

#[derive(Debug, Clone)]
pub struct TaskContext {
    pub task_type: String,
    pub required_capabilities: Vec<String>,
    pub max_latency_ms: Option<u64>,
    pub max_cost_per_token: Option<f64>,
    pub context_size: usize,
    pub priority: u8, // 1-10, 10 being highest
}

#[derive(Debug)]
pub struct ModelMetrics {
    pub success_rate: f64,
    pub avg_latency_ms: u64,
    pub avg_cost_per_request: f64,
    pub last_used: std::time::SystemTime,
    pub total_requests: u64,
    pub failed_requests: u64,
}

pub struct ModelSwitcher {
    models: Arc<RwLock<HashMap<String, ModelConfig>>>,
    metrics: Arc<RwLock<HashMap<String, ModelMetrics>>>,
    current_model: Arc<RwLock<Option<String>>>,
    strategy: Arc<RwLock<SwitchingStrategy>>,
}

impl ModelSwitcher {
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            current_model: Arc::new(RwLock::new(None)),
            strategy: Arc::new(RwLock::new(SwitchingStrategy::CostOptimized)),
        }
    }

    /// Register a new model with the switcher
    pub async fn register_model(&self, config: ModelConfig) -> Result<()> {
        let mut models = self.models.write().await;
        let model_name = config.name.clone();
        
        models.insert(model_name.clone(), config);
        
        // Initialize metrics for new model
        let mut metrics = self.metrics.write().await;
        metrics.insert(model_name, ModelMetrics {
            success_rate: 1.0,
            avg_latency_ms: 0,
            avg_cost_per_request: 0.0,
            last_used: std::time::SystemTime::now(),
            total_requests: 0,
            failed_requests: 0,
        });

        Ok(())
    }

    /// Set the switching strategy
    pub async fn set_strategy(&self, strategy: SwitchingStrategy) {
        let mut current_strategy = self.strategy.write().await;
        *current_strategy = strategy;
    }

    /// Select the best model for a given task context
    pub async fn select_model(&self, context: &TaskContext) -> Result<String> {
        let models = self.models.read().await;
        let metrics = self.metrics.read().await;
        let strategy = self.strategy.read().await;

        if models.is_empty() {
            return Err(anyhow!("No models registered"));
        }

        // Filter available models that meet requirements
        let mut candidates: Vec<(&String, &ModelConfig)> = models
            .iter()
            .filter(|(_, config)| {
                config.is_available 
                    && config.context_window >= context.context_size
                    && context.required_capabilities.iter().all(|cap| config.capabilities.contains(cap))
            })
            .collect();

        if candidates.is_empty() {
            return Err(anyhow!("No available models meet the requirements"));
        }

        // Apply filtering based on constraints
        if let Some(max_latency) = context.max_latency_ms {
            candidates.retain(|(name, config)| {
                metrics.get(*name)
                    .map(|m| m.avg_latency_ms <= max_latency)
                    .unwrap_or(config.latency_ms <= max_latency)
            });
        }

        if let Some(max_cost) = context.max_cost_per_token {
            candidates.retain(|(_, config)| config.cost_per_token <= max_cost);
        }

        if candidates.is_empty() {
            return Err(anyhow!("No models meet the cost/latency constraints"));
        }

        // Select best model based on strategy
        let selected = match &*strategy {
            SwitchingStrategy::CostOptimized => {
                candidates.iter()
                    .min_by(|(_, a), (_, b)| a.cost_per_token.partial_cmp(&b.cost_per_token).unwrap())
                    .unwrap()
            },
            SwitchingStrategy::LatencyOptimized => {
                candidates.iter()
                    .min_by_key(|(name, config)| {
                        metrics.get(*name)
                            .map(|m| m.avg_latency_ms)
                            .unwrap_or(config.latency_ms)
                    })
                    .unwrap()
            },
            SwitchingStrategy::CapabilityBased(required_caps) => {
                // Score models by how many additional capabilities they have
                candidates.iter()
                    .max_by_key(|(_, config)| {
                        config.capabilities.iter()
                            .filter(|cap| required_caps.contains(cap))
                            .count()
                    })
                    .unwrap()
            },
            SwitchingStrategy::LoadBalanced => {
                // Select least recently used model with good success rate
                candidates.iter()
                    .min_by_key(|(name, _)| {
                        metrics.get(*name)
                            .map(|m| (m.last_used, (m.success_rate * 1000.0) as u64))
                            .unwrap_or((std::time::SystemTime::UNIX_EPOCH, 1000))
                    })
                    .unwrap()
            },
            SwitchingStrategy::Manual(model_name) => {
                candidates.iter()
                    .find(|(name, _)| *name == model_name)
                    .ok_or_else(|| anyhow!("Manually specified model '{}' not available", model_name))?
            },
        };

        let model_name = selected.0.clone();
        
        // Update current model
        let mut current = self.current_model.write().await;
        *current = Some(model_name.clone());

        Ok(model_name)
    }

    /// Switch to a specific model
    pub async fn switch_to_model(&self, model_name: &str) -> Result<()> {
        let models = self.models.read().await;
        
        if !models.contains_key(model_name) {
            return Err(anyhow!("Model '{}' not registered", model_name));
        }

        let config = models.get(model_name).unwrap();
        if !config.is_available {
            return Err(anyhow!("Model '{}' is not available", model_name));
        }

        let mut current = self.current_model.write().await;
        *current = Some(model_name.to_string());

        Ok(())
    }

    /// Get current active model
    pub async fn get_current_model(&self) -> Option<String> {
        self.current_model.read().await.clone()
    }

    /// Get model configuration
    pub async fn get_model_config(&self, model_name: &str) -> Result<ModelConfig> {
        let models = self.models.read().await;
        models.get(model_name)
            .cloned()
            .ok_or_else(|| anyhow!("Model '{}' not found", model_name))
    }

    /// Update model availability
    pub async fn set_model_availability(&self, model_name: &str, available: bool) -> Result<()> {
        let mut models = self.models.write().await;
        
        if let Some(config) = models.get_mut(model_name) {
            config.is_available = available;
            Ok(())
        } else {
            Err(anyhow!("Model '{}' not found", model_name))
        }
    }

    /// Record metrics for a model after a request
    pub async fn record_metrics(&self, model_name: &str, latency_ms: u64, cost: f64, success: bool) {
        let mut metrics = self.metrics.write().await;
        
        if let Some(metric) = metrics.get_mut(model_name) {
            metric.total_requests += 1;
            if !success {
                metric.failed_requests += 1;
            }
            
            metric.success_rate = 1.0 - (metric.failed_requests as f64 / metric.total_requests as f64);
            
            // Update rolling averages (simple approach)
            metric.avg_latency_ms = (metric.avg_latency_ms + latency_ms) / 2;
            metric.avg_cost_per_request = (metric.avg_cost_per_request + cost) / 2.0;
            metric.last_used = std::time::SystemTime::now();
        }
    }

    /// Get health status of all models
    pub async fn get_health_status(&self) -> HashMap<String, (bool, f64)> {
        let models = self.models.read().await;
        let metrics = self.metrics.read().await;
        
        models.iter()
            .map(|(name, config)| {
                let success_rate = metrics.get(name)
                    .map(|m| m.success_rate)
                    .unwrap_or(1.0);
                (name.clone(), (config.is_available, success_rate))
            })
            .collect()
    }

    /// Auto-switch based on model health and performance
    pub async fn auto_optimize(&self, context: &TaskContext) -> Result<String> {
        // Check if current model is still optimal
        if let Some(current) = self.get_current_model().await {
            let health = self.get_health_status().await;
            
            if let Some((available, success_rate)) = health.get(&current) {
                // Switch if current model is unhealthy
                if !available || *success_rate < 0.8 {
                    return self.select_model(context).await;
                }
            }
        }

        // Select optimal model for the task
        self.select_model(context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_model_registration() {
        let switcher = ModelSwitcher::new();
        
        let config = ModelConfig {
            name: "gpt-4".to_string(),
            endpoint: "https://api.openai.com/v1".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            cost_per_token: 0.00003,
            latency_ms: 1500,
            capabilities: vec!["text-generation".to_string(), "reasoning".to_string()],
            context_window: 8192,
            is_available: true,
        };

        assert!(switcher.register_model(config).await.is_ok());
    }

    #[tokio::test]
    async fn test_model_selection() {
        let switcher = ModelSwitcher::new();
        
        // Register test models
        let cheap_model = ModelConfig {
            name: "gpt-3.5".to_string(),
            endpoint: "https://api.openai.com/v1".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            cost_per_token: 0.000002,
            latency_ms: 800,
            capabilities: vec!["text-generation".to_string()],
            context_window: 4096,
            is_available: true,
        };

        let expensive_model = ModelConfig {
            name: "gpt-4".to_string(),
            endpoint: "https://api.openai.com/v1".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            cost_per_token: 0.00003,
            latency_ms: 1500,
            capabilities: vec!["text-generation".to_string(), "reasoning".to_string()],
            context_window: 8192,
            is_available: true,
        };

        switcher.register_model(cheap_model).await.unwrap();
        switcher.register_model(expensive_model).await.unwrap();

        let context = TaskContext {
            task_type: "simple_generation".to_string(),
            required_capabilities: vec!["text-generation".to_string()],
            max_latency_ms: None,
            max_cost_per_token: None,
            context_size: 2048,
            priority: 5,
        };

        // Should select cheaper model with cost-optimized strategy
        let selected = switcher.select_model(&context).await.unwrap();
        assert_eq!(selected, "gpt-3.5");
    }
}