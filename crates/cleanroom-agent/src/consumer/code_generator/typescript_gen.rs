//! TypeScript code generator.
//!
//! Transforms S.DEF (Software Definition Exchange Format) entities into
//! TypeScript/JavaScript source code, including interfaces, classes, and functions.
//!
//! # Generated Code
//!
//! - Data models become `export interface` and `export class`
//! - Interfaces become `export interface`
//! - Functions become `export function`
//! - Field names use camelCase
//! - Types are mapped from S.DEF types to TypeScript types
//!
//! # Type Mapping
//!
//! | S.DEF Type | TypeScript Type |
//! |------------|----------------|
//! | UUID       | string |
//! | timestamp  | Date |
//! | string     | string |
//! | integer    | number |
//! | boolean    | boolean |
//! | json       | any |

use super::{CodeGenerator, GeneratedCode};
use sdef_core::{DataModel, InterfaceContract, ClassContract, FunctionSpec};

/// TypeScript language code generator.
///
/// Implements the [`CodeGenerator`] trait to produce TypeScript source code
/// from S.DEF entities. Generates interfaces and classes with proper JSDoc
/// comments and type annotations.
pub struct TypeScriptGenerator;

impl CodeGenerator for TypeScriptGenerator {
    fn generate_data_model(&self, model: &DataModel) -> Vec<GeneratedCode> {
        let mut output = String::new();
        
        // Generate interface
        output.push_str("/**\n");
        if let Some(desc) = &model.description {
            output.push_str(&format!(" * {}\n", desc));
        }
        output.push_str(" */\n");
        output.push_str(&format!("export interface {} {{\n", to_pascal_case(&model.entity)));
        
        if let Some(attrs) = &model.attributes {
            for attr in attrs {
                if let Some(desc) = &attr.description {
                    output.push_str(&format!("  /** {} */\n", desc));
                }
                let optional = if attr.required { "" } else { "?" };
                let ty = ts_type(&attr.attr_type, attr.required);
                output.push_str(&format!(
                    "  {}{}: {};\n",
                    to_camel_case(&attr.name),
                    optional,
                    ty
                ));
            }
        }
        
        output.push_str("}\n");
        
        // Generate class implementation
        output.push_str(&format!(
            "\nexport class {} {{\n",
            to_pascal_case(&model.entity)
        ));
        
        if let Some(attrs) = &model.attributes {
            for attr in attrs {
                let ty = ts_type(&attr.attr_type, attr.required);
                output.push_str(&format!(
                    "  public {}{}: {};\n",
                    to_camel_case(&attr.name),
                    if attr.required { "" } else { "?" },
                    ty
                ));
            }
        }
        
        output.push_str(&format!(
            "\n  constructor(data: Partial<{}>) {{\n",
            to_pascal_case(&model.entity)
        ));
        
        if let Some(attrs) = &model.attributes {
            for attr in attrs {
                output.push_str(&format!(
                    "    this.{} = data.{};\n",
                    to_camel_case(&attr.name),
                    to_camel_case(&attr.name)
                ));
            }
        }
        output.push_str("  }\n}\n");
        
        vec![GeneratedCode {
            file_path: format!("{}.ts", to_snake_case(&model.entity)),
            content: output,
            language: "typescript".to_string(),
        }]
    }
    
    fn generate_interface(&self, interface: &InterfaceContract) -> Vec<GeneratedCode> {
        let mut output = String::new();
        
        output.push_str("/**\n");
        if let Some(desc) = &interface.description {
            output.push_str(&format!(" * {}\n", desc));
        }
        output.push_str(" */\n");
        output.push_str(&format!("export interface {} {{\n", to_pascal_case(&interface.name)));
        
        if let Some(methods) = &interface.methods {
            for method in methods {
                if let Some(behavior) = &method.behavior {
                    output.push_str(&format!("  /** {}\n", behavior));
                    output.push_str("   */\n");
                }
                let params = method.signature.split('(').nth(1)
                    .map(|s| s.trim_end_matches(')').to_string())
                    .unwrap_or_default();
                output.push_str(&format!(
                    "  {}({}): {};\n",
                    to_camel_case(method.signature.split('(').next().unwrap_or(&method.signature)),
                    params,
                    "void" // TODO: Extract return type
                ));
            }
        }
        
        output.push_str("}\n");
        
        vec![GeneratedCode {
            file_path: format!("{}.ts", to_snake_case(&interface.name)),
            content: output,
            language: "typescript".to_string(),
        }]
    }
    
