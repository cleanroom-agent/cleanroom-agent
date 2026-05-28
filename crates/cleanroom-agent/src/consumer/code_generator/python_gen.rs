//! Python code generator.

use super::{CodeGenerator, GeneratedCode};
use sdef_core::{DataModel, InterfaceContract, ClassContract, FunctionSpec};

/// Python language code generator.
pub struct PythonGenerator;

impl CodeGenerator for PythonGenerator {
    fn generate_data_model(&self, model: &DataModel) -> Vec<GeneratedCode> {
        let mut output = String::new();
        
        // Generate dataclass
        output.push_str("\"\"\"\n");
        if let Some(desc) = &model.description {
            output.push_str(&format!("{}\n", desc));
        }
        output.push_str("\"\"\"\n\n");
        output.push_str("from dataclasses import dataclass, field\n");
        output.push_str("from typing import Optional\n");
        output.push_str("from datetime import datetime\n\n");
        
        output.push_str(&format!("@dataclass\nclass {}:\n", to_pascal_case(&model.entity)));
        
        if let Some(attrs) = &model.attributes {
            for attr in attrs {
                let ty = python_type(&attr.attr_type, attr.required);
                let default = if attr.generated {
                    "= field(default_factory=dict)".to_string()
                } else if attr.required {
                    String::new()
                } else {
                    " = None".to_string()
                };
                let doc = attr.description.as_ref()
                    .map(|d| format!(": {}", d))
                    .unwrap_or_default();
                output.push_str(&format!(
                    "    {}{}: {}{}\n",
                    to_snake_case(&attr.name),
                    default,
                    ty,
                    doc
                ));
            }
        }
        
        // Generate methods
        output.push_str("\n    @classmethod\n");
        output.push_str(&format!(
            "    def from_dict(cls, data: dict) -> '{}':\n",
            to_pascal_case(&model.entity)
        ));
        output.push_str("        \"\"\"Create instance from dictionary.\"\"\"\n");
        output.push_str("        return cls(**data)\n");
        
        output.push_str("\n    def to_dict(self) -> dict:\n");
        output.push_str("        \"\"\"Convert instance to dictionary.\"\"\"\n");
        output.push_str("        return {\n");
        if let Some(attrs) = &model.attributes {
            for attr in attrs {
                output.push_str(&format!(
                    "            '{}': self.{},\n",
                    attr.name,
                    to_snake_case(&attr.name)
                ));
            }
        }
        output.push_str("        }\n");
        
        vec![GeneratedCode {
            file_path: format!("{}.py", to_snake_case(&model.entity)),
            content: output,
            language: "python".to_string(),
        }]
    }
    
    fn generate_interface(&self, interface: &InterfaceContract) -> Vec<GeneratedCode> {
        let mut output = String::new();
        
        output.push_str("\"\"\"\n");
        if let Some(desc) = &interface.description {
            output.push_str(&format!("{}\n", desc));
        }
        output.push_str("\"\"\"\n\n");
        output.push_str("from abc import ABC, abstractmethod\n\n");
        
        output.push_str(&format!("class {}Interface(ABC):\n", to_pascal_case(&interface.name)));
        
        if let Some(methods) = &interface.methods {
            for method in methods {
                if let Some(behavior) = &method.behavior {
                    output.push_str(&format!("    \"\"\"{} -- \"\"\"\n", behavior));
                }
                let params = method.signature.split('(').nth(1)
                    .map(|s| s.trim_end_matches(')').to_string())
                    .unwrap_or_default();
                output.push_str(&format!(
                    "    @abstractmethod\n    def {}({}) -> None:\n",
                    to_snake_case(method.signature.split('(').next().unwrap_or(&method.signature)),
                    params
                ));
                output.push_str("        pass\n\n");
            }
        }
        
        vec![GeneratedCode {
            file_path: format!("{}_interface.py", to_snake_case(&interface.name)),
            content: output,
            language: "python".to_string(),
        }]
    }
    
