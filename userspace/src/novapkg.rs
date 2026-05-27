use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub dependencies: Vec<String>,
}

pub struct PackageDatabase {
    pub available_packages: HashMap<String, Package>,
    pub installed_packages: HashSet<String>,
}

static PKG_DB: Mutex<Option<PackageDatabase>> = Mutex::new(None);

pub fn init_novapkg() {
    let mut available = HashMap::new();
    
    // Add default library dependencies
    available.insert("libc".to_string(), Package {
        name: "libc".to_string(),
        version: "2.35".to_string(),
        description: "NovaOS Standard C Library bindings".to_string(),
        dependencies: Vec::new(),
    });
    available.insert("libm".to_string(), Package {
        name: "libm".to_string(),
        version: "1.0.0".to_string(),
        description: "NovaMath math operations library".to_string(),
        dependencies: Vec::new(),
    });

    // Add utilities and packages
    available.insert("shell-utils".to_string(), Package {
        name: "shell-utils".to_string(),
        version: "1.0.0".to_string(),
        description: "Core CLI utilities for filesystem interaction (pwd, cd, mkdir, cat)".to_string(),
        dependencies: vec!["libc".to_string()],
    });
    available.insert("network-tools".to_string(), Package {
        name: "network-tools".to_string(),
        version: "1.0.0".to_string(),
        description: "Network card stack interface tools (ping, sshd, socket)".to_string(),
        dependencies: vec!["libc".to_string()],
    });

    // Add compilers and interpreters
    available.insert("gcc".to_string(), Package {
        name: "gcc".to_string(),
        version: "12.2.0".to_string(),
        description: "GNU Compiler Collection C compiler".to_string(),
        dependencies: vec!["libc".to_string(), "libm".to_string()],
    });
    available.insert("python".to_string(), Package {
        name: "python".to_string(),
        version: "3.11.2".to_string(),
        description: "Python Programming Language Interpreter".to_string(),
        dependencies: vec!["libc".to_string(), "libm".to_string()],
    });
    available.insert("git".to_string(), Package {
        name: "git".to_string(),
        version: "2.39.0".to_string(),
        description: "Distributed version control system".to_string(),
        dependencies: vec!["libc".to_string()],
    });

    let mut installed = HashSet::new();
    installed.insert("libc".to_string()); // standard lib already present

    *PKG_DB.lock().unwrap() = Some(PackageDatabase {
        available_packages: available,
        installed_packages: installed,
    });
}

// Resolve dependencies and install package
pub fn install_package(name: &str) -> Result<Vec<String>, String> {
    let mut db_lock = PKG_DB.lock().unwrap();
    let db = db_lock.as_mut().ok_or("Package manager not initialized")?;

    if !db.available_packages.contains_key(name) {
        return Err(format!("Package '{}' not found in NovaOS repository", name));
    }

    if db.installed_packages.contains(name) {
        return Err(format!("Package '{}' is already installed", name));
    }

    // Dependency Resolution (DFS top-sort)
    let mut resolved_install_order = Vec::new();
    let mut visited = HashSet::new();
    let mut temporary = HashSet::new();

    fn resolve(
        node: &str,
        db: &PackageDatabase,
        resolved: &mut Vec<String>,
        visited: &mut HashSet<String>,
        temp: &mut HashSet<String>,
    ) -> Result<(), String> {
        if temp.contains(node) {
            return Err("Cyclic dependency detected!".to_string());
        }
        if !visited.contains(node) {
            temp.insert(node.to_string());
            if let Some(pkg) = db.available_packages.get(node) {
                for dep in &pkg.dependencies {
                    resolve(dep, db, resolved, visited, temp)?;
                }
            } else {
                return Err(format!("Unresolved dependency: {}", node));
            }
            temp.remove(node);
            visited.insert(node.to_string());
            resolved.push(node.to_string());
        }
        Ok(())
    }

    resolve(name, db, &mut resolved_install_order, &mut visited, &mut temporary)?;

    // Filter out already installed components
    let mut actual_install = Vec::new();
    for pkg in resolved_install_order {
        if !db.installed_packages.contains(&pkg) {
            db.installed_packages.insert(pkg.clone());
            actual_install.push(pkg);
        }
    }

    Ok(actual_install)
}

// Remove package
pub fn remove_package(name: &str) -> Result<(), String> {
    let mut db_lock = PKG_DB.lock().unwrap();
    let db = db_lock.as_mut().unwrap();

    if !db.installed_packages.contains(name) {
        return Err(format!("Package '{}' is not installed", name));
    }

    // Check if other installed packages depend on this
    for inst in &db.installed_packages {
        if inst != name {
            if let Some(pkg) = db.available_packages.get(inst) {
                if pkg.dependencies.contains(&name.to_string()) {
                    return Err(format!("Cannot remove '{}': Package '{}' depends on it", name, inst));
                }
            }
        }
    }

    db.installed_packages.remove(name);
    Ok(())
}

// Get installed packages
pub fn list_installed() -> Vec<String> {
    let db_lock = PKG_DB.lock().unwrap();
    if let Some(ref db) = *db_lock {
        db.installed_packages.iter().cloned().collect()
    } else {
        Vec::new()
    }
}

// Get all packages
pub fn get_all_available() -> Vec<Package> {
    let db_lock = PKG_DB.lock().unwrap();
    if let Some(ref db) = *db_lock {
        db.available_packages.values().cloned().collect()
    } else {
        Vec::new()
    }
}
