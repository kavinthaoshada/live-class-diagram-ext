use crate::model::{ClassKind, ClassNode, FieldNode, MethodNode, Param, Visibility};
use std::path::Path;
use tree_sitter::Node;

pub fn parse(path: &Path, source: &str) -> Vec<ClassNode> {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&tree_sitter_c_sharp::LANGUAGE.into()).is_err() {
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
        "class_declaration" => out.push(extract_type(node, src, file, false)),
        "interface_declaration" => out.push(extract_type(node, src, file, true)),
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

fn has_modifier(node: Node, keyword: &str) -> bool {
    children_by_kind(node, "modifier").iter().any(|m| child_by_kind(*m, keyword).is_some())
}

fn visibility_of(node: Node) -> Visibility {
    if has_modifier(node, "private") {
        Visibility::Private
    } else if has_modifier(node, "protected") {
        Visibility::Protected
    } else {
        Visibility::Public
    }
}

fn looks_like_interface_name(name: &str) -> bool {
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some('I'), Some(second)) => second.is_uppercase(),
        _ => false,
    }
}

fn base_type_names(node: Node, src: &str) -> Vec<String> {
    let Some(base_list) = child_by_kind(node, "base_list") else {
        return Vec::new();
    };
    let mut cursor = base_list.walk();
    base_list
        .children(&mut cursor)
        .filter(|c| c.kind() == "identifier" || c.kind() == "generic_name" || c.kind() == "qualified_name")
        .map(|c| text(c, src).to_string())
        .collect()
}

fn split_bases(bases: Vec<String>) -> (Vec<String>, Vec<String>) {
    if bases.is_empty() {
        return (Vec::new(), Vec::new());
    }
    if looks_like_interface_name(&bases[0]) {
        (Vec::new(), bases)
    } else {
        let mut iter = bases.into_iter();
        let base_class = iter.next().unwrap();
        (vec![base_class], iter.collect())
    }
}

fn extract_type(node: Node, src: &str, file: &str, is_interface: bool) -> ClassNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_else(|| "Anonymous".to_string());

    let bases = base_type_names(node, src);
    let (extends, implements) = if is_interface {
        (bases, Vec::new())
    } else {
        split_bases(bases)
    };

    let (fields, methods) = extract_members(node, src);

    ClassNode {
        id: name.clone(),
        name,
        kind: if is_interface {
            ClassKind::Interface
        } else if has_modifier(node, "abstract") {
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

fn extract_enum(node: Node, src: &str, file: &str) -> ClassNode {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, src).to_string())
        .unwrap_or_else(|| "Anonymous".to_string());

    let mut fields = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        for member in children_by_kind(body, "enum_member_declaration") {
            if let Some(member_name) = member.child_by_field_name("name") {
                fields.push(FieldNode {
                    name: text(member_name, src).to_string(),
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
                let Some(variable_declaration) = child_by_kind(member, "variable_declaration") else {
                    continue;
                };
                let type_name = variable_declaration
                    .child_by_field_name("type")
                    .map(|t| text(t, src).to_string())
                    .unwrap_or_else(|| "any".to_string());
                let is_static = has_modifier(member, "static");
                let visibility = visibility_of(member);
                for declarator in children_by_kind(variable_declaration, "variable_declarator") {
                    if let Some(field_name) = declarator.child_by_field_name("name") {
                        fields.push(FieldNode {
                            name: text(field_name, src).to_string(),
                            type_name: type_name.clone(),
                            visibility,
                            is_static,
                        });
                    }
                }
            }
            "property_declaration" => {
                let type_name = member
                    .child_by_field_name("type")
                    .map(|t| text(t, src).to_string())
                    .unwrap_or_else(|| "any".to_string());
                if let Some(field_name) = member.child_by_field_name("name") {
                    fields.push(FieldNode {
                        name: text(field_name, src).to_string(),
                        type_name,
                        visibility: visibility_of(member),
                        is_static: has_modifier(member, "static"),
                    });
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
        for param in children_by_kind(parameters, "parameter") {
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
        .child_by_field_name("returns")
        .map(|t| text(t, src).to_string())
        .unwrap_or_default();

    MethodNode {
        name,
        params,
        return_type,
        visibility: visibility_of(node),
        is_static: has_modifier(node, "static"),
        is_abstract: has_modifier(node, "abstract") || node.child_by_field_name("body").is_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse_cs(source: &str) -> Vec<ClassNode> {
        parse(Path::new("Test.cs"), source)
    }

    fn find<'a>(classes: &'a [ClassNode], name: &str) -> &'a ClassNode {
        classes
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("class {name} not found in {classes:?}"))
    }

    #[test]
    fn base_class_and_interface_are_split_by_naming_convention() {
        let classes = parse_cs("public class Dog : Animal, IMovable { }");

        let dog = find(&classes, "Dog");
        assert_eq!(dog.extends, vec!["Animal".to_string()]);
        assert_eq!(dog.implements, vec!["IMovable".to_string()]);
    }

    #[test]
    fn only_interfaces_when_first_base_looks_like_an_interface() {
        let classes = parse_cs("public class Cat : IShape, IMovable { }");

        let cat = find(&classes, "Cat");
        assert!(cat.extends.is_empty());
        assert_eq!(cat.implements, vec!["IShape".to_string(), "IMovable".to_string()]);
    }

    #[test]
    fn abstract_class_and_abstract_method() {
        let classes = parse_cs(
            "public abstract class Animal {\n\
             \x20   public abstract double Area();\n\
             }\n",
        );

        let animal = find(&classes, "Animal");
        assert_eq!(animal.kind, ClassKind::AbstractClass);
        assert!(animal.methods[0].is_abstract);
    }

    #[test]
    fn auto_property_is_treated_as_field() {
        let classes = parse_cs(
            "public class Animal {\n\
             \x20   public Person Owner { get; set; }\n\
             }\n",
        );

        let animal = find(&classes, "Animal");
        let owner = animal.fields.iter().find(|f| f.name == "Owner").unwrap();
        assert_eq!(owner.type_name, "Person");
        assert_eq!(owner.visibility, Visibility::Public);
    }

    #[test]
    fn enum_members_become_fields() {
        let classes = parse_cs("public enum Color { Red, Green, Blue }");
        let color = find(&classes, "Color");
        assert_eq!(color.kind, ClassKind::Enum);
        let names: Vec<_> = color.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["Red", "Green", "Blue"]);
    }

    #[test]
    fn field_visibility_and_static_modifier() {
        let classes = parse_cs(
            "public class Animal {\n\
             \x20   private static int Count = 0;\n\
             \x20   protected string Name;\n\
             }\n",
        );

        let animal = find(&classes, "Animal");
        let count = animal.fields.iter().find(|f| f.name == "Count").unwrap();
        assert_eq!(count.visibility, Visibility::Private);
        assert!(count.is_static);

        let name = animal.fields.iter().find(|f| f.name == "Name").unwrap();
        assert_eq!(name.visibility, Visibility::Protected);
    }
}
