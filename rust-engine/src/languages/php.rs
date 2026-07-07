use crate::model::{ClassKind, ClassNode, FieldNode, MethodNode, Param, Visibility};
use std::path::Path;
use tree_sitter::Node;

pub fn parse(path: &Path, source: &str) -> Vec<ClassNode> {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&tree_sitter_php::LANGUAGE_PHP.into()).is_err() {
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
        "trait_declaration" => out.push(extract_trait(node, src, file)),
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

fn child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

fn children_by_kind<'a>(node: Node<'a>, kind: &str) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).filter(|c| c.kind() == kind).collect()
}

fn names_in(node: Node, src: &str) -> Vec<String> {
    children_by_kind(node, "name").into_iter().map(|n| text(n, src).to_string()).collect()
}

fn strip_sigil(raw: &str) -> String {
    raw.trim_start_matches('$').to_string()
}

fn extract_class(node: Node, src: &str, file: &str) -> ClassNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_else(|| "Anonymous".to_string());

    let extends = child_by_kind(node, "base_clause")
        .map(|c| names_in(c, src))
        .unwrap_or_default();

    // PHP traits are mixed into a class's own behavior rather than being a
    // supertype, but there is no separate UML relationship kind for that in
    // this tool yet, so `use Trait;` is rendered the same way as an
    // interface realization (dashed line, hollow triangle).
    let mut implements = child_by_kind(node, "class_interface_clause")
        .map(|c| names_in(c, src))
        .unwrap_or_default();

    let (fields, methods) = extract_members(node, src, &mut implements);

    ClassNode {
        id: name.clone(),
        name,
        kind: if child_by_kind(node, "abstract_modifier").is_some() {
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

    let extends = child_by_kind(node, "base_clause")
        .map(|c| names_in(c, src))
        .unwrap_or_default();

    let mut unused = Vec::new();
    let (fields, methods) = extract_members(node, src, &mut unused);

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

fn extract_trait(node: Node, src: &str, file: &str) -> ClassNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_else(|| "Anonymous".to_string());

    let mut unused = Vec::new();
    let (fields, methods) = extract_members(node, src, &mut unused);

    ClassNode {
        id: name.clone(),
        name,
        kind: ClassKind::Trait,
        file: file.to_string(),
        line: node.start_position().row as u32 + 1,
        fields,
        methods,
        extends: Vec::new(),
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
        for case in children_by_kind(body, "enum_case") {
            if let Some(case_name) = case.child_by_field_name("name") {
                fields.push(FieldNode {
                    name: text(case_name, src).to_string(),
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

fn extract_members(node: Node, src: &str, traits: &mut Vec<String>) -> (Vec<FieldNode>, Vec<MethodNode>) {
    let mut fields = Vec::new();
    let mut methods = Vec::new();

    let Some(body) = node.child_by_field_name("body") else {
        return (fields, methods);
    };

    let mut cursor = body.walk();
    for member in body.children(&mut cursor) {
        match member.kind() {
            "use_declaration" => {
                traits.extend(names_in(member, src));
            }
            "property_declaration" => {
                let type_name = member
                    .child_by_field_name("type")
                    .map(|t| text(t, src).to_string())
                    .unwrap_or_else(|| "any".to_string());
                let is_static = child_by_kind(member, "static_modifier").is_some();
                let visibility = visibility_of(member, src);
                for element in children_by_kind(member, "property_element") {
                    if let Some(field_name) = element.child_by_field_name("name") {
                        fields.push(FieldNode {
                            name: strip_sigil(text(field_name, src)),
                            type_name: type_name.clone(),
                            visibility,
                            is_static,
                        });
                    }
                }
            }
            "method_declaration" => {
                methods.push(extract_method(member, src));
            }
            _ => {}
        }
    }

    (fields, methods)
}

fn visibility_of(node: Node, src: &str) -> Visibility {
    match child_by_kind(node, "visibility_modifier").map(|m| text(m, src)) {
        Some("private") => Visibility::Private,
        Some("protected") => Visibility::Protected,
        _ => Visibility::Public,
    }
}

fn extract_method(node: Node, src: &str) -> MethodNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_default();

    let mut params = Vec::new();
    if let Some(parameters) = node.child_by_field_name("parameters") {
        let mut cursor = parameters.walk();
        for param in parameters.children(&mut cursor) {
            if !matches!(
                param.kind(),
                "simple_parameter" | "variadic_parameter" | "property_promotion_parameter"
            ) {
                continue;
            }
            let param_name = param
                .child_by_field_name("name")
                .map(|n| strip_sigil(text(n, src)))
                .unwrap_or_default();
            let type_name = param
                .child_by_field_name("type")
                .map(|t| text(t, src).to_string())
                .unwrap_or_else(|| "any".to_string());
            params.push(Param { name: param_name, type_name });
        }
    }

    let return_type = node
        .child_by_field_name("return_type")
        .map(|t| text(t, src).to_string())
        .unwrap_or_else(|| "void".to_string());

    MethodNode {
        name,
        params,
        return_type,
        visibility: visibility_of(node, src),
        is_static: child_by_kind(node, "static_modifier").is_some(),
        is_abstract: child_by_kind(node, "abstract_modifier").is_some() || node.child_by_field_name("body").is_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse_php(source: &str) -> Vec<ClassNode> {
        let with_tag = format!("<?php\n{source}");
        parse(Path::new("Test.php"), &with_tag)
    }

    fn find<'a>(classes: &'a [ClassNode], name: &str) -> &'a ClassNode {
        classes
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("class {name} not found in {classes:?}"))
    }

    #[test]
    fn class_extends_and_implements() {
        let classes = parse_php(
            "abstract class Animal extends Base implements Shape {\n\
             \x20   abstract public function area(): float;\n\
             }\n",
        );

        let animal = find(&classes, "Animal");
        assert_eq!(animal.kind, ClassKind::AbstractClass);
        assert_eq!(animal.extends, vec!["Base".to_string()]);
        assert_eq!(animal.implements, vec!["Shape".to_string()]);
        assert!(animal.methods[0].is_abstract);
    }

    #[test]
    fn trait_use_is_folded_into_implements_and_kind_is_trait() {
        let classes = parse_php(
            "trait HasFactory {\n    public function factory() {}\n}\n\
             class Animal {\n    use HasFactory;\n}\n",
        );

        let has_factory = find(&classes, "HasFactory");
        assert_eq!(has_factory.kind, ClassKind::Trait);

        let animal = find(&classes, "Animal");
        assert_eq!(animal.implements, vec!["HasFactory".to_string()]);
    }

    #[test]
    fn property_visibility_and_static() {
        let classes = parse_php(
            "class Animal {\n\
             \x20   private static int $count = 0;\n\
             \x20   protected string $name;\n\
             \x20   public Person $owner;\n\
             }\n",
        );

        let animal = find(&classes, "Animal");
        let count = animal.fields.iter().find(|f| f.name == "count").unwrap();
        assert_eq!(count.visibility, Visibility::Private);
        assert!(count.is_static);

        let name = animal.fields.iter().find(|f| f.name == "name").unwrap();
        assert_eq!(name.visibility, Visibility::Protected);

        let owner = animal.fields.iter().find(|f| f.name == "owner").unwrap();
        assert_eq!(owner.type_name, "Person");
    }

    #[test]
    fn enum_cases_become_fields() {
        let classes = parse_php("enum Color {\n    case Red;\n    case Green;\n}\n");
        let color = find(&classes, "Color");
        assert_eq!(color.kind, ClassKind::Enum);
        let names: Vec<_> = color.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["Red", "Green"]);
    }

    #[test]
    fn interface_declaration() {
        let classes = parse_php("interface Shape {\n    public function area(): float;\n}\n");
        let shape = find(&classes, "Shape");
        assert_eq!(shape.kind, ClassKind::Interface);
        assert!(shape.methods[0].is_abstract);
    }
}
