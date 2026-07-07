use crate::model::{ClassKind, ClassNode, FieldNode, MethodNode, Param, Visibility};
use std::path::Path;
use tree_sitter::Node;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    TypeScript,
    JavaScript,
}

pub fn parse(path: &Path, source: &str, dialect: Dialect) -> Vec<ClassNode> {
    let mut parser = tree_sitter::Parser::new();
    let language = match dialect {
        Dialect::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Dialect::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
    };
    if parser.set_language(&language).is_err() {
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
        "class_declaration" | "abstract_class_declaration" => {
            out.push(extract_class(node, src, file));
        }
        "interface_declaration" => {
            out.push(extract_interface(node, src, file));
        }
        "enum_declaration" => {
            out.push(extract_enum(node, src, file));
        }
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

fn child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

fn children_by_kind<'a>(node: Node<'a>, kind: &str) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|c| c.kind() == kind)
        .collect()
}

fn has_modifier(node: Node, keyword: &str) -> bool {
    child_by_kind(node, keyword).is_some()
}

fn type_annotation_text(node: Node, src: &str) -> String {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|c| c.kind() != ":")
        .map(|c| text(c, src).to_string())
        .unwrap_or_default()
}

fn field_type_text(node: Node, src: &str) -> String {
    node.child_by_field_name("type")
        .map(|t| type_annotation_text(t, src))
        .unwrap_or_else(|| "any".to_string())
}

fn visibility_of(node: Node, src: &str) -> Visibility {
    if let Some(modifier) = child_by_kind(node, "accessibility_modifier") {
        return match text(modifier, src) {
            "private" => Visibility::Private,
            "protected" => Visibility::Protected,
            _ => Visibility::Public,
        };
    }
    Visibility::Public
}

fn extract_heritage(node: Node, src: &str) -> (Vec<String>, Vec<String>) {
    let mut extends = Vec::new();
    let mut implements = Vec::new();
    if let Some(heritage) = child_by_kind(node, "class_heritage") {
        if let Some(extends_clause) = child_by_kind(heritage, "extends_clause") {
            if let Some(value) = extends_clause.child_by_field_name("value") {
                extends.push(base_type_name(text(value, src)));
            }
        }
        if let Some(implements_clause) = child_by_kind(heritage, "implements_clause") {
            let mut cursor = implements_clause.walk();
            for child in implements_clause.children(&mut cursor) {
                if child.kind() == "type_identifier" || child.kind() == "generic_type" {
                    implements.push(base_type_name(text(child, src)));
                }
            }
        }
    }
    (extends, implements)
}

fn base_type_name(raw: &str) -> String {
    raw.split(['<', '(']).next().unwrap_or(raw).trim().to_string()
}

