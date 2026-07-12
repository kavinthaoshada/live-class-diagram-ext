use crate::model::{ClassKind, ClassNode, FieldNode, MethodNode, Param, Visibility};
use std::path::Path;
use tree_sitter::Node;

const ENUM_BASES: &[&str] = &["Enum", "IntEnum", "Flag", "IntFlag", "StrEnum"];

pub fn parse(path: &Path, source: &str) -> Vec<ClassNode> {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&tree_sitter_python::LANGUAGE.into()).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };

    let file = path.display().to_string();
    let mut classes = Vec::new();
    walk(tree.root_node(), source, &file, &mut classes);
    classes
}

fn walk(node: Node, src: &str, file: &str, out: &mut Vec<ClassNode>) {
    if node.kind() == "class_definition" {
        out.push(extract_class(node, src, file));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, file, out);
    }
}

fn text<'a>(node: Node, src: &'a str) -> &'a str {
    node.utf8_text(src.as_bytes()).unwrap_or("").trim()
}

fn line_of(node: Node) -> u32 {
    node.start_position().row as u32 + 1
}

fn visibility_of(name: &str) -> Visibility {
    if name.starts_with("__") && !name.ends_with("__") {
        Visibility::Private
    } else if name.starts_with('_') {
        Visibility::Protected
    } else {
        Visibility::Public
    }
}

fn base_names(node: Node, src: &str) -> Vec<String> {
    let Some(superclasses) = node.child_by_field_name("superclasses") else {
        return Vec::new();
    };
    let mut cursor = superclasses.walk();
    superclasses
        .children(&mut cursor)
        .filter(|c| c.kind() == "identifier" || c.kind() == "attribute")
        .map(|c| text(c, src).to_string())
        .collect()
}

fn extract_class(node: Node, src: &str, file: &str) -> ClassNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_else(|| "Anonymous".to_string());

    let bases = base_names(node, src);
    let is_enum = bases.iter().any(|b| ENUM_BASES.contains(&b.as_str()));
    let is_abstract = bases.iter().any(|b| b == "ABC" || b == "ABCMeta");
    let extends: Vec<String> = bases
        .into_iter()
        .filter(|b| {
            b != "ABC" && b != "ABCMeta" && b != "object" && b != "Generic" && !ENUM_BASES.contains(&b.as_str())
        })
        .collect();

    let mut fields: Vec<FieldNode> = Vec::new();
    let mut methods = Vec::new();
    let mut seen_fields = std::collections::HashSet::new();

    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            match member.kind() {
                "expression_statement" => {
                    if let Some(field) = class_level_field(member, src) {
                        if seen_fields.insert(field.name.clone()) {
                            fields.push(field);
                        }
                    }
                }
                "function_definition" => {
                    collect_self_fields(member, src, &mut fields, &mut seen_fields);
                    methods.push(extract_method(member, src, false, false));
                }
                "decorated_definition" => {
                    if let Some(func) = member.child_by_field_name("definition") {
                        if func.kind() != "function_definition" {
                            continue;
                        }
                        let decorators = decorator_names(member, src);
                        let is_static = decorators.iter().any(|d| d == "staticmethod" || d == "classmethod");
                        let is_abstract_method = decorators.iter().any(|d| d == "abstractmethod");
                        collect_self_fields(func, src, &mut fields, &mut seen_fields);
                        methods.push(extract_method(func, src, is_static, is_abstract_method));
                    }
                }
                _ => {}
            }
        }
    }

    if is_enum {
        methods.clear();
    }

    ClassNode {
        id: name.clone(),
        name,
        kind: if is_enum {
            ClassKind::Enum
        } else if is_abstract {
            ClassKind::AbstractClass
        } else {
            ClassKind::Class
        },
        file: file.to_string(),
        line: node.start_position().row as u32 + 1,
        fields,
        methods,
        extends,
        implements: Vec::new(),
    }
}