    fn generate_class(&self, class: &ClassContract) -> Vec<GeneratedCode> {
        let mut output = String::new();
        
        output.push_str("\"\"\"\n");
        if let Some(desc) = &class.description {
            output.push_str(&format!("{}\n", desc));
        }
        output.push_str("\"\"\"\n\n");
        
        // Import base classes
        if let Some(implements) = &class.implements {
            for impl_trait in implements {
                output.push_str(&format!(
                    "from .{} import {}Interface\n",
                    to_snake_case(impl_trait),
                    to_pascal_case(impl_trait)
                ));
            }
        }
        
        let bases: String = if let Some(implements) = &class.implements {
            if !implements.is_empty() {
                let bases_list: Vec<String> = implements.iter()
                    .map(|i| format!("{}Interface", to_pascal_case(i)))
                    .collect();
                format!("({})", bases_list.join(", "))
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        output.push_str(&format!(
            "class {}{}:\n",
            to_pascal_case(&class.name),
            bases
        ));
        output.push_str("    \"\"\"Class implementation.\"\"\"\n\n");
        output.push_str("    def __init__(self) -> None:\n");
        output.push_str("        super().__init__()\n");
        output.push_str("        # TODO: Initialize attributes\n");
        output.push_str("        pass\n");
        
        vec![GeneratedCode {
            file_path: format!("{}.py", to_snake_case(&class.name)),
            content: output,
            language: "python".to_string(),
        }]
    }
    
    fn generate_function(&self, func: &FunctionSpec) -> GeneratedCode {
        let mut output = String::new();
        
        output.push_str("\"\"\"\n");
        if let Some(desc) = &func.description {
            output.push_str(&format!("{}\n", desc));
        }
        output.push_str("\"\"\"\n\n");
        
        let fn_name = to_snake_case(&func.name);
        let params = func.inputs.as_ref().map_or(String::new(), |inputs| {
            inputs.iter()
                .map(|p| {
                    let ty = python_type(&p.param_type, false);
                    if ty == "object" {
                        to_snake_case(&p.name)
                    } else {
                        format!("{}: {}", to_snake_case(&p.name), ty)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        });
        
        let ret_type = func.outputs.as_ref()
            .map(|outputs| {
                if outputs.len() == 1 {
                    python_type(&outputs[0].param_type, false)
                } else if outputs.len() > 1 {
                    format!("Tuple[{}]", outputs.iter()
                        .map(|o| python_type(&o.param_type, false))
                        .collect::<Vec<_>>()
                        .join(", "))
                } else {
                    "None".to_string()
                }
            })
            .unwrap_or_else(|| "None".to_string());
        
        output.push_str(&format!(
            "def {}({}) -> {}:\n",
            fn_name,
            params,
            ret_type
        ));
        
        if let Some(logic) = &func.logic {
            output.push_str(&format!("    # {}\n", logic));
        }
        
        output.push_str("    raise NotImplementedError()\n");
        
        GeneratedCode {
            file_path: format!("{}.py", fn_name),
            content: output,
            language: "python".to_string(),
        }
    }
    
    fn file_extension(&self) -> &str {
        "py"
    }
    
    fn language_id(&self) -> &str {
        "python"
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
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

fn python_type(sdef_type: &str, _required: bool) -> String {
    match sdef_type.to_lowercase().as_str() {
        "string" | "text" | "varchar" => "str".to_string(),
        "integer" | "int" | "int32" => "int".to_string(),
        "int64" | "bigint" => "int".to_string(),
        "float" | "decimal" | "double" => "float".to_string(),
        "boolean" | "bool" => "bool".to_string(),
        "uuid" => "str".to_string(),
        "timestamp" | "datetime" => "datetime".to_string(),
        "date" => "date".to_string(),
        "json" | "jsonb" => "object".to_string(),
        "bytes" | "bytea" => "bytes".to_string(),
        "any" => "object".to_string(),
        _ => "object".to_string(),
    }
}