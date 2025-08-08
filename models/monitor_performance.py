#!/usr/bin/env python3
"""
M1 MacBook Air Multi-Agent Model Performance Monitor
Based on specialized AI model research for optimal deployment tracking
"""

import psutil
import time
import json
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional

class M1ModelMonitor:
    def __init__(self):
        self.models_dir = Path(__file__).parent
        self.performance_log = self.models_dir / "performance_metrics.json"
        self.load_model_registry()
        
    def load_model_registry(self):
        """Load model configuration from registry"""
        try:
            import toml
            registry_path = self.models_dir / "model_registry.toml"
            with open(registry_path, 'r') as f:
                self.registry = toml.load(f)
        except ImportError:
            print("⚠️  toml package required: pip install toml")
            sys.exit(1)
        except FileNotFoundError:
            print("❌ model_registry.toml not found")
            sys.exit(1)
    
    def get_system_metrics(self) -> Dict:
        """Get current system performance metrics"""
        # Memory usage
        memory = psutil.virtual_memory()
        
        # CPU usage
        cpu_percent = psutil.cpu_percent(interval=1)
        
        # GPU/Metal usage (approximated via activity monitor)
        try:
            gpu_info = subprocess.run(
                ["system_profiler", "SPDisplaysDataType"], 
                capture_output=True, text=True
            )
            gpu_active = "Metal" in gpu_info.stdout
        except:
            gpu_active = False
        
        # Temperature (if available)
        try:
            temp_result = subprocess.run(
                ["sudo", "powermetrics", "--samplers", "smc", "-n", "1"], 
                capture_output=True, text=True
            )
            temp_data = temp_result.stdout
        except:
            temp_data = "N/A"
        
        return {
            "timestamp": datetime.now().isoformat(),
            "memory_total_gb": round(memory.total / (1024**3), 2),
            "memory_used_gb": round(memory.used / (1024**3), 2),
            "memory_percent": memory.percent,
            "memory_available_gb": round(memory.available / (1024**3), 2),
            "cpu_percent": cpu_percent,
            "gpu_metal_active": gpu_active,
            "temperature_info": "Available via powermetrics" if "CPU die temperature" in temp_data else "N/A"
        }
    
    def get_ollama_models(self) -> List[Dict]:
        """Get currently loaded Ollama models"""
        try:
            result = subprocess.run(
                ["ollama", "list"], 
                capture_output=True, text=True
            )
            
            models = []
            lines = result.stdout.strip().split('\n')[1:]  # Skip header
            
            for line in lines:
                if line.strip():
                    parts = line.split()
                    if len(parts) >= 3:
                        models.append({
                            "name": parts[0],
                            "id": parts[1] if len(parts) > 1 else "",
                            "size": parts[2] if len(parts) > 2 else "",
                            "modified": " ".join(parts[3:]) if len(parts) > 3 else ""
                        })
            
            return models
        except:
            return []
    
    def estimate_model_memory_usage(self) -> Dict:
        """Estimate memory usage based on model registry"""
        estimated_usage = {}
        
        if "models" in self.registry:
            for model_name, config in self.registry["models"].items():
                if isinstance(config, dict) and "memory_usage_mb" in config:
                    estimated_usage[model_name] = {
                        "estimated_mb": config["memory_usage_mb"],
                        "estimated_gb": round(config["memory_usage_mb"] / 1024, 2),
                        "type": config.get("type", "unknown"),
                        "quantization": config.get("quantization", "none")
                    }
        
        return estimated_usage
    
    def check_deployment_profiles(self) -> Dict:
        """Check which deployment profiles are feasible"""
        system_metrics = self.get_system_metrics()
        available_memory = system_metrics["memory_available_gb"]
        
        profile_status = {}
        
        if "profiles" in self.registry:
            for profile_name, config in self.registry["profiles"].items():
                if isinstance(config, dict) and "memory_total_mb" in config:
                    required_gb = config["memory_total_mb"] / 1024
                    feasible = available_memory >= required_gb
                    
                    profile_status[profile_name] = {
                        "required_gb": round(required_gb, 2),
                        "feasible": feasible,
                        "models": config.get("models", []),
                        "use_case": config.get("use_case", "")
                    }
        
        return profile_status
    
    def run_performance_test(self, model_name: str = "hermes3:8b") -> Dict:
        """Run a simple performance test on specified model"""
        try:
            start_time = time.time()
            
            # Simple test prompt
            test_prompt = "Explain the concept of quantization in AI models in one sentence."
            
            result = subprocess.run([
                "ollama", "run", model_name, test_prompt
            ], capture_output=True, text=True, timeout=30)
            
            end_time = time.time()
            response_time = end_time - start_time
            
            if result.returncode == 0:
                # Estimate tokens (rough approximation)
                words = len(result.stdout.split())
                estimated_tokens = int(words * 1.3)  # Rough word-to-token conversion
                tokens_per_second = estimated_tokens / response_time if response_time > 0 else 0
                
                return {
                    "model": model_name,
                    "success": True,
                    "response_time_seconds": round(response_time, 2),
                    "estimated_tokens": estimated_tokens,
                    "tokens_per_second": round(tokens_per_second, 2),
                    "response_preview": result.stdout[:100] + "..." if len(result.stdout) > 100 else result.stdout
                }
            else:
                return {
                    "model": model_name,
                    "success": False,
                    "error": result.stderr
                }
        
        except subprocess.TimeoutExpired:
            return {
                "model": model_name,
                "success": False,
                "error": "Timeout after 30 seconds"
            }
        except Exception as e:
            return {
                "model": model_name,
                "success": False,
                "error": str(e)
            }
    
    def generate_report(self) -> Dict:
        """Generate comprehensive performance report"""
        print("🔍 Generating M1 MacBook Air Model Performance Report...")
        
        report = {
            "report_timestamp": datetime.now().isoformat(),
            "system_metrics": self.get_system_metrics(),
            "ollama_models": self.get_ollama_models(),
            "estimated_memory_usage": self.estimate_model_memory_usage(),
            "deployment_profiles": self.check_deployment_profiles(),
            "performance_tests": {}
        }
        
        # Test available models
        available_models = [model["name"] for model in report["ollama_models"]]
        
        for model in available_models[:3]:  # Test up to 3 models to avoid overload
            print(f"🧪 Testing {model}...")
            report["performance_tests"][model] = self.run_performance_test(model)
        
        return report
    
    def print_report(self, report: Dict):
        """Print formatted report to console"""
        print("\n" + "="*60)
        print("🚀 M1 MACBOOK AIR MODEL PERFORMANCE REPORT")
        print("="*60)
        
        # System Metrics
        sys_metrics = report["system_metrics"]
        print(f"\n💻 System Status:")
        print(f"   Memory: {sys_metrics['memory_used_gb']:.1f}GB / {sys_metrics['memory_total_gb']:.1f}GB ({sys_metrics['memory_percent']:.1f}%)")
        print(f"   Available: {sys_metrics['memory_available_gb']:.1f}GB")
        print(f"   CPU Usage: {sys_metrics['cpu_percent']:.1f}%")
        print(f"   Metal GPU: {'✅ Active' if sys_metrics['gpu_metal_active'] else '❌ Inactive'}")
        
        # Deployment Profiles
        print(f"\n📋 Deployment Profile Feasibility:")
        for profile_name, status in report["deployment_profiles"].items():
            feasible_icon = "✅" if status["feasible"] else "❌"
            print(f"   {feasible_icon} {profile_name}: {status['required_gb']:.1f}GB required")
            if not status["feasible"]:
                print(f"      Need {status['required_gb'] - sys_metrics['memory_available_gb']:.1f}GB more memory")
        
        # Current Models
        print(f"\n🤖 Loaded Models ({len(report['ollama_models'])}):")
        for model in report["ollama_models"]:
            print(f"   • {model['name']} ({model['size']})")
        
        # Performance Tests
        if report["performance_tests"]:
            print(f"\n⚡ Performance Tests:")
            for model_name, test_result in report["performance_tests"].items():
                if test_result["success"]:
                    print(f"   ✅ {model_name}:")
                    print(f"      Response Time: {test_result['response_time_seconds']}s")
                    print(f"      Tokens/Second: {test_result['tokens_per_second']:.1f}")
                else:
                    print(f"   ❌ {model_name}: {test_result['error']}")
        
        # Memory Estimates
        print(f"\n📊 Memory Usage Estimates:")
        for model_name, usage in report["estimated_memory_usage"].items():
            print(f"   • {model_name}: {usage['estimated_gb']:.1f}GB ({usage['type']})")
        
        print(f"\n📝 Report saved to: {self.performance_log}")
        print("="*60)
    
    def save_report(self, report: Dict):
        """Save report to JSON file"""
        with open(self.performance_log, 'w') as f:
            json.dump(report, f, indent=2)
    
    def monitor(self, save_to_file: bool = True):
        """Run complete monitoring cycle"""
        report = self.generate_report()
        self.print_report(report)
        
        if save_to_file:
            self.save_report(report)

def main():
    """Main entry point"""
    if len(sys.argv) > 1 and sys.argv[1] == "--no-save":
        save_to_file = False
    else:
        save_to_file = True
    
    monitor = M1ModelMonitor()
    monitor.monitor(save_to_file=save_to_file)

if __name__ == "__main__":
    main()
