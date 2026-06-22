use std::collections::{HashMap, HashSet};

use syntax::{
    Diagnostic, Expr, ExprKind, HtmlAttr, HtmlAttrValue, HtmlElement, HtmlNode, SourceFile, Span,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Expansion {
    pub source: SourceFile,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MacroDef {
    params: Vec<String>,
    template: Expr,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MacroDefinitions {
    pub macros: HashMap<String, MacroDef>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Default)]
struct Expander {
    macros: HashMap<String, MacroDef>,
    imported_macro_names: HashSet<String>,
    diagnostics: Vec<Diagnostic>,
    gensym_counter: usize,
}

struct MacroInvocation<'a> {
    env: HashMap<String, Expr>,
    gensyms: HashMap<String, String>,
    invocation_span: Span,
    expander: &'a mut Expander,
}

pub fn expand_source(source: &SourceFile) -> Expansion {
    expand_source_with_imported_macros(source, &HashMap::new())
}

pub fn expand_source_with_imported_macros(
    source: &SourceFile,
    imported_macros: &HashMap<String, MacroDef>,
) -> Expansion {
    let mut expander = Expander {
        macros: imported_macros.clone(),
        imported_macro_names: imported_macros.keys().cloned().collect(),
        diagnostics: Vec::new(),
        gensym_counter: 0,
    };
    let mut forms = Vec::new();

    for form in &source.forms {
        if let Some((name, macro_def)) = parse_macro_def(form, &mut expander.diagnostics) {
            expander.macros.insert(name, macro_def);
            continue;
        }

        if let Some(pruned) = expander.prune_imported_macro_names(form) {
            if let Some(form) = pruned {
                forms.push(expander.expand_expr(&form, 0));
            }
            continue;
        }

        forms.push(expander.expand_expr(form, 0));
    }

    Expansion {
        source: SourceFile {
            forms,
            diagnostics: source.diagnostics.clone(),
        },
        diagnostics: expander.diagnostics,
    }
}

pub fn collect_macro_defs(source: &SourceFile) -> MacroDefinitions {
    let mut definitions = MacroDefinitions::default();
    for form in &source.forms {
        if let Some((name, macro_def)) = parse_macro_def(form, &mut definitions.diagnostics) {
            definitions.macros.insert(name, macro_def);
        }
    }
    definitions
}

impl Expander {
    fn prune_imported_macro_names(&self, expr: &Expr) -> Option<Option<Expr>> {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        if items.len() != 3 || !matches_symbol(&items[0], "import") {
            return None;
        }
        let ExprKind::Vector(names) = &items[2].kind else {
            return Some(Some(expr.clone()));
        };

        let kept = names
            .iter()
            .filter(|name| match &name.kind {
                ExprKind::Symbol(name) => !self.imported_macro_names.contains(name),
                _ => true,
            })
            .cloned()
            .collect::<Vec<_>>();
        if kept.len() == names.len() {
            return Some(Some(expr.clone()));
        }
        if kept.is_empty() {
            return Some(None);
        }

        Some(Some(Expr::new(
            ExprKind::List(vec![
                items[0].clone(),
                items[1].clone(),
                Expr::new(ExprKind::Vector(kept), items[2].span),
            ]),
            expr.span,
        )))
    }

    fn expand_expr(&mut self, expr: &Expr, depth: usize) -> Expr {
        if depth > 128 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "macro expansion exceeded the recursion limit",
            ));
            return expr.clone();
        }

        match &expr.kind {
            ExprKind::List(items) => self.expand_list(expr, items, depth),
            ExprKind::Vector(items) => Expr::new(
                ExprKind::Vector(
                    items
                        .iter()
                        .map(|item| self.expand_expr(item, depth))
                        .collect(),
                ),
                expr.span,
            ),
            ExprKind::Map(entries) => Expr::new(
                ExprKind::Map(
                    entries
                        .iter()
                        .map(|(key, value)| {
                            (self.expand_expr(key, depth), self.expand_expr(value, depth))
                        })
                        .collect(),
                ),
                expr.span,
            ),
            ExprKind::Set(items) => Expr::new(
                ExprKind::Set(
                    items
                        .iter()
                        .map(|item| self.expand_expr(item, depth))
                        .collect(),
                ),
                expr.span,
            ),
            ExprKind::HtmlTemplate(node) => Expr::new(
                ExprKind::HtmlTemplate(Box::new(self.expand_html_node(node, depth))),
                expr.span,
            ),
            ExprKind::Unquote(_) | ExprKind::UnquoteSplicing(_) => {
                self.diagnostics.push(Diagnostic::error(
                    expr.span,
                    "unquote can only appear inside a macro quasiquote",
                ));
                expr.clone()
            }
            ExprKind::Quote(_) | ExprKind::QuasiQuote(_) => expr.clone(),
            ExprKind::Nil
            | ExprKind::Bool(_)
            | ExprKind::Number(_)
            | ExprKind::String(_)
            | ExprKind::Keyword(_)
            | ExprKind::Symbol(_) => expr.clone(),
        }
    }

    fn expand_list(&mut self, expr: &Expr, items: &[Expr], depth: usize) -> Expr {
        let Some((head, args)) = items.split_first() else {
            return expr.clone();
        };

        if let ExprKind::Symbol(name) = &head.kind {
            if name == "compile-error" {
                self.report_compile_error(expr.span, args);
                return Expr::new(ExprKind::Nil, expr.span);
            }

            if let Some(def) = self.macros.get(name).cloned() {
                if def.params.len() != args.len() {
                    self.diagnostics.push(Diagnostic::error(
                        expr.span,
                        format!(
                            "macro `{}` expects {} arguments, found {}",
                            name,
                            def.params.len(),
                            args.len()
                        ),
                    ));
                    return expr.clone();
                }

                let env = def
                    .params
                    .iter()
                    .cloned()
                    .zip(args.iter().cloned())
                    .collect::<HashMap<_, _>>();
                let mut invocation = MacroInvocation {
                    env,
                    gensyms: HashMap::new(),
                    invocation_span: expr.span,
                    expander: self,
                };
                let expanded = invocation.expand_template(&def.template);
                return invocation.expander.expand_expr(&expanded, depth + 1);
            }
        }

        Expr::new(
            ExprKind::List(
                items
                    .iter()
                    .map(|item| self.expand_expr(item, depth))
                    .collect(),
            ),
            expr.span,
        )
    }

    fn expand_html_node(&mut self, node: &HtmlNode, depth: usize) -> HtmlNode {
        match node {
            HtmlNode::Element(element) => HtmlNode::Element(HtmlElement {
                tag: element.tag.clone(),
                attrs: element
                    .attrs
                    .iter()
                    .map(|attr| self.expand_html_attr(attr, depth))
                    .collect(),
                children: element
                    .children
                    .iter()
                    .map(|child| self.expand_html_node(child, depth))
                    .collect(),
                self_closing: element.self_closing,
                span: element.span,
            }),
            HtmlNode::Text { .. } => node.clone(),
            HtmlNode::Expr { expr, span } => HtmlNode::Expr {
                expr: Box::new(self.expand_expr(expr, depth)),
                span: *span,
            },
        }
    }

    fn expand_html_attr(&mut self, attr: &HtmlAttr, depth: usize) -> HtmlAttr {
        let value = match &attr.value {
            HtmlAttrValue::Dynamic { expr, span } => HtmlAttrValue::Dynamic {
                expr: Box::new(self.expand_expr(expr, depth)),
                span: *span,
            },
            HtmlAttrValue::Bool(_) | HtmlAttrValue::Static(_) => attr.value.clone(),
        };
        HtmlAttr {
            name: attr.name.clone(),
            value,
            span: attr.span,
        }
    }

    fn report_compile_error(&mut self, span: Span, args: &[Expr]) {
        let [message] = args else {
            self.diagnostics.push(Diagnostic::error(
                span,
                "compile-error expects one string message",
            ));
            return;
        };

        let ExprKind::String(message) = &message.kind else {
            self.diagnostics.push(Diagnostic::error(
                message.span,
                "compile-error expects one string message",
            ));
            return;
        };

        self.diagnostics
            .push(Diagnostic::error(span, message.clone()));
    }
}

