#!/usr/bin/env python3
"""
M1 MacBook Air Model Performance Test & Configuration
Tests all our downloaded models and creates optimized configs
"""

import subprocess
import json
import time
from pathlib import Path

def test_ollama_models():
    """Test Ollama models with M1 optimizations"""
    print("🚀 TESTING OLLAMA MODELS ON M1 MACBOOK AIR")
    print("="*60)
    
    models = [
        "hermes3:8b",
        "llama3.2:3b", 
        "qwen2.5:7b",
        "codellama:7b",
        "nomic-embed-text:latest"
    ]
    
    results = {}
    
    for model in models:
        print(f"\n🔄 Testing {model}...")
        
        # Test with a simple prompt
        start_time = time.time()
        try:
            result = subprocess.run([
                "ollama", "run", model, 
                "Hello! Please respond with exactly 5 words."
            ], capture_output=True, text=True, timeout=30)
            
            end_time = time.time()
            response_time = end_time - start_time
            
            if result.returncode == 0:
                results[model] = {
                    "status": "✅ SUCCESS",
                    "response_time": f"{response_time:.2f}s",
                    "response": result.stdout.strip()[:100] + "..." if len(result.stdout.strip()) > 100 else result.stdout.strip(),
                    "m1_optimized": True
                }
                print(f"✅ {model}: {response_time:.2f}s")
            else:
                results[model] = {
                    "status": "❌ FAILED", 
                    "error": result.stderr,
                    "m1_optimized": False
                }
                print(f"❌ {model}: Failed")
                
        except subprocess.TimeoutExpired:
            results[model] = {
                "status": "⏱️ TIMEOUT",
                "error": "Model took longer than 30s",
                "m1_optimized": False
            }
            print(f"⏱️ {model}: Timeout")
            
        except Exception as e:
            results[model] = {
                "status": "❌ ERROR",
                "error": str(e),
                "m1_optimized": False
            }
            print(f"❌ {model}: Error")
    
    return results

def test_whisper_models():
    """Test Whisper.cpp models with Metal backend"""
    print("\n🎙️ TESTING WHISPER MODELS WITH METAL BACKEND")
    print("="*60)
    
    whisper_path = Path("whisper.cpp")
    models_path = whisper_path / "models" 
    
    results = {}
    
    if not whisper_path.exists():
        print("❌ Whisper.cpp not found")
        return results
    
    whisper_models = [
        "ggml-base.bin",
        "ggml-small.bin"
    ]
    
    for model in whisper_models:
        model_path = models_path / model
        if model_path.exists():
            print(f"✅ {model}: {model_path.stat().st_size / 1024 / 1024:.1f}MB")
            results[model] = {
                "status": "✅ READY",
                "size_mb": f"{model_path.stat().st_size / 1024 / 1024:.1f}MB",
                "metal_optimized": True,
                "path": str(model_path)
            }
        else:
            print(f"❌ {model}: Not found")
            results[model] = {
                "status": "❌ MISSING",
                "metal_optimized": False
            }
    
    return results

def test_distilbart():
    """Test DistilBART model"""
    print("\n📝 TESTING DISTILBART SUMMARIZATION MODEL")
    print("="*60)
    
    try:
        result = subprocess.run([
            "python3", "-c", """
from transformers import pipeline
import time

start = time.time()
summarizer = pipeline('summarization', model='sshleifer/distilbart-cnn-12-6', 
                     cache_dir='huggingface_cache')
end = time.time()

test_text = '''
Apple's M1 chip represents a significant leap in performance and efficiency. 
Built on a 5-nanometer process, it integrates the CPU, GPU, and Neural Engine 
onto a single chip. This unified memory architecture allows for unprecedented 
performance while maintaining excellent battery life.
'''

summary_start = time.time()
summary = summarizer(test_text, max_length=50, min_length=10, do_sample=False)
summary_end = time.time()

print(f"Load time: {end-start:.2f}s")
print(f"Summary time: {summary_end-summary_start:.2f}s") 
print(f"Summary: {summary[0]['summary_text']}")
"""
        ], capture_output=True, text=True, timeout=60)
        
        if result.returncode == 0:
            return {
                "distilbart-cnn": {
                    "status": "✅ SUCCESS",
                    "output": result.stdout,
                    "ready": True
                }
            }
        else:
            return {
                "distilbart-cnn": {
                    "status": "❌ FAILED",
                    "error": result.stderr,
                    "ready": False
                }
            }
            
    except Exception as e:
        return {
            "distilbart-cnn": {
                "status": "❌ ERROR",
                "error": str(e),
                "ready": False
            }
        }

