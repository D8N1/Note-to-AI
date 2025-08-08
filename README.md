# note-to-ai
> *Transform your Signal "Note to Self" into an AI-powered knowledge base and personal assistant*

**Your most private thoughts deserve the most private AI.**

note-to-ai bridges the gap between your casual voice notes and professional-grade intelligence briefings. Send a voice message to Signal's "Note to Self", and receive back a structured, searchable knowledge base with AI-generated insights — all processed locally on your M1 MacBook.

## 🎯 The Value Proposition

**Signal "Note to Self" → Local AI → LLM Processing → Local AI → Structured .md "President's Brief"**

### The Workflow
1. **💬 Capture**: Send voice notes, photos, or text to Signal "Note to Self"
2. **🤖 Process**: Local AI transcribes, analyzes, and structures your input
3. **🧠 Understand**: Specialized LLMs extract insights, generate questions, and create summaries
4. **📊 Brief**: Output structured markdown "President's Brief" with key insights, action items, and connections
5. **🔍 Discover**: Semantic search across your entire knowledge base reveals hidden patterns

### Why This Matters
- **Privacy First**: Protected by Signal's proven encryption suite - your thoughts are secured in transit and processed locally.
- **Intelligence Amplification**: Transform scattered thoughts into structured knowledge
- **Effortless Capture**: Use the app you already have (Signal) as your input interface
- **Professional Output**: Generate executive-level briefings from casual voice notes
- **Knowledge Compound**: Each note enhances your searchable knowledge graph

## ✨ Key Features 

### 🎤 Voice-First Intelligence
- **Whisper Integration**: M1-optimized speech-to-text with 13.3x real-time processing
- **Smart Transcription**: Context-aware transcription that understands your speaking patterns
- **Multi-Modal Input**: Voice notes, photos with OCR, text messages, and document uploads

### 🧠 Specialized AI Pipeline
- **Hermes 3 8B**: Advanced agentic model for reasoning and analysis
- **DistilBART-CNN**: 97% BART performance for document summarization (44% ROUGE-1)
- **Question Generation**: Automatic follow-up questions and conversation starters
- **Semantic Search**: Find connections across your entire knowledge base

### 📊 Executive-Grade Output
- **President's Brief Format**: Structured daily/weekly intelligence summaries
- **Action Item Extraction**: Automatically identify and track tasks
- **Trend Analysis**: Spot patterns across your notes and conversations
- **Knowledge Graphs**: Visual connections between ideas and topics

### 🔐 Privacy & Security
- **Signal-Protected Communication**: All data in transit secured by Signal's proven E2E encryption
- **Local AI Processing**: Zero cloud dependencies, all AI runs on your M1 Mac
- **Quantum-Resistant Encryption**: ML-KEM + Signal hybrid cryptography
- **IPFS Private Swarm**: Distributed sync without central servers
- **zkPassport Integration**: Identity verification with zero-knowledge proofs

### ⚡ M1 MacBook Optimized
- **Metal Backend**: GPU acceleration for all AI models
- **Memory Efficient**: 4-8GB usage with dynamic model loading
- **Real-Time Processing**: Sub-second response times for most operations
- **Battery Optimized**: Efficient inference pipelines designed for mobile workflows

## � Quick Start

### Prerequisites
- M1 MacBook Air/Pro (8GB+ RAM recommended)
- Signal Desktop/Mobile with "Note to Self" enabled
- macOS 13+ with Xcode command line tools

### Installation
```bash
# Clone the repository
git clone https://github.com/D8N1/note-to-ai.git
cd note-to-ai

# Install dependencies and download models
./scripts/install.sh

# Configure Signal integration
cargo run -- signal setup

# Start the service
cargo run -- start
```

### First Use
1. **Setup Signal**: Link your Signal account and enable "Note to Self" monitoring
2. **Send Test Note**: Send a voice message to "Note to Self": *"This is a test of my new AI assistant"*
3. **Receive Brief**: Get back a structured markdown summary with insights and questions
4. **Explore**: Use `cargo run -- query "test"` to search your knowledge base

