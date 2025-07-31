# Note-to-AI

A Rust-based, local-first application for managing Obsidian vault data using an IPFS private swarm and CRDTs for conflict-free replication. Integrates Signal messaging, quantum-resistant cryptography, and local AI models (Whisper, Hermes, LLaMA).

## Features
- **Distributed Vault**: Stores Obsidian markdown files in an IPFS private swarm with `libp2p`.
- **Conflict Resolution**: Uses `automerge-rs` for CRDT-based merging of concurrent edits.
- **Privacy**: Quantum-resistant encryption with ML-KEM and zero-knowledge proofs (zkPassport).
- **Local AI**: Processes queries with Candle-based models (Whisper, MiniLM, Hermes).
- **Offline Support**: 


note-to-ai/
├── Cargo.toml                    # Quantum-resistant crypto stack
├── src/
│   ├── main.rs                   # Single binary entry point
│   ├── lib.rs                    # Core library
│   │
│   ├── signal/
│   │   ├── mod.rs
│   │   ├── client.rs             # libsignal-protocol integration
│   │   ├── crypto.rs             # Signal crypto + ML-KEM hybrid
│   │   └── protocol.rs           # Signal protocol with PQ extensions
│   │
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── whisper.rs            # Candle-based Whisper
│   │   └── formats.rs            # Audio format conversion
│   │
│   ├── vault/
│   │   ├── mod.rs
│   │   ├── indexer.rs            # .md file indexer with BLAKE3
│   │   ├── embeddings.rs         # Candle-based embeddings
│   │   ├── search.rs             # Vector similarity search
│   │   ├── parser.rs             # Obsidian markdown parser
│   │   ├── cache.rs              # Intelligent caching layer
│   │   └── crdt.rs               # Automerge CRDT for sync
│   │
│   ├── ai/
│   │   ├── mod.rs
│   │   ├── local_llm.rs          # Multi-model support (Llama/Hermes)
│   │   ├── hermes_integration.rs # Nous Research Hermes models
│   │   ├── model_switcher.rs     # Dynamic model switching
│   │   ├── api_client.rs         # Quantum-safe HTTP clients
│   │   └── context.rs            # RAG context builder
│   │
│   ├── identity/
│   │   ├── mod.rs
│   │   ├── zkpassport.rs         # zkPassport integration
│   │   ├── passport_nfc.rs       # NFC passport chip reading
│   │   ├── zk_circuits.rs        # Zero-knowledge proof circuits
│   │   ├── british_passport.rs   # UK passport specific handling
│   │   └── spam_filter.rs        # Identity-based spam resistance
│   │
│   ├── crypto/
│   │   ├── mod.rs
│   │   ├── pq_vault.rs           # ML-KEM + Signal hybrid encryption
│   │   ├── keys.rs               # Quantum-resistant key derivation
│   │   ├── blake3_hasher.rs      # BLAKE3 content addressing
│   │   ├── hybrid_crypto.rs      # Classical + PQ crypto wrapper
│   │   └── zk_proofs.rs          # Zero-knowledge proof utilities
│   │
│   ├── scheduler/
│   │   ├── mod.rs
│   │   └── tasks.rs              # Async task scheduling
│   │
│   ├── swarm/                    # IPFS private swarm integration
│   │   ├── mod.rs
│   │   ├── ipfs.rs               # Private IPFS node
│   │   ├── sync.rs               # Cross-device synchronization
│   │   └── discovery.rs          # Device discovery in private swarm
│   │
│   ├── logger.rs                 # Structured logging with tracing
│   │
│   └── config/
│       ├── mod.rs
│       └── settings.rs           # Configuration management
│
├── models/                       # AI model files
│   ├── whisper-base.safetensors  # ~290MB Candle-compatible
│   ├── all-MiniLM-L6-v2.safetensors # ~90MB embeddings
│   ├── hermes-3-8b.safetensors   # ~16GB Nous Research Hermes 3
│   ├── llama-3.2-3b.safetensors  # ~6GB fallback Llama model
│   └── model_registry.toml       # Model switching configuration
│
├── config/
│   ├── config.toml
│   └── .env.template
│
├── scripts/
│   ├── install.sh
│   └── download-models.sh
│
└── README.md
