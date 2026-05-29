//! Rust code generator.

use super::{CodeGenerator, GeneratedCode};
use sdef_core::{DataModel, InterfaceContract, ClassContract, FunctionSpec};

/// Rust language code generator.
pub struct RustGenerator;

impl CodeGenerator for RustGenerator {
    fn generate_data_model(&self, model: &DataModel) -> Vec<GeneratedCode> {
        let mut output = String::new();
        
        // Deduplicate attributes by name to avoid duplicate field definitions
        let deduped_attrs: Vec<&sdef_core::DataAttribute> = model.attributes.as_ref()
            .map(|attrs| {
                let mut seen = std::collections::HashSet::new();
                attrs.iter().filter(|a| seen.insert(a.name.as_str())).collect()
            })
            .unwrap_or_default();
        
        // Generate struct
        if let Some(desc) = &model.description {
            output.push_str("/// ");
            output.push_str(desc);
            output.push('\n');
        }
        output.push_str(&format!("pub struct {} {{\n", to_pascal_case(&model.entity)));
        
        // Generate fields
        for attr in &deduped_attrs {
            if let Some(desc) = &attr.description {
                output.push_str("    /// ");
                output.push_str(desc);
                output.push('\n');
            }
            let ty = rust_type(&attr.attr_type, attr.required);
            output.push_str(&format!(
                "    pub {}: {},\n",
                to_snake_case(&attr.name),
                ty,
            ));
        }
        
        output.push_str("}\n");
        
        // Generate impl block
        output.push_str(&format!("\nimpl {} {{\n", to_pascal_case(&model.entity)));
        output.push_str("    /// Create a new instance.\n");
        output.push_str("    pub fn new(");
        
        // Constructor params (skip generated/internal)
        let ctor_params: Vec<String> = deduped_attrs.iter()
            .filter(|a| !a.generated && !a.internal)
            .map(|attr| {
                format!("{}: {}", to_snake_case(&attr.name), rust_type(&attr.attr_type, attr.required))
            })
            .collect();
        output.push_str(&ctor_params.join(", "));
        output.push_str(") -> Self {\n");
        output.push_str(&format!("        Self {{\n"));
        // Constructor body — only assign non-generated, non-internal fields
        for attr in deduped_attrs.iter().filter(|a| !a.generated && !a.internal) {
            output.push_str(&format!(
                "            {}: {},\n",
                to_snake_case(&attr.name),
                to_snake_case(&attr.name)
            ));
        }
        output.push_str("        }\n    }\n}\n");
        
        // Generate serde derive
        if model.attributes.as_ref().map_or(false, |a| !a.is_empty()) {
            output.push_str("\n#[derive(serde::Serialize, serde::Deserialize)]\n");
        }
        
        vec![GeneratedCode {
            file_path: format!("{}.rs", to_snake_case(&model.entity)),
            content: output,
            language: "rust".to_string(),
        }]
    }
    
    fn generate_interface(&self, interface: &InterfaceContract) -> Vec<GeneratedCode> {
        let mut output = String::new();
        
        if let Some(desc) = &interface.description {
            output.push_str("/// ");
            output.push_str(desc);
            output.push('\n');
        }
        output.push_str(&format!("pub trait {} {{\n", to_pascal_case(&interface.name)));
        
        if let Some(methods) = &interface.methods {
            for method in methods {
                if let Some(behavior) = &method.behavior {
                    output.push_str("    /// ");
                    output.push_str(behavior);
                    output.push('\n');
                }
                // Extract method name from signature
                let method_name = method.signature.split('(').next().unwrap_or(&method.signature);
                let params = method.signature.split('(').nth(1)
                    .map(|s| s.trim_end_matches(')').to_string())
                    .unwrap_or_default();
                output.push_str(&format!("    fn {}({});\n", to_snake_case(method_name), params));
            }
        }
        
        output.push_str("}\n");
        
        vec![GeneratedCode {
            file_path: format!("{}.rs", to_snake_case(&interface.name)),
            content: output,
            language: "rust".to_string(),
        }]
    }
    
