# CLI Test Examples

Run from the project root (`C:\Users\harun\Desktop\Developments\ocean`).

## Build

```
cargo build
```

## Help

```
cargo run --bin ocean -- --help
cargo run --bin ocean -- help read
cargo run --bin ocean -- help graph
cargo run --bin ocean -- help index
cargo run --bin ocean -- help vector-search
```

---

## TXT (`test-cwd/test_0.txt` — "test content")

```
cargo run --bin ocean -- info test-cwd/test_0.txt
cargo run --bin ocean -- read test-cwd/test_0.txt --skip 0 --take 1
cargo run --bin ocean -- search test-cwd/test_0.txt content
cargo run --bin ocean -- chunk test-cwd/test_0.txt
```

---

## Markdown (`test-cwd/test_15.md`)

```
cargo run --bin ocean -- info test-cwd/test_15.md
cargo run --bin ocean -- outline test-cwd/test_15.md
cargo run --bin ocean -- read test-cwd/test_15.md --heading "Heading"
cargo run --bin ocean -- read test-cwd/test_15.md --paragraph 0
cargo run --bin ocean -- search test-cwd/test_15.md Para
cargo run --bin ocean -- chunk test-cwd/test_15.md
```

---

## HTML (`test-cwd/test_8.html` — contains `h1` + `p`)

```
cargo run --bin ocean -- info test-cwd/test_8.html
cargo run --bin ocean -- outline test-cwd/test_8.html
cargo run --bin ocean -- read test-cwd/test_8.html --heading "Title"
cargo run --bin ocean -- read test-cwd/test_8.html --paragraph 0
cargo run --bin ocean -- search test-cwd/test_8.html Para
cargo run --bin ocean -- chunk test-cwd/test_8.html
```

---

## DOCX (`test-cwd/15.docx`, `test-cwd/2025.docx`)

```
cargo run --bin ocean -- info test-cwd/15.docx
cargo run --bin ocean -- info test-cwd/2025.docx
cargo run --bin ocean -- read test-cwd/15.docx --paragraph 0
cargo run --bin ocean -- read test-cwd/2025.docx --skip 0 --take 3
cargo run --bin ocean -- search test-cwd/2025.docx የ
cargo run --bin ocean -- chunk test-cwd/15.docx
cargo run --bin ocean -- chunk test-cwd/2025.docx
```

---

## XLSX (`test-cwd/AAIAHC_Organizational_Diagnostic_Tools.xlsx`)

```
cargo run --bin ocean -- info test-cwd/AAIAHC_Organizational_Diagnostic_Tools.xlsx
cargo run --bin ocean -- outline test-cwd/AAIAHC_Organizational_Diagnostic_Tools.xlsx
cargo run --bin ocean -- read test-cwd/AAIAHC_Organizational_Diagnostic_Tools.xlsx --sheet "Skills Inventory Matrix"
cargo run --bin ocean -- read test-cwd/AAIAHC_Organizational_Diagnostic_Tools.xlsx --cell A1
cargo run --bin ocean -- search test-cwd/AAIAHC_Organizational_Diagnostic_Tools.xlsx Staff
cargo run --bin ocean -- chunk test-cwd/AAIAHC_Organizational_Diagnostic_Tools.xlsx
```

---

## PPTX (`test-cwd/AAIAHC_Strong_Org_Culture_Presentation.pptx`)

```
cargo run --bin ocean -- info test-cwd/AAIAHC_Strong_Org_Culture_Presentation.pptx
cargo run --bin ocean -- outline test-cwd/AAIAHC_Strong_Org_Culture_Presentation.pptx
cargo run --bin ocean -- page-count test-cwd/AAIAHC_Strong_Org_Culture_Presentation.pptx
cargo run --bin ocean -- read test-cwd/AAIAHC_Strong_Org_Culture_Presentation.pptx --slide 3
cargo run --bin ocean -- read test-cwd/AAIAHC_Strong_Org_Culture_Presentation.pptx --skip 0 --take 2
cargo run --bin ocean -- search test-cwd/AAIAHC_Strong_Org_Culture_Presentation.pptx Islamic
cargo run --bin ocean -- chunk test-cwd/AAIAHC_Strong_Org_Culture_Presentation.pptx
```

---

## PDF (`test-cwd/022-article-A016-en.pdf`)

```
cargo run --bin ocean -- info test-cwd/022-article-A016-en.pdf
cargo run --bin ocean -- page-count test-cwd/022-article-A016-en.pdf
cargo run --bin ocean -- outline test-cwd/022-article-A016-en.pdf
cargo run --bin ocean -- read test-cwd/022-article-A016-en.pdf --page 1
cargo run --bin ocean -- search test-cwd/022-article-A016-en.pdf the
cargo run --bin ocean -- chunk test-cwd/022-article-A016-en.pdf
```

---

## General Commands

```
cargo run --bin ocean -- scan test-cwd
cargo run --bin ocean -- scan test-cwd --no-hash
cargo run --bin ocean -- hash test-cwd/test_0.txt
cargo run --bin ocean -- hash test-cwd/15.docx
cargo run --bin ocean -- verify test-cwd/test_0.txt 6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72
```

---

## Graph Commands (no indexing needed for syntax check)

```
cargo run --bin ocean -- graph stats
cargo run --bin ocean -- graph stats --db-path test.db
cargo run --bin ocean -- graph help
cargo run --bin ocean -- help graph info
cargo run --bin ocean -- help graph expand
cargo run --bin ocean -- help graph path
cargo run --bin ocean -- help graph stats
cargo run --bin ocean -- graph info test-cwd --db-path test.db
```

---

## Index (requires Ollama or other embedding provider)

```
cargo run --bin ocean -- index test-cwd --db-path test.db --no-graph
```

---

## Vector Search (requires indexed DB)

```
cargo run --bin ocean -- vector-search "culture" --db-path test.db --top-k 5
cargo run --bin ocean -- vector-search "strategy" --db-path test.db --top-k 3 --expand-depth 1
cargo run --bin ocean -- vector-search "budget" --db-path test.db --hybrid
```
