use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

use clap::Parser;
use nuke::cli::Args;
use nuke::delete::{delete_targets, force_remove};
use nuke::scan::scan;
use nuke::size::measure;
use nuke::targets::{match_target, Hit};

#[test]
fn test_cli_argument_parsing() {
    let args = Args::try_parse_from(["nuke"]).unwrap();
    assert_eq!(args.path, None);
    assert!(!args.yes);
    assert_eq!(args.depth, 4);
    assert!(!args.dry_run);
    assert!(args.target.is_empty());
    assert!(!args.force);

    // Test help flag parsing
    assert!(Args::try_parse_from(["nuke", "--help"]).is_err());
    assert!(Args::try_parse_from(["nuke", "-h"]).is_err());

    let args = Args::try_parse_from([
        "nuke",
        "D:/test/path",
        "-y",
        "-d",
        "2",
        "--dry-run",
        "-t",
        "cache,artifacts",
        "-f",
    ])
    .unwrap();

    assert_eq!(args.path.unwrap().to_str().unwrap(), "D:/test/path");
    assert!(args.yes);
    assert_eq!(args.depth, 2);
    assert!(args.dry_run);
    assert_eq!(args.target, vec!["cache", "artifacts"]);
    assert!(args.force);
}

#[test]
fn test_match_target_node() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // node_modules requires no validator
    let nm = root.join("node_modules");
    assert_eq!(
        match_target("node_modules", root, &nm, &[]),
        Some("Node".to_string())
    );

    // .next
    let next = root.join(".next");
    assert_eq!(
        match_target(".next", root, &next, &[]),
        Some("Next.js".to_string())
    );

    // dist without package.json sibling should not match
    let dist = root.join("dist");
    assert_eq!(match_target("dist", root, &dist, &[]), None);

    // dist with package.json sibling should match
    File::create(root.join("package.json")).unwrap();
    assert_eq!(
        match_target("dist", root, &dist, &[]),
        Some("Build/Dist".to_string())
    );
}

#[test]
fn test_match_target_dotnet() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let bin = root.join("bin");
    let obj = root.join("obj");

    // Without csproj or sln
    assert_eq!(match_target("bin", root, &bin, &[]), None);
    assert_eq!(match_target("obj", root, &obj, &[]), None);

    // Add csproj
    File::create(root.join("App.csproj")).unwrap();
    assert_eq!(
        match_target("bin", root, &bin, &[]),
        Some(".NET".to_string())
    );
    assert_eq!(
        match_target("obj", root, &obj, &[]),
        Some(".NET".to_string())
    );
}

#[test]
fn test_match_target_unity() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let lib = root.join("Library");
    let temp = root.join("Temp");

    // Without Assets or ProjectSettings
    assert_eq!(match_target("Library", root, &lib, &[]), None);
    assert_eq!(match_target("Temp", root, &temp, &[]), None);

    // Add Assets folder
    fs::create_dir(root.join("Assets")).unwrap();
    assert_eq!(
        match_target("Library", root, &lib, &[]),
        Some("Unity".to_string())
    );
    assert_eq!(
        match_target("Temp", root, &temp, &[]),
        Some("Unity".to_string())
    );
}

#[test]
fn test_match_target_rust() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let target_dir = root.join("target");

    // Without Cargo.toml
    assert_eq!(match_target("target", root, &target_dir, &[]), None);

    // With Cargo.toml
    File::create(root.join("Cargo.toml")).unwrap();
    assert_eq!(
        match_target("target", root, &target_dir, &[]),
        Some("Rust".to_string())
    );
}

#[test]
fn test_match_target_python() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let pycache = root.join("__pycache__");
    assert_eq!(
        match_target("__pycache__", root, &pycache, &[]),
        Some("Python".to_string())
    );

    let venv = root.join(".venv");
    assert_eq!(match_target(".venv", root, &venv, &[]), None);

    // With pyvenv.cfg inside .venv
    fs::create_dir(&venv).unwrap();
    File::create(venv.join("pyvenv.cfg")).unwrap();
    assert_eq!(
        match_target(".venv", root, &venv, &[]),
        Some("Python venv".to_string())
    );
}

#[test]
fn test_custom_targets() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let custom_dir = root.join("my_custom_cache");
    assert_eq!(
        match_target(
            "my_custom_cache",
            root,
            &custom_dir,
            &["my_custom_cache".to_string()]
        ),
        Some("Custom".to_string())
    );
}

#[test]
fn test_scan_and_measure() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // Create a mock node project: root/web/node_modules/pkg/index.js
    let node_dir = root.join("web").join("node_modules");
    fs::create_dir_all(&node_dir.join("pkg")).unwrap();

    {
        let mut file = File::create(node_dir.join("pkg").join("index.js")).unwrap();
        file.write_all(b"console.log('hello');").unwrap(); // 21 bytes
        file.flush().unwrap();
    }

    // Create a mock rust project: root/rust_app/target/debug/app.exe
    let rust_app = root.join("rust_app");
    fs::create_dir_all(&rust_app).unwrap();
    File::create(rust_app.join("Cargo.toml")).unwrap();

    let target_dir = rust_app.join("target").join("debug");
    fs::create_dir_all(&target_dir).unwrap();
    {
        let mut file2 = File::create(target_dir.join("app.exe")).unwrap();
        file2.write_all(&[0u8; 1024]).unwrap(); // 1024 bytes
        file2.flush().unwrap();
    }

    // Scan
    let hits = scan(root, 4, &[]);
    assert_eq!(hits.len(), 2);

    let measured = measure(hits);
    assert_eq!(measured.len(), 2);

    let total_size: u64 = measured.iter().map(|(_, s)| *s).sum();
    assert_eq!(total_size, 21 + 1024);
}

#[test]
fn test_scan_depth_limit() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // Depth:
    // root (0) -> a (1) -> b (2) -> c (3) -> d (4) -> node_modules (5)
    let deep_dir = root.join("a").join("b").join("c").join("d").join("node_modules");
    fs::create_dir_all(&deep_dir).unwrap();
    File::create(deep_dir.join("dummy.txt")).unwrap();

    // Max depth 4 should not find node_modules at depth 5
    let hits = scan(root, 4, &[]);
    assert_eq!(hits.len(), 0);

    // Max depth 5 should find it
    let hits5 = scan(root, 5, &[]);
    assert_eq!(hits5.len(), 1);
}

#[test]
fn test_force_remove_and_readonly() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let target_to_delete = root.join("node_modules");
    fs::create_dir_all(&target_to_delete).unwrap();

    let file_path = target_to_delete.join("locked.txt");
    {
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"read only content").unwrap();
    }

    // Set file to read-only
    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&file_path, perms).unwrap();

    // Ensure it was actually set readonly
    assert!(fs::metadata(&file_path).unwrap().permissions().readonly());

    // force_remove should succeed even with read-only file
    force_remove(&target_to_delete).unwrap();
    assert!(!target_to_delete.exists());
}

#[test]
fn test_delete_targets() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let dir1 = root.join("dir1");
    fs::create_dir(&dir1).unwrap();
    let mut f1 = File::create(dir1.join("file.txt")).unwrap();
    f1.write_all(b"12345").unwrap(); // 5 bytes

    let hits = vec![(
        Hit {
            path: dir1.clone(),
            kind: "Test".to_string(),
        },
        5u64,
    )];

    let result = delete_targets(&hits, None);
    assert_eq!(result.deleted_count, 1);
    assert_eq!(result.deleted_bytes, 5);
    assert!(result.failures.is_empty());
    assert!(!dir1.exists());
}