## 💡 Example Workflows

### Daily Executive Brief
**Input** (Voice to Signal):
> *"Had a great call with the Tokyo team about Q1 projections. Revenue looking strong, but supply chain still concerning. Need to follow up with Sarah about the European distribution deal. Also, remind me to prep for the board presentation next week."*

**Output** (Structured .md):
```markdown
# Executive Brief - Tokyo Team Call
**Date**: 2025-08-08 14:30
**Type**: Strategic Update

## Key Insights
- **Revenue Outlook**: Q1 projections showing strength from Tokyo operations
- **Risk Factor**: Supply chain constraints remain a concern
- **Partnership Opportunity**: European distribution deal in progress with Sarah

## Action Items
- [ ] Follow up with Sarah re: European distribution deal
- [ ] Prepare board presentation materials for next week
- [ ] Deep dive on supply chain mitigation strategies

## Strategic Questions
- What specific supply chain bottlenecks are impacting Tokyo operations?
- How does the European deal timeline align with Q1 revenue targets?
- What data points should be highlighted in the board presentation?

## Connections
Related to previous notes: [Supply Chain Strategy 2025], [Q4 Board Deck], [Sarah Partnership Discussions]
```

### Research & Learning
**Input**: Share research papers, articles, or voice summaries
**Output**: Structured knowledge cards with key concepts, questions for further research, and connections to existing knowledge

### Meeting Intelligence
**Input**: Voice notes during/after meetings
**Output**: Action items, follow-ups, strategic insights, and relationship mapping

## 🏗️ Architecture

### Core Pipeline
```
Signal "Note to Self" 
    ↓
[Local Signal Monitor]
    ↓
[Multi-Modal Processing]
├── Voice → Whisper → Transcription
├── Images → OCR → Text Extraction  
├── Documents → Parser → Content
└── Text → Direct Processing
    ↓
[AI Analysis Pipeline]
├── Hermes 3 8B → Reasoning & Context
├── DistilBART → Summarization
├── T5-Small → Question Generation
└── MiniLM → Semantic Embeddings
    ↓
[Knowledge Integration]
├── Semantic Search → Related Context
├── CRDT Sync → Multi-Device State
├── Graph Building → Concept Connections
└── Trend Analysis → Pattern Recognition
    ↓
[Executive Brief Generation]
└── Structured .md → President's Brief Format
```

### Privacy Architecture
- **Signal-Encrypted Transport**: All communication secured by Signal's proven E2E encryption
- **Local AI Processing**: All AI inference happens on your M1 Mac
- **Quantum-Resistant**: ML-KEM encryption for future-proof security
- **Distributed Sync**: IPFS private swarm for multi-device access without servers
- **Identity Sovereignty**: zkPassport integration for decentralized identity

### 📁 Project Structure
```text
note-to-ai/
├── 🧠 AI Models & Intelligence
│   ├── models/
│   │   ├── hermes-3-8b.safetensors      # Primary reasoning model
│   │   ├── distilbart-cnn.safetensors   # Document summarization
│   │   ├── whisper-distil-large-v3.safetensors # Voice transcription
│   │   ├── all-MiniLM-L6-v2.safetensors # Semantic embeddings
│   │   └── model_registry.toml          # M1-optimized configurations
│
├── 📱 Signal Integration
│   ├── src/signal/
│   │   ├── client.rs                    # Signal protocol client
│   │   ├── crypto.rs                    # E2E encryption + ML-KEM
│   │   └── protocol.rs                  # "Note to Self" monitoring
│
├── 🎤 Multi-Modal Processing  
│   ├── src/audio/
│   │   ├── whisper.rs                   # Voice transcription
│   │   └── formats.rs                   # Audio processing
│
├── 🗄️ Knowledge Management
│   ├── src/vault/
│   │   ├── indexer.rs                   # Content indexing
│   │   ├── embeddings.rs                # Semantic understanding
│   │   ├── search.rs                    # RAG and semantic search
│   │   ├── parser.rs                    # Multi-format parsing
│   │   └── storage/                     # Hybrid DuckDB + Lance storage
│
├── 🤖 AI Orchestration
│   ├── src/ai/
│   │   ├── local_llm.rs                 # Model switching & inference
│   │   ├── hermes_integration.rs        # Agentic capabilities
│   │   ├── context.rs                   # RAG context building
│   │   └── model_switcher.rs            # Dynamic model loading
│
├── 🔐 Privacy & Security
│   ├── src/crypto/
│   │   ├── pq_vault.rs                  # Quantum-resistant encryption
│   │   ├── hybrid_crypto.rs             # ML-KEM + Signal integration
│   │   └── blake3_hasher.rs             # Content addressing
│   │
│   └── src/identity/
│       ├── zkpassport.rs                # Decentralized identity
│       └── passport_nfc.rs              # Hardware identity verification
│
├── 🌐 Distributed Sync
│   └── src/swarm/
│       ├── ipfs.rs                      # Private IPFS node
│       ├── sync.rs                      # Multi-device synchronization
│       └── discovery.rs                 # Device discovery
│
└── ⚙️  Configuration & Operations
    ├── config/config.toml               # System configuration
    ├── scripts/install.sh               # Automated setup
    └── src/main.rs                      # CLI interface
```

