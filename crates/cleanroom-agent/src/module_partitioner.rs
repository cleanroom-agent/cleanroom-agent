//! Module partitioner — groups source files into logical modules.
//!
//! This module provides functionality for partitioning source files into
//! logical modules based on package manager boundaries and directory structure.
//!
//! # Module Types
//!
//! - [`ModuleType::CargoCrate`]: Rust crate (detected via `Cargo.toml`)
//! - [`ModuleType::NpmPackage`]: npm package (detected via `package.json`)
//! - [`ModuleType::PythonPackage`]: Python package (detected via `__init__.py`)
//! - [`ModuleType::GoModule`]: Go module (detected via `go.mod`)
//! - [`ModuleType::Directory`]: Directory-based module (no package manager)
//!
//! # Usage
//!
//! ```no_run
//! use cleanroom_agent::module_partitioner::{partition_files, PartitionConfig};
//! use cleanroom_agent::repo_scanner::scan_repository;
//!
//! let files = scan_repository(&Default::default());
//! let config = PartitionConfig::default();
//! let modules = partition_files(files, &config);
//! for module in modules {
//!     println!("Module: {} ({} files)", module.name, module.files.len());
//! }
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use crate::repo_scanner::SourceFile;

/// A logical module identified in the codebase.
#[derive(Debug, Clone)]
pub struct Module {
    /// Module name (e.g. "auth", "api", "models").
    pub name: String,
    /// Source files belonging to this module.
    pub files: Vec<SourceFile>,
    /// Module type (crate, package, directory).
    pub module_type: ModuleType,
    /// Relative path to the module root.
    pub root_path: PathBuf,
    /// Languages used in this module.
    pub languages: Vec<String>,
}

/// Type of module boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleType {
    /// Cargo crate (Cargo.toml).
    CargoCrate,
    /// npm package (package.json).
    NpmPackage,
    /// Python package (__init__.py).
    PythonPackage,
    /// Go module (go.mod).
    GoModule,
    /// Directory-based (no package manager).
    Directory,
}

/// Partition configuration.
#[derive(Debug, Clone)]
pub struct PartitionConfig {
    /// Maximum files per module before splitting.
    pub max_files_per_module: usize,
    /// Whether to split by language.
    pub split_by_language: bool,
    /// Minimum files to form a module.
    pub min_files_for_module: usize,
}

impl Default for PartitionConfig {
    fn default() -> Self {
        Self {
            max_files_per_module: 50,
            split_by_language: false,
            min_files_for_module: 1,
        }
    }
}

/// Partition files into modules by detecting package boundaries.
pub fn partition_files(files: Vec<SourceFile>, config: &PartitionConfig) -> Vec<Module> {
    let mut modules: Vec<Module> = Vec::new();
    
    // 1. Group files by parent directory
    let mut dir_groups: HashMap<PathBuf, Vec<SourceFile>> = HashMap::new();
    for file in &files {
        let parent = if let Some(p) = file.path.parent() {
            p.to_path_buf()
        } else {
            PathBuf::from(".")
        };
        dir_groups.entry(parent).or_default().push(file.clone());
    }
    
    // 2. Detect module boundaries
    let module_indicators = [
        "Cargo.toml",
        "package.json", 
        "__init__.py",
        "go.mod",
        "pom.xml",
        "build.gradle",
    ];
    
    for (dir_path, dir_files) in &dir_groups {
        // Determine module type
        let module_type = detect_module_type(dir_path, &module_indicators);
        let module_name = dir_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "root".to_string());
        
        // Collect languages
        let mut languages: Vec<String> = dir_files.iter()
            .filter_map(|f| f.language.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        languages.sort();
        
        let module = Module {
            name: module_name,
            files: dir_files.clone(),
            module_type,
            root_path: dir_path.clone(),
            languages,
        };
        
        // Only add if meets minimum file count
        if module.files.len() >= config.min_files_for_module {
            modules.push(module);
        }
    }
    
    // 3. Split large modules if configured
    if config.max_files_per_module > 0 {
        modules = split_large_modules(modules, config.max_files_per_module);
    }
    
    modules
}

fn detect_module_type(dir_path: &PathBuf, indicators: &[&str]) -> ModuleType {
    for indicator in indicators {
        if dir_path.join(indicator).exists() {
            return match *indicator {
                "Cargo.toml" => ModuleType::CargoCrate,
                "package.json" => ModuleType::NpmPackage,
                "__init__.py" => ModuleType::PythonPackage,
                "go.mod" => ModuleType::GoModule,
                _ => ModuleType::Directory,
            };
        }
    }
    ModuleType::Directory
}

fn split_large_modules(modules: Vec<Module>, max_files: usize) -> Vec<Module> {
    let mut result = Vec::new();
    for module in modules {
        if module.files.len() > max_files {
            // Split into chunks
            for (i, chunk) in module.files.chunks(max_files).enumerate() {
                let suffix = if i == 0 { String::new() } else { format!("_{}", i + 1) };
                result.push(Module {
                    name: format!("{}{}", module.name, suffix),
                    files: chunk.to_vec(),
                    module_type: module.module_type.clone(),
                    root_path: module.root_path.clone(),
                    languages: module.languages.clone(),
                });
            }
        } else {
            result.push(module);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn make_file(path: &str, lang: &str) -> SourceFile {
        SourceFile {
            path: PathBuf::from(path),
            language: Some(lang.to_string()),
            size_bytes: 100,
            relative_path: PathBuf::from(path),
        }
    }

    #[test]
    fn test_detect_module_type_default() {
        let path = PathBuf::from("/tmp/some_dir");
        assert_eq!(detect_module_type(&path, &["Cargo.toml", "package.json"]), ModuleType::Directory);
    }

    #[test]
    fn test_split_large_modules() {
        let files: Vec<SourceFile> = (0..10).map(|i| make_file(&format!("file{}.rs", i), "rust")).collect();
        let module = Module {
            name: "big_mod".to_string(),
            files,
            module_type: ModuleType::Directory,
            root_path: PathBuf::from("/"),
            languages: vec!["rust".to_string()],
        };
        
        let result = split_large_modules(vec![module], 3);
        assert_eq!(result.len(), 4); // 10 files / 3 = 4 chunks
        assert_eq!(result[0].name, "big_mod");
        assert_eq!(result[1].name, "big_mod_2");
        assert_eq!(result[0].files.len(), 3);
        assert_eq!(result[3].files.len(), 1);
    }

    #[test]
    fn test_partition_empty() {
        let result = partition_files(vec![], &PartitionConfig::default());
        assert!(result.is_empty());
    }

    #[test]
    fn test_detect_module_type_cargo() {
        // Create a temp dir with Cargo.toml
        let tmp = std::env::temp_dir().join(format!("test_modules_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let _ = std::fs::write(tmp.join("Cargo.toml"), "[package]\nname = \"test\"\n");
        
        let module_type = detect_module_type(&tmp, &["Cargo.toml"]);
        assert_eq!(module_type, ModuleType::CargoCrate);
        
        let _ = std::fs::remove_dir_all(&tmp);
    }
}