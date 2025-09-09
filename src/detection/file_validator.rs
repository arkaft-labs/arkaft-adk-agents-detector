use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

/// File validation result with size and type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileValidationResult {
    pub path: PathBuf,
    pub is_valid: bool,
    pub file_size: u64,
    pub file_type: FileType,
    pub reason: Option<String>,
}

/// Supported file types for ADK development
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileType {
    /// Rust source files
    Rust,
    /// Python source files
    Python,
    /// Configuration files (TOML, JSON, YAML)
    Config,
    /// Documentation files
    Documentation,
    /// Environment files
    Environment,
    /// Build/dependency files
    Build,
    /// Unknown or unsupported file type
    Unknown,
}

/// File validator for ADK projects with size and type constraints
pub struct FileValidator {
    /// Maximum file size in bytes
    max_file_size: u64,
    /// Minimum file size in bytes (to filter out empty files)
    min_file_size: u64,
    /// Allowed file extensions
    allowed_extensions: Vec<String>,
    /// Excluded file patterns
    excluded_patterns: Vec<String>,
}

impl Default for FileValidator {
    fn default() -> Self {
        Self {
            max_file_size: 50 * 1024 * 1024, // 50MB
            min_file_size: 1, // At least 1 byte
            allowed_extensions: vec![
                // Rust files
                "rs".to_string(),
                // Python files
                "py".to_string(),
                "pyi".to_string(),
                // Configuration files
                "toml".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
                // Documentation
                "md".to_string(),
                "rst".to_string(),
                "txt".to_string(),
                // Environment files (no extension, handled separately)
            ],
            excluded_patterns: vec![
                // Build artifacts
                "target/**".to_string(),
                "build/**".to_string(),
                "dist/**".to_string(),
                // Dependencies
                "node_modules/**".to_string(),
                ".venv/**".to_string(),
                "__pycache__/**".to_string(),
                // Version control
                ".git/**".to_string(),
                ".svn/**".to_string(),
                // IDE files
                ".vscode/**".to_string(),
                ".idea/**".to_string(),
                // Temporary files
                "*.tmp".to_string(),
                "*.temp".to_string(),
                "*.log".to_string(),
                "*.bak".to_string(),
            ],
        }
    }
}

impl FileValidator {
    /// Create a new file validator with custom settings
    pub fn new(max_file_size: u64, min_file_size: u64) -> Self {
        Self {
            max_file_size,
            min_file_size,
            ..Default::default()
        }
    }

    /// Create a validator optimized for code review (smaller files)
    pub fn for_code_review() -> Self {
        Self {
            max_file_size: 1024 * 1024, // 1MB for code review
            min_file_size: 10, // At least 10 bytes
            allowed_extensions: vec!["rs".to_string(), "py".to_string()],
            ..Default::default()
        }
    }

    /// Create a validator for configuration files
    pub fn for_config_files() -> Self {
        Self {
            max_file_size: 10 * 1024, // 10KB for config files
            min_file_size: 1,
            allowed_extensions: vec![
                "toml".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
            ],
            ..Default::default()
        }
    }

    /// Validate a single file
    pub fn validate_file<P: AsRef<Path>>(&self, file_path: P) -> Result<FileValidationResult> {
        let file_path = file_path.as_ref();
        let path_buf = file_path.to_path_buf();

        // Check if file exists
        if !file_path.exists() {
            return Ok(FileValidationResult {
                path: path_buf,
                is_valid: false,
                file_size: 0,
                file_type: FileType::Unknown,
                reason: Some("File does not exist".to_string()),
            });
        }

        // Check if it's actually a file (not a directory)
        if !file_path.is_file() {
            return Ok(FileValidationResult {
                path: path_buf,
                is_valid: false,
                file_size: 0,
                file_type: FileType::Unknown,
                reason: Some("Path is not a file".to_string()),
            });
        }

        // Get file metadata
        let metadata = fs::metadata(file_path)
            .with_context(|| format!("Failed to get metadata for {:?}", file_path))?;
        
        let file_size = metadata.len();

        // Determine file type
        let file_type = self.determine_file_type(file_path);

        // Check if file matches excluded patterns
        if self.is_excluded_file(file_path) {
            return Ok(FileValidationResult {
                path: path_buf,
                is_valid: false,
                file_size,
                file_type,
                reason: Some("File matches excluded pattern".to_string()),
            });
        }

        // Check file size constraints
        if file_size < self.min_file_size {
            return Ok(FileValidationResult {
                path: path_buf,
                is_valid: false,
                file_size,
                file_type,
                reason: Some(format!("File too small: {} bytes", file_size)),
            });
        }

        if file_size > self.max_file_size {
            return Ok(FileValidationResult {
                path: path_buf,
                is_valid: false,
                file_size,
                file_type,
                reason: Some(format!("File too large: {} bytes (max: {})", file_size, self.max_file_size)),
            });
        }

        // Check file extension/type
        if !self.is_allowed_file_type(file_path) {
            return Ok(FileValidationResult {
                path: path_buf,
                is_valid: false,
                file_size,
                file_type,
                reason: Some("File type not allowed".to_string()),
            });
        }

        // File is valid
        Ok(FileValidationResult {
            path: path_buf,
            is_valid: true,
            file_size,
            file_type,
            reason: None,
        })
    }

