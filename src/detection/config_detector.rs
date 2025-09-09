use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

/// ADK-specific configuration detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdkConfigInfo {
    pub config_files: Vec<ConfigFileInfo>,
    pub has_adk_config: bool,
    pub adk_version: Option<String>,
    pub google_api_configured: bool,
    pub vertex_ai_configured: bool,
    pub mcp_server_configured: bool,
    pub environment_variables: HashMap<String, String>,
}

/// Information about a detected configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFileInfo {
    pub path: PathBuf,
    pub config_type: ConfigType,
    pub contains_adk_settings: bool,
    pub detected_settings: Vec<String>,
}

/// Types of configuration files relevant to ADK projects
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigType {
    /// Environment configuration (.env files)
    Environment,
    /// Cargo.toml for Rust projects
    CargoToml,
    /// Python requirements.txt
    Requirements,
    /// Python setup.py or pyproject.toml
    PythonBuild,
    /// JSON configuration files
    Json,
    /// YAML configuration files
    Yaml,
    /// TOML configuration files (non-Cargo)
    Toml,
    /// MCP server configuration
    McpConfig,
    /// Unknown configuration type
    Unknown,
}

/// Configuration detector for ADK-specific settings and markers
pub struct AdkConfigDetector {
    /// Known ADK environment variables
    adk_env_vars: Vec<String>,
    /// Known ADK configuration keys
    adk_config_keys: Vec<String>,
    /// Known Google API configuration patterns
    google_api_patterns: Vec<String>,
    /// Known Vertex AI configuration patterns
    vertex_ai_patterns: Vec<String>,
}

impl Default for AdkConfigDetector {
    fn default() -> Self {
        Self {
            adk_env_vars: vec![
                "GOOGLE_API_KEY".to_string(),
                "GOOGLE_APPLICATION_CREDENTIALS".to_string(),
                "GOOGLE_GENAI_USE_VERTEXAI".to_string(),
                "VERTEXAI_PROJECT".to_string(),
                "VERTEXAI_LOCATION".to_string(),
                "ADK_VERSION".to_string(),
                "ADK_DOCS_VERSION".to_string(),
                "RUST_LOG".to_string(), // Common in ADK Rust projects
            ],
            adk_config_keys: vec![
                "google-adk".to_string(),
                "google-genai".to_string(),
                "vertexai".to_string(),
                "adk-core".to_string(),
                "adk-runtime".to_string(),
                "rmcp".to_string(), // MCP servers
                "arkaft-mcp-google-adk".to_string(),
            ],
            google_api_patterns: vec![
                "GOOGLE_API_KEY".to_string(),
                "google_api_key".to_string(),
                "googleApiKey".to_string(),
                "GOOGLE_APPLICATION_CREDENTIALS".to_string(),
                "google-cloud".to_string(),
            ],
            vertex_ai_patterns: vec![
                "VERTEXAI".to_string(),
                "vertex_ai".to_string(),
                "vertexAi".to_string(),
                "GOOGLE_GENAI_USE_VERTEXAI".to_string(),
                "vertex-ai".to_string(),
            ],
        }
    }
}

impl AdkConfigDetector {
    /// Detect ADK configuration in a project directory
    pub fn detect_adk_config<P: AsRef<Path>>(&self, project_path: P) -> Result<AdkConfigInfo> {
        let project_path = project_path.as_ref();
        let mut config_info = AdkConfigInfo {
            config_files: Vec::new(),
            has_adk_config: false,
            adk_version: None,
            google_api_configured: false,
            vertex_ai_configured: false,
            mcp_server_configured: false,
            environment_variables: HashMap::new(),
        };

        // Scan for configuration files
        let config_files = self.find_config_files(project_path)?;
        
        for config_file in config_files {
            let file_info = self.analyze_config_file(&config_file)?;
            
            // Update overall configuration status
            if file_info.contains_adk_settings {
                config_info.has_adk_config = true;
            }

            // Extract specific configuration details
            self.extract_config_details(&file_info, &mut config_info)?;
            
            config_info.config_files.push(file_info);
        }

        Ok(config_info)
    }

