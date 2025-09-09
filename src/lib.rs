//! Arkaft ADK Agents - Utilities for detecting and working with Google ADK projects
//! 
//! This library provides utilities for:
//! - Detecting ADK projects based on dependencies and configuration
//! - Validating files for processing (size, type, etc.)
//! - Analyzing ADK-specific configuration files
//! 
//! # Examples
//! 
//! ```rust
//! use arkaft_adk_agents::detection::{AdkProjectDetector, FileValidator, AdkConfigDetector};
//! use std::path::Path;
//! 
//! // Create detectors
//! let detector = AdkProjectDetector::default();
//! let validator = FileValidator::for_code_review();
//! let config_detector = AdkConfigDetector::default();
//! 
//! // These would work with actual project directories:
//! // let project_info = detector.detect_adk_project("./my-project").unwrap();
//! // let is_suitable = validator.is_suitable_for_review("src/main.rs").unwrap();
//! // let config_info = config_detector.detect_adk_config("./my-project").unwrap();
//! ```

pub mod detection;

pub use detection::*;

/// Version of the arkaft-adk-agents library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if the library is compatible with a given ADK version
pub fn is_compatible_adk_version(adk_version: &str) -> bool {
    // For now, we support all versions, but this could be extended
    // to check for specific version compatibility requirements
    !adk_version.is_empty()
}

/// Get the default configuration for ADK project detection
pub fn get_default_detection_config() -> DetectionConfig {
    DetectionConfig::default()
}

/// Configuration for ADK project detection
#[derive(Debug, Clone)]
pub struct DetectionConfig {
    /// Maximum file size to process (in bytes)
    pub max_file_size: u64,
    /// Minimum file size to process (in bytes)  
    pub min_file_size: u64,
    /// Whether to include build artifacts in detection
    pub include_build_artifacts: bool,
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
    /// Maximum directory depth to search
    pub max_depth: usize,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            max_file_size: 50 * 1024 * 1024, // 50MB
            min_file_size: 1,
            include_build_artifacts: false,
            follow_symlinks: false,
            max_depth: 3,
        }
    }
}

impl DetectionConfig {
    /// Create a configuration optimized for code review
    pub fn for_code_review() -> Self {
        Self {
            max_file_size: 1024 * 1024, // 1MB
            min_file_size: 10,
            include_build_artifacts: false,
            follow_symlinks: false,
            max_depth: 5,
        }
    }

    /// Create a configuration for comprehensive project analysis
    pub fn for_project_analysis() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            min_file_size: 1,
            include_build_artifacts: true,
            follow_symlinks: true,
            max_depth: 10,
        }
    }
}