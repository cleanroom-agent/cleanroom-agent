//! C-specific tree-sitter parser helpers.
//!
//! Extracts structs, enums, unions, typedefs, and functions from C source
//! using tree-sitter CST nodes.

use tree_sitter::Node;

use crate::ir_to_sdef::{IrEntity, IrAttribute, IrMethod, IrParam};

/// Extract all top-level definitions from a C translation unit.
pub fn extract_c_declarations(
    root: &Node,
    source: &str,
) -> Vec<IrEntity> {
    let mut entities = Vec::new();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        match node.kind() {
            "struct_specifier" => {
                if let Some(entity) = extract_c_struct(&node, source) {
                    entities.push(entity);
                }
            }
            "enum_specifier" => {
                if let Some(entity) = extract_c_enum(&node, source) {
                    entities.push(entity);
                }
            }
            "function_definition" => {
                if let Some(entity) = extract_c_function(&node, source) {
                    entities.push(entity);
                }
            }
            "declaration" => {
                let mut decl_cursor = node.walk();
                for child in node.children(&mut decl_cursor) {
                    if child.kind() == "struct_specifier" {
                        if let Some(entity) = extract_c_struct(&child, source) {
                            entities.push(entity);
                        }
                    } else if child.kind() == "enum_specifier" {
                        if let Some(entity) = extract_c_enum(&child, source) {
                            entities.push(entity);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    entities
}

/// Extract a C struct as IrEntity::DataModel.
fn extract_c_struct(node: &Node, source: &str) -> Option<IrEntity> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

    let mut attrs = Vec::new();
    // Parse field declarations from the raw text using simple heuristics
    if let Some(body) = node.child_by_field_name("body") {
        if let Ok(body_text) = body.utf8_text(source.as_bytes()) {
            for line in body_text.lines() {
                let trimmed = line.trim();
                // Skip braces and empty lines
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" { continue; }
                // Remove trailing semicolon
                let cleaned = trimmed.trim_end_matches(';').trim();
                // Split on '//' to remove inline comments
                let code_part = cleaned.split("//").next().unwrap_or(cleaned).trim();
                if code_part.is_empty() { continue; }

                // Try to find field name: last identifier before potential array/pointer
                // Patterns: "type name;", "type *name;", "type name[n];", "type name, name2;"
                let parts: Vec<&str> = code_part.split(|c: char| c == ',' || c == ';').collect();
                for part in parts {
                    let part = part.trim();
                    if part.is_empty() { continue; }

                    // Find the type and field name by looking at the last word
                    let tokens: Vec<&str> = part.split_whitespace().collect();
                    if tokens.len() >= 2 {
                        // The type is everything except the last identifier token
                        let field_name = tokens.last().unwrap_or(&"");
                        let field_name = field_name.trim_start_matches('*');
                        // Clean up array notation
                        let field_name = field_name.split('[').next().unwrap_or(field_name);
                        let field_type = tokens[..tokens.len()-1].join(" ");

                        if !field_name.is_empty() && !field_type.is_empty() {
                            attrs.push(IrAttribute {
                                name: field_name.to_string(),
                                attr_type: field_type,
                                description: None,
                                required: true,
                            });
                        }
                    }
                }
            }
        }
    }

    Some(IrEntity::DataModel {
        name,
        description: None,
        attributes: attrs,
    })
}

/// Extract a C enum as IrEntity::DataModel (with enum values as attrs).
fn extract_c_enum(node: &Node, source: &str) -> Option<IrEntity> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

    let mut values = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for enumerator in body.children(&mut cursor) {
            if enumerator.kind() == "enumerator" {
                if let Some(val_node) = enumerator.child_by_field_name("name") {
                    if let Ok(text) = val_node.utf8_text(source.as_bytes()) {
                        values.push(IrAttribute {
                            name: text.to_string(),
                            attr_type: "int".to_string(),
                            description: None,
                            required: true,
                        });
                    }
                }
            }
        }
    }

    Some(IrEntity::DataModel {
        name,
        description: None,
        attributes: values,
    })
}

/// Extract a C function as IrEntity::Function.
fn extract_c_function(node: &Node, source: &str) -> Option<IrEntity> {
    let declarator = node.child_by_field_name("declarator")?;

    // Find the function name by looking through pointer/identifier chains
    let name = find_function_name(declarator, source)?;

    // Get return type
    let return_type = node.child_by_field_name("type")
        .and_then(|t| t.utf8_text(source.as_bytes()).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    // Get parameters
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    if let Some(params_node) = declarator.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        for param in params_node.children(&mut cursor) {
            if param.kind() == "parameter_declaration" {
                let pname = param.child_by_field_name("declarator")
                    .and_then(|d| find_param_name(d, source))
                    .unwrap_or("unnamed");
                let ptype = param.child_by_field_name("type")
                    .and_then(|t| t.utf8_text(source.as_bytes()).ok())
                    .unwrap_or("void")
                    .to_string();
                inputs.push(IrParam {
                    name: pname.to_string(),
                    param_type: ptype,
                    description: None,
                });
            }
        }
    }

    // Return type is output
    if !return_type.is_empty() && return_type != "void" {
        outputs.push(IrParam {
            name: "result".to_string(),
            param_type: return_type,
            description: None,
        });
    }

    Some(IrEntity::Function {
        name,
        description: None,
        inputs,
        outputs,
    })
}

/// Walk pointer/identifier chains to find the function name.
fn find_function_name<'a>(node: Node<'a>, source: &str) -> Option<String> {
    // Try common patterns: identifier directly
    if let Ok(text) = node.utf8_text(source.as_bytes()) {
        if !text.contains('(') && !text.contains('*') && !text.contains('[') {
            return Some(text.to_string());
        }
    }

    // Walk children looking for identifiers
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                return Some(text.to_string());
            }
        }
        // Recurse into pointer/array/function declarators
        if matches!(child.kind(), "pointer_declarator" | "array_declarator" | "function_declarator") {
            if let Some(name) = find_function_name(child, source) {
                return Some(name);
            }
        }
    }
    None
}