fn decorator_names(decorated: Node, src: &str) -> Vec<String> {
    let mut cursor = decorated.walk();
    decorated
        .children(&mut cursor)
        .filter(|c| c.kind() == "decorator")
        .filter_map(|d| {
            let mut inner = d.walk();
            d.children(&mut inner)
                .find(|c| c.kind() == "identifier" || c.kind() == "attribute" || c.kind() == "call")
        })
        .map(|n| {
            let raw = text(n, src);
            raw.split(['(', '.']).last().unwrap_or(raw).to_string()
        })
        .collect()
}

fn class_level_field(expression_statement: Node, src: &str) -> Option<FieldNode> {
    let assignment = expression_statement.child(0)?;
    if assignment.kind() != "assignment" {
        return None;
    }
    let left = assignment.child_by_field_name("left")?;
    if left.kind() != "identifier" {
        return None;
    }
    let name = text(left, src).to_string();
    let type_name = assignment
        .child_by_field_name("type")
        .map(|t| text(t, src).to_string())
        .unwrap_or_else(|| "any".to_string());
    // A bare annotation with no value (`name: str`) only declares the type of
    // an instance attribute that gets assigned in `__init__`; it is not a
    // real class variable unless it also has a value (`count: int = 0`).
    let is_static = assignment.child_by_field_name("right").is_some();
    Some(FieldNode {
        visibility: visibility_of(&name),
        name,
        type_name,
        is_static,
        line: line_of(assignment),
    })
}

fn collect_self_fields(
    func: Node,
    src: &str,
    fields: &mut Vec<FieldNode>,
    seen: &mut std::collections::HashSet<String>,
) {
    let Some(body) = func.child_by_field_name("body") else {
        return;
    };
    find_self_assignments(body, src, fields, seen);
}

