//! Data migration generator — generates migration code from S.DEF DataMigration specs.

use sdef_core::DataMigration;

/// Generated migration code.
#[derive(Debug, Clone)]
pub struct MigrationCode {
    /// Source entity name (old version).
    pub from_entity: String,
    /// Target entity name (new version).
    pub to_entity: String,
    /// Source code content.
    pub code: String,
    /// File name (without extension).
    pub file_name: String,
}

/// Migration generator for different languages.
pub struct MigrationGenerator {
    target_language: String,
}

impl MigrationGenerator {
    pub fn new(target_language: &str) -> Self {
        Self {
            target_language: target_language.to_string(),
        }
    }

    /// Generate migration code for a single DataMigration spec.
    pub fn generate(&self, migration: &DataMigration) -> Result<MigrationCode, String> {
        match self.target_language.as_str() {
            "rust" => self.generate_rust(migration),
            "typescript" => self.generate_typescript(migration),
            "python" => self.generate_python(migration),
            _ => Err(format!("Unsupported language: {}", self.target_language)),
        }
    }

    fn generate_rust(&self, migration: &DataMigration) -> Result<MigrationCode, String> {
        let from_snake = migration.from_entity.to_lowercase();
        let to_snake = migration.to_entity.to_lowercase();
        let from_pascal = to_pascal_case(&migration.from_entity);
        let to_pascal = to_pascal_case(&migration.to_entity);
        let algorithm = migration.algorithm.as_deref().unwrap_or("Migration algorithm not specified");
        let from_ver = migration.from_version.as_deref().unwrap_or("previous");
        let to_ver = migration.to_version.as_deref().unwrap_or("current");
        let description = migration.description.as_deref().unwrap_or("");

        let code = vec![
            format!("/// Data migration: {} → {}", migration.from_entity, migration.to_entity),
            format!("/// Description: {}", description),
            format!("/// From version: {} → To version: {}", from_ver, to_ver),
            format!("///"),
            format!("/// {}", algorithm),
            format!("pub fn migrate_{}_to_{}(source: {}) -> {} {{", from_snake, to_snake, from_pascal, to_pascal),
            format!("    // Algorithm: {}", algorithm),
            format!("    // Steps:"),
            format!("    //   1. Transform fields"),
            format!("    //   2. Validate constraints"),
            format!("    //   3. Return new entity"),
            format!("    todo!(\"Migration: {} -> {}\", from_ver, to_ver)", "{}", "{}"),
            format!("}}"),
            String::new(),
            format!("#[cfg(test)]"),
            format!("mod tests {{"),
            format!("    use super::*;"),
            String::new(),
            format!("    #[test]"),
            format!("    fn test_migrate_{}_to_{}() {{", from_snake, to_snake),
            format!("        let _result = migrate_{}_to_{}({} {{}});", from_snake, to_snake, from_pascal),
            format!("    }}"),
            format!("}}"),
        ].join("\n");

        Ok(MigrationCode {
            from_entity: migration.from_entity.clone(),
            to_entity: migration.to_entity.clone(),
            code,
            file_name: format!("migration_{}_to_{}_rs", from_snake, to_snake),
        })
    }

    fn generate_typescript(&self, migration: &DataMigration) -> Result<MigrationCode, String> {
        let from_camel = to_camel_case(&migration.from_entity);
        let to_camel = to_camel_case(&migration.to_entity);
        let from_pascal = to_pascal_case(&migration.from_entity);
        let to_pascal = to_pascal_case(&migration.to_entity);
        let algorithm = migration.algorithm.as_deref().unwrap_or("Migration algorithm not specified");
        let description = migration.description.as_deref().unwrap_or("");

        let code = format!(
            r#"/**
 * Data migration: {} → {}
 * Description: {}
 * Algorithm: {}
 */
export function migrate{}(source: {}): {} {{
    // TODO: Implement migration logic
    // Steps:
    //   1. Transform fields
    //   2. Validate constraints
    //   3. Return new entity
    throw new Error('Migration not implemented');
}}
"#,
            migration.from_entity, migration.to_entity,
            description,
            algorithm,
            to_pascal, from_pascal, to_pascal,
        );

        Ok(MigrationCode {
            from_entity: migration.from_entity.clone(),
            to_entity: migration.to_entity.clone(),
            code,
            file_name: format!("migration_{}_to_{}.ts", from_camel, to_camel),
        })
    }

