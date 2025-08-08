# Models Directory - M1 MacBook Air Optimized

This directory contains specialized AI models optimized for M1 MacBook Air deployment, based on 2025 research into efficient multi-agent systems.

## 🎯 Philosophy

Following the "outsider philosophy" of prioritizing practical deployment over benchmark supremacy, this model collection focuses on:

- **Specialized models** for specific tasks vs general-purpose alternatives
- **M1-optimized quantization** for memory efficiency
- **Real-world performance** optimized for Apple Silicon
- **Local deployment** ensuring privacy and control

## 📁 Directory Structure

```
models/
├── model_registry.toml           # Central model configuration
├── M1_DEPLOYMENT_GUIDE.md        # Comprehensive deployment guide
├── download-specialized-models.sh # Automated model download
├── monitor_performance.py        # Performance monitoring tool
├── performance_metrics.json     # Performance tracking data
└── [model files]                # Actual model weights
```

## 🚀 Quick Start

### 1. Download Specialized Models
```bash
./download-specialized-models.sh
```

### 2. Monitor Performance
```bash
python3 monitor_performance.py
```

### 3. Check Configuration
```bash
cat model_registry.toml
```

## 🧠 Model Specializations

### Voice Transcription
- **Primary**: Whisper.cpp distil-large-v3 (Q5_1)
- **Memory**: 2-3GB RAM
- **Performance**: 13.3x real-time on M1 Air
- **Use Case**: Signal voice note transcription

### Document Summarization  
- **Primary**: DistilBART-CNN (Q4_K)
- **Memory**: 4GB RAM
- **Performance**: 97% of full BART, 60% faster
- **Use Case**: Morning briefings, document processing

### Question Generation
- **Primary**: LMQG T5-small End2end QAG
- **Memory**: 500MB RAM
- **Performance**: 100+ tokens/second
- **Use Case**: Intelligent prompt generation

### Core Language Model
- **Primary**: Hermes 3 8B (Q4_0)
- **Memory**: 6-8GB RAM
- **Performance**: 8-15 tokens/second
- **Use Case**: Multi-agent orchestration

## 📊 Deployment Profiles

### Morning Briefing (6.1GB total)
- DistilBART-CNN (summarization)
- all-MiniLM-L6-v2 (embeddings)
- T5-small-unified (structured briefings)

### Voice Processing (2.5GB total)
- Whisper-distil-large-v3 (transcription)
- LMQG-T5-small (question generation)

### Full Multi-Agent (12.8GB total)
- All specialized models loaded
- Requires 8GB M1 MacBook Air

## ⚙️ Configuration

### model_registry.toml Structure
```toml
[deployment]
platform = "m1_macos"
memory_budget = "8-16GB"
quantization_strategy = "Q4_K_M"

[models.model_name]
path = "model_file.safetensors"
type = "model_type"
memory_usage_mb = 4096
quantization = "Q4_K"
specialization = "task_category"
```

### Key Features
- **Memory tracking**: Each model's RAM requirements
- **Quantization settings**: Optimized for M1 performance
- **Specialization tags**: Clear task assignments
- **Performance metrics**: Expected tokens/second

## 🔧 Tools

### Download Script
- Automated model acquisition
- System requirement checking
- Ollama + MLX framework installation
- Quantization optimization

### Performance Monitor
- Real-time memory usage tracking
- Model performance benchmarking
- Deployment profile feasibility checking
- System optimization recommendations

## 📈 Performance Optimization

### Quantization Strategy
- **Q4_K_M**: Optimal balance (90% quality, 75% memory reduction)
- **Q5_1**: Higher quality for critical models
- **Q4_0**: Maximum compression for large models

### Apple Silicon Features
- **Metal Backend**: 3x performance for Whisper
- **CoreML Integration**: Neural Engine utilization
- **Unified Memory**: Zero-copy operations
- **Dynamic Loading**: Intelligent model swapping

## 🎮 Memory Management

### 8GB M1 MacBook Air Allocation
- **System Operations**: 4GB
- **Primary Models**: 6-8GB
- **Secondary Models**: 2-4GB
- **Caching/Switching**: 2GB

### 8GB M1 MacBook Air
- Use single deployment profiles
- Maximum 2-3 specialized models
- Prefer lightweight alternatives

## 📚 Integration

### RAG Workflows
```
Document → DistilBART (Summary) → MiniLM (Embed) → Vector Search → LMQG (Questions)
```

### Voice Workflows
```
Audio → Whisper (Transcribe) → Hermes (Understand) → LMQG (Follow-up)
```

### Multi-Agent Orchestration
- **LangChain**: Complex workflows
- **LlamaIndex**: RAG optimization
- **LocalAI**: OpenAI API compatibility

## 🔍 Monitoring

### Key Metrics
- Memory utilization (<80% recommended)
- Token generation speed (8-15 tokens/sec for 7B models)
- Model switching latency (<2 seconds)
- Cache hit rates (>70% target)

### Performance Tracking
```bash
# Real-time monitoring
python3 monitor_performance.py

# Save metrics to file
python3 monitor_performance.py --save

# View historical data
cat performance_metrics.json
```

## 🆘 Troubleshooting

### Memory Pressure
1. Switch to lighter model variants
2. Reduce concurrent model count
3. Enable aggressive model unloading
4. Check Activity Monitor for memory hogs

### Performance Issues
1. Verify Metal backend activation
2. Check MLX framework installation
3. Monitor thermal throttling
4. Review quantization settings

## 🌐 Community & Resources

- **LMQG Suite**: Comprehensive question generation
- **Whisper Ecosystem**: Regular M1 optimizations
- **MLX Framework**: Apple-maintained optimization
- **Ollama Community**: Model management best practices

## 📋 Requirements

### System Requirements
- macOS (Apple Silicon preferred)
- 8GB RAM minimum (16GB recommended)
- 20GB+ free disk space
- Ollama installed
- Python 3.8+ with required packages

### Dependencies
```bash
pip install psutil toml
```

## 🔄 Updates

This configuration is based on 2025 model research and will be updated as:
- New specialized models become available
- Apple Silicon optimizations improve
- Community feedback identifies better alternatives
- Performance benchmarks reveal superior options

---

*This model collection embodies practical AI deployment for M1 MacBook Air, prioritizing efficiency, specialization, and real-world performance over theoretical benchmarks.*