impl MacroInvocation<'_> {
    fn expand_template(&mut self, expr: &Expr) -> Expr {
        match &expr.kind {
            ExprKind::QuasiQuote(inner) => self.expand_quasiquote(inner),
            ExprKind::Symbol(name) => self.env.get(name).cloned().unwrap_or_else(|| expr.clone()),
            ExprKind::List(items) => self.expand_template_list(expr, items),
            ExprKind::Vector(items) => Expr::new(
                ExprKind::Vector(
                    items
                        .iter()
                        .map(|item| self.expand_template(item))
                        .collect(),
                ),
                self.invocation_span,
            ),
            ExprKind::Map(entries) => Expr::new(
                ExprKind::Map(
                    entries
                        .iter()
                        .map(|(key, value)| {
                            (self.expand_template(key), self.expand_template(value))
                        })
                        .collect(),
                ),
                self.invocation_span,
            ),
            ExprKind::Set(items) => Expr::new(
                ExprKind::Set(
                    items
                        .iter()
                        .map(|item| self.expand_template(item))
                        .collect(),
                ),
                self.invocation_span,
            ),
            ExprKind::HtmlTemplate(node) => Expr::new(
                ExprKind::HtmlTemplate(Box::new(self.expand_template_html_node(node))),
                self.invocation_span,
            ),
            ExprKind::Quote(_)
            | ExprKind::Unquote(_)
            | ExprKind::UnquoteSplicing(_)
            | ExprKind::Nil
            | ExprKind::Bool(_)
            | ExprKind::Number(_)
            | ExprKind::String(_)
            | ExprKind::Keyword(_) => expr.clone(),
        }
    }

    fn expand_template_list(&mut self, expr: &Expr, items: &[Expr]) -> Expr {
        let Some((head, args)) = items.split_first() else {
            return Expr::new(ExprKind::List(Vec::new()), self.invocation_span);
        };

        if matches_symbol(head, "do") {
            return self.expand_macro_do(args);
        }

        if matches_symbol(head, "let") {
            return self.expand_macro_let(expr, args);
        }

        if matches_symbol(head, "with-gensyms") {
            return self.expand_with_gensyms(expr, args);
        }

        if matches_symbol(head, "gensym") {
            return self.expand_gensym_call(expr, args);
        }

        if matches_symbol(head, "compile-error") {
            self.report_macro_compile_error(args);
            return Expr::new(ExprKind::Nil, self.invocation_span);
        }

        Expr::new(
            ExprKind::List(
                items
                    .iter()
                    .map(|item| self.expand_template(item))
                    .collect(),
            ),
            self.invocation_span,
        )
    }

    fn expand_macro_do(&mut self, forms: &[Expr]) -> Expr {
        let mut result = Expr::new(ExprKind::Nil, self.invocation_span);
        for form in forms {
            result = self.expand_template(form);
        }
        result
    }

    fn expand_with_gensyms(&mut self, expr: &Expr, args: &[Expr]) -> Expr {
        let [names_expr, body @ ..] = args else {
            self.expander.diagnostics.push(Diagnostic::error(
                self.invocation_span,
                "with-gensyms expects a name vector and body",
            ));
            return Expr::new(ExprKind::Nil, self.invocation_span);
        };

        let ExprKind::Vector(names) = &names_expr.kind else {
            self.expander.diagnostics.push(Diagnostic::error(
                names_expr.span,
                "with-gensyms names must be a vector",
            ));
            return Expr::new(ExprKind::Nil, self.invocation_span);
        };

        let previous_env = self.env.clone();
        for name_expr in names {
            let ExprKind::Symbol(name) = &name_expr.kind else {
                self.expander.diagnostics.push(Diagnostic::error(
                    name_expr.span,
                    "with-gensyms name must be a symbol",
                ));
                self.env = previous_env;
                return Expr::new(ExprKind::Nil, self.invocation_span);
            };
            let fresh = self.fresh_gensym(name);
            self.env.insert(
                name.clone(),
                Expr::new(ExprKind::Symbol(fresh), self.invocation_span),
            );
        }

        let result = self.expand_macro_do(body);
        self.env = previous_env;
        if body.is_empty() {
            self.expander.diagnostics.push(Diagnostic::error(
                expr.span,
                "with-gensyms expects a body expression",
            ));
        }
        result
    }

    fn expand_macro_let(&mut self, expr: &Expr, args: &[Expr]) -> Expr {
        let [bindings_expr, body @ ..] = args else {
            self.expander.diagnostics.push(Diagnostic::error(
                self.invocation_span,
                "macro let expects a binding vector and body",
            ));
            return Expr::new(ExprKind::Nil, self.invocation_span);
        };

        let ExprKind::Vector(bindings) = &bindings_expr.kind else {
            self.expander.diagnostics.push(Diagnostic::error(
                self.invocation_span,
                "macro let bindings must be a vector",
            ));
            return Expr::new(ExprKind::Nil, self.invocation_span);
        };

        if bindings.len() % 2 != 0 {
            self.expander.diagnostics.push(Diagnostic::error(
                self.invocation_span,
                "macro let bindings must contain name/value pairs",
            ));
            return Expr::new(ExprKind::Nil, self.invocation_span);
        }

        let previous_env = self.env.clone();
        for pair in bindings.chunks(2) {
            let [name_expr, value_expr] = pair else {
                continue;
            };
            let ExprKind::Symbol(name) = &name_expr.kind else {
                self.expander.diagnostics.push(Diagnostic::error(
                    self.invocation_span,
                    "macro let binding name must be a symbol",
                ));
                self.env = previous_env;
                return Expr::new(ExprKind::Nil, self.invocation_span);
            };
            let value = self.expand_template(value_expr);
            self.env.insert(name.clone(), value);
        }

        let result = self.expand_macro_do(body);
        self.env = previous_env;
        if body.is_empty() {
            self.expander.diagnostics.push(Diagnostic::error(
                expr.span,
                "macro let expects a body expression",
            ));
        }
        result
    }

    fn expand_gensym_call(&mut self, expr: &Expr, args: &[Expr]) -> Expr {
        let base = match args {
            [] => "g".to_string(),
            [prefix] => match &self.expand_template(prefix).kind {
                ExprKind::String(prefix) if prefix.is_empty() => "g".to_string(),
                ExprKind::String(prefix) => prefix.clone(),
                _ => {
                    self.expander.diagnostics.push(Diagnostic::error(
                        self.invocation_span,
                        "gensym expects zero arguments or one string prefix",
                    ));
                    return expr.clone();
                }
            },
            _ => {
                self.expander.diagnostics.push(Diagnostic::error(
                    self.invocation_span,
                    "gensym expects zero arguments or one string prefix",
                ));
                return expr.clone();
            }
        };

        Expr::new(
            ExprKind::Symbol(self.fresh_gensym(&base)),
            self.invocation_span,
        )
    }

    fn report_macro_compile_error(&mut self, args: &[Expr]) {
        let [message] = args else {
            self.expander.diagnostics.push(Diagnostic::error(
                self.invocation_span,
                "compile-error expects one string message",
            ));
            return;
        };

        let expanded = self.expand_template(message);
        let ExprKind::String(message) = &expanded.kind else {
            self.expander.diagnostics.push(Diagnostic::error(
                self.invocation_span,
                "compile-error expects one string message",
            ));
            return;
        };

        self.expander
            .diagnostics
            .push(Diagnostic::error(self.invocation_span, message.clone()));
    }

    fn expand_quasiquote(&mut self, expr: &Expr) -> Expr {
        match &expr.kind {
            ExprKind::Unquote(inner) => self.expand_unquote(inner),
            ExprKind::UnquoteSplicing(_) => {
                self.expander.diagnostics.push(Diagnostic::error(
                    expr.span,
                    "unquote-splicing can only appear inside a list or vector",
                ));
                expr.clone()
            }
            ExprKind::List(items) => Expr::new(
                ExprKind::List(self.expand_quasiquote_sequence(items)),
                self.invocation_span,
            ),
            ExprKind::Vector(items) => Expr::new(
                ExprKind::Vector(self.expand_quasiquote_sequence(items)),
                self.invocation_span,
            ),
            ExprKind::Map(entries) => Expr::new(
                ExprKind::Map(
                    entries
                        .iter()
                        .map(|(key, value)| {
                            (self.expand_quasiquote(key), self.expand_quasiquote(value))
                        })
                        .collect(),
                ),
                self.invocation_span,
            ),
            ExprKind::Set(items) => Expr::new(
                ExprKind::Set(
                    items
                        .iter()
                        .map(|item| self.expand_quasiquote(item))
                        .collect(),
                ),
                self.invocation_span,
            ),
            ExprKind::Symbol(name) if name.ends_with('#') => Expr::new(
                ExprKind::Symbol(self.gensym_for(name.trim_end_matches('#'))),
                self.invocation_span,
            ),
            ExprKind::HtmlTemplate(node) => Expr::new(
                ExprKind::HtmlTemplate(Box::new(self.expand_quasiquote_html_node(node))),
                self.invocation_span,
            ),
            ExprKind::Quote(_)
            | ExprKind::QuasiQuote(_)
            | ExprKind::Nil
            | ExprKind::Bool(_)
            | ExprKind::Number(_)
            | ExprKind::String(_)
            | ExprKind::Keyword(_)
            | ExprKind::Symbol(_) => expr.clone(),
        }
    }

    fn expand_quasiquote_sequence(&mut self, items: &[Expr]) -> Vec<Expr> {
        let mut expanded = Vec::new();
        for item in items {
            if let ExprKind::UnquoteSplicing(inner) = &item.kind {
                let replacement = self.expand_unquote(inner);
                match replacement.kind {
                    ExprKind::List(items) | ExprKind::Vector(items) => expanded.extend(items),
                    _ => self.expander.diagnostics.push(Diagnostic::error(
                        item.span,
                        "unquote-splicing expected a list or vector argument",
                    )),
                }
            } else {
                expanded.push(self.expand_quasiquote(item));
            }
        }
        expanded
    }

    fn expand_unquote(&mut self, expr: &Expr) -> Expr {
        match &expr.kind {
            ExprKind::Symbol(name) => self.env.get(name).cloned().unwrap_or_else(|| expr.clone()),
            _ => self.expand_template(expr),
        }
    }

    fn expand_template_html_node(&mut self, node: &HtmlNode) -> HtmlNode {
        match node {
            HtmlNode::Element(element) => HtmlNode::Element(HtmlElement {
                tag: element.tag.clone(),
                attrs: element
                    .attrs
                    .iter()
                    .map(|attr| self.expand_template_html_attr(attr))
                    .collect(),
                children: element
                    .children
                    .iter()
                    .map(|child| self.expand_template_html_node(child))
                    .collect(),
                self_closing: element.self_closing,
                span: element.span,
            }),
            HtmlNode::Text { .. } => node.clone(),
            HtmlNode::Expr { expr, span } => HtmlNode::Expr {
                expr: Box::new(self.expand_template(expr)),
                span: *span,
            },
        }
    }

    fn expand_template_html_attr(&mut self, attr: &HtmlAttr) -> HtmlAttr {
        let value = match &attr.value {
            HtmlAttrValue::Dynamic { expr, span } => HtmlAttrValue::Dynamic {
                expr: Box::new(self.expand_template(expr)),
                span: *span,
            },
            HtmlAttrValue::Bool(_) | HtmlAttrValue::Static(_) => attr.value.clone(),
        };
        HtmlAttr {
            name: attr.name.clone(),
            value,
            span: attr.span,
        }
    }

    fn expand_quasiquote_html_node(&mut self, node: &HtmlNode) -> HtmlNode {
        match node {
            HtmlNode::Element(element) => HtmlNode::Element(HtmlElement {
                tag: element.tag.clone(),
                attrs: element
                    .attrs
                    .iter()
                    .map(|attr| self.expand_quasiquote_html_attr(attr))
                    .collect(),
                children: element
                    .children
                    .iter()
                    .map(|child| self.expand_quasiquote_html_node(child))
                    .collect(),
                self_closing: element.self_closing,
                span: element.span,
            }),
            HtmlNode::Text { .. } => node.clone(),
            HtmlNode::Expr { expr, span } => HtmlNode::Expr {
                expr: Box::new(self.expand_quasiquote(expr)),
                span: *span,
            },
        }
    }

    fn expand_quasiquote_html_attr(&mut self, attr: &HtmlAttr) -> HtmlAttr {
        let value = match &attr.value {
            HtmlAttrValue::Dynamic { expr, span } => HtmlAttrValue::Dynamic {
                expr: Box::new(self.expand_quasiquote(expr)),
                span: *span,
            },
            HtmlAttrValue::Bool(_) | HtmlAttrValue::Static(_) => attr.value.clone(),
        };
        HtmlAttr {
            name: attr.name.clone(),
            value,
            span: attr.span,
        }
    }

    fn gensym_for(&mut self, base: &str) -> String {
        if let Some(existing) = self.gensyms.get(base) {
            return existing.clone();
        }

        let name = format!("{}__gensym{}", base, self.expander.gensym_counter);
        self.expander.gensym_counter += 1;
        self.gensyms.insert(base.to_string(), name.clone());
        name
    }

    fn fresh_gensym(&mut self, base: &str) -> String {
        let name = format!("{}__gensym{}", base, self.expander.gensym_counter);
        self.expander.gensym_counter += 1;
        name
    }
}

