//! Comprehensive unit tests for ADK project detection utilities
//!
//! This module provides thorough unit tests for all project detection
//! functionality including Cargo.toml parsing, dependency detection,
//! and project type classification.

#[cfg(test)]
mod tests {
    use crate::detection::{
        project_detector::{AdkProjectDetector, AdkProjectType},
        file_validator::FileValidator,
    };
    use std::fs;
    use tempfile::TempDir;

    /// Helper function to create a temporary test project
    fn create_test_project() -> TempDir {
        TempDir::new().expect("Failed to create temporary directory")
    }

    /// Helper function to create a Cargo.toml with specified dependencies
    fn create_cargo_toml(dir: &TempDir, dependencies: &[(&str, &str)]) {
        let cargo_path = dir.path().join("Cargo.toml");
        let mut content = String::from(
            r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
        );

        for (name, version) in dependencies {
            content.push_str(&format!("{} = \"{}\"\n", name, version));
        }

        fs::write(&cargo_path, content).expect("Failed to write Cargo.toml");
    }

    /// Helper function to create a requirements.txt with specified dependencies
    fn create_requirements_txt(dir: &TempDir, dependencies: &[&str]) {
        let req_path = dir.path().join("requirements.txt");
        let content = dependencies.join("\n");
        fs::write(&req_path, content).expect("Failed to write requirements.txt");
    }

    /// Helper function to create a basic Rust project structure
    fn create_rust_project_structure(dir: &TempDir) {
        let src_dir = dir.path().join("src");
        fs::create_dir(&src_dir).expect("Failed to create src directory");
        
        let main_rs = src_dir.join("main.rs");
        fs::write(main_rs, "fn main() { println!(\"Hello, ADK!\"); }")
            .expect("Failed to write main.rs");
        
        let lib_rs = src_dir.join("lib.rs");
        fs::write(lib_rs, "pub mod utils;").expect("Failed to write lib.rs");
    }

    #[test]
    fn test_detect_rust_adk_project() {
        let temp_dir = create_test_project();
        
        // Create Cargo.toml with ADK dependencies
        create_cargo_toml(&temp_dir, &[
            ("google-adk", "0.1.0"),
            ("tokio", "1.0"),
            ("serde", "1.0"),
        ]);
        
        create_rust_project_structure(&temp_dir);
        
        let detector = AdkProjectDetector::default();
        let result = detector.detect_adk_project(temp_dir.path())
            .expect("Failed to detect project");
        
        assert_eq!(result.project_type, AdkProjectType::RustAdk);
        assert!(result.has_cargo_toml);
        assert!(result.has_adk_dependencies);
    }

    #[test]
    fn test_detect_python_adk_project() {
        let temp_dir = create_test_project();
        
        // Create requirements.txt with ADK dependencies
        create_requirements_txt(&temp_dir, &[
            "google-adk-agents==0.1.0",
            "google-cloud-adk==1.0.0",
            "asyncio",
        ]);
        
        // Create Python project structure
        let init_py = temp_dir.path().join("__init__.py");
        fs::write(init_py, "").expect("Failed to write __init__.py");
        
        let main_py = temp_dir.path().join("main.py");
        fs::write(main_py, "import google.adk.agents").expect("Failed to write main.py");
        
        let detector = AdkProjectDetector::default();
        let result = detector.detect_adk_project(temp_dir.path())
            .expect("Failed to detect project");
        
        assert_eq!(result.project_type, AdkProjectType::PythonAdk);
        assert!(result.has_requirements_txt);
        assert!(result.has_adk_dependencies);
    }

    #[test]
    fn test_detect_non_adk_project() {
        let temp_dir = create_test_project();
        
        // Create regular Rust project without ADK dependencies
        create_cargo_toml(&temp_dir, &[
            ("serde", "1.0"),
            ("tokio", "1.0"),
            ("clap", "4.0"),
        ]);
        
        create_rust_project_structure(&temp_dir);
        
        let detector = AdkProjectDetector::default();
        let result = detector.detect_adk_project(temp_dir.path())
            .expect("Failed to detect project");
        
        assert_eq!(result.project_type, AdkProjectType::None);
        assert!(result.has_cargo_toml);
        assert!(!result.has_adk_dependencies);
    }

