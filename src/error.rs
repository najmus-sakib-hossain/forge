//! Production Error Handling and Retry Logic
//!
//! Provides robust error handling, retry mechanisms, and detailed error reporting
//! for DX tools orchestration.

use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial delay between retries
    pub initial_delay: Duration,

    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,

    /// Maximum delay between retries
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(5),
        }
    }
}

impl RetryPolicy {
    /// Create a no-retry policy
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            ..Default::default()
        }
    }

    /// Create an aggressive retry policy
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(50),
            backoff_multiplier: 1.5,
            max_delay: Duration::from_secs(3),
        }
    }
}

/// Execute with retry logic
pub async fn with_retry<F, T, E>(policy: &RetryPolicy, mut operation: F) -> Result<T>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
{
    let mut attempts = 0;
    let mut delay = policy.initial_delay;

    loop {
        attempts += 1;

        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempts >= policy.max_attempts {
                    return Err(anyhow::anyhow!(
                        "Operation failed after {} attempts: {}",
                        attempts,
                        e
                    ));
                }

                eprintln!(
                    "⚠️  Attempt {}/{} failed: {}. Retrying in {:?}...",
                    attempts, policy.max_attempts, e, delay
                );

                sleep(delay).await;

                // Exponential backoff
                delay = Duration::from_secs_f64(
                    (delay.as_secs_f64() * policy.backoff_multiplier).min(policy.max_delay.as_secs_f64())
                );
            }
        }
    }
}

/// Categorized error types for better handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Network-related errors (retryable)
    Network,

    /// File system errors (may be retryable)
    FileSystem,

    /// Configuration errors (not retryable)
    Configuration,

    /// Validation errors (not retryable)
    Validation,

    /// Dependency errors (not retryable)
    Dependency,

    /// Timeout errors (may be retryable)
    Timeout,

    /// Unknown errors
    Unknown,
}

impl ErrorCategory {
    /// Check if this error category is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ErrorCategory::Network | ErrorCategory::FileSystem | ErrorCategory::Timeout
        )
    }
}

/// Categorize an error
pub fn categorize_error(error: &anyhow::Error) -> ErrorCategory {
    let error_str = error.to_string().to_lowercase();

    if error_str.contains("network")
        || error_str.contains("connection")
        || error_str.contains("timeout")
        || error_str.contains("dns")
    {
        ErrorCategory::Network
    } else if error_str.contains("file")
        || error_str.contains("directory")
        || error_str.contains("permission")
        || error_str.contains("io")
    {
        ErrorCategory::FileSystem
    } else if error_str.contains("config") || error_str.contains("invalid") {
        ErrorCategory::Configuration
    } else if error_str.contains("dependency") || error_str.contains("version") {
        ErrorCategory::Dependency
    } else if error_str.contains("timeout") {
        ErrorCategory::Timeout
    } else {
        ErrorCategory::Unknown
    }
}

/// Enhanced error with context and suggestions
#[derive(Debug)]
pub struct EnhancedError {
    pub error: anyhow::Error,
    pub category: ErrorCategory,
    pub context: Vec<String>,
    pub suggestions: Vec<String>,
}

impl EnhancedError {
    /// Create an enhanced error
    pub fn new(error: anyhow::Error) -> Self {
        let category = categorize_error(&error);
        let (context, suggestions) = generate_context_and_suggestions(&category, &error);

        Self {
            error,
            category,
            context,
            suggestions,
        }
    }

    /// Display the error with all context
    pub fn display(&self) -> String {
        let mut output = format!("❌ Error: {}\n", self.error);

        if !self.context.is_empty() {
            output.push_str("\n📋 Context:\n");
            for ctx in &self.context {
                output.push_str(&format!("   • {}\n", ctx));
            }
        }

        if !self.suggestions.is_empty() {
            output.push_str("\n💡 Suggestions:\n");
            for suggestion in &self.suggestions {
                output.push_str(&format!("   • {}\n", suggestion));
            }
        }

        output
    }
}

/// Generate helpful context and suggestions based on error category
fn generate_context_and_suggestions(
    category: &ErrorCategory,
    error: &anyhow::Error,
) -> (Vec<String>, Vec<String>) {
    let mut context = Vec::new();
    let mut suggestions = Vec::new();

    match category {
        ErrorCategory::Network => {
            context.push("Network operation failed".to_string());
            suggestions.push("Check your internet connection".to_string());
            suggestions.push("Verify firewall settings".to_string());
            suggestions.push("Try again in a few moments".to_string());
        }
        ErrorCategory::FileSystem => {
            context.push("File system operation failed".to_string());
            suggestions.push("Check file permissions".to_string());
            suggestions.push("Verify the path exists".to_string());
            suggestions.push("Ensure sufficient disk space".to_string());
        }
        ErrorCategory::Configuration => {
            context.push("Configuration error detected".to_string());
            suggestions.push("Review your configuration file".to_string());
            suggestions.push("Check environment variables".to_string());
            suggestions.push("Refer to documentation for valid options".to_string());
        }
        ErrorCategory::Dependency => {
            context.push("Dependency resolution failed".to_string());
            suggestions.push("Check tool dependencies".to_string());
            suggestions.push("Verify version compatibility".to_string());
            suggestions.push("Run 'forge update' to sync dependencies".to_string());
        }
        ErrorCategory::Timeout => {
            context.push("Operation timed out".to_string());
            suggestions.push("The operation may need more time".to_string());
            suggestions.push("Try increasing timeout settings".to_string());
            suggestions.push("Check system resources".to_string());
        }
        ErrorCategory::Validation => {
            context.push("Validation error".to_string());
            suggestions.push("Review input data".to_string());
            suggestions.push("Check for required fields".to_string());
        }
        ErrorCategory::Unknown => {
            context.push(format!("Unexpected error: {}", error));
            suggestions.push("Check logs for more details".to_string());
            suggestions.push("Report this issue if it persists".to_string());
        }
    }

    (context, suggestions)
}

/// Result type with enhanced error
pub type EnhancedResult<T> = Result<T, EnhancedError>;

/// Convert regular Result to EnhancedResult
pub trait ToEnhanced<T> {
    fn enhance(self) -> EnhancedResult<T>;
}

impl<T, E: Into<anyhow::Error>> ToEnhanced<T> for Result<T, E> {
    fn enhance(self) -> EnhancedResult<T> {
        self.map_err(|e| EnhancedError::new(e.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        let net_err = anyhow::anyhow!("Network connection failed");
        assert_eq!(categorize_error(&net_err), ErrorCategory::Network);

        let fs_err = anyhow::anyhow!("File not found");
        assert_eq!(categorize_error(&fs_err), ErrorCategory::FileSystem);

        let config_err = anyhow::anyhow!("Invalid config value");
        assert_eq!(categorize_error(&config_err), ErrorCategory::Configuration);
    }

    #[test]
    fn test_retryable() {
        assert!(ErrorCategory::Network.is_retryable());
        assert!(!ErrorCategory::Configuration.is_retryable());
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);

        let no_retry = RetryPolicy::no_retry();
        assert_eq!(no_retry.max_attempts, 1);
    }
}
