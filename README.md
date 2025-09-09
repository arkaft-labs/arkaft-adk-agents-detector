# Arkaft ADK Agents - Project Detection Utilities

This library provides comprehensive utilities for detecting and working with Google ADK projects in the Kiro IDE environment. **Status: ✅ Complete and ready for use**

## Features

### Project Detection

- **ADK Project Detection**: Automatically detect ADK projects based on dependencies and configuration
- **Project Type Classification**: Distinguish between Rust ADK, Python ADK, MCP servers, and mixed projects
- **Dependency Analysis**: Parse Cargo.toml and requirements.txt for ADK-related dependencies
- **Project Structure Recognition**: Identify ADK-specific directory patterns and file structures

### File Validation
- **Size Constraints**: Validate files against configurable size limits
- **Type Filtering**: Support for Rust, Python, configuration, and documentation files
- **Exclusion Patterns**: Skip build artifacts, dependencies, and temporary files
- **Code Review Suitability**: Specialized validation for code review scenarios

### Configuration Detection

- **Environment Variables**: Detect ADK-related environment configuration
- **API Configuration**: Identify Google API and Vertex AI setup
- **MCP Server Detection**: Find MCP server configurations
- **Version Extraction**: Extract ADK version information from various sources

## Usage

### Basic Project Detection

```rust
use arkaft_adk_agents::detection::AdkProjectDetector;

let detector = AdkProjectDetector::default();
let project_info = detector.detect_adk_project("./my-adk-project")?;

match project_info.project_type {
    AdkProjectType::RustAdk => println!("Found Rust ADK project"),
    AdkProjectType::PythonAdk => println!("Found Python ADK project"),
    AdkProjectType::McpAdkServer => println!("Found MCP ADK server"),
    AdkProjectType::Mixed => println!("Found mixed ADK project"),
    AdkProjectType::None => println!("Not an ADK project"),
}
```

### File Validation

```rust
use arkaft_adk_agents::detection::FileValidator;

// For code review (smaller files)
let validator = FileValidator::for_code_review();
let is_suitable = validator.is_suitable_for_review("src/main.rs")?;

// For general validation
let validator = FileValidator::default();
let result = validator.validate_file("config.toml")?;

if result.is_valid {
    println!("File is valid: {} ({})", 
        result.path.display(), 
        FileValidator::format_file_size(result.file_size)
    );
}
```

### Configuration Analysis

```rust
use arkaft_adk_agents::detection::AdkConfigDetector;

let config_detector = AdkConfigDetector::default();
let config_info = config_detector.detect_adk_config("./my-project")?;

if config_info.has_adk_config {
    println!("ADK version: {:?}", config_info.adk_version);
    println!("Google API configured: {}", config_info.google_api_configured);
    println!("Vertex AI configured: {}", config_info.vertex_ai_configured);
    println!("MCP server configured: {}", config_info.mcp_server_configured);
}

// Validate configuration and get recommendations
let issues = config_detector.validate_adk_config(&config_info);
let recommendations = config_detector.get_config_recommendations(&config_info);
```

### Finding Multiple Projects

```rust
use arkaft_adk_agents::detection::AdkProjectDetector;

let detector = AdkProjectDetector::default();
let projects = detector.find_adk_projects("./workspace")?;

for project in projects {
    println!("Found {} project at: {}", 
        project.project_type, 
        project.root_path.display()
    );
}
```

## Project Types

The library can detect the following ADK project types:

- **RustAdk**: Rust projects using Google ADK libraries
- **PythonAdk**: Python projects using Google ADK libraries  
- **McpAdkServer**: MCP servers that provide ADK expertise (like arkaft-mcp-google-adk)
- **Mixed**: Projects containing both Rust and Python ADK components
- **None**: Projects that don't use ADK

## Configuration Detection

The library detects ADK configuration from various sources:

### Environment Files

- `.env`, `.env.template`, `.env.local`, etc.
- `GOOGLE_API_KEY`, `VERTEXAI_PROJECT`, `ADK_VERSION`

### Build Files

- `Cargo.toml` with ADK dependencies
- `requirements.txt` with ADK packages
- `setup.py` and `pyproject.toml`

### Configuration Files

- `adk.toml`, `adk-config.json`
- `vertex-config.json`, `google-cloud-config.json`
- `.kiro/settings/mcp.json` for MCP server setup

## File Types

Supported file types for validation:

- **Rust**: `.rs` files
- **Python**: `.py`, `.pyi` files
- **Config**: `.toml`, `.json`, `.yaml`, `.yml` files
- **Documentation**: `.md`, `.rst`, `.txt` files
- **Environment**: `.env` files and variants
- **Build**: `Cargo.toml`, `requirements.txt`, `setup.py`

## Performance Considerations

- **File Size Limits**: Configurable limits prevent processing of overly large files
- **Exclusion Patterns**: Automatically skip build artifacts and dependencies
- **Depth Limits**: Configurable directory traversal depth
- **Debouncing**: Built-in support for debouncing in hook scenarios

## Testing

Run the comprehensive test suite:

```bash
cargo test
```

The test suite includes:

- Unit tests for each component
- Integration tests with realistic project structures
- Performance tests with various file sizes
- Configuration validation tests

## Requirements

This library is designed to work with:

- Rust 2021 edition
- Google ADK projects (Rust and Python)
- Kiro IDE environment
- MCP (Model Context Protocol) servers

## Dependencies

- `serde` - Serialization support
- `anyhow` - Error handling
- `glob` - Pattern matching
- `tempfile` - Testing utilities (dev dependency)