pub mod csharp;
pub mod ecma;
pub mod java;
pub mod php;
pub mod python;

use crate::model::ClassNode;
use std::path::Path;

pub fn parse_file(path: &Path, source: &str) -> Vec<ClassNode> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("ts") | Some("tsx") => ecma::parse(path, source, ecma::Dialect::TypeScript),
        Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => {
            ecma::parse(path, source, ecma::Dialect::JavaScript)
        }
        Some("py") => python::parse(path, source),
        Some("java") => java::parse(path, source),
        Some("cs") => csharp::parse(path, source),
        Some("php") => php::parse(path, source),
        _ => Vec::new(),
    }
}
