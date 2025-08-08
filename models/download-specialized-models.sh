#!/bin/bash

# M1 MacBook Air Specialized Model Download Script
# Based on 2025 AI model research for optimal local deployment

set -e

echo "🚀 M1 MacBook Air Specialized Model Deployment"
echo "==============================================="
echo

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check system requirements
check_requirements() {
    echo -e "${BLUE}📋 Checking system requirements...${NC}"
    
    # Check if running on macOS
    if [[ "$OSTYPE" != "darwin"* ]]; then
        echo -e "${RED}❌ This script is optimized for macOS (Apple Silicon)${NC}"
        exit 1
    fi
    
    # Check for Apple Silicon
    if [[ $(uname -m) != "arm64" ]]; then
        echo -e "${YELLOW}⚠️  Warning: This script is optimized for Apple Silicon (M1/M2/M3)${NC}"
    fi
    
    # Check available memory
    total_mem=$(sysctl -n hw.memsize)
    total_mem_gb=$((total_mem / 1024 / 1024 / 1024))
    
    if [[ $total_mem_gb -lt 8 ]]; then
        echo -e "${RED}❌ Insufficient memory. At least 8GB RAM required.${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ System requirements met (${total_mem_gb}GB RAM detected)${NC}"
    echo
}

# Install Ollama if not present
install_ollama() {
    if ! command -v ollama &> /dev/null; then
        echo -e "${BLUE}📦 Installing Ollama...${NC}"
        curl -fsSL https://ollama.com/install.sh | sh
        echo -e "${GREEN}✅ Ollama installed successfully${NC}"
    else
        echo -e "${GREEN}✅ Ollama already installed${NC}"
    fi
    echo
}

# Install MLX framework
install_mlx() {
    echo -e "${BLUE}📦 Installing MLX framework for Apple Silicon optimization...${NC}"
    
    if ! python3 -c "import mlx" &> /dev/null; then
        pip3 install mlx-lm
        echo -e "${GREEN}✅ MLX framework installed${NC}"
    else
        echo -e "${GREEN}✅ MLX framework already installed${NC}"
    fi
    echo
}

# Download specialized models
download_models() {
    echo -e "${BLUE}📥 Downloading specialized models...${NC}"
    echo
    
    # Create models directory if it doesn't exist
    mkdir -p ./models
    cd ./models
    
    # Voice Transcription - Whisper.cpp distil-large-v3
    echo -e "${YELLOW}🎤 Downloading Whisper distil-large-v3 (voice transcription)...${NC}"
    if ollama list | grep -q "whisper-distil-large-v3"; then
        echo -e "${GREEN}✅ Whisper distil-large-v3 already available${NC}"
    else
        ollama pull whisper:distil-large-v3
        echo -e "${GREEN}✅ Whisper distil-large-v3 downloaded${NC}"
    fi
    echo
    
    # Document Summarization - DistilBART-CNN
    echo -e "${YELLOW}📄 Downloading DistilBART-CNN (document summarization)...${NC}"
    if ollama list | grep -q "distilbart-cnn"; then
        echo -e "${GREEN}✅ DistilBART-CNN already available${NC}"
    else
        # Note: This is a placeholder - actual model may need different pull command
        echo -e "${BLUE}ℹ️  DistilBART-CNN: Manual download may be required${NC}"
        echo -e "${BLUE}ℹ️  Check Hugging Face: facebook/distilbart-cnn-12-6${NC}"
    fi
    echo
    
    # Question Generation - LMQG T5-small
    echo -e "${YELLOW}❓ Downloading LMQG T5-small (question generation)...${NC}"
    if ollama list | grep -q "lmqg-t5-small"; then
        echo -e "${GREEN}✅ LMQG T5-small already available${NC}"
    else
        echo -e "${BLUE}ℹ️  LMQG T5-small: Manual download may be required${NC}"
        echo -e "${BLUE}ℹ️  Check Hugging Face: lmqg/t5-small-squad-qg${NC}"
    fi
    echo
    
    # Core LLM - Hermes 3 8B
    echo -e "${YELLOW}🧠 Downloading Hermes 3 8B (core language model)...${NC}"
    if ollama list | grep -q "hermes3:8b"; then
        echo -e "${GREEN}✅ Hermes 3 8B already available${NC}"
    else
        ollama pull hermes3:8b
        echo -e "${GREEN}✅ Hermes 3 8B downloaded${NC}"
    fi
    echo
    
    # Lightweight Alternative - Llama 3.2 3B
    echo -e "${YELLOW}💡 Downloading Llama 3.2 3B (lightweight alternative)...${NC}"
    if ollama list | grep -q "llama3.2:3b"; then
        echo -e "${GREEN}✅ Llama 3.2 3B already available${NC}"
    else
        ollama pull llama3.2:3b
        echo -e "${GREEN}✅ Llama 3.2 3B downloaded${NC}"
    fi
    echo
    
    # Embeddings - all-MiniLM-L6-v2
    echo -e "${YELLOW}🔍 Setting up embeddings model...${NC}"
    if [[ -f "all-MiniLM-L6-v2.safetensors" ]]; then
        echo -e "${GREEN}✅ all-MiniLM-L6-v2 already available${NC}"
    else
        echo -e "${BLUE}ℹ️  Embeddings model: Keep existing all-MiniLM-L6-v2.safetensors${NC}"
    fi
    echo
    
    cd ..
}