fn find_self_assignments(
    node: Node,
    src: &str,
    fields: &mut Vec<FieldNode>,
    seen: &mut std::collections::HashSet<String>,
) {
    if node.kind() == "assignment" {
        if let Some(left) = node.child_by_field_name("left") {
            if left.kind() == "attribute" {
                if let Some(object) = left.child_by_field_name("object") {
                    if object.kind() == "identifier" && text(object, src) == "self" {
                        if let Some(attr) = left.child_by_field_name("attribute") {
                            let name = text(attr, src).to_string();
                            if seen.insert(name.clone()) {
                                let type_name = node
                                    .child_by_field_name("type")
                                    .map(|t| text(t, src).to_string())
                                    .unwrap_or_else(|| "any".to_string());
                                fields.push(FieldNode {
                                    visibility: visibility_of(&name),
                                    name,
                                    type_name,
                                    is_static: false,
                                    line: line_of(node),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    if node.kind() == "function_definition" {
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_self_assignments(child, src, fields, seen);
    }
}

fn extract_method(func: Node, src: &str, is_static: bool, is_abstract: bool) -> MethodNode {
    let name = func
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_default();

    let mut params = Vec::new();
    if let Some(parameters) = func.child_by_field_name("parameters") {
        let mut cursor = parameters.walk();
        for param in parameters.children(&mut cursor) {
            match param.kind() {
                "identifier" => {
                    if text(param, src) == "self" || text(param, src) == "cls" {
                        continue;
                    }
                    params.push(Param {
                        name: text(param, src).to_string(),
                        type_name: "any".to_string(),
                    });
                }
                "typed_parameter" => {
                    let param_name = param
                        .child(0)
                        .map(|n| text(n, src).to_string())
                        .unwrap_or_default();
                    let type_name = param
                        .child_by_field_name("type")
                        .map(|t| text(t, src).to_string())
                        .unwrap_or_else(|| "any".to_string());
                    params.push(Param { name: param_name, type_name });
                }
                "default_parameter" | "typed_default_parameter" => {
                    let param_name = param
                        .child_by_field_name("name")
                        .map(|n| text(n, src).to_string())
                        .unwrap_or_default();
                    let type_name = param
                        .child_by_field_name("type")
                        .map(|t| text(t, src).to_string())
                        .unwrap_or_else(|| "any".to_string());
                    params.push(Param { name: param_name, type_name });
                }
                _ => {}
            }
        }
    }

    let return_type = func
        .child_by_field_name("return_type")
        .map(|t| text(t, src).to_string())
        .unwrap_or_else(|| "any".to_string());

    MethodNode {
        visibility: visibility_of(&name),
        name,
        params,
        return_type,
        is_static,
        is_abstract,
        line: line_of(func),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse_py(source: &str) -> Vec<ClassNode> {
        parse(Path::new("test.py"), source)
    }

    fn find<'a>(classes: &'a [ClassNode], name: &str) -> &'a ClassNode {
        classes
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("class {name} not found in {classes:?}"))
    }

    #[test]
    fn abc_base_class_is_abstract() {
        let classes = parse_py(
            "from abc import ABC, abstractmethod\n\
             class Shape(ABC):\n\
             \x20   @abstractmethod\n\
             \x20   def area(self) -> float: pass\n",
        );

        let shape = find(&classes, "Shape");
        assert_eq!(shape.kind, ClassKind::AbstractClass);
        assert!(!shape.extends.contains(&"ABC".to_string()), "ABC should be filtered out of extends");
        assert!(shape.methods[0].is_abstract);
    }

    #[test]
    fn enum_base_class_is_rendered_as_enum() {
        let classes = parse_py("from enum import Enum\nclass Color(Enum):\n    RED = 1\n    GREEN = 2\n");

        let color = find(&classes, "Color");
        assert_eq!(color.kind, ClassKind::Enum);
        assert!(color.extends.is_empty(), "Enum base should be filtered out of extends");
        let names: Vec<_> = color.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["RED", "GREEN"]);
    }

    #[test]
    fn bare_annotation_is_instance_field_not_static() {
        let classes = parse_py(
            "class Animal:\n\
             \x20   species_count: int = 0\n\
             \x20   name: str\n",
        );

        let animal = find(&classes, "Animal");
        let species_count = animal.fields.iter().find(|f| f.name == "species_count").unwrap();
        assert!(species_count.is_static, "assigned class attribute should be static");

        let name = animal.fields.iter().find(|f| f.name == "name").unwrap();
        assert!(!name.is_static, "bare type-hint-only annotation should not be static");
    }

    #[test]
    fn self_assignment_in_init_becomes_instance_field() {
        let classes = parse_py(
            "class Animal:\n\
             \x20   def __init__(self, name):\n\
             \x20       self.name = name\n\
             \x20       self._owner = None\n\
             \x20       self.__secret = 1\n",
        );

        let animal = find(&classes, "Animal");
        let name = animal.fields.iter().find(|f| f.name == "name").unwrap();
        assert_eq!(name.visibility, Visibility::Public);
        assert!(!name.is_static);

        let owner = animal.fields.iter().find(|f| f.name == "_owner").unwrap();
        assert_eq!(owner.visibility, Visibility::Protected);

        let secret = animal.fields.iter().find(|f| f.name == "__secret").unwrap();
        assert_eq!(secret.visibility, Visibility::Private);
    }

    #[test]
    fn staticmethod_and_classmethod_decorators_mark_static() {
        let classes = parse_py(
            "class Factory:\n\
             \x20   @staticmethod\n\
             \x20   def make(): pass\n\
             \x20   @classmethod\n\
             \x20   def create(cls): pass\n\
             \x20   def instance_method(self): pass\n",
        );

        let factory = find(&classes, "Factory");
        let make = factory.methods.iter().find(|m| m.name == "make").unwrap();
        assert!(make.is_static);

        let create = factory.methods.iter().find(|m| m.name == "create").unwrap();
        assert!(create.is_static);

        let instance_method = factory.methods.iter().find(|m| m.name == "instance_method").unwrap();
        assert!(!instance_method.is_static);
    }

    #[test]
    fn class_inheritance_is_captured() {
        let classes = parse_py("class Dog(Animal):\n    pass\n");
        let dog = find(&classes, "Dog");
        assert_eq!(dog.extends, vec!["Animal".to_string()]);
    }
}