fn parse_macro_def(expr: &Expr, diagnostics: &mut Vec<Diagnostic>) -> Option<(String, MacroDef)> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    if items
        .first()
        .is_none_or(|head| !matches_symbol(head, "defmacro"))
    {
        return None;
    }

    if items.len() < 4 {
        diagnostics.push(Diagnostic::error(
            expr.span,
            "defmacro expects a name, parameter vector, and body",
        ));
        return None;
    }

    let ExprKind::Symbol(name) = &items[1].kind else {
        diagnostics.push(Diagnostic::error(
            items[1].span,
            "defmacro name must be a symbol",
        ));
        return None;
    };

    let ExprKind::Vector(params) = &items[2].kind else {
        diagnostics.push(Diagnostic::error(
            items[2].span,
            "defmacro parameters must be a vector",
        ));
        return None;
    };

    let mut param_names = Vec::new();
    for param in params {
        let ExprKind::Symbol(name) = &param.kind else {
            diagnostics.push(Diagnostic::error(
                param.span,
                "defmacro parameter must be a symbol",
            ));
            continue;
        };
        param_names.push(name.clone());
    }

    let template = if items.len() == 4 {
        items[3].clone()
    } else {
        let span = Span::new(items[3].span.start, expr.span.end);
        let mut body = Vec::with_capacity(items.len() - 2);
        body.push(symbol("do", span));
        body.extend(items[3..].iter().cloned());
        Expr::new(ExprKind::List(body), span)
    };

    Some((
        name.clone(),
        MacroDef {
            params: param_names,
            template,
        },
    ))
}

