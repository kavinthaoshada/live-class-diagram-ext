use crate::model::{ClassKind, ClassNode, FieldNode, MethodNode, Param, Visibility};
use std::path::Path;
use tree_sitter::Node;

pub fn parse(path: &Path, source: &str) -> Vec<ClassNode> {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&tree_sitter_java::LANGUAGE.into()).is_err() {
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
    match node.kind() {
        "class_declaration" => out.push(extract_class(node, src, file)),
        "interface_declaration" => out.push(extract_interface(node, src, file)),
        "enum_declaration" => out.push(extract_enum(node, src, file)),
        _ => {}
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

fn child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

fn children_by_kind<'a>(node: Node<'a>, kind: &str) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).filter(|c| c.kind() == kind).collect()
}

fn modifiers_of(node: Node) -> Option<Node> {
    child_by_kind(node, "modifiers")
}

fn has_modifier(node: Node, keyword: &str) -> bool {
    modifiers_of(node)
        .map(|m| child_by_kind(m, keyword).is_some())
        .unwrap_or(false)
}

fn visibility_of(node: Node) -> Visibility {
    let Some(modifiers) = modifiers_of(node) else {
        return Visibility::Public;
    };
    if child_by_kind(modifiers, "private").is_some() {
        Visibility::Private
    } else if child_by_kind(modifiers, "protected").is_some() {
        Visibility::Protected
    } else {
        Visibility::Public
    }
}

fn type_identifiers(node: Node, src: &str) -> Vec<String> {
    let Some(type_list) = child_by_kind(node, "type_list") else {
        return Vec::new();
    };
    children_by_kind(type_list, "type_identifier")
        .into_iter()
        .map(|n| text(n, src).to_string())
        .collect()
}

fn extract_class(node: Node, src: &str, file: &str) -> ClassNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_else(|| "Anonymous".to_string());

    let extends: Vec<String> = node
        .child_by_field_name("superclass")
        .and_then(|s| child_by_kind(s, "type_identifier"))
        .map(|n| vec![text(n, src).to_string()])
        .unwrap_or_default();

    let implements: Vec<String> = node
        .child_by_field_name("interfaces")
        .map(|s| type_identifiers(s, src))
        .unwrap_or_default();

    let (fields, methods) = extract_members(node, src);

    ClassNode {
        id: name.clone(),
        name,
        kind: if has_modifier(node, "abstract") {
            ClassKind::AbstractClass
        } else {
            ClassKind::Class
        },
        file: file.to_string(),
        line: node.start_position().row as u32 + 1,
        fields,
        methods,
        extends,
        implements,
    }
}

fn extract_interface(node: Node, src: &str, file: &str) -> ClassNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_else(|| "Anonymous".to_string());

    let extends = child_by_kind(node, "extends_interfaces")
        .map(|s| type_identifiers(s, src))
        .unwrap_or_default();

    let (fields, methods) = extract_members(node, src);

    ClassNode {
        id: name.clone(),
        name,
        kind: ClassKind::Interface,
        file: file.to_string(),
        line: node.start_position().row as u32 + 1,
        fields,
        methods,
        extends,
        implements: Vec::new(),
    }
}

fn extract_enum(node: Node, src: &str, file: &str) -> ClassNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_else(|| "Anonymous".to_string());

    let mut fields = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        for constant in children_by_kind(body, "enum_constant") {
            if let Some(constant_name) = constant.child_by_field_name("name") {
                fields.push(FieldNode {
                    name: text(constant_name, src).to_string(),
                    type_name: String::new(),
                    visibility: Visibility::Public,
                    is_static: false,
                    line: line_of(constant),
                });
            }
        }
    }

    ClassNode {
        id: name.clone(),
        name,
        kind: ClassKind::Enum,
        file: file.to_string(),
        line: node.start_position().row as u32 + 1,
        fields,
        methods: Vec::new(),
        extends: Vec::new(),
        implements: Vec::new(),
    }
}