    fn generate_class(&self, class: &ClassContract) -> Vec<GeneratedCode> {
        let mut output = String::new();
        
        // Derives
        let mut derives: Vec<String> = vec!["Debug".to_string(), "Clone".to_string()];
        if let Some(implements) = &class.implements {
            if !implements.is_empty() {
                derives.extend(implements.iter().cloned());
            }
        }
        output.push_str(&format!("#[derive({})]\n", derives.join(", ")));
        
        if let Some(desc) = &class.description {
            output.push_str("/// ");
            output.push_str(desc);
            output.push('\n');
        }
        output.push_str(&format!("pub struct {} {{\n", to_pascal_case(&class.name)));
        output.push_str("    // TODO: Add fields from data model\n");
        output.push_str("}\n\n");
        
        // Implement interfaces
        if let Some(implements) = &class.implements {
            for impl_trait in implements {
                output.push_str(&format!(
                    "impl {} for {} {{\n",
                    impl_trait,
                    to_pascal_case(&class.name)
                ));
                output.push_str("    // TODO: Implement trait methods\n}\n\n");
            }
        }
        
        // Impl block
        output.push_str(&format!("impl {} {{\n", to_pascal_case(&class.name)));
        output.push_str("    pub fn new() -> Self {\n");
        output.push_str("        Self { }\n    }\n}\n");
        
        vec![GeneratedCode {
            file_path: format!("{}.rs", to_snake_case(&class.name)),
            content: output,
            language: "rust".to_string(),
        }]
    }
    
    fn generate_function(&self, func: &FunctionSpec) -> GeneratedCode {
        let mut output = String::new();
        
        if let Some(desc) = &func.description {
            output.push_str("/// ");
            output.push_str(desc);
            output.push('\n');
        }
        
        let fn_name = to_snake_case(&func.name);
        
        // Generate parameters
        let params = if let Some(inputs) = &func.inputs {
            inputs.iter()
                .map(|p| format!("{}: {}", to_snake_case(&p.name), rust_type(&p.param_type, false)))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            String::new()
        };
        
        // Generate return type
        let ret_type = if let Some(outputs) = &func.outputs {
            if outputs.len() == 1 {
                rust_type(&outputs[0].param_type, false)
            } else if outputs.len() > 1 {
                let types: Vec<String> = outputs.iter()
                    .map(|o| rust_type(&o.param_type, false))
                    .collect();
                format!("({})", types.join(", "))
            } else {
                "()".to_string()
            }
        } else {
            "()".to_string()
        };
        
        output.push_str(&format!("pub fn {}({}) -> {} {{\n", fn_name, params, ret_type));
        
        if let Some(logic) = &func.logic {
            output.push_str("    // ");
            output.push_str(logic);
            output.push('\n');
        }
        
        output.push_str("    todo!()\n}\n");
        
        GeneratedCode {
            file_path: format!("{}.rs", fn_name),
            content: output,
            language: "rust".to_string(),
        }
    }
    
    fn file_extension(&self) -> &str {
        "rs"
    }
    
    fn language_id(&self) -> &str {
        "rust"
    }
}

fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect()
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

fn rust_type(sdef_type: &str, required: bool) -> String {
    let base = match sdef_type.to_lowercase().as_str() {
        "string" | "text" | "varchar" => "String",
        "integer" | "int" | "int32" => "i32",
        "int64" | "bigint" => "i64",
        "float" | "decimal" | "double" => "f64",
        "boolean" | "bool" => "bool",
        "uuid" => "uuid::Uuid",
        "timestamp" | "datetime" => "chrono::DateTime<chrono::Utc>",
        "date" => "chrono::NaiveDate",
        "json" | "jsonb" => "serde_json::Value",
        "bytes" | "bytea" => "Vec<u8>",
        "any" => "serde_json::Value",
        _ => "String",
    };
    
    if required {
        base.to_string()
    } else {
        format!("Option<{}>", base)
    }
}