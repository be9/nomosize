use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

#[macro_use] extern crate clap;
#[macro_use] extern crate prettytable;

use clap::{Arg, App};
use humansize::{FileSize, file_size_opts as options};
use pathdiff::diff_paths;
use prettytable::Table;
use serde::Deserialize;
use serde_json;

use walkdir::WalkDir;

#[derive(Debug)]
struct Package {
    name: String,
    version: String,
    path: String,
    disk_usage: u64,
}

#[derive(Clone, Debug)]
struct PackageWithAllVersions {
    name: String,
    versions: Vec<String>,
    paths: Vec<String>,
    disk_usage: u64,
}

fn main() {
    let matches = App::new("nomosize")
        .version("0.1.0")
        .author("Oleg Dashevskii <odashevskii@plaid.com>")
        .about("Calculates node_modules deps sizes")
        .arg(Arg::with_name("ROOT")
                 .help("The app root where top-level package.json and node_modules are")
                 .required(true)
                 .index(1))
        .arg(Arg::with_name("top")
                 .short("t")
                 .long("top")
                 .value_name("COUNT")
                 .help("How many packages to display (default: 10)")
                 .takes_value(true))
        .arg(Arg::with_name("sort")
                 .short("s")
                 .long("sort")
                 .value_name("SORT")
                 .help("How to sort packages")
                 .takes_value(true)
                 .possible_values(&["size", "versions"]))
        .arg(Arg::with_name("merge")
                 .short("m")
                 .help("Merge multiple versions into one record"))
        .get_matches();

    let mut packages: Vec<Package> = Vec::new();
    let root = Path::new(matches.value_of("ROOT").unwrap());
    traverse(root, &mut packages);

    let total_size = packages.iter().fold(0, |acc, p| acc + p.disk_usage);

    println!("Found {} package(s), consuming {} total",
        packages.len(),
        total_size.file_size(options::BINARY).unwrap());

    let top = value_t!(matches, "top", usize).unwrap_or(10);

    let mut table = Table::new();
    table.add_row(row!["Package", "Version(s)", "Disk Usage", "Path(s)"]);

    let mut total_listed_size: u64 = 0;

    if matches.is_present("merge") {
        let mut packages_with_all_versions = collect_versions(&packages);

        if matches.value_of("sort").unwrap_or("size") == "versions" {
            packages_with_all_versions.sort_by(
                |a, b| b.versions.len().cmp(&a.versions.len())
            );
        } else {
            packages_with_all_versions.sort_by(
                |a, b| b.disk_usage.cmp(&a.disk_usage)
            );
        }

        for p in &packages_with_all_versions[..top] {
            table.add_row(row![
                p.name,
                p.versions.join("\n"),
                p.disk_usage.file_size(options::BINARY).unwrap(),
                p.paths.iter().map(|path| diff_paths(Path::new(path), &root).unwrap())
                    .map(|pathbuf| pathbuf.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ]);

            total_listed_size += p.disk_usage;
        }
    } else {
        packages.sort_by(|a, b| b.disk_usage.cmp(&a.disk_usage));

        for p in &packages[..top] {
            let relative = diff_paths(Path::new(&p.path), &root).unwrap();
            total_listed_size += p.disk_usage;

            table.add_row(row![
                p.name,
                p.version,
                p.disk_usage.file_size(options::BINARY).unwrap(),
                relative.display(),
            ]);
        }
    }

    table.printstd();

    println!("The size of the packages listed above = {} (~{:.1}% of the whole bloat)",
        total_listed_size.file_size(options::BINARY).unwrap(),
        (total_listed_size as f64) / (total_size as f64) * 100.0);
}

fn traverse(root: &Path, packages: &mut Vec<Package>) {
    let node_modules = root.join("node_modules");

    if !node_modules.is_dir() {
        return;
    }

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
                if components.len() == 1 {
                    if prefix || components[0] == ".bin" {
                        continue;
                    }
                } else if !prefix {
                    // It's a subdir in the package, ignore
                    continue;
                }

                match get_package_info(path) {
                    Ok(package) => {
                        packages.push(package);

                        traverse(path, packages);
                    },
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
        disk_usage: calc_disk_usage(root),
    });
}

fn calc_disk_usage(path: &Path) -> u64 {
    let total_size = WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .fold(0, |acc, m| acc + m.len());

    return total_size;
}

fn collect_versions(packages: &[Package]) -> Vec<PackageWithAllVersions> {
    let mut packages_by_name = HashMap::<&str, Vec<&Package>>::new();

    for p in packages {
        packages_by_name.entry(&p.name)
            .or_insert_with(Vec::new)
            .push(&p);
    }

    let mut result = Vec::<PackageWithAllVersions>::new();

    for (name, package_versions) in &packages_by_name {
        result.push(PackageWithAllVersions {
            name: String::from(*name),
            versions: package_versions.iter().map(|p| String::from(&p.version)).collect(),
            paths: package_versions.iter().map(|p| String::from(&p.path)).collect(),
            disk_usage: package_versions.iter().fold(0, |acc, p| acc + p.disk_usage),
        });
    }

    return result;
}