fn extract_members(node: Node, src: &str) -> (Vec<FieldNode>, Vec<MethodNode>) {
    let mut fields = Vec::new();
    let mut methods = Vec::new();

    let Some(body) = node.child_by_field_name("body") else {
        return (fields, methods);
    };

    let mut cursor = body.walk();
    for member in body.children(&mut cursor) {
        match member.kind() {
            "field_declaration" => {
                let type_name = member
                    .child_by_field_name("type")
                    .map(|t| text(t, src).to_string())
                    .unwrap_or_else(|| "any".to_string());
                let is_static = has_modifier(member, "static");
                let visibility = visibility_of(member);
                for declarator in children_by_kind(member, "variable_declarator") {
                    if let Some(field_name) = declarator.child_by_field_name("name") {
                        fields.push(FieldNode {
                            name: text(field_name, src).to_string(),
                            type_name: type_name.clone(),
                            visibility,
                            is_static,
                            line: line_of(declarator),
                        });
                    }
                }
            }
            "method_declaration" | "constructor_declaration" => {
                methods.push(extract_method(member, src));
            }
            _ => {}
        }
    }

    (fields, methods)
}

fn extract_method(node: Node, src: &str) -> MethodNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_default();

    let mut params = Vec::new();
    if let Some(parameters) = node.child_by_field_name("parameters") {
        for param in children_by_kind(parameters, "formal_parameter") {
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
    }

    let return_type = node
        .child_by_field_name("type")
        .map(|t| text(t, src).to_string())
        .unwrap_or_default();

    MethodNode {
        name,
        params,
        return_type,
        visibility: visibility_of(node),
        is_static: has_modifier(node, "static"),
        is_abstract: has_modifier(node, "abstract") || node.child_by_field_name("body").is_none(),
        line: line_of(node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse_java(source: &str) -> Vec<ClassNode> {
        parse(Path::new("Test.java"), source)
    }

    fn find<'a>(classes: &'a [ClassNode], name: &str) -> &'a ClassNode {
        classes
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("class {name} not found in {classes:?}"))
    }

    #[test]
    fn class_extends_and_implements() {
        let classes = parse_java(
            "public abstract class Animal extends Base implements Shape {\n\
             \x20   public abstract double area();\n\
             }\n",
        );

        let animal = find(&classes, "Animal");
        assert_eq!(animal.kind, ClassKind::AbstractClass);
        assert_eq!(animal.extends, vec!["Base".to_string()]);
        assert_eq!(animal.implements, vec!["Shape".to_string()]);
        assert!(animal.methods[0].is_abstract);
    }

    #[test]
    fn interface_extending_multiple_interfaces() {
        let classes = parse_java("public interface Movable extends Shape, Serializable {\n    void move();\n}\n");

        let movable = find(&classes, "Movable");
        assert_eq!(movable.kind, ClassKind::Interface);
        assert_eq!(movable.extends, vec!["Shape".to_string(), "Serializable".to_string()]);
        assert!(movable.methods[0].is_abstract, "interface methods have no body, so they're abstract");
    }

    #[test]
    fn field_modifiers_and_types() {
        let classes = parse_java(
            "public class Cat {\n\
             \x20   private static int count = 0;\n\
             \x20   protected String name;\n\
             \x20   public List<Animal> friends;\n\
             }\n",
        );

        let cat = find(&classes, "Cat");
        let count = cat.fields.iter().find(|f| f.name == "count").unwrap();
        assert_eq!(count.visibility, Visibility::Private);
        assert!(count.is_static);

        let name = cat.fields.iter().find(|f| f.name == "name").unwrap();
        assert_eq!(name.visibility, Visibility::Protected);
        assert_eq!(name.type_name, "String");

        let friends = cat.fields.iter().find(|f| f.name == "friends").unwrap();
        assert_eq!(friends.type_name, "List<Animal>");
    }

    #[test]
    fn enum_constants_become_fields() {
        let classes = parse_java("enum Color { RED, GREEN, BLUE }");
        let color = find(&classes, "Color");
        assert_eq!(color.kind, ClassKind::Enum);
        let names: Vec<_> = color.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["RED", "GREEN", "BLUE"]);
    }

    #[test]
    fn constructor_is_captured_as_a_method() {
        let classes = parse_java(
            "public class Dog {\n\
             \x20   public Dog(String name) { }\n\
             }\n",
        );
        let dog = find(&classes, "Dog");
        assert!(dog.methods.iter().any(|m| m.name == "Dog"));
    }
}
