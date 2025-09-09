use super::*;
use std::fs;
use tempfile::TempDir;

/// Integration tests for ADK project detection utilities
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_adk_project_detection() {
        // Create a temporary directory structure that mimics a real ADK project
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create Cargo.toml with ADK dependencies
        let cargo_content = r#"
[package]
name = "my-adk-project"
version = "0.1.0"
edition = "2021"

[dependencies]
google-adk = { version = "1.0.0" }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
"#;
        fs::write(project_root.join("Cargo.toml"), cargo_content).unwrap();

        // Create .env file with ADK configuration
        let env_content = r#"
GOOGLE_API_KEY=your_api_key_here
GOOGLE_GENAI_USE_VERTEXAI=FALSE
RUST_LOG=info
ADK_VERSION=1.0.0
"#;
        fs::write(project_root.join(".env"), env_content).unwrap();

        // Create source files
        fs::create_dir_all(project_root.join("src")).unwrap();
        let main_rs_content = r#"
use google_adk::prelude::*;

fn main() {
    println!("Hello, ADK!");
}
"#;
        fs::write(project_root.join("src/main.rs"), main_rs_content).unwrap();

        let lib_rs_content = r#"
pub mod agent;
pub mod tools;

pub use agent::*;
"#;
        fs::write(project_root.join("src/lib.rs"), lib_rs_content).unwrap();

        // Create MCP configuration
        let kiro_dir = project_root.join(".kiro/settings");
        fs::create_dir_all(&kiro_dir).unwrap();
        let mcp_content = r#"
{
  "mcpServers": {
    "arkaft-google-adk": {
      "command": "./arkaft-mcp-google-adk/target/release/arkaft-mcp-google-adk",
      "args": [],
      "disabled": false,
      "autoApprove": ["adk_query", "review_rust_file"]
    }
  }
}
"#;
        fs::write(kiro_dir.join("mcp.json"), mcp_content).unwrap();

        // Test project detection
        let project_detector = AdkProjectDetector::default();
        let project_info = project_detector.detect_adk_project(project_root).unwrap();

        assert_eq!(project_info.project_type, AdkProjectType::RustAdk);
        assert!(project_info.has_cargo_toml);
        assert!(project_info.has_adk_dependencies);
        assert!(project_info.has_adk_config);
        assert_eq!(project_info.adk_version, Some("1.0.0".to_string()));

        // Test file validation
        let file_validator = FileValidator::for_code_review();
        let main_rs_path = project_root.join("src/main.rs");
        let validation_result = file_validator.validate_file(&main_rs_path).unwrap();

        assert!(validation_result.is_valid);
        assert_eq!(validation_result.file_type, FileType::Rust);
        assert!(validation_result.file_size > 0);

        // Test configuration detection
        let config_detector = AdkConfigDetector::default();
        let config_info = config_detector.detect_adk_config(project_root).unwrap();

        assert!(config_info.has_adk_config);
        assert!(config_info.google_api_configured);
        assert!(config_info.mcp_server_configured);
        assert_eq!(config_info.adk_version, Some("1.0.0".to_string()));
        assert!(config_info.environment_variables.contains_key("GOOGLE_API_KEY"));

        // Validate the configuration
        let validation_issues = config_detector.validate_adk_config(&config_info);
        // Should have minimal issues since we set up a proper configuration
        assert!(validation_issues.len() <= 1); // Might have minor issues like missing .env file check
    }

    #[test]
    fn test_python_adk_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create Python ADK project structure
        let requirements_content = r#"
google-adk==1.0.0
google-genai==0.3.0
python-dotenv==1.0.0
"#;
        fs::write(project_root.join("requirements.txt"), requirements_content).unwrap();

        let env_content = r#"
GOOGLE_API_KEY=your_api_key_here
GOOGLE_GENAI_USE_VERTEXAI=TRUE
VERTEXAI_PROJECT=my-project
VERTEXAI_LOCATION=us-central1
"#;
        fs::write(project_root.join(".env"), env_content).unwrap();

        // Create Python source files
        fs::create_dir_all(project_root.join("multi_tool_agent")).unwrap();
        let agent_py_content = r#"
import google_adk
from google.genai import Client

def main():
    print("Hello, ADK!")

if __name__ == "__main__":
    main()
"#;
        fs::write(project_root.join("multi_tool_agent/agent.py"), agent_py_content).unwrap();

        // Test detection
        let project_detector = AdkProjectDetector::default();
        let project_info = project_detector.detect_adk_project(project_root).unwrap();

        assert_eq!(project_info.project_type, AdkProjectType::PythonAdk);
        assert!(project_info.has_requirements_txt);
        assert!(project_info.has_adk_dependencies);
        assert!(project_info.has_adk_config);

        // Test configuration detection
        let config_detector = AdkConfigDetector::default();
        let config_info = config_detector.detect_adk_config(project_root).unwrap();

        assert!(config_info.has_adk_config);
        assert!(config_info.vertex_ai_configured);
        assert!(config_info.environment_variables.contains_key("VERTEXAI_PROJECT"));
    }

    #[test]
    fn test_mcp_server_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create MCP server project (like arkaft-mcp-google-adk)
        let cargo_content = r#"
[package]
name = "arkaft-mcp-google-adk"
version = "0.1.0"
edition = "2021"
description = "A Model Context Protocol server for Google ADK expertise"

