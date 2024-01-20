use std::{fs, path::Path, process::Command};

use cargo::{core::Workspace, ops::fetch, Config};
use serde_json::Value;
use tracing::{error, info, info_span, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    // clear && RUST_LOG=cargo_helper=trace cargo run | tee report.txt
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
    for package in packages {
        let span = info_span!(
            "package",
            name = package.name().to_string(),
            version = package.version().to_string(),
            url = package.manifest().metadata().repository,
            sha1 = tracing::field::Empty,
            path_in_vcs = tracing::field::Empty
        );
        let _guard = span.enter();
        if package.package_id().source_id().is_path() {
            info!("skipping path dependency");
            continue;
        }
        let url = package.manifest().metadata().repository.as_ref();
        if let Some(url) = url {
            let url = url.split("/tree/").next().unwrap();
            let vcs_info = package.root().join(".cargo_vcs_info.json");
            let vcs_info = fs::read_to_string(vcs_info);
            let id = package.package_id().to_string();
            //println!("{id}");
            let path = base_path.join(id);
            let path_display = path.display();
            let registry_name_with_hash = package
                .root()
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            let path_in_vcs = if let Ok(vcs_info) = vcs_info {
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
                let path_in_vcs = vcs_info
                    .get("path_in_vcs")
                    .map(|value| value.as_str().unwrap())
                    .unwrap_or_default();
                //println!("{:?}", hash);
                span.record("sha1", hash);
                span.record("path_in_vcs", path_in_vcs);

                // we could also just call package in our clone but that is likely dangerous
                // TODO FIXME honor these
                /*println!(
                    "{:?} {:?}",
                    package.manifest().exclude(),
                    package.manifest().include()
                );*/

                //println!("REGISTRY NAME {}", registry_name_with_hash);

                // TODO progress bar
                // TODO check if the commit is associated with a tag or somehow hidden
                // TODO use remote set-url
                // TODO checkout subtree if only subtree? or maybe don't because of top level files?

                // TODO FIXME only clone and checkout once as commits are immutable
                if !path.join(".git/.done").exists() {
                    let command = format!(
                        r#"(mkdir -p "{path_display}" && cd "{path_display}" && git init && git fetch --depth=1 {url} {hash} && git checkout FETCH_HEAD && touch .git/.done)"#
                    );
                    //println!("{}", command);
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(&command)
                        .output()
                        .expect("failed to execute process");
                    if !output.status.success() {
                        error!(
                            "{}\n{}\n{}",
                            command,
                            std::str::from_utf8(&output.stderr).unwrap(),
                            std::str::from_utf8(&output.stdout).unwrap()
                        );
                        continue;
                    }
                } else {
                    //println!("already cloned, skipping")
                }

                path_in_vcs.to_owned()
            } else {
                warn!("no vcs version info, bisecting...");

                // TODO FIXME use path without spaces as they are more conventient
                let id = package.package_id().to_string();
                //println!("{id}");
                let path = base_path.join(id);
                let path_display = path.display();

                if !path.join(".git/.done").exists() {
                    let version = package.version().to_string();
                    let package_name = package.name().to_string();
                    let command = format!(
                        r#"(mkdir -p "{path_display}" && cd "{path_display}" && git init && git fetch --filter=tree:0 {url} && git bisect start FETCH_HEAD $(git rev-list --max-parents=0 FETCH_HEAD) && git bisect run sh -c '! echo -e -n "{version}\n$(cargo metadata --format-version=1 --no-deps | jq --raw-output ".packages[] | select(.name == \"{package_name}\") | .version")" | sort -V -C')"#,
                    );

                    // maybe find commit by release date? shouldn't make too much sense because maybe you test code and release then
                    //
                    // git bisect start FETCH_HEAD $(git rev-list --max-parents=0 FETCH_HEAD) && git bisect run sh -c '! echo -e -n "0.3.50\n$(cargo metadata --format-version=1 --no-deps | jq --raw-output ".packages[] | select(.name == \"js-sys\") | .version")" | sort -V -C'

                    //println!("{}", command);
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(&command)
                        .output()
                        .expect("failed to execute process");
                    if !output.status.success() {
                        error!(
                            "{}\n{}\n{}",
                            command,
                            std::str::from_utf8(&output.stderr).unwrap(),
                            std::str::from_utf8(&output.stdout).unwrap()
                        );
                        continue;
                    }
                } else {
                    //println!("already cloned, skipping")
                }

                String::new()
            };
            // TODO FIXME workspace paths e.g. thiserror-impl
            // maybe really use cargo package as there are so many small details

            // cargo package --no-verify --package package-name

            // target/package/*.crate

            // ~/.cargo/registry/cache/index.crates.io-6f17d22bba15001f/

            // diffoscope

            // diffoscope /home/moritz/Documents/cargo-helper/tmp/thiserror-impl\ v1.0.56/target/package/thiserror-impl-1.0.56.crate ~/.cargo/registry/cache/index.crates.io-6f17d22bba15001f/thiserror-impl-1.0.56.crate

            // only package and extract once to reduce disk strain

            let target_directory = {
                let command = format!(
                    r#"(cd "{path_display}/{path_in_vcs}" && cargo metadata --format-version 1 --no-deps | jq --raw-output .target_directory)"#,
                );
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .output()
                    .expect("failed to execute process");
                if !output.status.success() {
                    error!(
                        "{}\n{}\n{}",
                        command,
                        std::str::from_utf8(&output.stderr).unwrap(),
                        std::str::from_utf8(&output.stdout).unwrap()
                    );
                    continue;
                } else {
                    let output = std::str::from_utf8(&output.stdout).unwrap().trim();
                    if output.is_empty() {
                        error!("failed to find target directory");
                        continue;
                    }
                    Path::new(output).parent().unwrap().join("target")
                }
            };
            let target_directory_display = target_directory.display();
            //println!("the target directory: \"{}\"", target_directory_display);

            if !target_directory.join(".done").exists() {
                let command = format!(
                    r#"(cd "{path_display}/{path_in_vcs}" && cargo package --allow-dirty --no-verify --package {} && tar -xf "{target_directory_display}/package/{}" -C "{target_directory_display}" && touch "{target_directory_display}/{}/.cargo-ok" && touch "{target_directory_display}/.done")"#,
                    package.name(),
                    package.package_id().tarball_name(),
                    package
                        .package_id()
                        .tarball_name()
                        .trim_end_matches(".crate")
                );
                //println!("{command}");
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .output()
                    .expect("failed to execute process");
                if !output.status.success() {
                    error!(
                        "{}\n{}\n{}",
                        command,
                        std::str::from_utf8(&output.stderr).unwrap(),
                        std::str::from_utf8(&output.stdout).unwrap()
                    );
                    continue;
                }
            } else {
                //println!("already packaged, skipping")
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

            // a few packages are distributed with whitespace changes only
            let command = format!(
                r#"diff -w --color -r --exclude ".github" --exclude ".cargo-ok" --exclude ".cargo_vcs_info.json" --exclude "Cargo.toml" --exclude "Cargo.lock" "{target_directory_display}/{}" "{crates_io}""#,
                package
                    .package_id()
                    .tarball_name()
                    .trim_end_matches(".crate"),
            );
            // println!("{command}");
            let output = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output()
                .expect("failed to execute process");
            if !output.status.success() {
                error!(
                    "{}\n{}\n{}",
                    command,
                    std::str::from_utf8(&output.stderr).unwrap(),
                    std::str::from_utf8(&output.stdout).unwrap()
                );
                continue;
            }
        } else {
            error!("no repository url, can't check anything");
        }
    }
}
