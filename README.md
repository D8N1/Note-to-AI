# Note-to-AI

A Rust-based, local-first application for managing Obsidian vault data using an IPFS private swarm and CRDTs for conflict-free replication. Integrates Signal messaging, quantum-resistant cryptography, and local AI models (Whisper, Hermes, LLaMA).

## Features
- **Distributed Vault**: Stores Obsidian markdown files in an IPFS private swarm with `libp2p`.
- **Conflict Resolution**: Uses `automerge-rs` for CRDT-based merging of concurrent edits.
- **Privacy**: Quantum-resistant encryption with ML-KEM and zero-knowledge proofs (zkPassport).
- **Local AI**: Processes queries with Candle-based models (Whisper, MiniLM, Hermes).
- **Offline Support**: 