fn extract_class(node: Node, src: &str, file: &str) -> ClassNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_else(|| "Anonymous".to_string());
    let (extends, implements) = extract_heritage(node, src);

    let mut fields = Vec::new();
    let mut methods = Vec::new();

    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            match member.kind() {
                "public_field_definition" | "field_definition" => {
                    fields.push(extract_field(member, src));
                }
                "method_definition" => {
                    let method = extract_method(member, src, has_modifier(member, "abstract"));
                    if method.name == "constructor" {
                        fields.extend(extract_constructor_properties(member, src));
                    }
                    methods.push(method);
                }
                "abstract_method_signature" | "method_signature" => {
                    methods.push(extract_method(member, src, true));
                }
                _ => {}
            }
        }
    }

    ClassNode {
        id: name.clone(),
        name,
        kind: if node.kind() == "abstract_class_declaration" {
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

fn extract_field(node: Node, src: &str) -> FieldNode {
    // Plain (TS) fields expose their name via the "name" field, but
    // JavaScript's `#private` fields expose the same node via "property"
    // instead.
    let name_node = node
        .child_by_field_name("name")
        .or_else(|| node.child_by_field_name("property"));
    let name = name_node
        .map(|n| text(n, src).trim_start_matches('#').to_string())
        .unwrap_or_default();
    let is_private_field = name_node
        .map(|n| n.kind() == "private_property_identifier")
        .unwrap_or(false);

    FieldNode {
        name,
        type_name: field_type_text(node, src),
        visibility: if is_private_field {
            Visibility::Private
        } else {
            visibility_of(node, src)
        },
        is_static: has_modifier(node, "static"),
    }
}

fn extract_constructor_properties(node: Node, src: &str) -> Vec<FieldNode> {
    let mut fields = Vec::new();
    let Some(params) = node.child_by_field_name("parameters") else {
        return fields;
    };
    let mut cursor = params.walk();
    for param in params.children(&mut cursor) {
        if param.kind() != "required_parameter" && param.kind() != "optional_parameter" {
            continue;
        }
        let is_property = has_modifier(param, "accessibility_modifier") || has_modifier(param, "readonly");
        if !is_property {
            continue;
        }
        let name = param
            .child_by_field_name("pattern")
            .map(|p| text(p, src).to_string())
            .unwrap_or_default();
        fields.push(FieldNode {
            name,
            type_name: field_type_text(param, src),
            visibility: visibility_of(param, src),
            is_static: false,
        });
    }
    fields
}

fn extract_method(node: Node, src: &str, is_abstract: bool) -> MethodNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_default();

    let mut params = Vec::new();
    if let Some(formal_params) = node.child_by_field_name("parameters") {
        let mut cursor = formal_params.walk();
        for param in formal_params.children(&mut cursor) {
            if !matches!(
                param.kind(),
                "required_parameter" | "optional_parameter" | "rest_parameter"
            ) {
                continue;
            }
            let param_name = param
                .child_by_field_name("pattern")
                .map(|p| text(p, src).to_string())
                .unwrap_or_else(|| "arg".to_string());
            params.push(Param {
                name: param_name,
                type_name: field_type_text(param, src),
            });
        }
    }

    let return_type = node
        .child_by_field_name("return_type")
        .map(|t| type_annotation_text(t, src))
        .unwrap_or_else(|| if name == "constructor" { String::new() } else { "any".to_string() });

    MethodNode {
        name,
        params,
        return_type,
        visibility: visibility_of(node, src),
        is_static: has_modifier(node, "static"),
        is_abstract,
    }
}

fn extract_interface(node: Node, src: &str, file: &str) -> ClassNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_else(|| "Anonymous".to_string());

    let mut extends = Vec::new();
    if let Some(clause) = child_by_kind(node, "extends_type_clause") {
        for t in children_by_kind(clause, "type_identifier") {
            extends.push(base_type_name(text(t, src)));
        }
    }

    let mut fields = Vec::new();
    let mut methods = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            match member.kind() {
                "property_signature" => {
                    let member_name = member
                        .child_by_field_name("name")
                        .map(|n| text(n, src).to_string())
                        .unwrap_or_default();
                    fields.push(FieldNode {
                        name: member_name,
                        type_name: field_type_text(member, src),
                        visibility: Visibility::Public,
                        is_static: false,
                    });
                }
                "method_signature" => {
                    methods.push(extract_method(member, src, true));
                }
                _ => {}
            }
        }
    }

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
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            let variant_name = match member.kind() {
                "property_identifier" => Some(text(member, src).to_string()),
                "enum_assignment" => member
                    .child_by_field_name("name")
                    .map(|n| text(n, src).to_string()),
                _ => None,
            };
            if let Some(variant_name) = variant_name {
                fields.push(FieldNode {
                    name: variant_name,
                    type_name: String::new(),
                    visibility: Visibility::Public,
                    is_static: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse_ts(source: &str) -> Vec<ClassNode> {
        parse(Path::new("test.ts"), source, Dialect::TypeScript)
    }

    fn find<'a>(classes: &'a [ClassNode], name: &str) -> &'a ClassNode {
        classes
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("class {name} not found in {classes:?}"))
    }

    #[test]
    fn class_with_extends_and_implements() {
        let classes = parse_ts(
            r#"
            export abstract class Animal extends Base implements Shape, Named {
                abstract area(): number;
            }
            "#,
        );

        let animal = find(&classes, "Animal");
        assert_eq!(animal.kind, ClassKind::AbstractClass);
        assert_eq!(animal.extends, vec!["Base".to_string()]);
        assert_eq!(animal.implements, vec!["Shape".to_string(), "Named".to_string()]);
        assert_eq!(animal.methods[0].name, "area");
        assert!(animal.methods[0].is_abstract);
    }

    #[test]
    fn fields_capture_visibility_and_static() {
        let classes = parse_ts(
            r#"
            class Cat {
                private static count: number = 0;
                protected name: string;
                owner: Person;
            }
            "#,
        );

        let cat = find(&classes, "Cat");
        let count = cat.fields.iter().find(|f| f.name == "count").unwrap();
        assert_eq!(count.visibility, Visibility::Private);
        assert!(count.is_static);

        let name = cat.fields.iter().find(|f| f.name == "name").unwrap();
        assert_eq!(name.visibility, Visibility::Protected);
        assert!(!name.is_static);

        let owner = cat.fields.iter().find(|f| f.name == "owner").unwrap();
        assert_eq!(owner.visibility, Visibility::Public);
        assert_eq!(owner.type_name, "Person");
    }

    #[test]
    fn constructor_parameter_properties_become_fields() {
        let classes = parse_ts(
            r#"
            class Person {
                constructor(private readonly id: string, public age: number) {}
            }
            "#,
        );

        let person = find(&classes, "Person");
        let id = person.fields.iter().find(|f| f.name == "id").unwrap();
        assert_eq!(id.visibility, Visibility::Private);
        assert_eq!(id.type_name, "string");

        let age = person.fields.iter().find(|f| f.name == "age").unwrap();
        assert_eq!(age.visibility, Visibility::Public);
        assert_eq!(age.type_name, "number");
    }

    #[test]
    fn interface_with_extends_and_methods() {
        let classes = parse_ts(
            r#"
            interface Movable extends Shape, Serializable {
                move(): void;
            }
            "#,
        );

        let movable = find(&classes, "Movable");
        assert_eq!(movable.kind, ClassKind::Interface);
        assert_eq!(movable.extends, vec!["Shape".to_string(), "Serializable".to_string()]);
        assert!(movable.methods[0].is_abstract);
    }

    #[test]
    fn enum_variants_become_fields() {
        let classes = parse_ts("enum Color { Red, Green, Blue }");

        let color = find(&classes, "Color");
        assert_eq!(color.kind, ClassKind::Enum);
        let names: Vec<_> = color.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["Red", "Green", "Blue"]);
    }

    #[test]
    fn javascript_private_field_is_private_visibility() {
        let classes = parse(
            Path::new("test.js"),
            r#"
            class Cat {
                #secret = 1;
                speak() { return this.#secret; }
            }
            "#,
            Dialect::JavaScript,
        );

        let cat = find(&classes, "Cat");
        let secret = cat.fields.iter().find(|f| f.name == "secret").unwrap();
        assert_eq!(secret.visibility, Visibility::Private);
    }
}