/// Find parameter name from declarator node.
fn find_param_name<'a>(node: Node<'a>, source: &'a str) -> Option<&'a str> {
    node.utf8_text(source.as_bytes()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn with_parser<F>(source: &str, f: F)
    where F: FnOnce(Node)
    {
        let mut parser = Parser::new();
        let lang: tree_sitter::Language = tree_sitter_c::LANGUAGE.into();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse(source, None).unwrap();
        f(tree.root_node());
    }

    #[test]
    fn test_extract_struct() {
        with_parser("struct redisServer { int port; char *bindaddr; long long maxmemory; };", |root| {
            let source = "struct redisServer { int port; char *bindaddr; long long maxmemory; };";
            let entities = extract_c_declarations(&root, source);
            assert_eq!(entities.len(), 1);
            if let IrEntity::DataModel { name, attributes, .. } = &entities[0] {
                assert_eq!(name, "redisServer");
                assert!(attributes.iter().any(|a| a.name == "port"));
            } else { panic!("Expected DataModel"); }
        });
    }

    #[test]
    fn test_extract_function() {
        with_parser("int dictAdd(dict *d, void *key, void *val) { return 0; }", |root| {
            let source = "int dictAdd(dict *d, void *key, void *val) { return 0; }";
            let entities = extract_c_declarations(&root, source);
            assert_eq!(entities.len(), 1);
            if let IrEntity::Function { name, inputs, .. } = &entities[0] {
                assert_eq!(name, "dictAdd");
                assert_eq!(inputs.len(), 3);
            } else { panic!("Expected Function"); }
        });
    }

    #[test]
    fn test_extract_enum() {
        with_parser("enum logLevel { DEBUG, VERBOSE, NOTICE };", |root| {
            let source = "enum logLevel { DEBUG, VERBOSE, NOTICE };";
            let entities = extract_c_declarations(&root, source);
            assert_eq!(entities.len(), 1);
        });
    }

    #[test]
    fn test_empty_source() {
        with_parser("", |root| {
            let source = "";
            let entities = extract_c_declarations(&root, source);
            assert!(entities.is_empty());
        });
    }

    #[test]
    fn test_void_function() {
        with_parser("void freeDict(dict *d) { return; }", |root| {
            let source = "void freeDict(dict *d) { return; }";
            let entities = extract_c_declarations(&root, source);
            assert_eq!(entities.len(), 1);
            if let IrEntity::Function { name, outputs, .. } = &entities[0] {
                assert_eq!(name, "freeDict");
                assert!(outputs.is_empty(), "void function should have no outputs");
            }
        });
    }

    #[test]
    fn test_mixed_declarations() {
        // Function with body IS a function_definition
        let source = "struct redisServer { int port; };\nint dictAdd(dict *d) { return 0; }";
        with_parser(source, |root| {
            let entities = extract_c_declarations(&root, source);
            assert_eq!(entities.len(), 2, "Should find struct + function");
        });
    }
}
