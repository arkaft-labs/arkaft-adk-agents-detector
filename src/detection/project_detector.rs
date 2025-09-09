use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents the type of ADK project detected
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AdkProjectType {
    /// Rust-based ADK project using Google ADK Rust libraries
    RustAdk,
    /// Python-based ADK project using Google ADK Python libraries
    PythonAdk,
    /// MCP server project that provides ADK expertise
    McpAdkServer,
    /// Mixed project containing multiple ADK components
    Mixed,
    /// Not an ADK project
    None,
}

/// Configuration and metadata for a detected ADK project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdkProjectInfo {
    pub project_type: AdkProjectType,
    pub root_path: PathBuf,
    pub has_cargo_toml: bool,
    pub has_requirements_txt: bool,
    pub has_adk_dependencies: bool,
    pub has_adk_config: bool,
    pub estimated_size: u64,
    pub adk_version: Option<String>,
}

/// Main project detector for ADK projects
pub struct AdkProjectDetector {
    /// Maximum file size to analyze (in bytes)
    max_file_size: u64,
    /// Known ADK dependency patterns
    adk_rust_dependencies: Vec<String>,
    adk_python_dependencies: Vec<String>,
}

impl Default for AdkProjectDetector {
    fn default() -> Self {
        Self {
            max_file_size: 50 * 1024 * 1024, // 50MB default limit
            adk_rust_dependencies: vec![
                "google-adk".to_string(),
                "google-cloud-adk".to_string(),
                "adk-core".to_string(),
                "adk-runtime".to_string(),
                "google-genai".to_string(),
                "vertexai".to_string(),
                "rmcp".to_string(), // MCP servers often support ADK
            ],
            adk_python_dependencies: vec![
                "google-adk".to_string(),
                "google-cloud-adk".to_string(),
                "google-genai".to_string(),
                "vertexai".to_string(),
                "google-cloud-aiplatform".to_string(),
                "adk-agents".to_string(),
            ],
        }
    }
}

impl AdkProjectDetector {
    /// Create a new detector with custom settings
    pub fn new(max_file_size: u64) -> Self {
        Self {
            max_file_size,
            ..Default::default()
        }
    }

    /// Detect if a directory contains an ADK project
    pub fn detect_adk_project<P: AsRef<Path>>(&self, path: P) -> Result<AdkProjectInfo> {
        let path = path.as_ref();
        let mut project_info = AdkProjectInfo {
            project_type: AdkProjectType::None,
            root_path: path.to_path_buf(),
            has_cargo_toml: false,
            has_requirements_txt: false,
            has_adk_dependencies: false,
            has_adk_config: false,
            estimated_size: 0,
            adk_version: None,
        };

        // Check for Cargo.toml (Rust project)
        let cargo_path = path.join("Cargo.toml");
        if cargo_path.exists() {
            project_info.has_cargo_toml = true;
            if let Ok(cargo_content) = fs::read_to_string(&cargo_path) {
                project_info.has_adk_dependencies =
                    self.check_rust_adk_dependencies(&cargo_content);
                project_info.adk_version = self.extract_adk_version_from_cargo(&cargo_content);
            }
        }

        // Check for requirements.txt or setup.py (Python project)
        let requirements_path = path.join("requirements.txt");
        let setup_py_path = path.join("setup.py");
        if requirements_path.exists() || setup_py_path.exists() {
            project_info.has_requirements_txt = requirements_path.exists();

            if requirements_path.exists() {
                if let Ok(req_content) = fs::read_to_string(&requirements_path) {
                    if self.check_python_adk_dependencies(&req_content) {
                        project_info.has_adk_dependencies = true;
                    }
                }
            }
        }

        // Check for ADK-specific configuration files
        project_info.has_adk_config = self.check_adk_config_files(path)?;

        // Estimate project size
        project_info.estimated_size = self.estimate_project_size(path)?;

        // Determine project type based on findings
        project_info.project_type = self.determine_project_type(&project_info);

        Ok(project_info)
    }

    /// Check if Cargo.toml contains ADK-related dependencies
    fn check_rust_adk_dependencies(&self, cargo_content: &str) -> bool {
        for dep in &self.adk_rust_dependencies {
            if cargo_content.contains(dep) {
                return true;
            }
        }
        false
    }

    /// Check if requirements.txt contains ADK-related dependencies
    fn check_python_adk_dependencies(&self, requirements_content: &str) -> bool {
        for dep in &self.adk_python_dependencies {
            if requirements_content.contains(dep) {
                return true;
            }
        }
        false
    }

