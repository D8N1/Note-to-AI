# M1 MacBook Air Multi-Agent Model Deployment Guide

Based on specialized AI model research for optimal local deployment on Apple Silicon.

## Overview

This guide implements the "outsider philosophy" of prioritizing practical deployment over benchmark supremacy, focusing on specialized, efficient models optimized for M1 MacBook Air hardware.

## Core Architecture

### Memory Budget Allocation (16GB M1 MacBook Air)
- **System Operations**: 4GB
- **Primary Models**: 6-8GB 
- **Secondary Specialized Models**: 2-4GB
- **Caching & Model Switching**: 2GB

### Recommended Model Stack

#### 1. Voice Transcription
**Primary**: Whisper.cpp distil-large-v3 (Q5_1 quantization)
- Memory: 2-3GB RAM
- Performance: 13.3x real-time on M1 Air
- Accuracy: High for English voice notes
- Backend: Metal acceleration (3x performance gain)

**Backup**: Vosk small English model
- Memory: 200MB RAM total
- Use case: Severely memory-constrained scenarios
- Architecture: Kaldi-based, zero Python dependencies

#### 2. Document Summarization  
**Primary**: DistilBART-CNN (Q4_K quantization)
- Memory: 4GB RAM
- Performance: 97% of full BART, 60% faster
- ROUGE-1 Score: 0.44
- Specialization: Abstractive summarization

**Alternative**: T5-small variants
- Memory: 2GB RAM
- Features: Text-to-text unified framework
- Strength: Structured briefing generation

#### 3. Question/Prompt Generation
**Primary**: LMQG T5-small End2end QAG
- Memory: 500MB RAM  
- Parameters: 60M
- Performance: 100+ tokens/second
- Specialization: High-quality question generation

**Advanced**: Hermes 3 series (8B)
- Memory: 8-16GB RAM with quantization
- Features: Advanced agentic capabilities
- Strength: Multi-turn conversation, context coherence

## Installation & Setup

### 1. Ollama Installation
```bash
curl -fsSL https://ollama.com/install.sh | sh
```

### 2. MLX Framework (Apple-specific optimization)
```bash
pip install mlx-lm
```

### 3. Model Downloads
The following models should be downloaded and quantized:

```bash
# Core models (update paths in model_registry.toml)
ollama pull distilbart-cnn:q4_k_m
ollama pull whisper-distil-large-v3:q5_1  
ollama pull lmqg-t5-small:latest
ollama pull hermes3:8b-q4_0
```

## Deployment Profiles

### Morning Briefing Profile
```toml
models = ["distilbart-cnn", "all-MiniLM-L6-v2", "t5-small-unified"]
memory_total = 6.1GB
use_case = "Document summarization and structured briefings"
```

### Voice Processing Profile  
```toml
models = ["whisper-distil-large-v3", "lmqg-t5-small"]
memory_total = 2.5GB
use_case = "Voice transcription and follow-up generation"
```

### Full Multi-Agent Profile
```toml
models = ["hermes-3-8b", "distilbart-cnn", "whisper-distil-large-v3", "lmqg-t5-small"]
memory_total = 12.8GB
use_case = "Complete specialized multi-agent system"
```

## Performance Optimization

### Quantization Strategy
- **Q4_K_M**: Optimal balance (90% quality, 75% memory reduction)
- **Q5_1**: Higher quality for critical models (Whisper)
- **Q4_0**: Maximum compression for large models

### Apple Silicon Optimizations
- **Metal Backend**: 3x performance gain for Whisper
- **CoreML Integration**: Neural Engine utilization
- **Unified Memory**: Zero-copy operations via MLX
- **KV Cache Reduction**: 37.5% memory savings

### Dynamic Model Management
- **Intelligent Loading**: Models loaded/unloaded based on task
- **Context Preservation**: Conversation state maintained during swaps
- **Predictive Caching**: Anticipated model loading

## Integration Patterns

### RAG System Integration
```
Document Input → DistilBART-CNN (Summarization) → 
all-MiniLM-L6-v2 (Embedding) → Vector Search → 
LMQG T5-small (Question Generation)
```

### Voice Workflow
```
Audio Input → Whisper.cpp (Transcription) → 
Hermes-3 (Understanding) → LMQG (Follow-up Questions)
```

### Multi-Agent Orchestration
- **LangChain**: Complex agent workflows
- **LlamaIndex**: RAG-optimized orchestration  
- **LocalAI**: OpenAI API compatibility

## Monitoring & Maintenance

### Performance Metrics
- Token generation speed (target: 8-15 tokens/sec for 7B models)
- Memory utilization (maintain <80% of available RAM)
- Model switching latency (<2 seconds)
- Cache hit rates (target: >70%)

### Optimization Schedule
- **Real-time**: Dynamic memory management
- **Hourly**: Background optimization
- **Daily**: Model performance analysis
- **Weekly**: Quantization optimization review

## Troubleshooting

### Memory Pressure
1. Switch to lighter model variants (Vosk, T5-small)
2. Reduce concurrent model count
3. Increase swap/virtual memory allocation
4. Enable aggressive model unloading

### Performance Issues
1. Verify Metal backend activation
2. Check MLX framework installation
3. Monitor thermal throttling
4. Review quantization settings

## Community Resources

- **LMQG Suite**: Active development, comprehensive docs
- **Whisper Ecosystem**: Regular M1 optimizations
- **MLX Framework**: Apple-maintained, frequent updates
- **Ollama Community**: Model management best practices

This deployment guide enables production-quality multi-agent systems entirely on M1 MacBook Air while maintaining privacy, control, and optimal performance through specialized model selection.
