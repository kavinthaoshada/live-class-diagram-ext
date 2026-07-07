use crate::model::{ClassDiagram, ClassNode, Relationship, RelationshipKind};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn build_diagram(classes: Vec<ClassNode>) -> ClassDiagram {
    let name_set: HashSet<&str> = classes.iter().map(|c| c.name.as_str()).collect();
    let mut relationships: Vec<Relationship> = Vec::new();
    let mut seen: HashSet<Relationship> = HashSet::new();

    for class in &classes {
        for base in &class.extends {
            add_relationship(
                &mut relationships,
                &mut seen,
                &name_set,
                class,
                base,
                RelationshipKind::Inheritance,
            );
        }
        for iface in &class.implements {
            add_relationship(
                &mut relationships,
                &mut seen,
                &name_set,
                class,
                iface,
                RelationshipKind::Implementation,
            );
        }

        let field_type_names: HashSet<&str> =
            class.fields.iter().map(|f| f.type_name.as_str()).collect();

        for field in &class.fields {
            let kind = if is_collection_type(&field.type_name) {
                RelationshipKind::Aggregation
            } else {
                RelationshipKind::Composition
            };
            for referenced in extract_referenced_types(&field.type_name, &name_set, &class.name) {
                add_relationship(&mut relationships, &mut seen, &name_set, class, &referenced, kind);
            }
        }

        for method in &class.methods {
            let mut candidates: Vec<String> = Vec::new();
            for param in &method.params {
                candidates.extend(extract_referenced_types(&param.type_name, &name_set, &class.name));
            }
            candidates.extend(extract_referenced_types(&method.return_type, &name_set, &class.name));

            for referenced in candidates {
                if field_type_names.iter().any(|t| t.contains(referenced.as_str())) {
                    continue;
                }
                add_relationship(
                    &mut relationships,
                    &mut seen,
                    &name_set,
                    class,
                    &referenced,
                    RelationshipKind::Dependency,
                );
            }
        }
    }

    ClassDiagram {
        classes,
        relationships,
        generated_at_ms: now_ms(),
    }
}

fn add_relationship(
    list: &mut Vec<Relationship>,
    seen: &mut HashSet<Relationship>,
    known: &HashSet<&str>,
    class: &ClassNode,
    target: &str,
    kind: RelationshipKind,
) {
    if target == class.name || !known.contains(target) {
        return;
    }
    let relationship = Relationship {
        from: class.name.clone(),
        to: target.to_string(),
        kind,
    };
    if seen.insert(relationship.clone()) {
        list.push(relationship);
    }
}

fn is_collection_type(type_name: &str) -> bool {
    type_name.ends_with("[]")
        || type_name.starts_with("Array<")
        || type_name.starts_with("Set<")
        || type_name.starts_with("Map<")
        || type_name.starts_with("ReadonlyArray<")
}

