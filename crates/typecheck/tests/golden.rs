use std::{fs, path::PathBuf};

#[test]
fn typecheck_golden_files() {
    let fixture_dir = workspace_fixture_dir("typecheck");
    let input = fs::read_to_string(fixture_dir.join("core.clsk")).unwrap();
    let expected = fs::read_to_string(fixture_dir.join("core.types")).unwrap();
    let source = syntax::parse_source(&input);
    assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

    let html = typecheck::CheckType::named("Html");
    let result = typecheck::check_source_with_module_imports_and_options(
        &source,
        &[],
        &[],
        typecheck::CheckOptions::default()
            .named_type("Html", 0)
            .named_type("TrustedHtml", 0)
            .html_templates(html, typecheck::CheckType::named("TrustedHtml")),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);

    let actual = result
        .forms
        .iter()
        .map(|form| format!("{} : {}", form.source, form.ty))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(actual.trim(), expected.trim());
}

fn workspace_fixture_dir(kind: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(kind)
}
