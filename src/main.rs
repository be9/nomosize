use std::error::Error;
use std::fs;
use std::path::{Component, Path};

use clap::{Arg, App};
use pathdiff::diff_paths;
use serde::Deserialize;
use serde_json;

use walkdir::{DirEntry, WalkDir};

struct Package {
    name: String,
    version: String,
    path: String,
    disk_usage: i64,
}

// TODO: подсчёт, сколько весит
// TODO: рекурсия вглубь

fn main() {
    let matches = App::new("nomosize")
        .version("0.1.0")
        .author("Oleg Dashevskii <odashevskii@plaid.com>")
        .about("Calculates node_modules deps sizes")
        .arg(Arg::with_name("ROOT")
                 .help("The app root where top-level package.json and node_modules are")
                 .required(true)
                 .index(1))
        .get_matches();

    let mut packages: Vec<Package> = Vec::new();
    traverse(Path::new(matches.value_of("ROOT").unwrap()), &mut packages);
}

fn traverse(root: &Path, packages: &mut Vec<Package>) {
    let node_modules = root.join("node_modules");

    for entry in WalkDir::new(root.join("node_modules"))
                         .min_depth(1)
                         .max_depth(2)
                         .into_iter()
                         .filter_entry(|e| match e.metadata() {
                                               Ok(m) => m.is_dir(),
                                               Err(_) => false,
                                           }) {

        match entry {
            Ok(entry) => {
                let path = entry.path();
                let relative = diff_paths(path, &node_modules).unwrap();

                // Options for relative here:
                // 1. "package-name"
                // 2. "package-name/bogus-path"
                // 3. "@prefix"
                // 4. "@prefix/package-name"
                let components: Vec<_> = relative.components().map(|comp| comp.as_os_str()).collect();
                let prefix = components[0].to_str().unwrap().starts_with("@");
                if (components.len() == 1) {
                    if (prefix || components[0] == ".bin") {
                        // Dive deeper
                        continue;
                    }
                } else {
                    if (!prefix) {
                        // It's a subdir in the package, ignore
                        continue;
                    }
                }

                match get_package_info(path) {
                    Ok(package) => packages.push(package),
                    Err(err) => {
                        println!("ERR: {}", err);
                    },
                }
            },
            Err(err) => {
                let path = err.path().unwrap_or(Path::new("")).display();
                println!("failed to access entry {}", path);
            }
        }
    }
}

#[derive(Deserialize, Debug)]
struct PackageJson {
    name: String,
    version: String,
}

fn get_package_info(root: &Path) -> Result<Package, Box<dyn Error>> {
    let package_json = fs::read_to_string(root.join("package.json"))?;
    let package_info: PackageJson = serde_json::from_str(&package_json)?;

    return Ok(Package {
        name: package_info.name,
        version: package_info.version,
        path: root.to_str().unwrap().to_string(),
        disk_usage: 0,
    });
}
