use std::collections::HashSet;
use std::fs;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_end_to_end_scan_hash_filter_normalize() {
    let dir = tempdir().unwrap();

    fs::write(dir.path().join("doc1.pdf"), b"%PDF-1.4 content").unwrap();
    fs::write(dir.path().join("doc2.txt"), b"Hello, World!").unwrap();
    fs::write(dir.path().join("image.png"), b"PNG content").unwrap();
    fs::write(dir.path().join("ignore.exe"), b"should be ignored").unwrap();
    fs::write(dir.path().join(".hidden.md"), b"hidden file").unwrap();

    let results = ocean_doc::ocean_fs::scan_dir(dir.path().to_str().unwrap()).unwrap();
    assert_eq!(results.len(), 3);

    for meta in &results {
        assert_eq!(meta.hash.len(), 64);
        assert!(meta.modified > 0);

        let normalized = ocean_doc::ocean_fs::normalize(meta.clone());
        assert_eq!(normalized.id, meta.id);
        assert_eq!(normalized.meta.hash, meta.hash);

        match meta.extension.as_str() {
            "pdf" => {
                assert_eq!(normalized.category, ocean_doc::ocean_fs::FileCategory::Document);
                assert_eq!(normalized.mime_type, "application/pdf");
            }
            "txt" => {
                assert_eq!(normalized.category, ocean_doc::ocean_fs::FileCategory::Text);
                assert_eq!(normalized.mime_type, "text/plain");
            }
            "png" => {
                assert_eq!(normalized.category, ocean_doc::ocean_fs::FileCategory::Image);
                assert_eq!(normalized.mime_type, "image/png");
            }
            _ => panic!("unexpected extension: {}", meta.extension),
        }
    }
}

#[test]
fn test_deterministic_scan() {
    let dir = tempdir().unwrap();

    fs::create_dir_all(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("a.txt"), b"aaa").unwrap();
    fs::write(dir.path().join("sub").join("b.txt"), b"bbb").unwrap();

    let results1 = ocean_doc::ocean_fs::scan_dir(dir.path().to_str().unwrap()).unwrap();
    let results2 = ocean_doc::ocean_fs::scan_dir(dir.path().to_str().unwrap()).unwrap();

    let mut paths1: Vec<&str> = results1.iter().map(|m| m.path.as_str()).collect();
    let mut paths2: Vec<&str> = results2.iter().map(|m| m.path.as_str()).collect();
    paths1.sort();
    paths2.sort();

    assert_eq!(paths1, paths2);

    for (m1, m2) in results1.iter().zip(results2.iter()) {
        assert_eq!(m1.hash, m2.hash);
        assert_eq!(m1.size, m2.size);
        assert_eq!(m1.extension, m2.extension);
    }
}

#[test]
fn test_path_resolver_move_chain() {
    let resolver = ocean_doc::ocean_fs::PathResolver::in_memory().unwrap();
    let file_id = ocean_doc::ocean_fs::generate_file_id();

    resolver
        .record_move(&file_id, "/docs/v1/report.pdf", "/docs/v2/report.pdf")
        .unwrap();
    resolver
        .record_move(&file_id, "/docs/v2/report.pdf", "/archive/report.pdf")
        .unwrap();

    let resolved = resolver.resolve_path(&file_id).unwrap();
    assert_eq!(resolved, "/archive/report.pdf");

    let history = resolver.get_move_history(&file_id);
    assert_eq!(history.len(), 2);
}

#[test]
fn test_filter_ignores_unsupported_dirs() {
    let dir = tempdir().unwrap();

    fs::create_dir_all(dir.path().join("node_modules")).unwrap();
    fs::create_dir_all(dir.path().join(".git")).unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();

    fs::write(
        dir.path().join("node_modules").join("lib.js"),
        b"code",
    )
    .unwrap();
    fs::write(dir.path().join(".git").join("config"), b"config").unwrap();
    fs::write(dir.path().join("src").join("main.rs"), b"fn main() {}").unwrap();
    fs::write(dir.path().join("readme.md"), b"# Readme").unwrap();

    let results = ocean_doc::ocean_fs::scan_dir(dir.path().to_str().unwrap()).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("readme.md"));
}

#[test]
fn test_large_directory_scan() {
    let dir = tempdir().unwrap();

    for i in 0..500 {
        let mut f = fs::File::create(dir.path().join(format!("file_{}.txt", i))).unwrap();
        f.write_all(format!("content_{}", i).as_bytes()).unwrap();
    }

    let results = ocean_doc::ocean_fs::scan_dir(dir.path().to_str().unwrap()).unwrap();
    assert_eq!(results.len(), 500);

    let unique_ids: HashSet<&str> =
        results.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(unique_ids.len(), 500);
}

#[test]
fn test_hash_consistent_across_normalize() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.md");
    fs::write(&path, b"# Markdown Content").unwrap();

    let metas = ocean_doc::ocean_fs::scan_dir(dir.path().to_str().unwrap()).unwrap();
    assert_eq!(metas.len(), 1);

    let normalized = ocean_doc::ocean_fs::normalize(metas[0].clone());
    assert_eq!(normalized.meta.hash, ocean_doc::ocean_fs::hasher::hash_file(path.to_str().unwrap()).unwrap());
}

#[test]
fn test_no_duplicate_ids_in_scan() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    fs::write(dir.path().join("b.txt"), b"b").unwrap();
    fs::write(dir.path().join("c.txt"), b"c").unwrap();

    let results = ocean_doc::ocean_fs::scan_dir(dir.path().to_str().unwrap()).unwrap();
    assert_eq!(results.len(), 3);

    let id_set: HashSet<&str> =
        results.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(id_set.len(), 3);
}

#[test]
fn test_verify_hash() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.txt");
    fs::write(&path, b"verify me").unwrap();

    let hash = ocean_doc::ocean_fs::hasher::hash_file(path.to_str().unwrap()).unwrap();
    assert!(ocean_doc::ocean_fs::hasher::verify_hash(path.to_str().unwrap(), &hash));
    assert!(!ocean_doc::ocean_fs::hasher::verify_hash(
        path.to_str().unwrap(),
        "0000000000000000000000000000000000000000000000000000000000000000"
    ));
}
