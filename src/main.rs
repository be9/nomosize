use std::error::Error;
use std::fs;
use std::path::Path;

use clap::{Arg, App};
use serde::Deserialize;
use serde_json;

use walkdir::WalkDir;

struct Package {
	name: String,
	version: String,
	path: String,
	disk_usage: i64,
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
        .get_matches();

	let mut packages: Vec<Package> = Vec::new();
    traverse(Path::new(matches.value_of("ROOT").unwrap()), &mut packages);
}

fn traverse(root: &Path, packages: &mut Vec<Package>) {
	let node_modules = root.join("node_modules");

	for entry in WalkDir::new(node_modules).max_depth(1) {
	    match entry {
    	    Ok(entry) => {
    	    	// println!("{}", entry.path().display());
    	    	let path = entry.path();
    	    	if path.is_dir() {
    	    		match get_package_info(path) {
    	    			Ok(package) => packages.push(package),
    	    			Err(err) => {
		    	    		println!("ERR: {}", err);
    	    			},
    	    		}
    	    	} else {
    	    		println!("WARN: unexpected file {}", path.display());
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
	println!("{:?}", root.join("package.json"));
	let packageJson = fs::read_to_string(root.join("package.json"))?;
    let packageInfo: PackageJson = serde_json::from_str(&packageJson)?;

    return Ok(Package {
    	name: packageInfo.name,
    	version: packageInfo.version,
    	path: root.to_str().unwrap().to_string(),
    	disk_usage: 0,
    });
}
