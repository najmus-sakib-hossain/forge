# Forge Demo Repository

This is a demonstration repository for the Forge version control system.

## Features Demonstrated

1. **Binary Blob Storage** - All files stored as compressed blobs
2. **Cloudflare R2 Integration** - Zero-egress cloud storage
3. **Traffic Branches** - Red/Yellow/Green deployment branches
4. **LSP Detection** - Smart change detection via Language Server Protocol
5. **CRDT Sync** - Conflict-free replicated data types for real-time collaboration

## Files in This Demo

- `src/main.rs` - Simple Rust application
- `src/lib.rs` - Library code
- `Cargo.toml` - Project manifest
- `README.md` - This file

## Usage

This repository is initialized with Forge, not Git. All version control operations
are handled by Forge and stored in Cloudflare R2.

```bash
# View commit history
forge log

# Create a new traffic branch
forge branch --traffic green

# View storage stats
forge stats
```

## Storage Backend

All blobs are stored in Cloudflare R2 with:

- LZ4 compression (10-50x faster than gzip)
- SHA-256 content addressing
- Zero egress fees
- 99.999999999% durability