    fn generate_class(&self, class: &ClassContract) -> Vec<GeneratedCode> {
        let mut output = String::new();
        
        output.push_str("/**\n");
        if let Some(desc) = &class.description {
            output.push_str(&format!(" * {}\n", desc));
        }
        output.push_str(" */\n");
        
        // Implements clause
        if let Some(implements) = &class.implements {
            if !implements.is_empty() {
                output.push_str(&format!(
                    "export class {} implements {} {{\n",
                    to_pascal_case(&class.name),
                    implements.iter()
                        .map(|i| to_pascal_case(i))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            } else {
                output.push_str(&format!("export class {} {{\n", to_pascal_case(&class.name)));
            }
        } else {
            output.push_str(&format!("export class {} {{\n", to_pascal_case(&class.name)));
        }
        
        output.push_str("  constructor() {}\n");
        
        // Methods placeholder
        output.push_str("\n  // TODO: Add class methods\n");
        
        output.push_str("}\n");
        
        vec![GeneratedCode {
            file_path: format!("{}.ts", to_snake_case(&class.name)),
            content: output,
            language: "typescript".to_string(),
        }]
    }
    
    fn generate_function(&self, func: &FunctionSpec) -> GeneratedCode {
        let mut output = String::new();
        
        output.push_str("/**\n");
        if let Some(desc) = &func.description {
            output.push_str(&format!(" * {}\n", desc));
        }
        output.push_str(" */\n");
        
        let fn_name = to_camel_case(&func.name);
        let params = func.inputs.as_ref().map_or(String::new(), |inputs| {
            inputs.iter()
                .map(|p| format!("{}: {}", to_camel_case(&p.name), ts_type(&p.param_type, false)))
                .collect::<Vec<_>>()
                .join(", ")
        });
        
        let ret_type = if let Some(outputs) = &func.outputs {
            if outputs.len() == 1 {
                ts_type(&outputs[0].param_type, false)
            } else if outputs.len() > 1 {
                let types: Vec<String> = outputs.iter()
                    .map(|o| ts_type(&o.param_type, false))
                    .collect();
                format!("[{}]", types.join(", "))
            } else {
                "void".to_string()
            }
        } else {
            "void".to_string()
        };
        
        let return_annotation = if ret_type == "void" {
            String::new()
        } else {
            format!(": {}", ret_type)
        };
        output.push_str(&format!(
            "export function {}({}){} {{\n",
            fn_name,
            params,
            return_annotation
        ));
        
        if let Some(logic) = &func.logic {
            output.push_str(&format!("  // {}\n", logic));
        }
        
        output.push_str("  throw new Error('Not implemented');\n");
        output.push_str("}\n");
        
        GeneratedCode {
            file_path: format!("{}.ts", to_snake_case(&func.name)),
            content: output,
            language: "typescript".to_string(),
        }
    }
    
    fn file_extension(&self) -> &str {
        "ts"
    }
    
    fn language_id(&self) -> &str {
        "typescript"
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

fn to_camel_case(s: &str) -> String {
    let parts: Vec<&str> = s.split(|c: char| c == '_' || c == '-').collect();
    parts.iter().enumerate().map(|(i, p)| {
        if i == 0 {
            p.to_lowercase()
        } else {
            to_pascal_case(p)
        }
    }).collect()
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

fn ts_type(sdef_type: &str, _required: bool) -> String {
    match sdef_type.to_lowercase().as_str() {
        "string" | "text" | "varchar" => "string".to_string(),
        "integer" | "int" | "int32" => "number".to_string(),
        "int64" | "bigint" => "bigint".to_string(),
        "float" | "decimal" | "double" => "number".to_string(),
        "boolean" | "bool" => "boolean".to_string(),
        "uuid" => "string".to_string(),
        "timestamp" | "datetime" => "Date".to_string(),
        "date" => "Date".to_string(),
        "json" | "jsonb" => "any".to_string(),
        "bytes" | "bytea" => "Uint8Array".to_string(),
        "any" => "any".to_string(),
        _ => "any".to_string(),
    }
}