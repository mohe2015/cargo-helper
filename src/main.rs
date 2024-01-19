use std::{collections::HashSet, fs, num::NonZeroU32, path::Path, sync::atomic::AtomicBool};

use cargo::{
    core::Workspace,
    ops::{fetch, FetchOptions},
    Config,
};
use gix::{
    bstr::BStr,
    progress,
    refs::file::transaction::prepare,
    remote::fetch::{self, Shallow},
    ThreadSafeRepository,
};
use serde_json::Value;

fn main() {
    println!("Checking supply chain security...");
    let config = Config::default().unwrap();
    let workspace = Workspace::new(
        Path::new("/home/moritz/Documents/perfect-group-allocation/Cargo.toml"),
        &config,
    )
    .unwrap();
    let fetch_options = FetchOptions {
        config: &config,
        targets: Vec::new(),
    };
    let (resolve, package_set) = fetch(&workspace, &fetch_options).unwrap();
    let packages = package_set.packages();
    let base_path = Path::new("tmp");
    for package in packages {
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
                /*
                fs::create_dir_all(&path).unwrap();
                let mut prepare_fetch = gix::prepare_clone(url.to_owned(), &path)
                    .unwrap()
                    .with_shallow(Shallow::DepthAtRemote(NonZeroU32::new(1).unwrap()));
                let (mut prepare_checkout, outcome) = prepare_fetch
                    .fetch_then_checkout(progress::Discard, &AtomicBool::new(false))
                    .unwrap();
                let (repository, outcome) = prepare_checkout
                    .main_worktree(progress::Discard, &AtomicBool::new(false))
                    .unwrap();
                */
                let _ = ThreadSafeRepository::init(
                    &path,
                    gix::create::Kind::WithWorktree,
                    gix::create::Options::default(),
                );
                let repository = ThreadSafeRepository::open(&path).unwrap();
                let repository = repository.to_thread_local();
                let remote = repository
                    .remote_at(url.to_owned())
                    .unwrap()
                    .with_refspecs(&[BStr::new(hash)], gix::remote::Direction::Fetch)
                    .unwrap();
                let connection = remote.connect(gix::remote::Direction::Fetch).unwrap();
                let prepare_fetch = connection
                    .prepare_fetch(progress::Discard, gix::remote::ref_map::Options::default())
                    .unwrap();
                let result = prepare_fetch
                    .with_shallow(Shallow::DepthAtRemote(NonZeroU32::new(1).unwrap()))
                    .receive(progress::Discard, &AtomicBool::new(false))
                    .unwrap();
                println!("{:#?}", result);
            }
        }
    }
}
