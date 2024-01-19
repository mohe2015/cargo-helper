use std::path::Path;

use cargo::{
    core::Workspace,
    ops::{fetch, FetchOptions},
    Config,
};
use gix::ThreadSafeRepository;

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
        println!("{:?}", package.manifest().metadata().repository);
        let id = package.package_id().to_string();
        println!("{id}");
        let path = base_path.join(id);
        let _ = ThreadSafeRepository::init(
            &path,
            gix::create::Kind::WithWorktree,
            gix::create::Options::default(),
        );
        let repository = ThreadSafeRepository::open(&path).unwrap();
        let repository = repository.to_thread_local();
    }
}