    /// Extract ADK version from Cargo.toml if available
    fn extract_adk_version_from_cargo(&self, cargo_content: &str) -> Option<String> {
        // Look for version patterns in ADK dependencies
        for line in cargo_content.lines() {
            if line.contains("google-adk") || line.contains("adk-core") {
                if let Some(version_start) = line.find("version = \"") {
                    let version_start = version_start + 11; // Length of "version = \""
                    if let Some(version_end) = line[version_start..].find('"') {
                        return Some(line[version_start..version_start + version_end].to_string());
                    }
                }
            }
        }
        None
    }

    /// Check for ADK-specific configuration files
    fn check_adk_config_files<P: AsRef<Path>>(&self, path: P) -> Result<bool> {
        let path = path.as_ref();

        // Common ADK configuration file patterns
        let adk_config_files = [
            ".env",
            ".env.template",
            "adk.toml",
            "adk-config.json",
            "vertex-config.json",
            "google-cloud-config.json",
        ];

        for config_file in &adk_config_files {
            let config_path = path.join(config_file);
            if config_path.exists() {
                // Check if the config file contains ADK-related content
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if content.contains("GOOGLE_API_KEY")
                        || content.contains("VERTEXAI")
                        || content.contains("ADK")
                        || content.contains("google-genai")
                    {
                        return Ok(true);
                    }
                }
            }
        }

        // Check for ADK-specific directory structures
        let adk_directories = ["multi_tool_agent", "adk_agents", "src/expert", "src/review"];

        for dir in &adk_directories {
            if path.join(dir).is_dir() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Estimate the total size of the project
    fn estimate_project_size<P: AsRef<Path>>(&self, path: P) -> Result<u64> {
        let path = path.as_ref();
        let mut total_size = 0u64;

        fn visit_dir(dir: &Path, total_size: &mut u64, max_size: u64) -> Result<()> {
            if *total_size > max_size {
                return Ok(()); // Stop if we exceed the limit
            }

            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                // Skip common build/cache directories
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if matches!(
                        name,
                        "target" | "node_modules" | ".git" | "__pycache__" | ".venv"
                    ) {
                        continue;
                    }
                }

                if path.is_dir() {
                    visit_dir(&path, total_size, max_size)?;
                } else if path.is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        *total_size += metadata.len();
                    }
                }
            }
            Ok(())
        }

        visit_dir(path, &mut total_size, self.max_file_size)?;
        Ok(total_size)
    }

    /// Determine the project type based on collected information
    fn determine_project_type(&self, info: &AdkProjectInfo) -> AdkProjectType {
        let has_rust = info.has_cargo_toml;
        let has_python = info.has_requirements_txt;
        let has_adk = info.has_adk_dependencies || info.has_adk_config;

        if !has_adk {
            return AdkProjectType::None;
        }

        match (has_rust, has_python) {
            (true, true) => AdkProjectType::Mixed,
            (true, false) => {
                // Check if it's an MCP server by looking for rmcp dependency
                if info.root_path.join("Cargo.toml").exists() {
                    if let Ok(cargo_content) = fs::read_to_string(info.root_path.join("Cargo.toml"))
                    {
                        if cargo_content.contains("rmcp") || cargo_content.contains("mcp") {
                            return AdkProjectType::McpAdkServer;
                        }
                    }
                }
                AdkProjectType::RustAdk
            }
            (false, true) => AdkProjectType::PythonAdk,
            (false, false) => {
                // Has ADK config but no clear language indicators
                if info.has_adk_config {
                    AdkProjectType::PythonAdk // Default to Python for config-only detection
                } else {
                    AdkProjectType::None
                }
            }
        }
    }

    /// Check if a specific file should be processed based on size and type
    pub fn should_process_file<P: AsRef<Path>>(&self, file_path: P) -> Result<bool> {
        let file_path = file_path.as_ref();

        if !file_path.exists() {
            return Ok(false);
        }

        let metadata = fs::metadata(file_path)
            .with_context(|| format!("Failed to get metadata for {:?}", file_path))?;

        // Check file size
        if metadata.len() > self.max_file_size {
            return Ok(false);
        }

        // Check file extension for relevant types
        if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
            match extension {
                "rs" | "py" | "toml" | "json" | "yaml" | "yml" | "md" => Ok(true),
                _ => Ok(false),
            }
        } else {
            // Files without extensions - check specific names
            if let Some(filename) = file_path.file_name().and_then(|name| name.to_str()) {
                match filename {
                    "Cargo.toml" | "requirements.txt" | "setup.py" | ".env" | ".env.template" => {
                        Ok(true)
                    }
                    _ => Ok(false),
                }
            } else {
                Ok(false)
            }
        }
    }

    /// Get a list of ADK projects in a directory tree
    pub fn find_adk_projects<P: AsRef<Path>>(&self, root_path: P) -> Result<Vec<AdkProjectInfo>> {
        let root_path = root_path.as_ref();
        let mut projects = Vec::new();

        fn search_directory(
            detector: &AdkProjectDetector,
            dir: &Path,
            projects: &mut Vec<AdkProjectInfo>,
            max_depth: usize,
            current_depth: usize,
        ) -> Result<()> {
            if current_depth >= max_depth {
                return Ok(());
            }

            // Check if current directory is an ADK project
            match detector.detect_adk_project(dir) {
                Ok(project_info) => {
                    if project_info.project_type != AdkProjectType::None {
                        projects.push(project_info);
                        return Ok(()); // Don't search subdirectories of detected projects
                    }
                }
                Err(_) => {
                    // Continue searching even if detection fails for this directory
                }
            }

            // Search subdirectories
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_dir() {
                            // Skip common non-project directories
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                if matches!(
                                    name,
                                    "target" | "node_modules" | ".git" | "__pycache__" | ".venv"
                                ) {
                                    continue;
                                }
                            }
                            search_directory(
                                detector,
                                &path,
                                projects,
                                max_depth,
                                current_depth + 1,
                            )?;
                        }
                    }
                }
            }

            Ok(())
        }

        search_directory(self, root_path, &mut projects, 3, 0)?; // Max depth of 3
        Ok(projects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_rust_adk_project() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_content = r#"
[package]
name = "test-adk"
version = "0.1.0"

[dependencies]
google-adk = "1.0"
tokio = "1.0"
"#;

        fs::write(temp_dir.path().join("Cargo.toml"), cargo_content).unwrap();

        let detector = AdkProjectDetector::default();
        let result = detector.detect_adk_project(temp_dir.path()).unwrap();

        assert_eq!(result.project_type, AdkProjectType::RustAdk);
        assert!(result.has_cargo_toml);
        assert!(result.has_adk_dependencies);
    }

    #[test]
    fn test_detect_python_adk_project() {
        let temp_dir = TempDir::new().unwrap();
        let requirements_content = "google-adk==1.0.0\nrequests==2.28.0";

        fs::write(
            temp_dir.path().join("requirements.txt"),
            requirements_content,
        )
        .unwrap();

        let detector = AdkProjectDetector::default();
        let result = detector.detect_adk_project(temp_dir.path()).unwrap();

        assert_eq!(result.project_type, AdkProjectType::PythonAdk);
        assert!(result.has_requirements_txt);
        assert!(result.has_adk_dependencies);
    }

    #[test]
    fn test_detect_mcp_adk_server() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_content = r#"