    /// Validate multiple files and return results
    pub fn validate_files<P: AsRef<Path>>(&self, file_paths: &[P]) -> Result<Vec<FileValidationResult>> {
        let mut results = Vec::new();
        
        for file_path in file_paths {
            match self.validate_file(file_path) {
                Ok(result) => results.push(result),
                Err(e) => {
                    // Create an error result for files that couldn't be validated
                    results.push(FileValidationResult {
                        path: file_path.as_ref().to_path_buf(),
                        is_valid: false,
                        file_size: 0,
                        file_type: FileType::Unknown,
                        reason: Some(format!("Validation error: {}", e)),
                    });
                }
            }
        }
        
        Ok(results)
    }

    /// Get all valid files from a list of validation results
    pub fn get_valid_files(results: &[FileValidationResult]) -> Vec<&FileValidationResult> {
        results.iter().filter(|r| r.is_valid).collect()
    }

    /// Get all invalid files from a list of validation results
    pub fn get_invalid_files(results: &[FileValidationResult]) -> Vec<&FileValidationResult> {
        results.iter().filter(|r| !r.is_valid).collect()
    }

    /// Determine the file type based on extension and name
    fn determine_file_type<P: AsRef<Path>>(&self, file_path: P) -> FileType {
        let file_path = file_path.as_ref();
        
        // Check specific filenames first
        if let Some(filename) = file_path.file_name().and_then(|name| name.to_str()) {
            match filename {
                "Cargo.toml" | "Cargo.lock" | "requirements.txt" | "setup.py" | "pyproject.toml" => {
                    return FileType::Build;
                }
                ".env" | ".env.template" | ".env.local" | ".env.production" => {
                    return FileType::Environment;
                }
                "README.md" | "CHANGELOG.md" | "LICENSE" | "CONTRIBUTING.md" => {
                    return FileType::Documentation;
                }
                _ => {}
            }
        }

        // Check by extension
        if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
            match extension.to_lowercase().as_str() {
                "rs" => FileType::Rust,
                "py" | "pyi" => FileType::Python,
                "toml" | "json" | "yaml" | "yml" => FileType::Config,
                "md" | "rst" | "txt" => FileType::Documentation,
                _ => FileType::Unknown,
            }
        } else {
            FileType::Unknown
        }
    }

    /// Check if a file type is allowed
    fn is_allowed_file_type<P: AsRef<Path>>(&self, file_path: P) -> bool {
        let file_path = file_path.as_ref();
        
        // Special handling for files without extensions
        if let Some(filename) = file_path.file_name().and_then(|name| name.to_str()) {
            match filename {
                "Cargo.toml" | "requirements.txt" | "setup.py" | ".env" | ".env.template" => {
                    return true;
                }
                _ => {}
            }
        }

        // Check extension
        if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
            self.allowed_extensions.contains(&extension.to_lowercase())
        } else {
            false
        }
    }

    /// Check if a file matches any excluded patterns
    fn is_excluded_file<P: AsRef<Path>>(&self, file_path: P) -> bool {
        let file_path = file_path.as_ref();
        let path_str = file_path.to_string_lossy();

        for pattern in &self.excluded_patterns {
            if self.matches_pattern(&path_str, pattern) {
                return true;
            }
        }

        false
    }

    /// Simple pattern matching for exclusion patterns
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        if pattern.contains("**") {
            // Handle recursive patterns like "target/**"
            let prefix = pattern.split("**").next().unwrap_or("");
            path.contains(prefix)
        } else if pattern.starts_with("*.") {
            // Handle extension patterns like "*.tmp"
            let extension = &pattern[2..];
            path.ends_with(extension)
        } else {
            // Exact match or contains
            path.contains(pattern)
        }
    }

    /// Get file size in a human-readable format
    pub fn format_file_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// Check if a file is suitable for code review based on size and type
    pub fn is_suitable_for_review<P: AsRef<Path>>(&self, file_path: P) -> Result<bool> {
        let result = self.validate_file(file_path)?;
        
        if !result.is_valid {
            return Ok(false);
        }

        // Additional checks for code review suitability
        match result.file_type {
            FileType::Rust | FileType::Python => {
                // Code files should be reasonably sized for review
                Ok(result.file_size <= 100 * 1024) // 100KB max for review
            }
            _ => Ok(false), // Only review code files
        }
    }

    /// Get statistics about a collection of files
    pub fn get_file_statistics(results: &[FileValidationResult]) -> FileStatistics {
        let mut stats = FileStatistics::default();
        
        for result in results {
            stats.total_files += 1;
            stats.total_size += result.file_size;
            
            if result.is_valid {
                stats.valid_files += 1;
                stats.valid_size += result.file_size;
            } else {
                stats.invalid_files += 1;
            }

            // Count by file type
            match result.file_type {
                FileType::Rust => stats.rust_files += 1,
                FileType::Python => stats.python_files += 1,
                FileType::Config => stats.config_files += 1,
                FileType::Documentation => stats.doc_files += 1,
                FileType::Environment => stats.env_files += 1,
                FileType::Build => stats.build_files += 1,
                FileType::Unknown => stats.unknown_files += 1,
            }
        }

        stats
    }
}

