# DX Forge Usage Guide

This guide explains how to use `dx-forge` as the orchestration engine for your DX tools.

## 1. Installation

Add `dx-forge` to your `Cargo.toml`:

```toml
[dependencies]
dx-forge = { version = "0.1.0", path = "../forge" } # Use path or crates.io version
anyhow = "1.0"
```

## 2. Creating a DX Tool

To create a tool that integrates with Forge, implement the `DxTool` trait.

```rust
use dx_forge::{DxTool, ExecutionContext, ToolOutput};
use anyhow::Result;

pub struct MyCustomTool;

impl DxTool for MyCustomTool {
    // Unique identifier for your tool
    fn name(&self) -> &str {
        "dx-my-tool"
    }

    // Semantic versioning
    fn version(&self) -> &str {
        "1.0.0"
    }

    // Execution priority (0-100, lower runs earlier)
    fn priority(&self) -> u32 {
        50
    }

    // Main execution logic
    fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> {
        println!("🚀 Running MyCustomTool");
        
        // Access changed files
        for file in &ctx.changed_files {
            println!("Processing: {:?}", file);
        }

        Ok(ToolOutput::success())
    }

    // Optional: Define when the tool should run
    fn should_run(&self, ctx: &ExecutionContext) -> bool {
        // Example: Only run if .ts files changed
        ctx.changed_files.iter().any(|f| f.extension().map_or(false, |e| e == "ts"))
    }
}
```

## 3. Initialization & Registration

In your application entry point (e.g., `main.rs`), initialize Forge and register your tools.

```rust
use dx_forge::{initialize_forge, register_tool, shutdown_forge};

fn main() -> anyhow::Result<()> {
    // 1. Initialize the Forge engine
    initialize_forge()?;

    // 2. Register your tools
    register_tool(Box::new(MyCustomTool))?;

    // ... application logic ...

    // 3. Graceful shutdown
    shutdown_forge()?;
    
    Ok(())
}
```

## 4. Orchestration & Pipelines

Forge manages tool execution via pipelines. You can trigger pipelines manually or let Forge handle them automatically based on events.

```rust
use dx_forge::{execute_pipeline, execute_tool_immediately};

// Run the default pipeline (executes all registered tools in priority order)
execute_pipeline("default")?;

// Run a specific tool immediately (bypassing queue)
execute_tool_immediately("dx-my-tool")?;
```

## 5. Reactivity (File Changes)

Forge automatically watches for file changes (via LSP or filesystem). You can also manually trigger events.

```rust
use dx_forge::{trigger_realtime_event, trigger_debounced_event};
use std::path::PathBuf;

// Trigger a realtime event (e.g., from an editor plugin)
trigger_realtime_event(PathBuf::from("src/App.tsx"), "content...".to_string())?;

// Trigger a debounced event (good for linters/formatters)
trigger_debounced_event(PathBuf::from("src/App.tsx"), "content...".to_string()).await?;
```

## 6. Branching System (Safe Writes)

**Crucial:** Never write files directly using `std::fs`. Use the Branching API to ensure safety (Traffic Light System).

```rust
use dx_forge::{
    apply_changes, FileChange, ChangeKind, ChangeSource, 
    BranchColor, query_predicted_branch_color
};

fn generate_code(path: PathBuf, content: String) -> anyhow::Result<()> {
    // 1. Check safety first
    let safety = query_predicted_branch_color(&path)?;
    
    if safety == BranchColor::Red {
        println!("⛔ Cannot auto-apply changes to this file!");
        return Ok(());
    }

    // 2. Create a change request
    let change = FileChange {
        path,
        kind: ChangeKind::Modified,
        source: ChangeSource::FileSystem, // or Lsp
        timestamp: std::time::SystemTime::now(),
        content: Some(content.clone()),
        new_content: content, // Field used by apply_changes
        tool_id: "dx-my-tool".to_string(),
        patterns: None,
    };

    // 3. Apply safely (Forge handles conflicts and branching)
    apply_changes(vec![change])?;
    
    Ok(())
}
```

## 7. Configuration Injection

Forge provides a unified configuration system (`dx.toml`). You can inject default configs for your tool.

```rust
use dx_forge::inject_full_config_section_at_cursor;

// In your editor extension or CLI setup
let config_template = inject_full_config_section_at_cursor("style")?;
// Returns:
// [style]
// processor = "tailwind"
// ...
```

## 8. LSP Integration

Forge includes a built-in LSP server that provides:

- **Auto-completion**: `dxButton`, `dxiHome`
- **Hover info**: Component documentation
- **Diagnostics**: Real-time feedback

To start the LSP server:

```bash
cargo run --bin forge-lsp
```

Your editor extension should connect to this server via stdio.

## 9. State Management

Forge maintains a history of tool states in `.dx/state/`.

```rust
use dx_forge::{commit_current_dx_state, checkout_dx_state};

// Save a snapshot of the current tool state
let commit_id = commit_current_dx_state("Applied UI updates")?;

// Revert to a previous state
checkout_dx_state(&commit_id)?;
```