    #[test]
    fn test_file_validator_basic() {
        let temp_dir = create_test_project();
        
        // Create files of different sizes
        let small_file = temp_dir.path().join("small.rs");
        fs::write(&small_file, "fn main() {}").expect("Failed to write small file");
        
        let large_file = temp_dir.path().join("large.rs");
        let large_content = format!("// {}", "x".repeat(100000)); // ~100KB
        fs::write(&large_file, large_content).expect("Failed to write large file");
        
        let validator = FileValidator::new(50 * 1024, 0); // 50KB limit, 0 min
        
        let small_result = validator.validate_file(&small_file)
            .expect("Failed to check small file");
        assert!(small_result.is_valid);
        
        let large_result = validator.validate_file(&large_file)
            .expect("Failed to check large file");
        assert!(!large_result.is_valid);
    }

    #[test]
    fn test_adk_dependency_detection() {
        let temp_dir = create_test_project();
        
        // Test various ADK dependency patterns
        let test_cases = vec![
            (vec![("google-adk", "0.1.0")], true),
            (vec![("google-cloud-adk", "1.0.0")], true),
            (vec![("adk-core", "0.5.0")], true),
            (vec![("serde", "1.0"), ("tokio", "1.0")], false),
        ];
        
        let detector = AdkProjectDetector::default();
        
        for (dependencies, should_detect_adk) in test_cases {
            create_cargo_toml(&temp_dir, &dependencies);
            let result = detector.detect_adk_project(temp_dir.path())
                .expect("Failed to detect project");
            
            assert_eq!(result.has_adk_dependencies, should_detect_adk, 
                "Failed for dependencies: {:?}", dependencies);
        }
    }

    #[test]
    fn test_project_size_estimation() {
        let temp_dir = create_test_project();
        
        create_cargo_toml(&temp_dir, &[("google-adk", "0.1.0")]);
        create_rust_project_structure(&temp_dir);
        
        // Create additional files to test size calculation
        let large_file = temp_dir.path().join("large_file.txt");
        let large_content = "x".repeat(10000); // 10KB file
        fs::write(large_file, large_content).expect("Failed to write large file");
        
        let detector = AdkProjectDetector::default();
        let result = detector.detect_adk_project(temp_dir.path())
            .expect("Failed to detect project");
        
        assert!(result.estimated_size > 0);
        assert!(result.estimated_size >= 10000); // At least the large file size
    }

    #[test]
    fn test_mixed_project_detection() {
        let temp_dir = create_test_project();
        
        // Create both Rust and Python components
        create_cargo_toml(&temp_dir, &[
            ("google-adk", "0.1.0"),
            ("tokio", "1.0"),
        ]);
        
        create_requirements_txt(&temp_dir, &[
            "google-adk-agents==0.1.0",
        ]);
        
        create_rust_project_structure(&temp_dir);
        
        // Create Python components
        let init_py = temp_dir.path().join("__init__.py");
        fs::write(init_py, "").expect("Failed to write __init__.py");
        
        let detector = AdkProjectDetector::default();
        let result = detector.detect_adk_project(temp_dir.path())
            .expect("Failed to detect project");
        
        assert_eq!(result.project_type, AdkProjectType::Mixed);
        assert!(result.has_cargo_toml);
        assert!(result.has_requirements_txt);
        assert!(result.has_adk_dependencies);
    }

    #[test]
    fn test_error_handling_invalid_cargo_toml() {
        let temp_dir = create_test_project();
        
        // Create invalid Cargo.toml
        let cargo_path = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_path, "invalid toml content [[[")
            .expect("Failed to write invalid Cargo.toml");
        
        let detector = AdkProjectDetector::default();
        let result = detector.detect_adk_project(temp_dir.path());
        
        // Should handle error gracefully
        assert!(result.is_err() || 
                result.unwrap().project_type == AdkProjectType::None);
    }

    #[test]
    fn test_performance_with_large_project() {
        let temp_dir = create_test_project();
        
        // Create a project with many files
        create_cargo_toml(&temp_dir, &[("google-adk", "0.1.0")]);
        
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).expect("Failed to create src directory");
        
        // Create multiple Rust files
        for i in 0..50 {  // Reduced from 100 to 50 for faster testing
            let file_path = src_dir.join(format!("module_{}.rs", i));
            fs::write(&file_path, format!("pub fn function_{}() {{}}", i))
                .expect("Failed to write module file");
        }
        
        let detector = AdkProjectDetector::default();
        let start = std::time::Instant::now();
        let result = detector.detect_adk_project(temp_dir.path())
            .expect("Failed to detect large project");
        let duration = start.elapsed();
        
        assert_eq!(result.project_type, AdkProjectType::RustAdk);
        assert!(duration.as_secs() < 5); // Should complete within 5 seconds
    }
}