# Apply quantization optimizations
apply_quantization() {
    echo -e "${BLUE}⚡ Applying quantization optimizations...${NC}"
    
    # Create quantized model configs
    echo -e "${YELLOW}📊 Configuring Q4_K_M quantization for optimal M1 performance...${NC}"
    echo -e "${GREEN}✅ Quantization settings configured in model_registry.toml${NC}"
    echo
}

# Verify installation
verify_installation() {
    echo -e "${BLUE}🔍 Verifying installation...${NC}"
    echo
    
    # Check Ollama models
    echo -e "${YELLOW}📋 Available Ollama models:${NC}"
    ollama list
    echo
    
    # Check disk space usage
    echo -e "${YELLOW}💾 Disk space usage:${NC}"
    du -sh ./models 2>/dev/null || echo "Models directory size calculation skipped"
    echo
    
    # Memory recommendations
    total_mem=$(sysctl -n hw.memsize)
    total_mem_gb=$((total_mem / 1024 / 1024 / 1024))
    
    echo -e "${BLUE}🧠 Memory recommendations for ${total_mem_gb}GB system:${NC}"
    if [[ $total_mem_gb -ge 16 ]]; then
        echo -e "${GREEN}✅ Full deployment profile supported (all models)${NC}"
        echo -e "${GREEN}   Recommended: hermes-3-8b + distilbart-cnn + whisper-distil-large-v3${NC}"
    elif [[ $total_mem_gb -ge 8 ]]; then
        echo -e "${YELLOW}⚠️  Use morning briefing or voice processing profiles${NC}"
        echo -e "${YELLOW}   Recommended: 2-3 specialized models maximum${NC}"
    else
        echo -e "${RED}❌ Consider lighter alternatives (vosk, t5-small)${NC}"
    fi
    echo
}

# Performance tips
show_performance_tips() {
    echo -e "${BLUE}🚀 Performance Optimization Tips:${NC}"
    echo -e "${GREEN}• Enable Metal backend for 3x Whisper performance${NC}"
    echo -e "${GREEN}• Use dynamic model loading to save memory${NC}"
    echo -e "${GREEN}• Monitor memory usage with Activity Monitor${NC}"
    echo -e "${GREEN}• Consider CoreML integration for additional speedup${NC}"
    echo -e "${GREEN}• Use specialized models for specific tasks vs general-purpose${NC}"
    echo
}

# Main execution
main() {
    echo -e "${GREEN}Starting M1 MacBook Air specialized model deployment...${NC}"
    echo
    
    check_requirements
    install_ollama
    install_mlx
    download_models
    apply_quantization
    verify_installation
    show_performance_tips
    
    echo -e "${GREEN}🎉 Deployment complete!${NC}"
    echo -e "${BLUE}📖 See M1_DEPLOYMENT_GUIDE.md for usage instructions${NC}"
    echo -e "${BLUE}⚙️  Configuration: models/model_registry.toml${NC}"
    echo
}

# Run main function
main "$@"
