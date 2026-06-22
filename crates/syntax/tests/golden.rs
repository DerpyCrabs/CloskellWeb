use std::{fs, path::PathBuf};

#[test]
fn syntax_golden_files() {
    let fixture_dir = workspace_fixture_dir("syntax");
    for name in ["core", "html"] {
        let input = fs::read_to_string(fixture_dir.join(format!("{}.clsk", name))).unwrap();
        let expected = fs::read_to_string(fixture_dir.join(format!("{}.pretty", name))).unwrap();
        let source = syntax::parse_source(&input);

        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);
        assert_eq!(source.pretty().trim(), expected.trim(), "{}", name);
    }
}

fn workspace_fixture_dir(kind: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(kind)
}