fn symbol(name: &str, span: Span) -> Expr {
    Expr::new(ExprKind::Symbol(name.to_string()), span)
}

fn matches_symbol(expr: &Expr, expected: &str) -> bool {
    matches!(&expr.kind, ExprKind::Symbol(name) if name == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_quasiquoted_macro() {
        let source = syntax::parse_source(
            "(defmacro unless [test body] `(if (not ~test) ~body nil))\n(unless connected? (start))",
        );
        let expanded = expand_source(&source);

        assert!(
            expanded.diagnostics.is_empty(),
            "{:?}",
            expanded.diagnostics
        );
        assert_eq!(
            expanded.source.pretty(),
            "(if (not connected?) (start) nil)"
        );
    }

    #[test]
    fn strips_macro_definitions_from_output() {
        let source = syntax::parse_source("(defmacro cmd-none [] `{:kind :none})\n(cmd-none)");
        let expanded = expand_source(&source);

        assert!(
            expanded.diagnostics.is_empty(),
            "{:?}",
            expanded.diagnostics
        );
        assert_eq!(expanded.source.forms.len(), 1);
        assert_eq!(expanded.source.pretty(), "{:kind :none}");
    }

    #[test]
    fn gensyms_are_deterministic_and_hygienic_within_invocation() {
        let source = syntax::parse_source(
            "(defmacro twice [value] `(let [tmp# ~value] (+ tmp# tmp#)))\n(twice 21)\n(twice 3)",
        );
        let first = expand_source(&source);
        let second = expand_source(&source);

        assert_eq!(first, second);
        assert_eq!(
            first.source.pretty(),
            "(let [tmp__gensym0 21] (+ tmp__gensym0 tmp__gensym0))\n(let [tmp__gensym1 3] (+ tmp__gensym1 tmp__gensym1))"
        );
    }

    #[test]
    fn explicit_gensym_returns_fresh_symbols() {
        let source =
            syntax::parse_source("(defmacro fresh [] (gensym \"tmp\"))\n(fresh)\n(fresh)\n(fresh)");
        let expanded = expand_source(&source);

        assert!(
            expanded.diagnostics.is_empty(),
            "{:?}",
            expanded.diagnostics
        );
        assert_eq!(
            expanded.source.pretty(),
            "tmp__gensym0\ntmp__gensym1\ntmp__gensym2"
        );
    }

    #[test]
    fn with_gensyms_binds_fresh_symbols_for_quasiquote() {
        let source = syntax::parse_source(
            "(defmacro twice [value]\n  (with-gensyms [tmp]\n    `(let [~tmp ~value] (+ ~tmp ~tmp))))\n(twice 21)",
        );
        let expanded = expand_source(&source);

        assert!(
            expanded.diagnostics.is_empty(),
            "{:?}",
            expanded.diagnostics
        );
        assert_eq!(
            expanded.source.pretty(),
            "(let [tmp__gensym0 21] (+ tmp__gensym0 tmp__gensym0))"
        );
    }

    #[test]
    fn with_gensyms_scope_does_not_leak_after_body() {
        let source = syntax::parse_source(
            "(defmacro scoped []\n  (do\n    (with-gensyms [tmp] `~tmp)\n    `tmp))\n(scoped)",
        );
        let expanded = expand_source(&source);

        assert!(
            expanded.diagnostics.is_empty(),
            "{:?}",
            expanded.diagnostics
        );
        assert_eq!(expanded.source.pretty(), "tmp");
    }

    #[test]
    fn macro_time_let_reuses_explicit_gensym_in_quasiquote() {
        let source = syntax::parse_source(
            "(defmacro twice [value]\n  (let [tmp (gensym \"tmp\")]\n    `(let [~tmp ~value] (+ ~tmp ~tmp))))\n(twice 21)",
        );
        let expanded = expand_source(&source);

        assert!(
            expanded.diagnostics.is_empty(),
            "{:?}",
            expanded.diagnostics
        );
        assert_eq!(
            expanded.source.pretty(),
            "(let [tmp__gensym0 21] (+ tmp__gensym0 tmp__gensym0))"
        );
    }

    #[test]
    fn reports_invalid_gensym_prefix() {
        let source = syntax::parse_source("(defmacro bad [] (gensym :tmp))\n(bad)");
        let expanded = expand_source(&source);

        assert!(expanded.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("gensym expects zero arguments or one string prefix")
        }));
    }

    #[test]
    fn reports_invalid_with_gensyms_names() {
        let source = syntax::parse_source("(defmacro bad [] (with-gensyms [:tmp] `nil))\n(bad)");
        let expanded = expand_source(&source);

        assert!(expanded.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("with-gensyms name must be a symbol")
        }));
    }

    #[test]
    fn reports_macro_arity_mismatch() {
        let source = syntax::parse_source("(defmacro one [x] `~x)\n(one 1 2)");
        let expanded = expand_source(&source);

        assert!(
            expanded
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expects 1 arguments"))
        );
    }

    #[test]
    fn reports_direct_compile_error() {
        let source = syntax::parse_source("(compile-error \"HRWeb macro guard failed\")");
        let expanded = expand_source(&source);

        assert_eq!(expanded.source.pretty(), "nil");
        assert!(
            expanded
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message == "HRWeb macro guard failed")
        );
    }

    #[test]
    fn reports_macro_compile_error_at_invocation_span() {
        let input = "(defmacro fail [msg] (compile-error msg))\n(fail \"missing :kind\")";
        let source = syntax::parse_source(input);
        let expanded = expand_source(&source);
        let diagnostic = expanded
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message == "missing :kind")
            .expect("macro compile-error should emit the authored diagnostic");
        let invocation_start = input
            .find("(fail")
            .expect("test input should include macro invocation");

        assert_eq!(diagnostic.span, Span::new(invocation_start, input.len()));
        assert_eq!(expanded.source.pretty(), "nil");
    }

    #[test]
    fn reports_invalid_compile_error_message() {
        let source =
            syntax::parse_source("(defmacro fail [] (compile-error :not-a-string))\n(fail)");
        let expanded = expand_source(&source);

        assert!(expanded.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("compile-error expects one string message")
        }));
    }

    #[test]
    fn expands_inside_html_template_expressions() {
        let source = syntax::parse_source(
            "(defmacro event [] `:start)\n#html <button on:click={(event)}>Go</button>",
        );
        let expanded = expand_source(&source);

        assert!(
            expanded.diagnostics.is_empty(),
            "{:?}",
            expanded.diagnostics
        );
        assert_eq!(
            expanded.source.pretty(),
            "#html <button on:click={:start}>Go</button>"
        );
    }

    #[test]
    fn expands_imported_macros_and_erases_macro_only_imports() {
        let macros = collect_macro_defs(&syntax::parse_source(
            "(defmacro cmd-none [] `{:kind :none})",
        ));
        let source = syntax::parse_source("(import \"./macros.clsk\" [cmd-none])\n(cmd-none)");
        let expanded = expand_source_with_imported_macros(&source, &macros.macros);

        assert!(
            expanded.diagnostics.is_empty(),
            "{:?}",
            expanded.diagnostics
        );
        assert_eq!(expanded.source.pretty(), "{:kind :none}");
    }

    #[test]
    fn keeps_runtime_names_when_pruning_macro_imports() {
        let macros = collect_macro_defs(&syntax::parse_source(
            "(defmacro cmd-none [] `{:kind :none})",
        ));
        let source = syntax::parse_source(
            "(import \"./macros.clsk\" [cmd-none helper-value])\n(def command (cmd-none))",
        );
        let expanded = expand_source_with_imported_macros(&source, &macros.macros);

        assert!(
            expanded.diagnostics.is_empty(),
            "{:?}",
            expanded.diagnostics
        );
        assert_eq!(
            expanded.source.pretty(),
            "(import \"./macros.clsk\" [helper-value])\n(def command {:kind :none})"
        );
    }
}
