# note-to-ai

A Rust-based, local-first application for managing Obsidian vault data using an IPFS private swarm and CRDTs for conflict-free replication. Integrates Signal messaging, quantum-resistant cryptography, and local AI models (Whisper, Hermes, LLaMA).

## Features

- **Distributed Vault**: Stores Obsidian markdown files in an IPFS private swarm with `libp2p`
- **Conflict Resolution**: Uses `automerge-rs` for CRDT-based merging of concurrent edits
- **Privacy**: Quantum-resistant encryption with ML-KEM and zero-knowledge proofs (zkPassport)
- **Local AI**: Processes queries with Candle-based models (Whisper, MiniLM, Hermes)
- **Offline Support**: Full offline functionality with local AI models

## Quick Start

1. **Install**: `./scripts/install.sh`
2. **Download Models**: `./scripts/download-models.sh`
3. **Setup Swarm**: `./scripts/setup-swarm.sh`
4. **Run**: `cargo run`

## Architecture

The application is built with a modular architecture supporting:
- **Vault Management**: Indexing, parsing, and searching Obsidian markdown
- **AI Integration**: Local LLM inference with model switching
- **Cryptography**: Quantum-resistant encryption with hybrid classical/PQ crypto
- **Distributed Storage**: IPFS private swarm with CRDT synchronization
- **Identity Verification**: zkPassport integration with NFC support

## Development

This project uses a team-based development approach with specialized AI assistants:
- **Lead Engineer**: Architecture and system design
- **Applications Engineer**: Core application logic and integration
- **Security Engineer**: Cryptography and identity systems
- **AI Engineer**: Machine learning model integration
- **DevOps Engineer**: Deployment and infrastructure

## License

MIT License