[dependencies]
rmcp = { version = "0.6.3", features = ["server", "transport-io"] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
google-adk = "1.0"
"#;
        fs::write(project_root.join("Cargo.toml"), cargo_content).unwrap();

        // Create MCP server source structure
        fs::create_dir_all(project_root.join("src/server")).unwrap();
        fs::create_dir_all(project_root.join("src/expert")).unwrap();
        
        let main_rs_content = r#"
use rmcp::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // MCP server implementation
    Ok(())
}
"#;
        fs::write(project_root.join("src/main.rs"), main_rs_content).unwrap();

        // Test detection
        let project_detector = AdkProjectDetector::default();
        let project_info = project_detector.detect_adk_project(project_root).unwrap();

        assert_eq!(project_info.project_type, AdkProjectType::McpAdkServer);
        assert!(project_info.has_cargo_toml);
        assert!(project_info.has_adk_dependencies);
    }

    #[test]
    fn test_mixed_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create a project with both Rust and Python components
        let cargo_content = r#"
[package]
name = "mixed-adk-project"
version = "0.1.0"

[dependencies]
google-adk = "1.0"
"#;
        fs::write(project_root.join("Cargo.toml"), cargo_content).unwrap();

        let requirements_content = "google-adk==1.0.0\n";
        fs::write(project_root.join("requirements.txt"), requirements_content).unwrap();

        // Test detection
        let project_detector = AdkProjectDetector::default();
        let project_info = project_detector.detect_adk_project(project_root).unwrap();

        assert_eq!(project_info.project_type, AdkProjectType::Mixed);
        assert!(project_info.has_cargo_toml);
        assert!(project_info.has_requirements_txt);
        assert!(project_info.has_adk_dependencies);
    }

    #[test]
    fn test_non_adk_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create a regular Rust project without ADK dependencies
        let cargo_content = r#"
[package]
name = "regular-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = "1.0"
"#;
        fs::write(project_root.join("Cargo.toml"), cargo_content).unwrap();

        let main_rs_content = "fn main() { println!(\"Hello, world!\"); }";
        fs::create_dir_all(project_root.join("src")).unwrap();
        fs::write(project_root.join("src/main.rs"), main_rs_content).unwrap();

        // Test detection
        let project_detector = AdkProjectDetector::default();
        let project_info = project_detector.detect_adk_project(project_root).unwrap();

        assert_eq!(project_info.project_type, AdkProjectType::None);
        assert!(project_info.has_cargo_toml);
        assert!(!project_info.has_adk_dependencies);
        assert!(!project_info.has_adk_config);
    }

    #[test]
    fn test_file_validation_with_various_sizes() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create files of different sizes
        let small_file = temp_dir.path().join("small.rs");
        let medium_file = temp_dir.path().join("medium.rs");
        let large_file = temp_dir.path().join("large.rs");
        
        fs::write(&small_file, "fn main() {}").unwrap();
        fs::write(&medium_file, "x".repeat(1024)).unwrap(); // 1KB
        fs::write(&large_file, "x".repeat(2 * 1024 * 1024)).unwrap(); // 2MB

        let validator = FileValidator::for_code_review(); // 1MB limit
        
        let small_result = validator.validate_file(&small_file).unwrap();
        let medium_result = validator.validate_file(&medium_file).unwrap();
        let large_result = validator.validate_file(&large_file).unwrap();

        assert!(small_result.is_valid);
        assert!(medium_result.is_valid);
        assert!(!large_result.is_valid);
        assert!(large_result.reason.unwrap().contains("too large"));
    }

    #[test]
    fn test_find_multiple_adk_projects() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create multiple ADK projects in subdirectories
        let project1 = root.join("project1");
        let project2 = root.join("project2");
        let non_adk = root.join("non-adk");

        fs::create_dir_all(&project1).unwrap();
        fs::create_dir_all(&project2).unwrap();
        fs::create_dir_all(&non_adk).unwrap();

        // Project 1: Rust ADK
        let cargo1 = r#"
[dependencies]
google-adk = "1.0"
"#;
        fs::write(project1.join("Cargo.toml"), cargo1).unwrap();

        // Project 2: Python ADK
        fs::write(project2.join("requirements.txt"), "google-adk==1.0.0").unwrap();

        // Non-ADK project
        let cargo_non_adk = r#"
[dependencies]
serde = "1.0"
"#;
        fs::write(non_adk.join("Cargo.toml"), cargo_non_adk).unwrap();

        // Find all ADK projects
        let detector = AdkProjectDetector::default();
        let projects = detector.find_adk_projects(root).unwrap();

        assert_eq!(projects.len(), 2);
        
        let rust_project = projects.iter().find(|p| p.project_type == AdkProjectType::RustAdk);
        let python_project = projects.iter().find(|p| p.project_type == AdkProjectType::PythonAdk);
        
        assert!(rust_project.is_some());
        assert!(python_project.is_some());
    }

    #[test]
    fn test_configuration_validation_and_recommendations() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create a project with incomplete configuration
        let cargo_content = r#"
[dependencies]
google-adk = "1.0"
"#;
        fs::write(project_root.join("Cargo.toml"), cargo_content).unwrap();

        // Missing .env file and API configuration
        let config_detector = AdkConfigDetector::default();
        let config_info = config_detector.detect_adk_config(project_root).unwrap();

        // Should detect ADK but have configuration issues
        assert!(config_info.has_adk_config);
        assert!(!config_info.google_api_configured);
        assert!(!config_info.vertex_ai_configured);

        let issues = config_detector.validate_adk_config(&config_info);
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|issue| issue.contains("Neither Google API nor Vertex AI")));

        let recommendations = config_detector.get_config_recommendations(&config_info);
        assert!(!recommendations.is_empty());
        // Should recommend MCP server setup since it's not configured
        assert!(recommendations.iter().any(|rec| rec.contains("MCP server") || rec.contains("arkaft-mcp-google-adk")));
    }
}