    fn generate_python(&self, migration: &DataMigration) -> Result<MigrationCode, String> {
        let from_snake = to_snake_case(&migration.from_entity);
        let to_snake = to_snake_case(&migration.to_entity);
        let from_pascal = to_pascal_case(&migration.from_entity);
        let to_pascal = to_pascal_case(&migration.to_entity);
        let algorithm = migration.algorithm.as_deref().unwrap_or("Migration algorithm not specified");
        let description = migration.description.as_deref().unwrap_or("");

        let code = format!(
            "'''Data migration: {} -> {}\nDescription: {}\nAlgorithm: {}\n'''\nfrom typing import Optional\n\n\ndef migrate_{}_to_{}(source: {}) -> {}:\n    raise NotImplementedError()\n",
            from_pascal, to_pascal, description, algorithm,
            from_snake, to_snake, from_pascal, to_pascal,
        );

        Ok(MigrationCode {
            from_entity: migration.from_entity.clone(),
            to_entity: migration.to_entity.clone(),
            code,
            file_name: format!("migration_{}_to_{}.py", from_snake, to_snake),
        })
    }
}

fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(f) => f.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect()
}

fn to_camel_case(s: &str) -> String {
    let parts: Vec<&str> = s.split(|c: char| c == '_' || c == '-').collect();
    parts.iter().enumerate().map(|(i, p)| {
        if i == 0 { p.to_lowercase() } else { to_pascal_case(p) }
    }).collect()
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 { result.push('_'); }
        result.push(c.to_ascii_lowercase());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_migration() -> DataMigration {
        DataMigration {
            id: "mig-001".to_string(),
            description: Some("Migrate UserV1 to UserV2 with new fields".to_string()),
            from_entity: "UserV1".to_string(),
            to_entity: "UserV2".to_string(),
            from_version: Some("1.0".to_string()),
            to_version: Some("2.0".to_string()),
            algorithm: Some("Transform: name -> firstName+lastName, add email field".to_string()),
        }
    }

    #[test]
    fn test_generate_rust_migration() {
        let gen = MigrationGenerator::new("rust");
        let migration = sample_migration();
        let result = gen.generate(&migration).unwrap();
        assert!(result.code.contains("migrate_userv1_to_userv2"));
        assert!(result.code.contains("UserV1"));
        assert!(result.code.contains("UserV2"));
        assert!(result.code.contains("Transfor"));
        assert!(result.file_name.contains("rs"));
    }

    #[test]
    fn test_generate_typescript_migration() {
        let gen = MigrationGenerator::new("typescript");
        let migration = sample_migration();
        let result = gen.generate(&migration).unwrap();
        assert!(result.code.contains("migrateUserV2"));
        assert!(result.code.contains("UserV1"));
        assert!(result.code.contains("UserV2"));
        assert!(result.file_name.contains(".ts"));
    }

    #[test]
    fn test_generate_python_migration() {
        let gen = MigrationGenerator::new("python");
        let migration = sample_migration();
        let result = gen.generate(&migration).unwrap();
        assert!(result.code.contains("migrate_user_v1_to_user_v2"));
        assert!(result.code.contains("UserV1"));
        assert!(result.code.contains("UserV2"));
        assert!(result.file_name.contains(".py"));
    }

    #[test]
    fn test_unsupported_language() {
        let gen = MigrationGenerator::new("brainfuck");
        let migration = sample_migration();
        let result = gen.generate(&migration);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_migrations() {
        let gen = MigrationGenerator::new("rust");
        let migrations = vec![
            sample_migration(),
            DataMigration {
                id: "mig-002".to_string(),
                description: Some("Drop legacy field".to_string()),
                from_entity: "OrderV1".to_string(),
                to_entity: "OrderV2".to_string(),
                from_version: Some("1.0".to_string()),
                to_version: Some("2.0".to_string()),
                algorithm: Some("Flatten nested address".to_string()),
            },
        ];

        for m in &migrations {
            let result = gen.generate(m).unwrap();
            assert!(result.code.contains(&m.from_entity));
            assert!(result.code.contains(&m.to_entity));
        }
    }
}