## 🎛️ CLI Commands

### Basic Operations
```bash
# Start the AI assistant service
cargo run -- start

# Query your knowledge base
cargo run -- query "quarterly projections"
cargo run -- query "supply chain" --semantic

# Get system status
cargo run -- status

# Export your knowledge base
cargo run -- export --format obsidian --output ./my-vault
```

### Signal Integration
```bash
# Setup Signal connection
cargo run -- signal setup

# Test Signal connectivity
cargo run -- signal test

# Monitor Signal messages (manual mode)
cargo run -- signal monitor --manual
```

### Model Management
```bash
# List available models
cargo run -- models list

# Download specific models
cargo run -- models download hermes-3-8b
cargo run -- models download distilbart-cnn

# Switch active model profile
cargo run -- models profile morning_briefing
cargo run -- models profile full_deployment
```

## 🔧 Configuration

### Model Profiles

**Morning Briefing** (6GB RAM):
- DistilBART-CNN for summarization
- T5-Small for structured briefings  
- MiniLM for semantic search

**Voice Processing** (2.5GB RAM):
- Whisper-DistilLarge-v3 for transcription
- Question generation for follow-ups

**Full Deployment** (12GB RAM):
- All models loaded for maximum capability
- Real-time processing with sub-second response

### Signal Configuration
```toml
[signal]
device_name = "note-to-ai-assistant"
monitor_note_to_self = true
response_format = "presidents_brief"
auto_summarize = true
generate_questions = true
```

## 🌟 Roadmap

### Phase 1: Core Intelligence (Current)
- ✅ Signal "Note to Self" integration
- ✅ Multi-modal AI processing pipeline
- ✅ Executive brief generation
- ✅ M1-optimized inference

### Phase 2: Advanced Features (Q1 2025)
- 🔄 Real-time collaboration via IPFS
- 🔄 Advanced trend analysis and predictions
- 🔄 Custom brief templates and formats
- 🔄 API for third-party integrations

### Phase 3: Enterprise Ready (Q2 2025)
- 📋 Team knowledge bases
- 📋 Advanced security and compliance
- 📋 Integration with business tools
- 📋 Scalable deployment options

## 🤝 Contributing

We welcome contributions! Areas where help is needed:

- **Model Optimization**: Improving inference speed and memory usage
- **Signal Protocol**: Enhancing message processing and formatting
- **Brief Templates**: Creating new output formats and structures
- **Testing**: Expanding test coverage for AI pipeline components

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Nous Research** for Hermes 3 model architecture
- **OpenAI** for Whisper speech recognition
- **Signal Foundation** for the Signal protocol
- **Hugging Face** for model hosting and transformers
- **Apple** for Metal Performance Shaders optimization

---

**Ready to transform your thoughts into intelligence?**

*Start with a simple voice note to Signal "Note to Self" and experience the future of personal AI assistance.*