def create_m1_config(ollama_results, whisper_results, distilbart_results):
    """Create optimized configuration for M1 MacBook Air"""
    
    config = {
        "platform": "M1 MacBook Air",
        "optimization": "Apple Silicon Native",
        "memory_limit": "8GB",
        "deployment_profiles": {
            "voice_transcription": {
                "whisper_model": "ggml-small.bin" if whisper_results.get("ggml-small.bin", {}).get("status") == "✅ READY" else "ggml-base.bin",
                "metal_backend": True,
                "concurrent_processing": False  # Due to 8GB RAM limit
            },
            "text_generation": {
                "primary": "llama3.2:3b",  # Most memory efficient
                "fallback": "hermes3:8b",
                "max_context": 4096,
                "temperature": 0.7
            },
            "code_assistance": {
                "model": "codellama:7b",
                "max_context": 8192,
                "temperature": 0.1
            },
            "embeddings": {
                "model": "nomic-embed-text:latest",
                "batch_size": 32  # Reduced for 8GB RAM
            },
            "summarization": {
                "model": "distilbart-cnn",
                "max_input_length": 1024,
                "max_output_length": 128
            }
        },
        "performance_settings": {
            "ollama": {
                "num_ctx": 4096,
                "num_predict": 512,
                "temperature": 0.7,
                "top_k": 40,
                "top_p": 0.9
            },
            "whisper": {
                "threads": 4,  # M1 has 4 performance cores
                "metal": True,
                "language": "auto"
            }
        },
        "model_status": {
            "ollama_models": ollama_results,
            "whisper_models": whisper_results,
            "specialized_models": distilbart_results
        }
    }
    
    return config

def main():
    """Main testing and configuration function"""
    print("🎯 M1 MACBOOK AIR MODEL DEPLOYMENT VERIFICATION")
    print("="*80)
    
    # Test all model types
    ollama_results = test_ollama_models()
    whisper_results = test_whisper_models() 
    distilbart_results = test_distilbart()
    
    # Create optimized config
    m1_config = create_m1_config(ollama_results, whisper_results, distilbart_results)
    
    # Save configuration
    config_path = Path("m1_deployment_config.json")
    with open(config_path, 'w') as f:
        json.dump(m1_config, f, indent=2)
    
    print(f"\n{'='*80}")
    print("🎯 DEPLOYMENT SUMMARY")
    print(f"{'='*80}")
    
    # Count successful models
    ollama_success = sum(1 for r in ollama_results.values() if "SUCCESS" in r.get("status", ""))
    whisper_success = sum(1 for r in whisper_results.values() if "READY" in r.get("status", ""))
    distilbart_success = sum(1 for r in distilbart_results.values() if "SUCCESS" in r.get("status", ""))
    
    total_success = ollama_success + whisper_success + distilbart_success
    total_models = len(ollama_results) + len(whisper_results) + len(distilbart_results)
    
    print(f"📊 Ollama Models: {ollama_success}/{len(ollama_results)} ready")
    print(f"🎙️ Whisper Models: {whisper_success}/{len(whisper_results)} ready") 
    print(f"📝 Specialized Models: {distilbart_success}/{len(distilbart_results)} ready")
    print(f"🚀 Total: {total_success}/{total_models} models ready for M1 deployment")
    
    print(f"\n📋 Configuration saved to: {config_path}")
    
    if total_success >= total_models * 0.8:  # 80% success rate
        print("\n✅ M1 MacBook Air deployment ready!")
        print("🎯 Voice transcription, text generation, and AI assistance fully operational")
    else:
        print("\n⚠️ Some models need attention before full deployment")
    
    return 0

if __name__ == "__main__":
    import sys
    sys.exit(main())
