use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use cargo::{core::Workspace, ops::fetch, Config};
use serde_json::Value;

fn main() {
    // clear && cargo run | tee report.txt
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
    let (_resolve, package_set) = fetch(&workspace, &fetch_options).unwrap();
    let packages: Vec<_> = package_set.packages().collect();
    let base_path = Path::new("tmp");
    for (index, package) in packages.iter().enumerate() {
        println!(
            "[{index}/{}] {}",
            packages.len(),
            package.package_id().tarball_name()
        );
        let url = package.manifest().metadata().repository.as_ref();
        if let Some(url) = url {
            //println!("url: {url}");
            let vcs_info = package.root().join(".cargo_vcs_info.json");
            let vcs_info = fs::read_to_string(vcs_info);
            if let Ok(vcs_info) = vcs_info {
                let vcs_info = serde_json::from_str::<Value>(&vcs_info).unwrap();
                //println!("{:?}", vcs_info);
                let git = vcs_info
                    .as_object()
                    .unwrap()
                    .get("git")
                    .unwrap()
                    .as_object()
                    .unwrap();
                let hash = git.get("sha1").unwrap().as_str().unwrap();
                let _path_in_vcs = git
                    .get("path_in_vcs")
                    .map(|value| value.as_str().unwrap())
                    .unwrap_or_default();
                //println!("{:?}", hash);

                let id = package.package_id().to_string();
                //println!("{id}");
                let path = base_path.join(id);
                let path_display = path.display();

                // we could also just call package in our clone but that is likely dangerous
                // TODO FIXME honor these
                /*println!(
                    "{:?} {:?}",
                    package.manifest().exclude(),
                    package.manifest().include()
                );*/

                let registry_name_with_hash = package
                    .root()
                    .parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap();
                //println!("REGISTRY NAME {}", registry_name_with_hash);

                // TODO progress bar
                // TODO check if the commit is associated with a tag or somehow hidden
                // TODO use remote set-url
                // TODO checkout subtree if only subtree? or maybe don't because of top level files?

                // TODO FIXME only clone and checkout once as commits are immutable
                let command = format!(
                    r#"(mkdir -p "{path}" && cd "{path}" && git init && git fetch --depth=1 {url} {hash} && git checkout FETCH_HEAD)"#
                );
                println!("{}", command);
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .output()
                    .expect("failed to execute process");
                if !output.status.success() {
                    println!(
                        "failed {} {}",
                        std::str::from_utf8(&output.stderr).unwrap(),
                        std::str::from_utf8(&output.stdout).unwrap()
                    );
                    continue;
                }

                // TODO FIXME workspace paths e.g. thiserror-impl
                // maybe really use cargo package as there are so many small details

                // cargo package --no-verify --package package-name

                // target/package/*.crate

                // ~/.cargo/registry/cache/index.crates.io-6f17d22bba15001f/

                // diffoscope

                // diffoscope /home/moritz/Documents/cargo-helper/tmp/thiserror-impl\ v1.0.56/target/package/thiserror-impl-1.0.56.crate ~/.cargo/registry/cache/index.crates.io-6f17d22bba15001f/thiserror-impl-1.0.56.crate

                // only package and extract once to reduce disk strain

                if !path.join("target/.done").exists() {
                    let command = format!(
                        r#"(cd "{path_display}" && cargo package --no-verify --package {} && tar -xf target/package/{} -C target && touch target/{}/.cargo-ok && touch target/.done)"#,
                        package.name(),
                        package.package_id().tarball_name(),
                        package
                            .package_id()
                            .tarball_name()
                            .trim_end_matches(".crate")
                    );
                    println!("{command}");
                    let _output = Command::new("sh")
                        .arg("-c")
                        .arg(command)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .expect("failed to execute process");
                } else {
                    println!("already cloned, skipping")
                }

                /*let registry_source_id = SourceId::alt_registry(
                    &config,
                    package.package_id().source_id().alt_registry_key().unwrap(),
                )
                .unwrap();*/
                /*let registry_source_id = SourceId::alt_registry(
                                    &config,
                                    package.package_id().source_id().alt_registry_key().unwrap(),
                                )
                                .unwrap();
                                let mut registry_source =
                                    registry_source_id.load(&config, &HashSet::new()).unwrap();
                                println!("registry source id: {}", registry_source_id);
                                let maybe_package = registry_source.download(package.package_id()).unwrap();
                                println!(
                                    "tetslfe {:#?}",
                                    match maybe_package {
                                        cargo::sources::source::MaybePackage::Ready(file) => file,
                                        cargo::sources::source::MaybePackage::Download {
                                            url,
                                            descriptor,
                                            authorization,
                                        } => unreachable!(),
                                    }
                                );
                */

                // mkdir -p tmp/adler\ v1.0.2/target/unpacked  && tar -xf tmp/adler\ v1.0.2/target/package/adler-1.0.2.crate -C tmp/adler\ v1.0.2/target/unpacked

                let crates_io_crate_file = config
                    .registry_cache_path()
                    .join(registry_name_with_hash)
                    .join(package.package_id().tarball_name());
                //println!("test {}", crates_io_crate_file.as_path_unlocked().display());
                let _crates_io_crate_file = crates_io_crate_file.display();
                let crates_io = package.root();
                let crates_io = crates_io.display();

                // diffoscope --exclude-directory-metadata=yes tmp/adler\ v1.0.2/target/unpacked/adler-1.0.2/ ~/.cargo/registry/src/index.crates.io-6f17d22bba15001f/adler-1.0.2/

                let command = format!(
                    r#"diffoscope --exclude "**/.cargo-ok" --exclude "**/.cargo_vcs_info.json" --exclude "**/Cargo.toml" --exclude "**/Cargo.lock" --exclude-directory-metadata=recursive "{path}/target/{}" {crates_io}"#,
                    package
                        .package_id()
                        .tarball_name()
                        .trim_end_matches(".crate"),
                );
                println!("{command}");
                let _output = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .status()
                    .expect("failed to execute process");
            } else {
                println!("no vcs version info, would need to guess or bruteforce version")
            }
        } else {
            println!("no repository url, can't check anything");
        }
    }
}
