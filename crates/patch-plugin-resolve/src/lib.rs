//! Resolve plugin install order from declared dependencies.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPackage {
    pub id: String,
    pub version: String,
    pub requires: Vec<PluginDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub package: String,
    pub version_req: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("missing dependency package: {0}")]
    Missing(String),
}

/// Validates dependencies and returns root + packages in declaration order.
pub fn resolve_install_set(
    root: &str,
    root_version: &str,
    packages: &[PluginPackage],
) -> Result<Vec<String>, ResolveError> {
    for pkg in packages {
        for dep in &pkg.requires {
            if dep.package == root {
                continue;
            }
            if !packages.iter().any(|p| p.id == dep.package) {
                return Err(ResolveError::Missing(dep.package.clone()));
            }
        }
    }
    let mut out = vec![format!("{}@{}", root, root_version)];
    for pkg in packages {
        out.push(format!("{}@{}", pkg.id, pkg.version));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_simple_chain() {
        let packages = vec![
            PluginPackage {
                id: "dep-a".into(),
                version: "1.0".into(),
                requires: vec![],
            },
            PluginPackage {
                id: "main".into(),
                version: "2.0".into(),
                requires: vec![PluginDependency {
                    package: "dep-a".into(),
                    version_req: "*".into(),
                }],
            },
        ];
        let out = resolve_install_set("network", "1", &packages).unwrap();
        assert_eq!(out.len(), 3);
    }
}
