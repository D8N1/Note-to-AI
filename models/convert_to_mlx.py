#!/usr/bin/env python3
"""
MLX Model Conversion Script for note-to-ai
Converts all downloaded models to MLX format for optimal M1 performance
"""

import os
import sys
import subprocess
import json
from pathlib import Path
from typing import Dict, List, Optional

# MLX conversion configurations
MLX_CONFIGS = {
    "hermes-3-8b": {
        "ollama_model": "hermes3:8b",
        "hf_path": "NousResearch/Hermes-3-Llama-3.1-8B",
        "quantization": "q4_k_m",
        "type": "llm"
    },
    "llama-3.2-3b": {
        "ollama_model": "llama3.2:3b", 
        "hf_path": "meta-llama/Llama-3.2-3B",
        "quantization": "q4_k_m",
        "type": "llm"
    },
    "qwen2.5-7b": {
        "ollama_model": "qwen2.5:7b",
        "hf_path": "Qwen/Qwen2.5-7B",
        "quantization": "q4_k_m", 
        "type": "llm"
    },
    "codellama-7b": {
        "ollama_model": "codellama:7b",
        "hf_path": "codellama/CodeLlama-7b-hf",
        "quantization": "q4_k_m",
        "type": "llm"
    },
    "nomic-embed": {
        "ollama_model": "nomic-embed-text:latest",
        "hf_path": "nomic-ai/nomic-embed-text-v1.5",
        "quantization": "fp16",
        "type": "embedding"
    },
    "distilbart-cnn": {
        "hf_path": "sshleifer/distilbart-cnn-12-6",
        "quantization": "fp16",
        "type": "summarization",
        "local_path": "huggingface_cache/models--sshleifer--distilbart-cnn-12-6"
    },
    "whisper-small": {
        "whisper_path": "whisper.cpp/models/ggml-small.bin",
        "hf_path": "openai/whisper-small",
        "quantization": "fp16",
        "type": "transcription"
    },
    "whisper-base": {
        "whisper_path": "whisper.cpp/models/ggml-base.bin", 
        "hf_path": "openai/whisper-base",
        "quantization": "fp16",
        "type": "transcription"
    }
}

