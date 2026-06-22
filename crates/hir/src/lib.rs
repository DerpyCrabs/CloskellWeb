use syntax::{Diagnostic, Expr, SourceFile};

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub forms: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LowerResult {
    pub module: Module,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn lower_source(source: &SourceFile) -> LowerResult {
    LowerResult {
        module: Module {
            forms: source.forms.clone(),
        },
        diagnostics: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_preserves_forms() {
        let source = syntax::parse_source("(def x 1)");
        let lowered = lower_source(&source);

        assert!(lowered.diagnostics.is_empty());
        assert_eq!(lowered.module.forms.len(), 1);
    }
}
