use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ClassKind {
    Class,
    Interface,
    Enum,
    AbstractClass,
    Trait,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldNode {
    pub name: String,
    pub type_name: String,
    pub visibility: Visibility,
    pub is_static: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Param {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodNode {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_abstract: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassNode {
    pub id: String,
    pub name: String,
    pub kind: ClassKind,
    pub file: String,
    pub line: u32,
    pub fields: Vec<FieldNode>,
    pub methods: Vec<MethodNode>,
    pub extends: Vec<String>,
    pub implements: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum RelationshipKind {
    Inheritance,
    Implementation,
    Composition,
    Aggregation,
    Association,
    Dependency,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub kind: RelationshipKind,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClassDiagram {
    pub classes: Vec<ClassNode>,
    pub relationships: Vec<Relationship>,
    pub generated_at_ms: u64,
}