/// Statistics about a collection of files
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileStatistics {
    pub total_files: usize,
    pub valid_files: usize,
    pub invalid_files: usize,
    pub total_size: u64,
    pub valid_size: u64,
    pub rust_files: usize,
    pub python_files: usize,
    pub config_files: usize,
    pub doc_files: usize,
    pub env_files: usize,
    pub build_files: usize,
    pub unknown_files: usize,
}

impl FileStatistics {
    /// Get the percentage of valid files
    pub fn valid_percentage(&self) -> f64 {
        if self.total_files == 0 {
            0.0
        } else {
            (self.valid_files as f64 / self.total_files as f64) * 100.0
        }
    }

    /// Get the average file size
    pub fn average_file_size(&self) -> u64 {
        if self.total_files == 0 {
            0
        } else {
            self.total_size / self.total_files as u64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_rust_file() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file = temp_dir.path().join("main.rs");
        fs::write(&rust_file, "fn main() { println!(\"Hello, world!\"); }").unwrap();

        let validator = FileValidator::default();
        let result = validator.validate_file(&rust_file).unwrap();

        assert!(result.is_valid);
        assert_eq!(result.file_type, FileType::Rust);
        assert!(result.file_size > 0);
        assert!(result.reason.is_none());
    }

    #[test]
    fn test_validate_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let large_file = temp_dir.path().join("large.rs");
        
        // Create a file larger than the default limit
        let large_content = "x".repeat(60 * 1024 * 1024); // 60MB
        fs::write(&large_file, large_content).unwrap();

        let validator = FileValidator::default();
        let result = validator.validate_file(&large_file).unwrap();

        assert!(!result.is_valid);
        assert!(result.reason.is_some());
        assert!(result.reason.unwrap().contains("too large"));
    }

    #[test]
    fn test_validate_excluded_file() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        let excluded_file = target_dir.join("debug").join("main");
        fs::create_dir_all(excluded_file.parent().unwrap()).unwrap();
        fs::write(&excluded_file, "binary content").unwrap();

        let validator = FileValidator::default();
        let result = validator.validate_file(&excluded_file).unwrap();

        assert!(!result.is_valid);
        assert!(result.reason.is_some());
        assert!(result.reason.unwrap().contains("excluded pattern"));
    }

    #[test]
    fn test_file_type_detection() {
        let validator = FileValidator::default();

        assert_eq!(validator.determine_file_type(Path::new("main.rs")), FileType::Rust);
        assert_eq!(validator.determine_file_type(Path::new("script.py")), FileType::Python);
        assert_eq!(validator.determine_file_type(Path::new("config.toml")), FileType::Config);
        assert_eq!(validator.determine_file_type(Path::new("README.md")), FileType::Documentation);
        assert_eq!(validator.determine_file_type(Path::new("Cargo.toml")), FileType::Build);
        assert_eq!(validator.determine_file_type(Path::new(".env")), FileType::Environment);
    }

    #[test]
    fn test_code_review_validator() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file = temp_dir.path().join("small.rs");
        let large_rust_file = temp_dir.path().join("large.rs");
        
        fs::write(&rust_file, "fn main() {}").unwrap();
        fs::write(&large_rust_file, "x".repeat(2 * 1024 * 1024)).unwrap(); // 2MB

        let validator = FileValidator::for_code_review();
        
        assert!(validator.is_suitable_for_review(&rust_file).unwrap());
        assert!(!validator.is_suitable_for_review(&large_rust_file).unwrap());
    }

    #[test]
    fn test_file_statistics() {
        let results = vec![
            FileValidationResult {
                path: PathBuf::from("main.rs"),
                is_valid: true,
                file_size: 1000,
                file_type: FileType::Rust,
                reason: None,
            },
            FileValidationResult {
                path: PathBuf::from("config.toml"),
                is_valid: true,
                file_size: 500,
                file_type: FileType::Config,
                reason: None,
            },
            FileValidationResult {
                path: PathBuf::from("large.py"),
                is_valid: false,
                file_size: 1000000,
                file_type: FileType::Python,
                reason: Some("Too large".to_string()),
            },
        ];

        let stats = FileValidator::get_file_statistics(&results);
        
        assert_eq!(stats.total_files, 3);
        assert_eq!(stats.valid_files, 2);
        assert_eq!(stats.invalid_files, 1);
        assert_eq!(stats.rust_files, 1);
        assert_eq!(stats.config_files, 1);
        assert_eq!(stats.python_files, 1);
        assert_eq!(stats.total_size, 1001500);
        assert_eq!(stats.valid_size, 1500);
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(FileValidator::format_file_size(500), "500 B");
        assert_eq!(FileValidator::format_file_size(1536), "1.5 KB");
        assert_eq!(FileValidator::format_file_size(1048576), "1.0 MB");
        assert_eq!(FileValidator::format_file_size(1073741824), "1.0 GB");
    }
}