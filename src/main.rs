use std::{fs, path::Path, process::Command};

use cargo::{core::Workspace, ops::fetch, Config};
use serde_json::Value;

fn main() {
    println!("Checking supply chain security...");
    let config = Config::default().unwrap();
    let workspace = Workspace::new(
        Path::new("/home/moritz/Documents/perfect-group-allocation/Cargo.toml"),
        &config,
    )
    .unwrap();
    let fetch_options = cargo::ops::FetchOptions {
        config: &config,
        targets: Vec::new(),
    };
    let (resolve, package_set) = fetch(&workspace, &fetch_options).unwrap();
    let packages: Vec<_> = package_set.packages().collect();
    let base_path = Path::new("tmp");
    for (index, package) in packages.iter().enumerate() {
        println!("{index}/{}", packages.len());
        let url = package.manifest().metadata().repository.as_ref();
        if let Some(url) = url {
            println!("url: {url}");
            let vcs_info = package.root().join(".cargo_vcs_info.json");
            let vcs_info = fs::read_to_string(vcs_info);
            if let Ok(vcs_info) = vcs_info {
                let vcs_info = serde_json::from_str::<Value>(&vcs_info).unwrap();
                println!("{:?}", vcs_info);
                let hash = vcs_info
                    .as_object()
                    .unwrap()
                    .get("git")
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .get("sha1")
                    .unwrap()
                    .as_str()
                    .unwrap();
                println!("{:?}", hash);

                let id = package.package_id().to_string();
                println!("{id}");
                let path = base_path.join(id);
                let path = path.display();

                // TODO progress bar
                // TODO check if the commit is associated with a tag or somehow hidden
                // TODO use remote set-url
                // TODO checkout subtree if only subtree? or maybe don't because of top level files?
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(format!(r#"(mkdir -p "{path}" && cd "{path}" && git init && (git remote add origin {url} || exit 0) && git fetch --depth=1 origin {hash}:{hash} && git checkout {hash})"#))
                    .output()
                    .expect("failed to execute process");
                println!("{output:?}");

                let crates_io = package.root();
            }
        }
    }
}
