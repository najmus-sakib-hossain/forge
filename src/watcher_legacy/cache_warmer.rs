use anyhow::Result;
use colored::*;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB

// Shared file handle pool
pub static FILE_POOL: Lazy<RwLock<HashMap<PathBuf, Arc<File>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Warm the OS page cache by reading all trackable files
pub fn warm_cache(repo_root: &Path) -> Result<CacheStats> {
    let start = Instant::now();

    // println!("{}", "📦 Warming OS page cache...".bright_cyan());

    // Collect all trackable files
    let files = collect_trackable_files(repo_root)?;
    let total_files = files.len();

    if total_files == 0 {
        println!("{} No files to cache", "✓".bright_green());
        return Ok(CacheStats::default());
    }

    // Progress tracking
    let cached_count = Arc::new(AtomicUsize::new(0));
    let cached_bytes = Arc::new(AtomicUsize::new(0));

    // Pre-open file handles and warm cache in parallel
    // This ensures subsequent reads are instant
    let handles: Vec<_> = files
        .par_iter()
        .filter_map(|path| {
            // Try to open and read to warm cache
            if let Ok(file) = File::open(path) {
                // Read to warm OS cache
                if let Ok(mmap) = unsafe { memmap2::Mmap::map(&file) } {
                    let size = mmap.len();
                    cached_count.fetch_add(1, Ordering::Relaxed);
                    cached_bytes.fetch_add(size, Ordering::Relaxed);
                    return Some((path.clone(), Arc::new(file)));
                }
            }
            None
        })
        .collect();

    // Populate pool with all opened handles
    let mut pool = FILE_POOL.write();
    for (path, file) in handles {
        pool.insert(path, file);
    }
    drop(pool);

    let final_count = cached_count.load(Ordering::Relaxed);
    let final_bytes = cached_bytes.load(Ordering::Relaxed);
    let elapsed = start.elapsed();

    // println!(
    //     "{} Cached {} files ({} KB) in {:?}",
    //     "✓".bright_green(),
    //     final_count,
    //     final_bytes / 1024,
    //     elapsed
    // );

    Ok(CacheStats {
        files_cached: final_count,
        bytes_cached: final_bytes,
        duration_ms: elapsed.as_millis() as u64,
    })
}

/// Incrementally warm cache for new files as they're discovered
pub fn warm_file(path: &Path) -> Result<()> {
    // Simply read the file to get it into OS cache
    let _ = fs::read(path)?;
    Ok(())
}

/// Collect all files that should be tracked (respecting .gitignore-like rules)
fn collect_trackable_files(root: &Path) -> Result<Vec<PathBuf>> {
    use ignore::WalkBuilder;

    let mut files = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .max_depth(None)
        .follow_links(false)
        .build();

    for entry in walker {
        if let Ok(entry) = entry {
            let path = entry.path();

            // Skip if not a file
            if !path.is_file() {
                continue;
            }

            // Skip if in ignored directories
            if !is_trackable(path) {
                continue;
            }

            // Skip if too large
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.len() > MAX_FILE_SIZE {
                    continue;
                }
            }

            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

fn is_trackable(path: &Path) -> bool {
    use std::path::Component;

    const IGNORED_COMPONENTS: [&str; 5] = [".git", ".dx", ".dx_client", "target", "node_modules"];

    for component in path.components() {
        if let Component::Normal(seg) = component {
            if let Some(segment) = seg.to_str() {
                let lower = segment.to_ascii_lowercase();
                if IGNORED_COMPONENTS.iter().any(|needle| needle == &lower) {
                    return false;
                }
            }
        }
    }

    true
}

#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
pub struct CacheStats {
    pub files_cached: usize,
    pub bytes_cached: usize,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_collect_trackable_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test structure
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("README.md"), "# Test").unwrap();

        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "ignored").unwrap();

        let files = collect_trackable_files(root).unwrap();

        assert!(files.iter().any(|p| p.ends_with("main.rs")));
        assert!(files.iter().any(|p| p.ends_with("README.md")));
        assert!(!files.iter().any(|p| p.to_str().unwrap().contains(".git")));
    }

    #[test]
    fn test_warm_cache() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("test.txt"), "test content").unwrap();

        let stats = warm_cache(root).unwrap();
        assert!(stats.files_cached > 0);
        assert!(stats.bytes_cached > 0);
    }
}