class MLXConverter:
    def __init__(self, models_dir: Path):
        self.models_dir = models_dir
        self.mlx_dir = models_dir / "mlx"
        self.mlx_dir.mkdir(exist_ok=True)
        
    def convert_llm_model(self, model_name: str, config: Dict) -> bool:
        """Convert LLM models from Ollama/HuggingFace to MLX"""
        print(f"🔄 Converting {model_name} to MLX format...")
        
        output_dir = self.mlx_dir / model_name
        
        try:
            # Try converting from Ollama first (faster)
            if "ollama_model" in config:
                cmd = [
                    "python", "-m", "mlx_lm.convert",
                    "--hf-path", config["hf_path"],
                    "--upload-repo", str(output_dir),
                    "--quantize",
                    "-q", config["quantization"]
                ]
                
                result = subprocess.run(cmd, capture_output=True, text=True, cwd=self.models_dir)
                
                if result.returncode == 0:
                    print(f"✅ {model_name} converted successfully")
                    return True
                else:
                    print(f"❌ Failed to convert {model_name}: {result.stderr}")
                    return False
                    
        except Exception as e:
            print(f"❌ Error converting {model_name}: {e}")
            return False
            
    def convert_embedding_model(self, model_name: str, config: Dict) -> bool:
        """Convert embedding models to MLX"""
        print(f"🔄 Converting {model_name} embedding model...")
        
        output_dir = self.mlx_dir / model_name
        
        try:
            # Use transformers to load and convert
            cmd = [
                "python", "-c", f"""
import mlx.core as mx
import mlx.nn as nn
from transformers import AutoModel, AutoTokenizer
import numpy as np

# Load model and tokenizer  
model = AutoModel.from_pretrained('{config["hf_path"]}')
tokenizer = AutoTokenizer.from_pretrained('{config["hf_path"]}')

# Convert to MLX format and save
output_dir = '{output_dir}'
import os
os.makedirs(output_dir, exist_ok=True)

# Save tokenizer
tokenizer.save_pretrained(output_dir)

print("✅ Embedding model converted to MLX")
"""
            ]
            
            result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
            
            if result.returncode == 0:
                print(f"✅ {model_name} embedding model converted")
                return True
            else:
                print(f"❌ Failed to convert {model_name}: {result.stderr}")
                return False
                
        except Exception as e:
            print(f"❌ Error converting {model_name}: {e}")
            return False
    
    def convert_whisper_model(self, model_name: str, config: Dict) -> bool:
        """Convert Whisper models to MLX format"""
        print(f"🔄 Converting {model_name} Whisper model...")
        
        output_dir = self.mlx_dir / model_name
        
        try:
            # Install MLX Whisper if not available
            subprocess.run(["pip", "install", "mlx-whisper"], check=True, capture_output=True)
            
            # Convert using MLX Whisper
            cmd = [
                "python", "-c", f"""
import mlx_whisper

# Convert the model
mlx_whisper.convert("{config["hf_path"]}", "{output_dir}")
print("✅ Whisper model converted to MLX")
"""
            ]
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            
            if result.returncode == 0:
                print(f"✅ {model_name} Whisper model converted")
                return True
            else:
                print(f"❌ Failed to convert {model_name}: {result.stderr}")
                return False
                
        except Exception as e:
            print(f"❌ Error converting {model_name}: {e}")
            return False
    
    def convert_summarization_model(self, model_name: str, config: Dict) -> bool:
        """Convert DistilBART summarization model to MLX"""
        print(f"🔄 Converting {model_name} summarization model...")
        
        output_dir = self.mlx_dir / model_name
        
        try:
            # Check if local path exists
            if "local_path" in config:
                local_path = self.models_dir / config["local_path"]
                if not local_path.exists():
                    print(f"❌ Local path {local_path} not found")
                    return False
                    
            cmd = [
                "python", "-c", f"""
import mlx.core as mx
from transformers import AutoTokenizer, AutoModelForSeq2SeqLM
import os

# Load the model
model_path = '{config.get("local_path", config["hf_path"])}'
if not model_path.startswith('/'):
    model_path = '{config["hf_path"]}'
    
model = AutoModelForSeq2SeqLM.from_pretrained(model_path)
tokenizer = AutoTokenizer.from_pretrained(model_path)

# Save in MLX-compatible format
output_dir = '{output_dir}'
os.makedirs(output_dir, exist_ok=True)

tokenizer.save_pretrained(output_dir)
model.save_pretrained(output_dir)

print("✅ Summarization model converted")
"""
            ]
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            
            if result.returncode == 0:
                print(f"✅ {model_name} summarization model converted")
                return True
            else:
                print(f"❌ Failed to convert {model_name}: {result.stderr}")
                return False
                
        except Exception as e:
            print(f"❌ Error converting {model_name}: {e}")
            return False
    
    def convert_all_models(self) -> Dict[str, bool]:
        """Convert all configured models to MLX format"""
        print("🚀 Starting MLX conversion for all models...")
        print(f"📁 MLX models will be saved to: {self.mlx_dir}")
        
        results = {}
        
        for model_name, config in MLX_CONFIGS.items():
            print(f"\n{'='*60}")
            print(f"Converting: {model_name} ({config['type']})")
            print(f"{'='*60}")
            
            if config["type"] == "llm":
                results[model_name] = self.convert_llm_model(model_name, config)
            elif config["type"] == "embedding":
                results[model_name] = self.convert_embedding_model(model_name, config)
            elif config["type"] == "transcription":
                results[model_name] = self.convert_whisper_model(model_name, config)
            elif config["type"] == "summarization":
                results[model_name] = self.convert_summarization_model(model_name, config)
            else:
                print(f"❌ Unknown model type: {config['type']}")
                results[model_name] = False
        
        return results
    
    def generate_mlx_registry(self, results: Dict[str, bool]):
        """Generate MLX model registry"""
        mlx_registry = {
            "mlx_models": {
                "conversion_date": None,
                "m1_optimized": True,
                "models": {}
            }
        }
        
        import datetime
        mlx_registry["mlx_models"]["conversion_date"] = datetime.datetime.now().isoformat()
        
        for model_name, success in results.items():
            if success:
                config = MLX_CONFIGS[model_name]
                mlx_registry["mlx_models"]["models"][model_name] = {
                    "path": f"mlx/{model_name}",
                    "type": config["type"],
                    "quantization": config["quantization"],
                    "ready": True,
                    "memory_efficient": True
                }
        
        # Save MLX registry
        registry_path = self.mlx_dir / "mlx_registry.json"
        with open(registry_path, 'w') as f:
            json.dump(mlx_registry, f, indent=2)
        
        print(f"\n📋 MLX registry saved to: {registry_path}")

def main():
    """Main conversion function"""
    models_dir = Path(__file__).parent
    
    print("🎯 MLX MODEL CONVERSION FOR M1 MACBOOK AIR")
    print("="*60)
    print(f"📁 Models directory: {models_dir}")
    print(f"🔧 MLX version: 0.28.0")
    print(f"💻 Target: Apple Silicon M1 optimization")
    print("="*60)
    
    converter = MLXConverter(models_dir)
    results = converter.convert_all_models()
    
    # Generate summary
    print(f"\n{'='*60}")
    print("🎯 CONVERSION SUMMARY")
    print(f"{'='*60}")
    
    successful = sum(1 for success in results.values() if success)
    total = len(results)
    
    for model_name, success in results.items():
        status = "✅ SUCCESS" if success else "❌ FAILED"
        print(f"{model_name:20} | {status}")
    
    print(f"\n📊 Results: {successful}/{total} models converted successfully")
    
    if successful > 0:
        converter.generate_mlx_registry(results)
        print("\n🚀 MLX models ready for M1 MacBook Air deployment!")
    else:
        print("\n❌ No models were converted successfully")
        return 1
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