    /// Find all configuration files in a project directory
    fn find_config_files<P: AsRef<Path>>(&self, project_path: P) -> Result<Vec<PathBuf>> {
        let project_path = project_path.as_ref();
        let mut config_files = Vec::new();

        // Known configuration file patterns
        let config_patterns = [
            // Environment files
            ".env",
            ".env.template",
            ".env.local",
            ".env.production",
            ".env.development",
            // Build files
            "Cargo.toml",
            "requirements.txt",
            "setup.py",
            "pyproject.toml",
            // Configuration files
            "config.json",
            "config.yaml",
            "config.yml",
            "config.toml",
            "adk.toml",
            "adk-config.json",
            "vertex-config.json",
            "google-cloud-config.json",
            // MCP configuration
            "mcp.json",
            ".kiro/settings/mcp.json",
        ];

        for pattern in &config_patterns {
            let config_path = project_path.join(pattern);
            if config_path.exists() && config_path.is_file() {
                config_files.push(config_path);
            }
        }

        // Also search in common subdirectories
        let subdirs = ["src", "config", ".kiro/settings"];
        for subdir in &subdirs {
            let subdir_path = project_path.join(subdir);
            if subdir_path.exists() && subdir_path.is_dir() {
                if let Ok(entries) = fs::read_dir(&subdir_path) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                    if self.is_config_file(filename) {
                                        config_files.push(path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(config_files)
    }

    /// Check if a filename indicates a configuration file
    fn is_config_file(&self, filename: &str) -> bool {
        let config_extensions = ["json", "yaml", "yml", "toml", "env"];
        let config_names = ["config", "settings", "adk", "vertex", "google"];

        // Check by extension
        if let Some(ext) = filename.split('.').last() {
            if config_extensions.contains(&ext) {
                return true;
            }
        }

        // Check by name patterns
        let filename_lower = filename.to_lowercase();
        for name in &config_names {
            if filename_lower.contains(name) {
                return true;
            }
        }

        false
    }

    /// Analyze a configuration file for ADK-related settings
    fn analyze_config_file<P: AsRef<Path>>(&self, config_path: P) -> Result<ConfigFileInfo> {
        let config_path = config_path.as_ref();
        let config_type = self.determine_config_type(config_path);
        
        let content = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        let mut detected_settings = Vec::new();
        let mut contains_adk_settings = false;

        // Check for ADK environment variables
        for env_var in &self.adk_env_vars {
            if content.contains(env_var) {
                detected_settings.push(format!("env:{}", env_var));
                contains_adk_settings = true;
            }
        }

        // Check for ADK configuration keys
        for config_key in &self.adk_config_keys {
            if content.contains(config_key) {
                detected_settings.push(format!("key:{}", config_key));
                contains_adk_settings = true;
            }
        }

        // Check for Google API patterns
        for pattern in &self.google_api_patterns {
            if content.contains(pattern) {
                detected_settings.push(format!("google:{}", pattern));
                contains_adk_settings = true;
            }
        }

        // Check for Vertex AI patterns
        for pattern in &self.vertex_ai_patterns {
            if content.contains(pattern) {
                detected_settings.push(format!("vertex:{}", pattern));
                contains_adk_settings = true;
            }
        }

        Ok(ConfigFileInfo {
            path: config_path.to_path_buf(),
            config_type,
            contains_adk_settings,
            detected_settings,
        })
    }

    /// Determine the type of configuration file
    fn determine_config_type<P: AsRef<Path>>(&self, config_path: P) -> ConfigType {
        let config_path = config_path.as_ref();
        
        if let Some(filename) = config_path.file_name().and_then(|n| n.to_str()) {
            match filename {
                "Cargo.toml" => return ConfigType::CargoToml,
                "requirements.txt" => return ConfigType::Requirements,
                "setup.py" | "pyproject.toml" => return ConfigType::PythonBuild,
                "mcp.json" => return ConfigType::McpConfig,
                _ => {}
            }

            if filename.starts_with(".env") {
                return ConfigType::Environment;
            }
        }

        // Check by extension
        if let Some(extension) = config_path.extension().and_then(|ext| ext.to_str()) {
            match extension.to_lowercase().as_str() {
                "json" => ConfigType::Json,
                "yaml" | "yml" => ConfigType::Yaml,
                "toml" => ConfigType::Toml,
                _ => ConfigType::Unknown,
            }
        } else {
            ConfigType::Unknown
        }
    }

    /// Extract specific configuration details from a config file
    fn extract_config_details(&self, file_info: &ConfigFileInfo, config_info: &mut AdkConfigInfo) -> Result<()> {
        if !file_info.contains_adk_settings {
            return Ok(());
        }

        let content = fs::read_to_string(&file_info.path)?;

        // Extract ADK version
        if config_info.adk_version.is_none() {
            config_info.adk_version = self.extract_adk_version(&content);
        }

        // Check for Google API configuration
        for pattern in &self.google_api_patterns {
            if content.contains(pattern) {
                config_info.google_api_configured = true;
                break;
            }
        }

        // Check for Vertex AI configuration
        for pattern in &self.vertex_ai_patterns {
            if content.contains(pattern) {
                config_info.vertex_ai_configured = true;
                break;
            }
        }

        // Check for MCP server configuration
        if content.contains("rmcp") || content.contains("arkaft-mcp-google-adk") || content.contains("mcpServers") {
            config_info.mcp_server_configured = true;
        }

        // Extract environment variables from .env files
        if file_info.config_type == ConfigType::Environment {
            self.extract_env_variables(&content, &mut config_info.environment_variables);
        }

        Ok(())
    }

    /// Extract ADK version from configuration content
    fn extract_adk_version(&self, content: &str) -> Option<String> {
        for line in content.lines() {
            // Simple pattern matching for version extraction
            if line.contains("google-adk") && line.contains("version") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        let version = &line[start + 1..start + 1 + end];
                        if !version.is_empty() && version.chars().next().unwrap().is_numeric() {
                            return Some(version.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract environment variables from .env file content
    fn extract_env_variables(&self, content: &str, env_vars: &mut HashMap<String, String>) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim().to_string();
                
                // Only store ADK-related environment variables
                if self.adk_env_vars.contains(&key) {
                    env_vars.insert(key, value);
                }
            }
        }
    }

    /// Check if a project has proper ADK configuration
    pub fn validate_adk_config(&self, config_info: &AdkConfigInfo) -> Vec<String> {
        let mut issues = Vec::new();

        if !config_info.has_adk_config {
            issues.push("No ADK configuration detected".to_string());
            return issues;
        }

        // Check for required configuration
        if !config_info.google_api_configured && !config_info.vertex_ai_configured {
            issues.push("Neither Google API nor Vertex AI is configured".to_string());
        }

        // Check for environment file
        let has_env_file = config_info.config_files.iter()
            .any(|f| f.config_type == ConfigType::Environment);
        
        if !has_env_file {
            issues.push("No .env file found for environment configuration".to_string());
        }

        // Check for missing API key configuration
        if config_info.google_api_configured {
            let has_api_key = config_info.environment_variables.contains_key("GOOGLE_API_KEY");
            if !has_api_key {
                issues.push("GOOGLE_API_KEY not found in environment variables".to_string());
            }
        }

        issues
    }

    /// Get configuration recommendations for ADK projects
    pub fn get_config_recommendations(&self, config_info: &AdkConfigInfo) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !config_info.has_adk_config {
            recommendations.push("Add ADK dependencies to your project configuration".to_string());
            recommendations.push("Create a .env file for API key configuration".to_string());
            return recommendations;
        }

        // Recommend MCP server setup if not configured
        if !config_info.mcp_server_configured {
            recommendations.push("Consider setting up arkaft-mcp-google-adk MCP server for enhanced ADK support".to_string());
        }

        // Recommend Vertex AI for production
        if config_info.google_api_configured && !config_info.vertex_ai_configured {
            recommendations.push("Consider using Vertex AI for production deployments".to_string());
        }

        // Recommend version pinning
        if config_info.adk_version.is_none() {
            recommendations.push("Pin ADK dependency versions for reproducible builds".to_string());
        }

        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_env_config() {
        let temp_dir = TempDir::new().unwrap();
        let env_content = r#"
GOOGLE_API_KEY=your_api_key_here
GOOGLE_GENAI_USE_VERTEXAI=FALSE
RUST_LOG=info
"#;
        fs::write(temp_dir.path().join(".env"), env_content).unwrap();

        let detector = AdkConfigDetector::default();
        let result = detector.detect_adk_config(temp_dir.path()).unwrap();

        assert!(result.has_adk_config);
        assert!(result.google_api_configured);
        assert_eq!(result.config_files.len(), 1);
        assert_eq!(result.config_files[0].config_type, ConfigType::Environment);
        assert!(result.environment_variables.contains_key("GOOGLE_API_KEY"));
    }

    #[test]
    fn test_detect_cargo_adk_config() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_content = r#"
[package]
name = "adk-project"
version = "0.1.0"

[dependencies]
google-adk = { version = "1.0.0" }
tokio = "1.0"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_content).unwrap();

        let detector = AdkConfigDetector::default();
        let result = detector.detect_adk_config(temp_dir.path()).unwrap();

        assert!(result.has_adk_config);
        assert_eq!(result.adk_version, Some("1.0.0".to_string()));
        assert_eq!(result.config_files.len(), 1);
        assert_eq!(result.config_files[0].config_type, ConfigType::CargoToml);
    }

    #[test]
    fn test_detect_mcp_config() {
        let temp_dir = TempDir::new().unwrap();
        let kiro_dir = temp_dir.path().join(".kiro/settings");
        fs::create_dir_all(&kiro_dir).unwrap();
        
        let mcp_content = r#"
{
  "mcpServers": {
    "arkaft-google-adk": {
      "command": "./arkaft-mcp-google-adk",
      "args": []
    }
  }
}
"#;
        fs::write(kiro_dir.join("mcp.json"), mcp_content).unwrap();

        let detector = AdkConfigDetector::default();
        let result = detector.detect_adk_config(temp_dir.path()).unwrap();

        assert!(result.has_adk_config);
        assert!(result.mcp_server_configured);
    }

    #[test]
    fn test_validate_adk_config() {
        let mut config_info = AdkConfigInfo {
            config_files: vec![],
            has_adk_config: true,
            adk_version: Some("1.0.0".to_string()),
            google_api_configured: false,
            vertex_ai_configured: false,
            mcp_server_configured: false,
            environment_variables: HashMap::new(),
        };

        let detector = AdkConfigDetector::default();
        let issues = detector.validate_adk_config(&config_info);

        assert!(!issues.is_empty());
        assert!(issues.iter().any(|issue| issue.contains("Neither Google API nor Vertex AI")));

        // Fix the configuration
        config_info.google_api_configured = true;
        config_info.environment_variables.insert("GOOGLE_API_KEY".to_string(), "test_key".to_string());
        
        let issues = detector.validate_adk_config(&config_info);
        // Should have fewer issues now
        assert!(!issues.iter().any(|issue| issue.contains("Neither Google API nor Vertex AI")));
    }

    #[test]
    fn test_config_recommendations() {
        let config_info = AdkConfigInfo {
            config_files: vec![],
            has_adk_config: false,
            adk_version: None,
            google_api_configured: false,
            vertex_ai_configured: false,
            mcp_server_configured: false,
            environment_variables: HashMap::new(),
        };

        let detector = AdkConfigDetector::default();
        let recommendations = detector.get_config_recommendations(&config_info);

        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|rec| rec.contains("Add ADK dependencies")));
        assert!(recommendations.iter().any(|rec| rec.contains("Create a .env file")));
    }

    #[test]
    fn test_extract_adk_version() {
        let detector = AdkConfigDetector::default();
        
        let cargo_content = r#"google-adk = { version = "1.2.3" }"#;
        let version = detector.extract_adk_version(cargo_content);
        assert_eq!(version, Some("1.2.3".to_string()));

        let no_version_content = "tokio = \"1.0\"";
        let version = detector.extract_adk_version(no_version_content);
        assert_eq!(version, None);
    }
}