use std::path::Path;

use ocean_doc::ocean_fs::parser::*;

const TEST_DIR: &str = "tests/test-cwd";

#[test]
fn test_real_files() {
    let dir = Path::new(TEST_DIR);
    assert!(dir.exists(), "test-cwd directory not found");

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.path())
        .collect();
    entries.sort();

    let mut passed = 0u32;
    let mut failed = 0u32;

    for path in &entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        match open(&path.to_string_lossy()) {
            Ok(doc) => {
                println!("  OK  {}  (size: {}, format: {:?})",
                    name, doc.metadata().size, doc.metadata().format);

                let meta = doc.metadata();
                assert_eq!(meta.path, path.as_path());
                let search_result = doc.search("the");

                if matches!(ext.as_str(), "docx" | "pdf" | "pptx") {
                    println!("       search matches for 'the': {}",
                        search_result.len());
                }

                if matches!(ext.as_str(), "docx" | "pdf") {
                    let page_read = doc.read(&Selector::Page(1));
                    println!("       read Page(1): {:?}",
                        page_read.as_ref().map(|r| std::mem::discriminant(r)));
                }

                if ext == "docx" {
                    let page_test = doc.read(&Selector::Page(1));
                    if page_test.is_ok() {
                        println!("       read Page(1): OK");
                    }
                }

                if ext == "pptx" {
                    let slide_test = doc.read(&Selector::Slide(1));
                    println!("       read Slide(1): {:?}",
                        slide_test.as_ref().map(|r| std::mem::discriminant(r)));
                }

                if ext == "xlsx" {
                    let sheet_test = doc.read(&Selector::Sheet("Sheet1".to_string()));
                    println!("       read Sheet1: {:?}",
                        sheet_test.as_ref().map(|r| std::mem::discriminant(r)));
                }

                if matches!(ext.as_str(), "txt" | "md" | "html") {
                    let p0 = doc.read(&Selector::Slice { skip: 0, take: 1 });
                    println!("       read Slice(0,1): {:?}",
                        p0.as_ref().map(|r| std::mem::discriminant(r)));
                }
            }
            Err(e) => {
                let is_expected = matches!(ext.as_str(), "doc" | "xyz" | "xls" | "ppt");
                if is_expected {
                    println!("  OK  {}  (unsupported format: {})", name, e);
                    passed += 1;
                } else {
                    println!("  FAIL {}  {}", name, e);
                    failed += 1;
                }
            }
        }
    }

    println!("\nResults: {} passed, {} failed out of {} files", passed, failed, entries.len());
    assert_eq!(failed, 0, "{} files failed to open", failed);
}