[package]
name = "arkaft-mcp-google-adk"
version = "0.1.0"

[dependencies]
rmcp = "0.6.3"
google-adk = "1.0"
"#;

        fs::write(temp_dir.path().join("Cargo.toml"), cargo_content).unwrap();

        let detector = AdkProjectDetector::default();
        let result = detector.detect_adk_project(temp_dir.path()).unwrap();

        assert_eq!(result.project_type, AdkProjectType::McpAdkServer);
    }

    #[test]
    fn test_detect_non_adk_project() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_content = r#"
[package]
name = "regular-rust"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = "1.0"
"#;

        fs::write(temp_dir.path().join("Cargo.toml"), cargo_content).unwrap();

        let detector = AdkProjectDetector::default();
        let result = detector.detect_adk_project(temp_dir.path()).unwrap();

        assert_eq!(result.project_type, AdkProjectType::None);
        assert!(result.has_cargo_toml);
        assert!(!result.has_adk_dependencies);
    }

    #[test]
    fn test_file_size_validation() {
        let detector = AdkProjectDetector::new(1024); // 1KB limit

        let temp_dir = TempDir::new().unwrap();
        let small_file = temp_dir.path().join("small.rs");
        let large_file = temp_dir.path().join("large.rs");

        fs::write(&small_file, "fn main() {}").unwrap();
        fs::write(&large_file, "x".repeat(2048)).unwrap(); // 2KB file

        assert!(detector.should_process_file(&small_file).unwrap());
        assert!(!detector.should_process_file(&large_file).unwrap());
    }
}
