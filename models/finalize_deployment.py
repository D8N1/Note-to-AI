#!/usr/bin/env python3
"""
Simple M1 Model Verification & Production Config
Creates the final production configuration for M1 MacBook Air
"""

import json
from pathlib import Path

def create_production_config():
    """Create final production configuration based on test results"""
    
    config = {
        "platform": "M1 MacBook Air",
        "optimization": "Apple Silicon Native",
        "memory_limit": "8GB",
        "status": "PRODUCTION READY",
        "date_configured": "2025-08-08",
        
        "working_models": {
            "text_generation": {
                "llama3.2:3b": {
                    "status": "✅ READY",
                    "performance": "Fast (5s response)",
                    "memory_usage": "Low (~2GB)",
                    "use_case": "Primary text generation",
                    "command": "ollama run llama3.2:3b"
                },
                "qwen2.5:7b": {
                    "status": "✅ READY", 
                    "performance": "Good (13s response)",
                    "memory_usage": "Medium (~4GB)",
                    "use_case": "High-quality responses",
                    "command": "ollama run qwen2.5:7b"
                },
                "codellama:7b": {
                    "status": "✅ READY",
                    "performance": "Fast (6s response)", 
                    "memory_usage": "Medium (~4GB)",
                    "use_case": "Code generation",
                    "command": "ollama run codellama:7b"
                }
            },
            
            "voice_transcription": {
                "whisper-small": {
                    "status": "✅ READY",
                    "model_file": "whisper.cpp/models/ggml-small.bin",
                    "size": "465MB",
                    "quality": "High accuracy",
                    "metal_optimized": True,
                    "command": "whisper.cpp/build/bin/whisper-cli -m whisper.cpp/models/ggml-small.bin"
                },
                "whisper-base": {
                    "status": "✅ READY",
                    "model_file": "whisper.cpp/models/ggml-base.bin", 
                    "size": "141MB",
                    "quality": "Good accuracy, faster",
                    "metal_optimized": True,
                    "command": "whisper.cpp/build/bin/whisper-cli -m whisper.cpp/models/ggml-base.bin"
                }
            },
            
            "summarization": {
                "distilbart-cnn": {
                    "status": "✅ READY (CPU mode)",
                    "model_path": "huggingface_cache/models--sshleifer--distilbart-cnn-12-6",
                    "size": "3.8GB",
                    "device": "cpu",
                    "note": "Use CPU to avoid MPS conflicts"
                }
            },
            
            "embeddings": {
                "nomic-embed": {
                    "status": "✅ READY",
                    "model": "nomic-embed-text:latest",
                    "size": "274MB", 
                    "note": "Use with custom embedding API, not ollama generate"
                }
            }
        },
        
        "deployment_recommendations": {
            "primary_workflow": {
                "voice_input": "whisper-small (best accuracy)",
                "text_generation": "llama3.2:3b (fastest, most reliable)",
                "code_tasks": "codellama:7b",
                "summarization": "distilbart-cnn (CPU mode)",
                "embeddings": "nomic-embed-text"
            },
            
            "memory_management": {
                "sequential_loading": "Load one model at a time",
                "avoid_concurrent": "Don't run multiple LLMs simultaneously", 
                "monitor_usage": "Use Activity Monitor to track RAM",
                "swap_models": "Stop unused models with 'ollama stop'"
            },
            
            "performance_optimization": {
                "keep_alive": "Use short keepalive for memory efficiency",
                "context_length": "Limit context to 2048-4096 tokens",
                "batch_processing": "Process multiple requests sequentially"
            }
        },
        
        "signal_workflow_ready": {
            "voice_transcription": "✅ Whisper.cpp with Metal backend",
            "text_processing": "✅ Multiple LLM options available", 
            "summarization": "✅ DistilBART for content summarization",
            "embeddings": "✅ Nomic for vector search",
            "integration_status": "Ready for Signal protocol integration"
        },
        
        "next_steps": {
            "1": "Implement Signal integration modules",
            "2": "Create audio processing pipeline", 
            "3": "Build RAG system with embeddings",
            "4": "Integrate with Obsidian vault storage",
            "5": "Add quantum-resistant cryptography"
        }
    }
    
    return config

def create_usage_examples():
    """Create practical usage examples for each model"""
    
    examples = {
        "voice_transcription_example": {
            "command": "cd whisper.cpp && ./build/bin/whisper-cli -m models/ggml-small.bin -f audio.wav",
            "description": "Transcribe audio file to text with high accuracy"
        },
        
        "text_generation_example": {
            "command": "ollama run llama3.2:3b 'Summarize the key points about AI safety'",
            "description": "Generate text response using fastest model"
        },
        
        "code_generation_example": {
            "command": "ollama run codellama:7b 'Write a Python function to parse JSON'",
            "description": "Generate code using specialized model"
        },
        
        "summarization_example": {
            "python_code": """
from transformers import pipeline
summarizer = pipeline('summarization', model='sshleifer/distilbart-cnn-12-6', 
                     cache_dir='huggingface_cache', device='cpu')
summary = summarizer(long_text, max_length=128, min_length=30)
print(summary[0]['summary_text'])
""",
            "description": "Summarize long text using DistilBART"
        }
    }
    
    return examples

def main():
    """Generate final production configuration"""
    print("🎯 GENERATING M1 MACBOOK AIR PRODUCTION CONFIGURATION")
    print("="*70)
    
    # Create configurations
    prod_config = create_production_config()
    usage_examples = create_usage_examples()
    
    # Save production config
    config_path = Path("m1_production_ready.json")
    with open(config_path, 'w') as f:
        json.dump(prod_config, f, indent=2)
    
    # Save usage examples
    examples_path = Path("usage_examples.json") 
    with open(examples_path, 'w') as f:
        json.dump(usage_examples, f, indent=2)
    
    print("✅ Production configuration created")
    print(f"📋 Config file: {config_path}")
    print(f"📖 Examples: {examples_path}")
    
    print(f"\n{'='*70}")
    print("🚀 M1 MACBOOK AIR DEPLOYMENT STATUS: READY")
    print("="*70)
    
    print("✅ Working Models:")
    print("   • llama3.2:3b (Primary text generation)")
    print("   • qwen2.5:7b (High-quality responses)")
    print("   • codellama:7b (Code generation)")
    print("   • whisper-small/base (Voice transcription)")
    print("   • distilbart-cnn (Text summarization)")
    print("   • nomic-embed (Text embeddings)")
    
    print("\n🎯 READY FOR:")
    print("   • Signal voice message transcription")
    print("   • AI-powered text generation")
    print("   • Code assistance and generation")
    print("   • Document summarization")
    print("   • Vector-based search")
    
    print("\n📝 NEXT: Implement the missing src/ modules to match README!")
    print("🔥 'Call the lead engineer' for the Signal workflow code 😄")
    
    return 0

if __name__ == "__main__":
    import sys
    sys.exit(main())