fn extract_referenced_types(type_name: &str, known: &HashSet<&str>, self_name: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, found: &mut Vec<String>| {
        if !current.is_empty() {
            if known.contains(current.as_str())
                && current.as_str() != self_name
                && !found.iter().any(|f| f == current)
            {
                found.push(current.clone());
            }
            current.clear();
        }
    };
    for ch in type_name.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            flush(&mut current, &mut found);
        }
    }
    flush(&mut current, &mut found);
    found
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ClassKind, FieldNode, MethodNode, Param, Visibility};

    fn class(name: &str, kind: ClassKind) -> ClassNode {
        ClassNode {
            id: name.to_string(),
            name: name.to_string(),
            kind,
            file: "test.ts".to_string(),
            line: 1,
            fields: Vec::new(),
            methods: Vec::new(),
            extends: Vec::new(),
            implements: Vec::new(),
        }
    }

    fn field(name: &str, type_name: &str) -> FieldNode {
        FieldNode {
            name: name.to_string(),
            type_name: type_name.to_string(),
            visibility: Visibility::Public,
            is_static: false,
        }
    }

    fn method(name: &str, params: &[(&str, &str)], return_type: &str) -> MethodNode {
        MethodNode {
            name: name.to_string(),
            params: params
                .iter()
                .map(|(n, t)| Param { name: n.to_string(), type_name: t.to_string() })
                .collect(),
            return_type: return_type.to_string(),
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
        }
    }

    fn find<'a>(diagram: &'a ClassDiagram, from: &str, to: &str) -> Option<&'a Relationship> {
        diagram.relationships.iter().find(|r| r.from == from && r.to == to)
    }

    #[test]
    fn inheritance_edge_between_known_classes() {
        let mut dog = class("Dog", ClassKind::Class);
        dog.extends.push("Animal".to_string());
        let animal = class("Animal", ClassKind::Class);

        let diagram = build_diagram(vec![dog, animal]);

        let rel = find(&diagram, "Dog", "Animal").expect("expected Dog -> Animal edge");
        assert_eq!(rel.kind, RelationshipKind::Inheritance);
    }

    #[test]
    fn extends_unknown_class_produces_no_edge() {
        let mut dog = class("Dog", ClassKind::Class);
        dog.extends.push("SomeExternalBaseClass".to_string());

        let diagram = build_diagram(vec![dog]);

        assert!(diagram.relationships.is_empty());
    }

    #[test]
    fn implementation_edge_for_interface() {
        let mut animal = class("Animal", ClassKind::Class);
        animal.implements.push("Shape".to_string());
        let shape = class("Shape", ClassKind::Interface);

        let diagram = build_diagram(vec![animal, shape]);

        let rel = find(&diagram, "Animal", "Shape").expect("expected Animal -> Shape edge");
        assert_eq!(rel.kind, RelationshipKind::Implementation);
    }

    #[test]
    fn single_value_field_is_composition() {
        let mut animal = class("Animal", ClassKind::Class);
        animal.fields.push(field("owner", "Person"));
        let person = class("Person", ClassKind::Class);

        let diagram = build_diagram(vec![animal, person]);

        let rel = find(&diagram, "Animal", "Person").expect("expected Animal -> Person edge");
        assert_eq!(rel.kind, RelationshipKind::Composition);
    }

    #[test]
    fn array_field_is_aggregation() {
        let mut animal = class("Animal", ClassKind::Class);
        animal.fields.push(field("friends", "Animal[]"));
        let diagram = build_diagram(vec![animal]);

        // A self-referential field should not create a self-loop edge.
        assert!(diagram.relationships.is_empty());

        let mut zoo = class("Zoo", ClassKind::Class);
        zoo.fields.push(field("animals", "Animal[]"));
        let animal = class("Animal", ClassKind::Class);
        let diagram = build_diagram(vec![zoo, animal]);

        let rel = find(&diagram, "Zoo", "Animal").expect("expected Zoo -> Animal edge");
        assert_eq!(rel.kind, RelationshipKind::Aggregation);
    }

    #[test]
    fn method_param_not_held_as_field_is_dependency() {
        let mut dog = class("Dog", ClassKind::Class);
        dog.methods.push(method("fetch", &[("item", "Toy")], "void"));
        let toy = class("Toy", ClassKind::Class);

        let diagram = build_diagram(vec![dog, toy]);

        let rel = find(&diagram, "Dog", "Toy").expect("expected Dog -> Toy edge");
        assert_eq!(rel.kind, RelationshipKind::Dependency);
    }

    #[test]
    fn method_param_already_a_field_does_not_duplicate_as_dependency() {
        let mut service = class("UserService", ClassKind::Class);
        service.fields.push(field("repo", "UserRepository"));
        service
            .methods
            .push(method("save", &[("repo", "UserRepository")], "void"));
        let repo = class("UserRepository", ClassKind::Class);

        let diagram = build_diagram(vec![service, repo]);

        let edges: Vec<_> = diagram
            .relationships
            .iter()
            .filter(|r| r.from == "UserService" && r.to == "UserRepository")
            .collect();
        assert_eq!(edges.len(), 1, "expected exactly one edge, got {edges:?}");
        assert_eq!(edges[0].kind, RelationshipKind::Composition);
    }

    #[test]
    fn generic_return_type_links_to_inner_type() {
        let mut repo = class("Repository", ClassKind::Class);
        repo.methods.push(method("find", &[], "Promise<User>"));
        let user = class("User", ClassKind::Class);

        let diagram = build_diagram(vec![repo, user]);

        let rel = find(&diagram, "Repository", "User").expect("expected Repository -> User edge");
        assert_eq!(rel.kind, RelationshipKind::Dependency);
    }

    #[test]
    fn no_duplicate_edges_for_same_relationship() {
        let mut dog = class("Dog", ClassKind::Class);
        dog.methods.push(method("fetch", &[("item", "Toy")], "Toy"));
        let toy = class("Toy", ClassKind::Class);

        let diagram = build_diagram(vec![dog, toy]);

        let edges: Vec<_> = diagram
            .relationships
            .iter()
            .filter(|r| r.from == "Dog" && r.to == "Toy")
            .collect();
        assert_eq!(edges.len(), 1, "expected relationship set to dedupe, got {edges:?}");
    }
}
