# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Parallel tool execution with dependency-based wave computation
- Git-like snapshot system with branching and merging
- Comprehensive API documentation
- Integration test suite
- Generated code tracking system
- Enhanced error handling with retry policies
- Traffic branch system for merge safety analysis
- Dual-watcher architecture (LSP + File System)
- Tool lifecycle management with events
- Component injection from R2 storage
- Pattern detection for DX tools
- Semantic versioning with compatibility checking
- Content-addressable storage with SHA-256
- CRDT-based document operations
- WebSocket server for real-time updates
- VSCode extension integration support

### Changed
- Restructured core modules for better API ergonomics
- Improved documentation with examples
- Enhanced orchestrator with parallel execution support
- Updated tool trait with comprehensive lifecycle hooks

### Fixed
- Compilation errors in core modules
- Missing exports in lib.rs
- Circular dependency detection

## [0.0.2] - 2025-01-21

### Added
- Initial crate structure
- Basic tool orchestration
- File watching capabilities
- Version control foundation
- Storage layer with SQLite

### Changed
- Project restructuring for library use

## [0.0.1] - 2025-01-20

### Added
- Initial project setup
- Basic CLI implementation
- LSP server foundation

[Unreleased]: https://github.com/najmus-sakib-hossain/forge/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/najmus-sakib-hossain/forge/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/najmus-sakib-hossain/forge/releases/tag/v0.0.1
