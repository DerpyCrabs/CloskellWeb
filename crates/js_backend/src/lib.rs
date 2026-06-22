use std::collections::{BTreeMap, BTreeSet};

use syntax::{
    Diagnostic, Expr, ExprKind, HtmlAttrValue, HtmlElement, HtmlNode, SourceFile, format_expr,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmitResult {
    pub code: String,
    pub diagnostics: Vec<Diagnostic>,
    pub source_mappings: Vec<SourceMapping>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceMapping {
    pub generated_line: usize,
    pub generated_column: usize,
    pub source_offset: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportSpec {
    path: String,
    names: Vec<String>,
}

pub fn emit_module(source: &SourceFile) -> EmitResult {
    let mut emitter = Emitter {
        diagnostics: Vec::new(),
        needs_html_runtime: false,
        component_fns: collect_template_defns(source),
        read_summaries: collect_read_summaries(source),
        next_template_id: 0,
        next_temp_id: 0,
    };
    let mut import_lines = Vec::new();
    let mut lines = Vec::new();

    for (index, form) in source.forms.iter().enumerate() {
        if let ExprKind::List(items) = &form.kind {
            if let Some(import) = parse_import_form(form) {
                match import {
                    Ok(spec) => {
                        if let Some(code) = emit_import(&spec) {
                            import_lines.push(EmittedLine {
                                code,
                                source_offset: form.span.start,
                            });
                        }
                    }
                    Err(diagnostic) => emitter.diagnostics.push(diagnostic),
                }
                continue;
            }
            if is_type_form(form) || is_ann_form(form) {
                continue;
            }

            if let [head, name, value] = items.as_slice() {
                if matches_symbol(head, "def") {
                    if let ExprKind::Symbol(name) = &name.kind {
                        lines.push(EmittedLine {
                            code: format!(
                                "export const {} = {};",
                                sanitize_identifier(name),
                                emitter.emit_expr(value)
                            ),
                            source_offset: form.span.start,
                        });
                        continue;
                    }
                }
            }

            if items.len() >= 4 && matches_symbol(&items[0], "defn") {
                if let ExprKind::Symbol(name) = &items[1].kind {
                    lines.push(EmittedLine {
                        code: emitter.emit_defn(name, form, &items[2..]),
                        source_offset: form.span.start,
                    });
                    continue;
                }
            }
        }

        lines.push(EmittedLine {
            code: format!("export const value{} = {};", index, emitter.emit_expr(form)),
            source_offset: form.span.start,
        });
    }

    let mut code = String::new();
    let mut source_mappings = Vec::new();
    let mut generated_line = 0;
    if emitter.needs_html_runtime {
        code.push_str(
            "import { createTemplateComponent as __closkellCreateTemplate, setAttr as __closkellSetAttr, setComponent as __closkellSetComponent, setConditional as __closkellSetConditional, setEvent as __closkellSetEvent, setKeyedList as __closkellSetKeyedList, setRef as __closkellSetRef, setText as __closkellSetText, shouldUpdateSlot as __closkellShouldUpdateSlot } from \"@closkell/runtime\";\n\n",
        );
        generated_line += 2;
    }
    if !import_lines.is_empty() {
        push_emitted_lines(
            &mut code,
            &import_lines,
            &mut source_mappings,
            &mut generated_line,
        );
        code.push('\n');
        generated_line += 1;
    }
    push_emitted_lines(&mut code, &lines, &mut source_mappings, &mut generated_line);

    EmitResult {
        code,
        diagnostics: emitter.diagnostics,
        source_mappings,
    }
}

#[derive(Clone, Debug)]
struct EmittedLine {
    code: String,
    source_offset: usize,
}

fn push_emitted_lines(
    code: &mut String,
    lines: &[EmittedLine],
    source_mappings: &mut Vec<SourceMapping>,
    generated_line: &mut usize,
) {
    for line in lines {
        source_mappings.push(SourceMapping {
            generated_line: *generated_line,
            generated_column: 0,
            source_offset: line.source_offset,
        });
        code.push_str(&line.code);
        code.push('\n');
        *generated_line += 1;
    }
}

fn parse_import_form(expr: &Expr) -> Option<Result<ImportSpec, Diagnostic>> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    if !items
        .first()
        .is_some_and(|head| matches_symbol(head, "import"))
    {
        return None;
    }
    if items.len() != 3 {
        return Some(Err(Diagnostic::error(
            expr.span,
            "import expects a path string and a vector of symbols",
        )));
    }

    let ExprKind::String(path) = &items[1].kind else {
        return Some(Err(Diagnostic::error(
            items[1].span,
            "import path must be a string",
        )));
    };

    let ExprKind::Vector(names) = &items[2].kind else {
        return Some(Err(Diagnostic::error(
            items[2].span,
            "import names must be a vector",
        )));
    };
    if names.is_empty() {
        return Some(Err(Diagnostic::error(
            items[2].span,
            "import names vector cannot be empty",
        )));
    }

    let mut imported = Vec::new();
    for name in names {
        let ExprKind::Symbol(symbol) = &name.kind else {
            return Some(Err(Diagnostic::error(
                name.span,
                "imported name must be a symbol",
            )));
        };
        imported.push(symbol.clone());
    }

    Some(Ok(ImportSpec {
        path: path.clone(),
        names: imported,
    }))
}

fn is_type_form(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::List(items) if items.first().is_some_and(|head| matches_symbol(head, "type")))
}

fn is_ann_form(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::List(items) if items.first().is_some_and(|head| matches_symbol(head, "ann")))
}

fn emit_import(spec: &ImportSpec) -> Option<String> {
    let names = spec
        .names
        .iter()
        .filter(|name| is_runtime_import_name(name))
        .map(|name| sanitize_identifier(name))
        .collect::<Vec<_>>()
        .join(", ");
    if names.is_empty() {
        return None;
    }
    Some(format!(
        "import {{ {} }} from \"{}\";",
        names,
        escape_js(&js_import_path(&spec.path))
    ))
}

pub fn is_runtime_import_name(name: &str) -> bool {
    !name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
}

fn js_import_path(path: &str) -> String {
    path.strip_suffix(".clsk")
        .map(|prefix| format!("{}.mjs", prefix))
        .unwrap_or_else(|| path.to_string())
}

struct Emitter {
    diagnostics: Vec<Diagnostic>,
    needs_html_runtime: bool,
    component_fns: BTreeSet<String>,
    read_summaries: BTreeMap<String, ReadSummary>,
    next_template_id: usize,
    next_temp_id: usize,
}

struct TailEmission {
    code: String,
    has_tail_call: bool,
}

impl TailEmission {
    fn without_tail(code: String) -> Self {
        Self {
            code,
            has_tail_call: false,
        }
    }
}

impl Emitter {
    fn emit_expr(&mut self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Nil => "null".to_string(),
            ExprKind::Bool(value) => value.to_string(),
            ExprKind::Number(value) => value.clone(),
            ExprKind::String(value) => format!("\"{}\"", escape_js(value)),
            ExprKind::Keyword(name) => format!("Symbol.for(\"{}\")", escape_js(name)),
            ExprKind::Symbol(name) => emit_symbol_read(name),
            ExprKind::Vector(items) => self.emit_array(items),
            ExprKind::Set(items) => format!("new Set({})", self.emit_array(items)),
            ExprKind::Map(entries) => self.emit_map_or_record(entries),
            ExprKind::Quote(_)
            | ExprKind::QuasiQuote(_)
            | ExprKind::Unquote(_)
            | ExprKind::UnquoteSplicing(_) => {
                format!("undefined /* syntax {} */", escape_js(&format_expr(expr)))
            }
            ExprKind::HtmlTemplate(node) => {
                self.needs_html_runtime = true;
                self.emit_template_component(node)
            }
            ExprKind::List(items) => self.emit_list(expr, items),
        }
    }

    fn emit_array(&mut self, items: &[Expr]) -> String {
        format!(
            "[{}]",
            items
                .iter()
                .map(|item| self.emit_expr(item))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn emit_list(&mut self, expr: &Expr, items: &[Expr]) -> String {
        let Some((head, args)) = items.split_first() else {
            return "[]".to_string();
        };

        if let ExprKind::Symbol(name) = &head.kind {
            match name.as_str() {
                "fn" => return self.emit_fn(expr, args),
                "let" => return self.emit_let(expr, args),
                "if" => return self.emit_if(expr, args),
                "match" => return self.emit_match(expr, args),
                "do" => return self.emit_do(args),
                "+" | "-" | "*" | "/" | "<" | ">" | "<=" | ">=" => {
                    return self.emit_infix(name, args);
                }
                "%" | "mod" => return self.emit_mod(args),
                "=" => return self.emit_infix("===", args),
                "max" => return self.emit_math_call("max", args),
                "min" => return self.emit_math_call("min", args),
                "max-of" => return self.emit_numeric_vector_aggregate("max", args),
                "min-of" => return self.emit_numeric_vector_aggregate("min", args),
                "sum" => return self.emit_sum(args),
                "abs" => return self.emit_math_call("abs", args),
                "round" => return self.emit_math_call("round", args),
                "floor" => return self.emit_math_call("floor", args),
                "ceil" => return self.emit_math_call("ceil", args),
                "date-start-of-week" => return self.emit_date_start_of_week(args),
                "date-start-of-month" => return self.emit_date_start_of_month(args),
                "date-add-days" => return self.emit_date_add_days(args),
                "date-month" => return self.emit_date_part("getMonth", args),
                "date-day" => return self.emit_date_part("getDate", args),
                "to-number" => return self.emit_to_number(args),
                "to-fixed" => return self.emit_to_fixed(args),
                "date-format" => return self.emit_date_format(args),
                "count" => return self.emit_count(args),
                "empty?" => return self.emit_empty(args),
                "some?" => return self.emit_some(args),
                "nil?" => return self.emit_nil(args),
                "number?" => return self.emit_type_predicate("number", args),
                "string?" => return self.emit_type_predicate("string", args),
                "bool?" => return self.emit_type_predicate("boolean", args),
                "keyword?" => return self.emit_type_predicate("symbol", args),
                "list?" => return self.emit_vector_predicate(args),
                "vector?" => return self.emit_vector_predicate(args),
                "set?" => return self.emit_set_predicate(args),
                "get" => return self.emit_get(args),
                "first" => return self.emit_vector_index(args, 0),
                "second" => return self.emit_vector_index(args, 1),
                "nth" => return self.emit_nth(args),
                "last" => return self.emit_last(args),
                "cons" => return self.emit_cons(args),
                "rest" => return self.emit_rest(args),
                "find" => return self.emit_find(args),
                "map" => return self.emit_map_transform(args, false),
                "map-indexed" => return self.emit_map_transform(args, true),
                "filter" => return self.emit_filter(args),
                "any?" => return self.emit_any(args),
                "every?" => return self.emit_every(args),
                "range" => return self.emit_range(args),
                "conj" => return self.emit_conj(args),
                "disj" => return self.emit_disj(args),
                "set-values" => return self.emit_set_values(args),
                "sort-by" => return self.emit_sort_by(args, false),
                "sort-by-desc" => return self.emit_sort_by(args, true),
                "sort-with" => return self.emit_sort_with(args),
                "slice" => return self.emit_slice(args),
                "drop-last" => return self.emit_drop_last(args),
                "take-last" => return self.emit_take_last(args),
                "reduce" => return self.emit_reduce(args, false),
                "reduce-indexed" => return self.emit_reduce(args, true),
                "trim" => return self.emit_string_method("trim", args),
                "lower-case" => return self.emit_string_method("toLowerCase", args),
                "to-radix" => return self.emit_to_radix(args),
                "string-slice" => return self.emit_string_slice(args),
                "pad-start" => return self.emit_pad_start(args),
                "regex-test?" => return self.emit_regex_test(args),
                "includes?" => return self.emit_includes(args),
                "contains?" => return self.emit_contains(args),
                "locale-compare" => return self.emit_locale_compare(args),
                "json-stringify" => return self.emit_json_stringify(args),
                "json-parse" => return self.emit_json_parse(args),
                "env-dev?" => return self.emit_env_dev(args),
                "not" => return self.emit_prefix("!", args),
                "and" => return self.emit_infix("&&", args),
                "or" => return self.emit_infix("||", args),
                "ok" => return self.emit_result_constructor("value", true, args),
                "err" => return self.emit_result_constructor("error", false, args),
                "ok?" => return self.emit_result_predicate(true, args),
                "err?" => return self.emit_result_predicate(false, args),
                "result-value" => return self.emit_result_projection("value", true, args),
                "result-error" => return self.emit_result_projection("error", false, args),
                "unwrap-or" => return self.emit_unwrap_or(args),
                "hash-map" => return self.emit_hash_map(args),
                "map?" => return self.emit_map_predicate(args),
                "map-get" => return self.emit_map_get(args),
                "map-assoc" => return self.emit_map_assoc(args),
                "map-dissoc" => return self.emit_map_dissoc(args),
                "map-entries" => return self.emit_map_entries(args),
                "map-keys" => return self.emit_map_projection(args, "keys"),
                "map-values" => return self.emit_map_projection(args, "values"),
                "assoc" => return self.emit_assoc(expr, args),
                "merge" => return self.emit_merge(args),
                "dissoc" => return self.emit_dissoc(expr, args),
                "str" => {
                    if args.len() == 1 {
                        return format!("String({})", self.emit_expr(&args[0]));
                    }
                    return args
                        .iter()
                        .map(|arg| format!("String({})", self.emit_expr(arg)))
                        .collect::<Vec<_>>()
                        .join(" + ");
                }
                "list" | "vector" => return self.emit_array(args),
                "set" => return format!("new Set({})", self.emit_array(args)),
                _ => {}
            }
        }

        let callee = self.emit_expr(head);
        let args = args
            .iter()
            .map(|arg| self.emit_expr(arg))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}({})", callee, args)
    }

    fn emit_defn(&mut self, name: &str, expr: &Expr, args: &[Expr]) -> String {
        let ExprKind::Vector(params) = &args[0].kind else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "defn params must be a vector",
            ));
            return format!("export const {} = undefined;", sanitize_identifier(name));
        };

        if let [template] = &args[1..] {
            if let ExprKind::HtmlTemplate(node) = &template.kind {
                let (param_names, param_idents) = self.emit_defn_symbol_params(params);
                return self.emit_template_defn(name, &param_idents, &param_names, node);
            }
            if let Some((bindings, node)) = let_template_parts(template) {
                let (param_names, param_idents) = self.emit_defn_symbol_params(params);
                return self.emit_let_template_defn(
                    name,
                    &param_idents,
                    &param_names,
                    bindings,
                    node,
                );
            }
        }

        let mut js_params = Vec::new();
        let mut simple_param_idents = Vec::new();
        let mut pattern_statements = Vec::new();
        let mut has_pattern_params = false;
        for param in params {
            match &param.kind {
                ExprKind::Symbol(name) if name != "_" => {
                    let ident = sanitize_identifier(name);
                    js_params.push(ident.clone());
                    simple_param_idents.push(ident);
                }
                _ => {
                    has_pattern_params = true;
                    let value_name = self.next_temp("__closkell_arg");
                    js_params.push(value_name.clone());
                    self.emit_pattern_value_statement(
                        param,
                        &value_name,
                        "defn parameter pattern did not match",
                        &mut pattern_statements,
                    );
                }
            }
        }

        let body = if has_pattern_params {
            let function_body = self.emit_function_body(expr, &args[1..]);
            [pattern_statements.join(" "), function_body]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            self.emit_tail_recursive_function_body(name, &simple_param_idents, &args[1..])
                .unwrap_or_else(|| self.emit_function_body(expr, &args[1..]))
        };
        let params = js_params.join(", ");
        format!(
            "export function {}({}) {{ {} }}",
            sanitize_identifier(name),
            params,
            body
        )
    }

    fn emit_defn_symbol_params(&mut self, params: &[Expr]) -> (Vec<String>, Vec<String>) {
        let params = params
            .iter()
            .filter_map(|param| match &param.kind {
                ExprKind::Symbol(name) => Some((name.clone(), sanitize_identifier(name))),
                _ => {
                    self.diagnostics.push(Diagnostic::error(
                        param.span,
                        "defn parameter must be a symbol",
                    ));
                    None
                }
            })
            .collect::<Vec<_>>();
        let param_names = params
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        let param_idents = params
            .iter()
            .map(|(_, ident)| ident.clone())
            .collect::<Vec<_>>();
        (param_names, param_idents)
    }

    fn emit_template_defn(
        &mut self,
        name: &str,
        params: &[String],
        param_names: &[String],
        node: &HtmlNode,
    ) -> String {
        let param_list = params.join(", ");
        let component_expr = self.emit_template_component(node);
        let param_metadata = js_string_array(param_names);
        let update_params = params
            .iter()
            .map(|param| format!("next_{}", param))
            .collect::<Vec<_>>();
        let update_signature = if update_params.is_empty() {
            "dispatch, updateContext".to_string()
        } else {
            format!("{}, dispatch, updateContext", update_params.join(", "))
        };
        let reassign = params
            .iter()
            .zip(update_params.iter())
            .map(|(param, update_param)| format!("{} = {};", param, update_param))
            .collect::<Vec<_>>()
            .join(" ");

        format!(
            "export function {}({}) {{ const __closkellComponent = {}; __closkellComponent.definition.params = {}; return {{ mount(parent, dispatch) {{ return __closkellComponent.mount(parent, dispatch); }}, update({}) {{ {} return __closkellComponent.update(dispatch, updateContext); }}, dispose() {{ __closkellComponent.dispose(); }}, get root() {{ return __closkellComponent.root; }}, definition: __closkellComponent.definition }}; }}",
            sanitize_identifier(name),
            param_list,
            component_expr,
            param_metadata,
            update_signature,
            reassign
        )
    }

    fn emit_let_template_defn(
        &mut self,
        name: &str,
        params: &[String],
        param_names: &[String],
        bindings: &[Expr],
        node: &HtmlNode,
    ) -> String {
        let param_list = params.join(", ");
        let mut locals = BTreeSet::new();
        let mut refresh_lines = Vec::new();
        let mut read_aliases = ReadAliases::new();

        for pair in bindings.chunks(2) {
            let [binding, value] = pair else {
                self.diagnostics.push(Diagnostic::error(
                    node.span(),
                    "let emission requires complete binding pairs",
                ));
                continue;
            };
            let value_reads = expand_reads(
                collect_template_reads(value, &self.read_summaries),
                &read_aliases,
            );
            let can_project = can_project_read_alias(value, &read_aliases);
            collect_pattern_read_aliases(binding, &value_reads, can_project, &mut read_aliases);
            collect_pattern_symbols(binding, &mut locals);

            match &binding.kind {
                ExprKind::Symbol(name) if name == "_" => {
                    refresh_lines.push(format!("{};", self.emit_expr(value)));
                }
                ExprKind::Symbol(name) => {
                    refresh_lines.push(format!(
                        "{} = {};",
                        sanitize_identifier(name),
                        self.emit_expr(value)
                    ));
                }
                _ => {
                    let value_name = self.next_temp("__closkell_template_let");
                    refresh_lines.push(format!(
                        "const {} = {};",
                        value_name,
                        self.emit_expr(value)
                    ));
                    self.emit_pattern_assignment_statement(
                        binding,
                        &value_name,
                        "let pattern did not match",
                        &mut refresh_lines,
                    );
                }
            }
        }

        let declarations = locals
            .iter()
            .map(|local| format!("let {};", local))
            .collect::<Vec<_>>()
            .join(" ");
        let refresh_body = refresh_lines.join(" ");
        let component_expr = self.emit_template_component_with_read_aliases(node, &read_aliases);
        let param_metadata = js_string_array(param_names);
        let update_params = params
            .iter()
            .map(|param| format!("next_{}", param))
            .collect::<Vec<_>>();
        let update_signature = if update_params.is_empty() {
            "dispatch, updateContext".to_string()
        } else {
            format!("{}, dispatch, updateContext", update_params.join(", "))
        };
        let reassign = params
            .iter()
            .zip(update_params.iter())
            .map(|(param, update_param)| format!("{} = {};", param, update_param))
            .collect::<Vec<_>>()
            .join(" ");

        format!(
            "export function {}({}) {{ {} const __closkellRefresh = () => {{ {} }}; __closkellRefresh(); const __closkellComponent = {}; __closkellComponent.definition.params = {}; return {{ mount(parent, dispatch) {{ return __closkellComponent.mount(parent, dispatch); }}, update({}) {{ {} __closkellRefresh(); return __closkellComponent.update(dispatch, updateContext); }}, dispose() {{ __closkellComponent.dispose(); }}, get root() {{ return __closkellComponent.root; }}, definition: __closkellComponent.definition }}; }}",
            sanitize_identifier(name),
            param_list,
            declarations,
            refresh_body,
            component_expr,
            param_metadata,
            update_signature,
            reassign
        )
    }

    fn emit_fn(&mut self, expr: &Expr, args: &[Expr]) -> String {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "fn emission expects params and body",
            ));
            return "(() => undefined)".to_string();
        }

        let ExprKind::Vector(params) = &args[0].kind else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "fn params must be a vector",
            ));
            return "(() => undefined)".to_string();
        };

        let mut js_params = Vec::new();
        let mut statements = Vec::new();
        let mut needs_block = false;
        for param in params {
            match &param.kind {
                ExprKind::Symbol(name) if name != "_" => {
                    js_params.push(sanitize_identifier(name));
                }
                _ => {
                    needs_block = true;
                    let value_name = self.next_temp("__closkell_arg");
                    js_params.push(value_name.clone());
                    self.emit_pattern_value_statement(
                        param,
                        &value_name,
                        "fn parameter pattern did not match",
                        &mut statements,
                    );
                }
            }
        }
        let params = js_params.join(", ");
        let body = self.emit_do(&args[1..]);
        if needs_block {
            statements.push(format!("return {};", body));
            return format!("(({}) => {{ {} }})", params, statements.join(" "));
        }
        format!("(({}) => {})", params, parenthesize_arrow_body(&body))
    }

    fn emit_let(&mut self, expr: &Expr, args: &[Expr]) -> String {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "let emission expects bindings and body",
            ));
            return "undefined".to_string();
        }

        let ExprKind::Vector(bindings) = &args[0].kind else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "let bindings must be a vector",
            ));
            return "undefined".to_string();
        };

        let mut statements = Vec::new();
        for pair in bindings.chunks(2) {
            let [pattern, value] = pair else {
                self.diagnostics.push(Diagnostic::error(
                    args[0].span,
                    "let emission requires complete binding pairs",
                ));
                continue;
            };
            self.emit_let_pattern_statement(pattern, value, &mut statements);
        }
        statements.push(format!("return {};", self.emit_do(&args[1..])));
        format!("(() => {{ {} }})()", statements.join(" "))
    }

    fn emit_let_pattern_statement(
        &mut self,
        pattern: &Expr,
        value: &Expr,
        statements: &mut Vec<String>,
    ) {
        if let ExprKind::Symbol(name) = &pattern.kind {
            if name == "_" {
                statements.push(format!("{};", self.emit_expr(value)));
            } else {
                statements.push(format!(
                    "const {} = {};",
                    sanitize_identifier(name),
                    self.emit_expr(value)
                ));
            }
            return;
        }

        let value_name = self.next_temp("__closkell_let");
        statements.push(format!("const {} = {};", value_name, self.emit_expr(value)));
        self.emit_pattern_value_statement(
            pattern,
            &value_name,
            "let pattern did not match",
            statements,
        );
    }

    fn emit_pattern_value_statement(
        &mut self,
        pattern: &Expr,
        value_name: &str,
        error_message: &str,
        statements: &mut Vec<String>,
    ) {
        let compiled = self.emit_pattern(pattern, value_name);
        if compiled.test != "true" {
            statements.push(format!(
                "if (!({})) throw new Error(\"{}\");",
                compiled.test,
                escape_js(error_message)
            ));
        }
        if !compiled.bindings.is_empty() {
            statements.push(compiled.bindings);
        }
    }

    fn emit_pattern_assignment_statement(
        &mut self,
        pattern: &Expr,
        value_name: &str,
        error_message: &str,
        statements: &mut Vec<String>,
    ) {
        let compiled = self.emit_pattern(pattern, value_name);
        if compiled.test != "true" {
            statements.push(format!(
                "if (!({})) throw new Error(\"{}\");",
                compiled.test,
                escape_js(error_message)
            ));
        }
        let assignments = const_bindings_to_assignments(&compiled.bindings);
        if !assignments.is_empty() {
            statements.push(assignments);
        }
    }

    fn emit_if(&mut self, expr: &Expr, args: &[Expr]) -> String {
        if args.len() != 3 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "if emission expects 3 arguments",
            ));
            return "undefined".to_string();
        }
        format!(
            "({} ? {} : {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            self.emit_expr(&args[2])
        )
    }

    fn emit_match(&mut self, expr: &Expr, args: &[Expr]) -> String {
        if args.len() < 3 || args.len() % 2 == 0 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "match emission expects a value followed by pattern/body pairs",
            ));
            return "undefined".to_string();
        }

        let value_name = self.next_temp("__closkell_match");
        let value = self.emit_expr(&args[0]);
        let mut lines = vec![format!("const {} = {};", value_name, value)];

        for (index, arm) in args[1..].chunks(2).enumerate() {
            let [pattern, body] = arm else {
                continue;
            };
            let compiled = self.emit_pattern(pattern, &value_name);
            let body = self.emit_expr(body);
            let keyword = if index == 0 { "if" } else { "else if" };
            lines.push(format!(
                "{} ({}) {{ {} return {}; }}",
                keyword, compiled.test, compiled.bindings, body
            ));
        }

        lines.push("throw new Error(\"non-exhaustive match\");".to_string());
        format!("(() => {{ {} }})()", lines.join(" "))
    }

    fn emit_do(&mut self, args: &[Expr]) -> String {
        match args {
            [] => "null".to_string(),
            [single] => self.emit_expr(single),
            many => {
                let mut statements = Vec::new();
                for expr in &many[..many.len() - 1] {
                    statements.push(format!("{};", self.emit_expr(expr)));
                }
                statements.push(format!("return {};", self.emit_expr(&many[many.len() - 1])));
                format!("(() => {{ {} }})()", statements.join(" "))
            }
        }
    }

    fn emit_infix(&mut self, op: &str, args: &[Expr]) -> String {
        if args.is_empty() {
            return "undefined".to_string();
        }
        args.iter()
            .map(|arg| parenthesize_expression(self.emit_expr(arg)))
            .collect::<Vec<_>>()
            .join(&format!(" {} ", op))
    }

    fn emit_mod(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "({} % {})",
            parenthesize_expression(self.emit_expr(&args[0])),
            parenthesize_expression(self.emit_expr(&args[1]))
        )
    }

    fn emit_prefix(&mut self, op: &str, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("{}({})", op, self.emit_expr(&args[0]))
    }

    fn emit_math_call(&mut self, name: &str, args: &[Expr]) -> String {
        let args = args
            .iter()
            .map(|arg| self.emit_expr(arg))
            .collect::<Vec<_>>()
            .join(", ");
        format!("Math.{}({})", name, args)
    }

    fn emit_numeric_vector_aggregate(&mut self, method: &str, args: &[Expr]) -> String {
        let Some((values, fallbacks)) = args.split_first() else {
            return "undefined".to_string();
        };
        let values = parenthesize_expression(self.emit_expr(values));
        let mut terms = vec![format!("...{}", values)];
        terms.extend(fallbacks.iter().map(|fallback| self.emit_expr(fallback)));
        format!("Math.{}({})", method, terms.join(", "))
    }

    fn emit_sum(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "((__values) => __values.reduce((__sum, __value) => __sum + __value, 0))({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_date_start_of_week(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "((__timestamp) => {{ const __date = new Date(__timestamp); const __day = __date.getDay(); const __diff = __day === 0 ? -6 : 1 - __day; __date.setHours(0, 0, 0, 0); __date.setDate(__date.getDate() + __diff); return __date.getTime(); }})({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_date_start_of_month(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "((__timestamp) => {{ const __date = new Date(__timestamp); __date.setHours(0, 0, 0, 0); __date.setDate(1); return __date.getTime(); }})({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_date_add_days(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "((__timestamp, __days) => {{ const __date = new Date(__timestamp); __date.setDate(__date.getDate() + __days); return __date.getTime(); }})({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_date_part(&mut self, method: &str, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("new Date({}).{}()", self.emit_expr(&args[0]), method)
    }

    fn emit_to_fixed(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "({}).toFixed({})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_to_number(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "((__value) => {{ try {{ return Number(__value); }} catch {{ return Number.NaN; }} }})({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_date_format(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "((__timestamp, __style) => {{ const __date = new Date(__timestamp); const __key = Symbol.keyFor(__style) ?? __style; const __options = __key === \"month-year\" ? {{ month: \"short\", year: \"2-digit\" }} : __key === \"month-day-time\" ? {{ month: \"short\", day: \"numeric\", hour: \"2-digit\", minute: \"2-digit\" }} : __key === \"month-day\" ? {{ month: \"short\", day: \"numeric\" }} : __key === \"month\" ? {{ month: \"short\" }} : __key === \"day\" ? {{ day: \"numeric\" }} : undefined; return __key === \"iso-date\" ? __date.toISOString().slice(0, 10) : new Intl.DateTimeFormat(undefined, __options).format(__date); }})({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_env_dev(&mut self, args: &[Expr]) -> String {
        if !args.is_empty() {
            return "undefined".to_string();
        }
        "Boolean(globalThis.__CLOSKELL_ENV__?.DEV ?? (import.meta.env && import.meta.env.DEV))"
            .to_string()
    }

    fn emit_count(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "((__collection) => __collection instanceof Set || __collection instanceof Map ? __collection.size : (Array.isArray(__collection) || typeof __collection === \"string\" ? __collection.length : (__collection == null ? 0 : Object.keys(__collection).length)))({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_empty(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "((__collection) => (__collection instanceof Set || __collection instanceof Map ? __collection.size : (Array.isArray(__collection) || typeof __collection === \"string\" ? __collection.length : (__collection == null ? 0 : Object.keys(__collection).length))) === 0)({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_some(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("{} != null", self.emit_expr(&args[0]))
    }

    fn emit_nil(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("({}) == null", self.emit_expr(&args[0]))
    }

    fn emit_type_predicate(&mut self, js_type: &str, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        let value = self.emit_expr(&args[0]);
        if js_type == "number" {
            return format!("Number.isFinite({})", value);
        }
        format!("typeof ({}) === \"{}\"", value, js_type)
    }

    fn emit_vector_predicate(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("Array.isArray({})", self.emit_expr(&args[0]))
    }

    fn emit_set_predicate(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("{} instanceof Set", self.emit_expr(&args[0]))
    }

    fn emit_map_predicate(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("{} instanceof Map", self.emit_expr(&args[0]))
    }

    fn emit_get(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        let base = parenthesize_member_base(self.emit_expr(&args[0]));
        match &args[1].kind {
            ExprKind::Keyword(name) | ExprKind::Symbol(name) | ExprKind::String(name) => {
                format!("({}{} ?? null)", base, optional_property_access(name))
            }
            _ => format!("({}?.[{}] ?? null)", base, self.emit_expr(&args[1])),
        }
    }

    fn emit_hash_map(&mut self, args: &[Expr]) -> String {
        if args.len() % 2 != 0 {
            return "undefined".to_string();
        }

        let entries = args
            .chunks(2)
            .filter_map(|pair| match pair {
                [key, value] => Some(format!(
                    "[{}, {}]",
                    self.emit_expr(key),
                    self.emit_expr(value)
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("new Map([{}])", entries)
    }

    fn emit_map_get(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "((__map, __key) => __map instanceof Map && __map.has(__key) ? __map.get(__key) : null)({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_map_assoc(&mut self, args: &[Expr]) -> String {
        if args.len() < 3 || args[1..].len() % 2 != 0 {
            return "undefined".to_string();
        }

        let statements = args[1..]
            .chunks(2)
            .filter_map(|pair| match pair {
                [key, value] => Some(format!(
                    "__next.set({}, {});",
                    self.emit_expr(key),
                    self.emit_expr(value)
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "((__map) => {{ const __next = new Map(__map); {} return __next; }})({})",
            statements,
            self.emit_expr(&args[0])
        )
    }

    fn emit_map_dissoc(&mut self, args: &[Expr]) -> String {
        if args.len() < 2 {
            return "undefined".to_string();
        }

        let statements = args[1..]
            .iter()
            .map(|key| format!("__next.delete({});", self.emit_expr(key)))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "((__map) => {{ const __next = new Map(__map); {} return __next; }})({})",
            statements,
            self.emit_expr(&args[0])
        )
    }

    fn emit_map_entries(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }

        format!(
            "((__map) => __map instanceof Map ? Array.from(__map.entries(), ([__key, __value]) => ({{ key: __key, value: __value }})) : [])({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_map_projection(&mut self, args: &[Expr], method: &str) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }

        format!(
            "((__map) => __map instanceof Map ? Array.from(__map.{}()) : [])({})",
            method,
            self.emit_expr(&args[0])
        )
    }

    fn emit_vector_index(&mut self, args: &[Expr], index: usize) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "({}[{}] ?? null)",
            parenthesize_member_base(self.emit_expr(&args[0])),
            index
        )
    }

    fn emit_nth(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "({}[{}] ?? null)",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_last(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "({}.at(-1) ?? null)",
            parenthesize_member_base(self.emit_expr(&args[0]))
        )
    }

    fn emit_cons(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "((__item, __list) => [__item, ...(Array.isArray(__list) ? __list : [])])({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_rest(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "((__list) => Array.isArray(__list) ? __list.slice(1) : [])({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_find(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "({}.find({}) ?? null)",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_map_transform(&mut self, args: &[Expr], indexed: bool) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        let collection = parenthesize_member_base(self.emit_expr(&args[0]));
        let mapper = self.emit_expr(&args[1]);
        if indexed {
            format!(
                "{}.map((__item, __index) => {}(__item, __index))",
                collection, mapper
            )
        } else {
            format!("{}.map((__item) => {}(__item))", collection, mapper)
        }
    }

    fn emit_filter(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "{}.filter((__item) => {}(__item))",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_any(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "{}.some((__item) => {}(__item))",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_every(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "{}.every((__item) => {}(__item))",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_range(&mut self, args: &[Expr]) -> String {
        let (start, end, step) = match args {
            [end] => ("0".to_string(), self.emit_expr(end), "1".to_string()),
            [start, end] => (self.emit_expr(start), self.emit_expr(end), "1".to_string()),
            [start, end, step] => (
                self.emit_expr(start),
                self.emit_expr(end),
                self.emit_expr(step),
            ),
            _ => return "undefined".to_string(),
        };

        format!(
            "((__start, __end, __step) => {{ if (__step === 0) return []; const __count = Math.max(0, Math.ceil((__end - __start) / __step)); return Array.from({{ length: __count }}, (_, __index) => __start + __index * __step); }})({}, {}, {})",
            start, end, step
        )
    }

    fn emit_conj(&mut self, args: &[Expr]) -> String {
        if args.len() < 2 {
            return "undefined".to_string();
        }

        let collection = self.emit_expr(&args[0]);
        let items = args[1..]
            .iter()
            .map(|item| self.emit_expr(item))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "((__collection) => __collection instanceof Set ? new Set([...__collection, {}]) : [...__collection, {}])({})",
            items, items, collection
        )
    }

    fn emit_disj(&mut self, args: &[Expr]) -> String {
        if args.len() < 2 {
            return "undefined".to_string();
        }

        let collection = self.emit_expr(&args[0]);
        let items = args[1..]
            .iter()
            .map(|item| self.emit_expr(item))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "((__collection, ...__items) => {{ const __next = new Set(__collection); for (const __item of __items) __next.delete(__item); return __next; }})({}, {})",
            collection, items
        )
    }

    fn emit_set_values(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("Array.from({})", self.emit_expr(&args[0]))
    }

    fn emit_sort_by(&mut self, args: &[Expr], desc: bool) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        let collection = parenthesize_member_base(self.emit_expr(&args[0]));
        let key_fn = self.emit_expr(&args[1]);
        let less = if desc { "1" } else { "-1" };
        let greater = if desc { "-1" } else { "1" };
        format!(
            "[...{}].sort((__left, __right) => {{ const __leftKey = {}(__left); const __rightKey = {}(__right); return __leftKey < __rightKey ? {} : (__leftKey > __rightKey ? {} : 0); }})",
            collection, key_fn, key_fn, less, greater
        )
    }

    fn emit_sort_with(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        let collection = parenthesize_member_base(self.emit_expr(&args[0]));
        let comparator = self.emit_expr(&args[1]);
        format!(
            "[...{}].sort((__left, __right) => {}(__left, __right))",
            collection, comparator
        )
    }

    fn emit_slice(&mut self, args: &[Expr]) -> String {
        if !(2..=3).contains(&args.len()) {
            return "undefined".to_string();
        }
        let collection = parenthesize_member_base(self.emit_expr(&args[0]));
        let start = self.emit_expr(&args[1]);
        if let Some(end) = args.get(2) {
            format!("{}.slice({}, {})", collection, start, self.emit_expr(end))
        } else {
            format!("{}.slice({})", collection, start)
        }
    }

    fn emit_drop_last(&mut self, args: &[Expr]) -> String {
        if !(1..=2).contains(&args.len()) {
            return "undefined".to_string();
        }
        let collection = parenthesize_member_base(self.emit_expr(&args[0]));
        let count = args
            .get(1)
            .map(|count| self.emit_expr(count))
            .unwrap_or_else(|| "1".to_string());
        format!("{}.slice(0, -({}))", collection, count)
    }

    fn emit_take_last(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "{}.slice(-({}))",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_reduce(&mut self, args: &[Expr], indexed: bool) -> String {
        if args.len() != 3 {
            return "undefined".to_string();
        }
        let collection = parenthesize_member_base(self.emit_expr(&args[0]));
        let initial = self.emit_expr(&args[1]);
        let reducer = self.emit_expr(&args[2]);
        if indexed {
            format!(
                "{}.reduce((__acc, __item, __index) => {}(__acc, __item, __index), {})",
                collection, reducer, initial
            )
        } else {
            format!(
                "{}.reduce((__acc, __item) => {}(__acc, __item), {})",
                collection, reducer, initial
            )
        }
    }

    fn emit_string_method(&mut self, method: &str, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "{}.{}()",
            parenthesize_member_base(self.emit_expr(&args[0])),
            method
        )
    }

    fn emit_to_radix(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "({}).toString({})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_string_slice(&mut self, args: &[Expr]) -> String {
        if !(2..=3).contains(&args.len()) {
            return "undefined".to_string();
        }
        let value = parenthesize_member_base(self.emit_expr(&args[0]));
        let start = self.emit_expr(&args[1]);
        if let Some(end) = args.get(2) {
            format!("{}.slice({}, {})", value, start, self.emit_expr(end))
        } else {
            format!("{}.slice({})", value, start)
        }
    }

    fn emit_regex_test(&mut self, args: &[Expr]) -> String {
        if !(2..=3).contains(&args.len()) {
            return "undefined".to_string();
        }
        let flags = args
            .get(2)
            .map(|flags| self.emit_expr(flags))
            .unwrap_or_else(|| "\"\"".to_string());
        format!(
            "new RegExp({}, {}).test({})",
            self.emit_expr(&args[1]),
            flags,
            self.emit_expr(&args[0])
        )
    }

    fn emit_includes(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "{}.includes({})",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_locale_compare(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "{}.localeCompare({})",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_contains(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "((__collection, __value) => {{ if (__collection instanceof Set || __collection instanceof Map) return __collection.has(__value); if (Array.isArray(__collection) || typeof __collection === \"string\") return __collection.includes(__value); return __collection != null && Object.prototype.hasOwnProperty.call(__collection, __value); }})({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_result_constructor(&mut self, field: &str, ok: bool, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("{{ ok: {}, {}: {} }}", ok, field, self.emit_expr(&args[0]))
    }

    fn emit_result_predicate(&mut self, expected: bool, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("{}?.ok === {}", self.emit_expr(&args[0]), expected)
    }

    fn emit_result_projection(&mut self, field: &str, expected: bool, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        let result = self.emit_expr(&args[0]);
        format!(
            "((__result) => __result?.ok === {} ? __result.{} : null)({})",
            expected, field, result
        )
    }

    fn emit_unwrap_or(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "((__result, __fallback) => __result?.ok === true ? __result.value : __fallback)({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_pad_start(&mut self, args: &[Expr]) -> String {
        if args.len() != 3 {
            return "undefined".to_string();
        }
        format!(
            "{}.padStart({}, {})",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1]),
            self.emit_expr(&args[2])
        )
    }

    fn emit_json_stringify(&mut self, args: &[Expr]) -> String {
        if !(1..=2).contains(&args.len()) {
            return "undefined".to_string();
        }

        let value = self.emit_expr(&args[0]);
        if let Some(space) = args.get(1) {
            format!("JSON.stringify({}, null, {})", value, self.emit_expr(space))
        } else {
            format!("JSON.stringify({})", value)
        }
    }

    fn emit_json_parse(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }

        format!("JSON.parse({})", self.emit_expr(&args[0]))
    }

    fn emit_assoc(&mut self, expr: &Expr, args: &[Expr]) -> String {
        if args.len() < 3 || args[1..].len() % 2 != 0 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "assoc emission expects a record followed by key/value pairs",
            ));
            return "undefined".to_string();
        }

        let base = self.emit_expr(&args[0]);
        let mut fields = Vec::new();
        for pair in args[1..].chunks(2) {
            let [key, value] = pair else {
                continue;
            };
            let Some(key) = object_key(key) else {
                self.diagnostics.push(Diagnostic::error(
                    key.span,
                    "assoc keys must be keywords, strings, or symbols",
                ));
                continue;
            };
            fields.push(format!("{}: {}", key, self.emit_expr(value)));
        }

        format!("{{ ...({}), {} }}", base, fields.join(", "))
    }

    fn emit_merge(&mut self, args: &[Expr]) -> String {
        if args.is_empty() {
            return "{}".to_string();
        }
        let args = args
            .iter()
            .map(|arg| self.emit_expr(arg))
            .collect::<Vec<_>>()
            .join(", ");
        format!("Object.assign({{}}, {})", args)
    }

    fn emit_dissoc(&mut self, expr: &Expr, args: &[Expr]) -> String {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "dissoc emission expects a record and at least one key",
            ));
            return "undefined".to_string();
        }

        let record = self.next_temp("__closkell_record");
        let mut statements = vec![format!(
            "const {} = {{ ...({}) }};",
            record,
            self.emit_expr(&args[0])
        )];
        for key in &args[1..] {
            let Some(name) = object_key_name(key) else {
                self.diagnostics.push(Diagnostic::error(
                    key.span,
                    "dissoc keys must be keywords, strings, or symbols",
                ));
                continue;
            };
            statements.push(format!("delete {}{};", record, property_access(&name)));
        }
        statements.push(format!("return {};", record));
        format!("(() => {{ {} }})()", statements.join(" "))
    }

    fn emit_map_or_record(&mut self, entries: &[(Expr, Expr)]) -> String {
        if entries.iter().all(|(key, _)| object_key(key).is_some()) {
            let fields = entries
                .iter()
                .filter_map(|(key, value)| {
                    object_key(key).map(|key| format!("{}: {}", key, self.emit_expr(value)))
                })
                .collect::<Vec<_>>()
                .join(", ");
            return format!("{{ {} }}", fields);
        }

        let entries = entries
            .iter()
            .map(|(key, value)| format!("[{}, {}]", self.emit_expr(key), self.emit_expr(value)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("new Map([{}])", entries)
    }

    fn emit_function_body(&mut self, _expr: &Expr, body: &[Expr]) -> String {
        match body {
            [] => "return null;".to_string(),
            [single] => format!("return {};", self.emit_expr(single)),
            many => {
                let mut statements = Vec::new();
                for expr in &many[..many.len() - 1] {
                    statements.push(format!("{};", self.emit_expr(expr)));
                }
                statements.push(format!("return {};", self.emit_expr(&many[many.len() - 1])));
                statements.join(" ")
            }
        }
    }

    fn emit_tail_recursive_function_body(
        &mut self,
        self_name: &str,
        params: &[String],
        body: &[Expr],
    ) -> Option<String> {
        if !Self::has_self_tail_call_in_sequence(self_name, params.len(), body) {
            return None;
        }
        let emitted = self.emit_tail_sequence(self_name, params, body);
        Some(format!("while (true) {{ {} }}", emitted.code))
    }

    fn has_self_tail_call_in_sequence(self_name: &str, param_count: usize, body: &[Expr]) -> bool {
        body.last()
            .is_some_and(|expr| Self::has_self_tail_call_in_expr(self_name, param_count, expr))
    }

    fn has_self_tail_call_in_expr(self_name: &str, param_count: usize, expr: &Expr) -> bool {
        let ExprKind::List(items) = &expr.kind else {
            return false;
        };
        let Some((head, args)) = items.split_first() else {
            return false;
        };

        if matches_symbol(head, self_name) && args.len() == param_count {
            return true;
        }

        if matches_symbol(head, "if") && args.len() == 3 {
            return Self::has_self_tail_call_in_expr(self_name, param_count, &args[1])
                || Self::has_self_tail_call_in_expr(self_name, param_count, &args[2]);
        }

        if matches_symbol(head, "do") {
            return Self::has_self_tail_call_in_sequence(self_name, param_count, args);
        }

        if matches_symbol(head, "let") && args.len() >= 2 {
            return Self::has_self_tail_call_in_sequence(self_name, param_count, &args[1..]);
        }

        if matches_symbol(head, "match") && args.len() >= 3 && args.len() % 2 == 1 {
            return args[1..]
                .chunks(2)
                .filter_map(|arm| arm.get(1))
                .any(|body| Self::has_self_tail_call_in_expr(self_name, param_count, body));
        }

        false
    }

    fn emit_tail_sequence(
        &mut self,
        self_name: &str,
        params: &[String],
        body: &[Expr],
    ) -> TailEmission {
        match body {
            [] => TailEmission::without_tail("return null;".to_string()),
            [single] => self.emit_tail_expr(self_name, params, single),
            many => {
                let mut statements = many[..many.len() - 1]
                    .iter()
                    .map(|expr| format!("{};", self.emit_expr(expr)))
                    .collect::<Vec<_>>();
                let tail = self.emit_tail_expr(self_name, params, &many[many.len() - 1]);
                statements.push(tail.code);
                TailEmission {
                    code: statements.join(" "),
                    has_tail_call: tail.has_tail_call,
                }
            }
        }
    }

    fn emit_tail_expr(&mut self, self_name: &str, params: &[String], expr: &Expr) -> TailEmission {
        let ExprKind::List(items) = &expr.kind else {
            return TailEmission::without_tail(format!("return {};", self.emit_expr(expr)));
        };
        let Some((head, args)) = items.split_first() else {
            return TailEmission::without_tail("return [];".to_string());
        };

        if matches_symbol(head, self_name) && args.len() == params.len() {
            return self.emit_tail_self_call(params, args);
        }

        if matches_symbol(head, "if") && args.len() == 3 {
            let then_branch = self.emit_tail_expr(self_name, params, &args[1]);
            let else_branch = self.emit_tail_expr(self_name, params, &args[2]);
            return TailEmission {
                code: format!(
                    "if ({}) {{ {} }} else {{ {} }}",
                    self.emit_expr(&args[0]),
                    then_branch.code,
                    else_branch.code
                ),
                has_tail_call: then_branch.has_tail_call || else_branch.has_tail_call,
            };
        }

        if matches_symbol(head, "do") {
            return self.emit_tail_sequence(self_name, params, args);
        }

        if matches_symbol(head, "let") {
            return self.emit_tail_let(self_name, params, expr, args);
        }

        if matches_symbol(head, "match") {
            return self.emit_tail_match(self_name, params, expr, args);
        }

        TailEmission::without_tail(format!("return {};", self.emit_expr(expr)))
    }

    fn emit_tail_self_call(&mut self, params: &[String], args: &[Expr]) -> TailEmission {
        let mut statements = Vec::new();
        let mut temps = Vec::new();
        for arg in args {
            let temp = self.next_temp("__closkell_tail");
            statements.push(format!("const {} = {};", temp, self.emit_expr(arg)));
            temps.push(temp);
        }
        for (param, temp) in params.iter().zip(temps.iter()) {
            statements.push(format!("{} = {};", param, temp));
        }
        statements.push("continue;".to_string());
        TailEmission {
            code: statements.join(" "),
            has_tail_call: true,
        }
    }

    fn emit_tail_let(
        &mut self,
        self_name: &str,
        params: &[String],
        expr: &Expr,
        args: &[Expr],
    ) -> TailEmission {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "let emission expects bindings and body",
            ));
            return TailEmission::without_tail("return undefined;".to_string());
        }

        let ExprKind::Vector(bindings) = &args[0].kind else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "let bindings must be a vector",
            ));
            return TailEmission::without_tail("return undefined;".to_string());
        };

        let mut statements = Vec::new();
        for pair in bindings.chunks(2) {
            let [pattern, value] = pair else {
                self.diagnostics.push(Diagnostic::error(
                    args[0].span,
                    "let emission requires complete binding pairs",
                ));
                continue;
            };
            self.emit_let_pattern_statement(pattern, value, &mut statements);
        }

        let tail = self.emit_tail_sequence(self_name, params, &args[1..]);
        statements.push(tail.code);
        TailEmission {
            code: format!("{{ {} }}", statements.join(" ")),
            has_tail_call: tail.has_tail_call,
        }
    }

    fn emit_tail_match(
        &mut self,
        self_name: &str,
        params: &[String],
        expr: &Expr,
        args: &[Expr],
    ) -> TailEmission {
        if args.len() < 3 || args.len() % 2 == 0 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "match emission expects a value followed by pattern/body pairs",
            ));
            return TailEmission::without_tail("return undefined;".to_string());
        }

        let value_name = self.next_temp("__closkell_match");
        let mut lines = vec![format!(
            "const {} = {};",
            value_name,
            self.emit_expr(&args[0])
        )];
        let mut has_tail_call = false;
        for (index, arm) in args[1..].chunks(2).enumerate() {
            let [pattern, body] = arm else {
                continue;
            };
            let compiled = self.emit_pattern(pattern, &value_name);
            let body = self.emit_tail_expr(self_name, params, body);
            has_tail_call |= body.has_tail_call;
            let keyword = if index == 0 { "if" } else { "else if" };
            lines.push(format!(
                "{} ({}) {{ {} {} }}",
                keyword, compiled.test, compiled.bindings, body.code
            ));
        }
        lines.push("throw new Error(\"non-exhaustive match\");".to_string());
        TailEmission {
            code: format!("{{ {} }}", lines.join(" ")),
            has_tail_call,
        }
    }

    fn next_temp(&mut self, prefix: &str) -> String {
        let id = self.next_temp_id;
        self.next_temp_id += 1;
        format!("{}_{}", prefix, id)
    }

    fn emit_pattern(&mut self, pattern: &Expr, value: &str) -> CompiledPattern {
        match &pattern.kind {
            ExprKind::Symbol(name) if name == "_" => CompiledPattern::always(),
            ExprKind::Symbol(name) => CompiledPattern {
                test: "true".to_string(),
                bindings: format!("const {} = {};", sanitize_identifier(name), value),
            },
            ExprKind::List(items)
                if items.first().is_some_and(|head| matches_symbol(head, "as")) =>
            {
                self.emit_as_pattern(pattern, items, value)
            }
            ExprKind::List(items)
                if items
                    .first()
                    .and_then(symbol_name)
                    .is_some_and(is_data_constructor_pattern) =>
            {
                self.emit_data_constructor_pattern(pattern, items, value)
            }
            ExprKind::Keyword(name) => CompiledPattern {
                test: format!("{} === Symbol.for(\"{}\")", value, escape_js(name)),
                bindings: String::new(),
            },
            ExprKind::String(value_pattern) => CompiledPattern {
                test: format!("{} === \"{}\"", value, escape_js(value_pattern)),
                bindings: String::new(),
            },
            ExprKind::Number(value_pattern) => CompiledPattern {
                test: format!("{} === {}", value, value_pattern),
                bindings: String::new(),
            },
            ExprKind::Bool(value_pattern) => CompiledPattern {
                test: format!("{} === {}", value, value_pattern),
                bindings: String::new(),
            },
            ExprKind::Nil => CompiledPattern {
                test: format!("{} == null", value),
                bindings: String::new(),
            },
            ExprKind::Map(entries) => self.emit_record_pattern(entries, value),
            ExprKind::Vector(items) => self.emit_vector_pattern(items, value),
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    pattern.span,
                    "unsupported match pattern for JS emission",
                ));
                CompiledPattern {
                    test: "false".to_string(),
                    bindings: String::new(),
                }
            }
        }
    }

    fn emit_data_constructor_pattern(
        &mut self,
        pattern: &Expr,
        items: &[Expr],
        value: &str,
    ) -> CompiledPattern {
        let Some(name) = items.first().and_then(symbol_name) else {
            return CompiledPattern {
                test: "false".to_string(),
                bindings: String::new(),
            };
        };
        if name == "list" {
            return self.emit_vector_pattern(&items[1..], value);
        }
        if name == "cons" {
            return self.emit_cons_pattern(pattern, &items[1..], value);
        }
        if items.len() != 2 {
            self.diagnostics.push(Diagnostic::error(
                pattern.span,
                format!("{} pattern expects `({} pattern)`", name, name),
            ));
            return CompiledPattern {
                test: "false".to_string(),
                bindings: String::new(),
            };
        }

        match name {
            "some" => {
                let inner = self.emit_pattern(&items[1], value);
                CompiledPattern {
                    test: format!("{} != null && ({})", value, inner.test),
                    bindings: inner.bindings,
                }
            }
            "ok" | "err" => {
                let expected = if name == "ok" { "true" } else { "false" };
                let field = if name == "ok" { "value" } else { "error" };
                let field_value = format!("{}{}", value, property_access(field));
                let inner = self.emit_pattern(&items[1], &field_value);
                CompiledPattern {
                    test: format!(
                        "{} !== null && typeof {} === \"object\" && {}.ok === {} && ({})",
                        value, value, value, expected, inner.test
                    ),
                    bindings: inner.bindings,
                }
            }
            _ => CompiledPattern {
                test: "false".to_string(),
                bindings: String::new(),
            },
        }
    }

    fn emit_cons_pattern(
        &mut self,
        pattern: &Expr,
        items: &[Expr],
        value: &str,
    ) -> CompiledPattern {
        if items.len() != 2 {
            self.diagnostics.push(Diagnostic::error(
                pattern.span,
                "cons pattern expects `(cons head tail)`",
            ));
            return CompiledPattern {
                test: "false".to_string(),
                bindings: String::new(),
            };
        }

        let head_value = format!("{}[0]", value);
        let tail_value = format!("{}.slice(1)", value);
        let head = self.emit_pattern(&items[0], &head_value);
        let tail = self.emit_pattern(&items[1], &tail_value);
        let bindings = [head.bindings, tail.bindings]
            .into_iter()
            .filter(|binding| !binding.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        CompiledPattern {
            test: [
                format!("Array.isArray({})", value),
                format!("{}.length > 0", value),
                head.test,
                tail.test,
            ]
            .join(" && "),
            bindings,
        }
    }

    fn emit_as_pattern(&mut self, pattern: &Expr, items: &[Expr], value: &str) -> CompiledPattern {
        if items.len() != 3 {
            self.diagnostics.push(Diagnostic::error(
                pattern.span,
                "as pattern expects `(as pattern name)`",
            ));
            return CompiledPattern {
                test: "false".to_string(),
                bindings: String::new(),
            };
        }
        let ExprKind::Symbol(name) = &items[2].kind else {
            self.diagnostics.push(Diagnostic::error(
                items[2].span,
                "as pattern name must be a symbol",
            ));
            return CompiledPattern {
                test: "false".to_string(),
                bindings: String::new(),
            };
        };
        let inner = self.emit_pattern(&items[1], value);
        let alias = if name == "_" {
            String::new()
        } else {
            format!("const {} = {};", sanitize_identifier(name), value)
        };
        CompiledPattern {
            test: inner.test,
            bindings: [inner.bindings, alias]
                .into_iter()
                .filter(|binding| !binding.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    fn emit_record_pattern(&mut self, entries: &[(Expr, Expr)], value: &str) -> CompiledPattern {
        let mut tests = vec![format!(
            "{} !== null && typeof {} === \"object\"",
            value, value
        )];
        let mut bindings = Vec::new();

        for (key, pattern) in entries {
            let Some(key) = object_key_name(key) else {
                self.diagnostics.push(Diagnostic::error(
                    key.span,
                    "record pattern keys must be keywords, strings, or symbols",
                ));
                continue;
            };
            let field = format!("{}{}", value, property_access(&key));
            let compiled = self.emit_pattern(pattern, &field);
            tests.push(compiled.test);
            if !compiled.bindings.is_empty() {
                bindings.push(compiled.bindings);
            }
        }

        CompiledPattern {
            test: tests.join(" && "),
            bindings: bindings.join(" "),
        }
    }

    fn emit_vector_pattern(&mut self, items: &[Expr], value: &str) -> CompiledPattern {
        let mut tests = vec![
            format!("Array.isArray({})", value),
            format!("{}.length === {}", value, items.len()),
        ];
        let mut bindings = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let field = format!("{}[{}]", value, index);
            let compiled = self.emit_pattern(item, &field);
            tests.push(compiled.test);
            if !compiled.bindings.is_empty() {
                bindings.push(compiled.bindings);
            }
        }
        CompiledPattern {
            test: tests.join(" && "),
            bindings: bindings.join(" "),
        }
    }

    fn emit_template_component(&mut self, root: &HtmlNode) -> String {
        self.emit_template_component_with_read_aliases(root, &ReadAliases::new())
    }

    fn emit_template_component_with_read_aliases(
        &mut self,
        root: &HtmlNode,
        read_aliases: &ReadAliases,
    ) -> String {
        self.needs_html_runtime = true;
        let template_id = self.next_template_id;
        self.next_template_id += 1;

        let mut template = TemplateEmitter {
            owner: self,
            template_id,
            read_aliases,
            nodes: Vec::new(),
            slots: Vec::new(),
            create_lines: Vec::new(),
        };
        let root_var = template.emit_node(root);
        let node_vars = template.nodes.join(", ");
        template.create_lines.push(format!(
            "return {{ root: {}, nodes: [{}] }};",
            root_var, node_vars
        ));
        let create_body = template.create_lines.join(" ");
        let update_body = template.emit_update_body();
        let metadata = template.emit_metadata();

        format!(
            "__closkellCreateTemplate({{ name: \"template{}\", slots: {}, create() {{ {} }}, update(instance, dispatch, updateContext) {{ {} }} }})",
            template_id, metadata, create_body, update_body
        )
    }
}

struct TemplateEmitter<'a> {
    owner: &'a mut Emitter,
    template_id: usize,
    read_aliases: &'a ReadAliases,
    nodes: Vec<String>,
    slots: Vec<TemplateSlot>,
    create_lines: Vec<String>,
}

struct CompiledPattern {
    test: String,
    bindings: String,
}

impl CompiledPattern {
    fn always() -> Self {
        Self {
            test: "true".to_string(),
            bindings: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct TemplateSlot {
    id: usize,
    node_id: usize,
    kind: TemplateSlotKind,
    expr: String,
    reads: Vec<String>,
}

#[derive(Clone, Debug)]
enum TemplateSlotKind {
    Text,
    Attr(String),
    Event(String),
    Ref,
    Conditional {
        condition: String,
        render_then: String,
        render_else: String,
    },
    Component {
        name: String,
        render: String,
        args: String,
    },
    KeyedList {
        collection: String,
        item: String,
        index: Option<String>,
        key: String,
        render: String,
    },
}

impl TemplateEmitter<'_> {
    fn emit_node(&mut self, node: &HtmlNode) -> String {
        match node {
            HtmlNode::Element(element) => self.emit_element(element),
            HtmlNode::Text { text, .. } => {
                let var = self.next_node_var();
                self.create_lines.push(format!(
                    "const {} = document.createTextNode(\"{}\");",
                    var,
                    escape_js(text)
                ));
                var
            }
            HtmlNode::Expr { expr, .. } => {
                if let Some(spec) = ForSpec::parse(expr) {
                    return self.emit_keyed_for(expr, spec);
                }
                if let Some(spec) = IfSpec::parse(expr, &self.owner.component_fns) {
                    return self.emit_conditional(expr, spec);
                }
                if let Some(spec) = ComponentSpec::parse(expr, &self.owner.component_fns) {
                    return self.emit_component_call(expr, spec);
                }

                let var = self.next_node_var();
                self.create_lines
                    .push(format!("const {} = document.createTextNode(\"\");", var));
                self.push_slot(self.node_id_for_var(&var), TemplateSlotKind::Text, expr);
                var
            }
        }
    }

    fn emit_element(&mut self, element: &HtmlElement) -> String {
        let var = self.next_node_var();
        let node_id = self.node_id_for_var(&var);
        self.create_lines.push(format!(
            "const {} = document.createElement(\"{}\");",
            var,
            escape_js(&element.tag)
        ));

        for attr in &element.attrs {
            if attr.name == "ref" {
                match &attr.value {
                    HtmlAttrValue::Bool(true) | HtmlAttrValue::Bool(false) => {}
                    HtmlAttrValue::Static(value) => {
                        self.push_static_slot(
                            node_id,
                            TemplateSlotKind::Ref,
                            format!("\"{}\"", escape_js(value)),
                        );
                    }
                    HtmlAttrValue::Dynamic { expr, .. } => {
                        self.push_slot(node_id, TemplateSlotKind::Ref, expr);
                    }
                }
                continue;
            }

            match &attr.value {
                HtmlAttrValue::Bool(true) => self.create_lines.push(format!(
                    "{}.setAttribute(\"{}\", \"\");",
                    var,
                    escape_js(&attr.name)
                )),
                HtmlAttrValue::Bool(false) => {}
                HtmlAttrValue::Static(value) => self.create_lines.push(format!(
                    "{}.setAttribute(\"{}\", \"{}\");",
                    var,
                    escape_js(&attr.name),
                    escape_js(value)
                )),
                HtmlAttrValue::Dynamic { expr, .. } => {
                    let kind = if let Some(event) = attr.name.strip_prefix("on:") {
                        TemplateSlotKind::Event(event.to_string())
                    } else {
                        TemplateSlotKind::Attr(attr.name.clone())
                    };
                    self.push_slot(node_id, kind, expr);
                }
            }
        }

        for child in &element.children {
            let child_var = self.emit_node(child);
            self.create_lines
                .push(format!("{}.appendChild({});", var, child_var));
        }
        var
    }

    fn emit_keyed_for(&mut self, expr: &Expr, spec: ForSpec<'_>) -> String {
        let var = self.next_node_var();
        self.create_lines
            .push(format!("const {} = document.createTextNode(\"\");", var));

        let collection = self.owner.emit_expr(spec.collection);
        let key = self.owner.emit_expr(spec.key);
        let item = sanitize_identifier(spec.item);
        let index = spec.index.map(sanitize_identifier);
        let item_param = format!("__closkell_{}", item);
        let item_update_param = format!("__closkell_next_{}", item);
        let (render_params, index_binding, update_params, index_update) =
            if let Some(index) = &index {
                let index_param = format!("__closkell_{}", index);
                let index_update_param = format!("__closkell_next_{}", index);
                (
                    format!("{}, {}", item_param, index_param),
                    format!(" let {} = {};", index, index_param),
                    format!("{}, {}", item_update_param, index_update_param),
                    format!(" {} = {};", index, index_update_param),
                )
            } else {
                (
                    item_param.clone(),
                    String::new(),
                    item_update_param.clone(),
                    String::new(),
                )
            };
        let component_expr = self.owner.emit_template_component(spec.template);
        let render = format!(
            "({}) => {{ let {} = {};{} const __closkellItemComponent = {}; return {{ mount(parent, dispatch) {{ return __closkellItemComponent.mount(parent, dispatch); }}, update({}, dispatch, updateContext) {{ {} = {};{} return __closkellItemComponent.update(dispatch, updateContext); }}, dispose() {{ __closkellItemComponent.dispose(); }}, get root() {{ return __closkellItemComponent.root; }}, definition: __closkellItemComponent.definition }}; }}",
            render_params,
            item,
            item_param,
            index_binding,
            component_expr,
            update_params,
            item,
            item_update_param,
            index_update
        );
        let id = self.slots.len();
        self.slots.push(TemplateSlot {
            id,
            node_id: self.node_id_for_var(&var),
            kind: TemplateSlotKind::KeyedList {
                collection,
                item,
                index,
                key,
                render,
            },
            expr: format_expr(expr),
            reads: expand_reads(
                collect_keyed_reads(&spec, &self.owner.component_fns, &self.owner.read_summaries),
                self.read_aliases,
            ),
        });
        var
    }

    fn emit_conditional(&mut self, expr: &Expr, spec: IfSpec<'_>) -> String {
        let var = self.next_node_var();
        self.create_lines
            .push(format!("const {} = document.createTextNode(\"\");", var));

        let condition = self.owner.emit_expr(spec.condition);
        let then_component = self.emit_branch_component(&spec.then_branch);
        let else_component = self.emit_branch_component(&spec.else_branch);
        let render_then = format!("() => {}", then_component);
        let render_else = format!("() => {}", else_component);
        let id = self.slots.len();
        self.slots.push(TemplateSlot {
            id,
            node_id: self.node_id_for_var(&var),
            kind: TemplateSlotKind::Conditional {
                condition,
                render_then,
                render_else,
            },
            expr: format_expr(expr),
            reads: expand_reads(
                collect_conditional_reads(
                    &spec,
                    &self.owner.component_fns,
                    &self.owner.read_summaries,
                ),
                self.read_aliases,
            ),
        });
        var
    }

    fn emit_branch_component(&mut self, branch: &TemplateBranch<'_>) -> String {
        match branch {
            TemplateBranch::Html(template) => self.owner.emit_template_component(template),
            TemplateBranch::If(spec) => self.emit_inline_conditional_component(spec),
            TemplateBranch::Component { expr, spec } => self.emit_branch_component_call(expr, spec),
        }
    }

    fn emit_branch_component_call(&mut self, expr: &Expr, spec: &ComponentSpec<'_>) -> String {
        let component = self.owner.next_temp("__closkellComponent");
        let render = self.owner.emit_expr(expr);
        let args = spec
            .args
            .iter()
            .map(|arg| self.owner.emit_expr(arg))
            .collect::<Vec<_>>()
            .join(", ");
        let update_args = if args.is_empty() {
            String::new()
        } else {
            format!("{}, ", args)
        };

        format!(
            "(() => {{ const {component} = {render}; return {{ mount(parent, dispatch) {{ return {component}.mount(parent, dispatch); }}, update(dispatch, updateContext) {{ return {component}.update({update_args}dispatch, updateContext); }}, dispose() {{ {component}.dispose?.(); }}, get root() {{ return {component}.root; }}, definition: {component}.definition }}; }})()"
        )
    }

    fn emit_inline_conditional_component(&mut self, spec: &IfSpec<'_>) -> String {
        let branch = self.owner.next_temp("__closkellBranch");
        let component = self.owner.next_temp("__closkellComponent");
        let placeholder = self.owner.next_temp("__closkellPlaceholder");
        let condition = self.owner.emit_expr(spec.condition);
        let then_component = self.emit_branch_component(&spec.then_branch);
        let else_component = self.emit_branch_component(&spec.else_branch);

        format!(
            "(() => {{ let {branch} = null; let {component} = null; const {placeholder} = document.createTextNode(\"\"); const __closkellDispose = () => {{ if ({component}?.root?.parentNode) {component}.root.parentNode.removeChild({component}.root); {component}?.dispose?.(); }}; return {{ update(dispatch, updateContext) {{ const __closkellCondition = {condition}; const __closkellNextBranch = __closkellCondition ? \"then\" : \"else\"; let __closkellFresh = false; if ({branch} !== __closkellNextBranch) {{ __closkellDispose(); {component} = __closkellCondition ? {then_component} : {else_component}; {branch} = __closkellNextBranch; __closkellFresh = true; }} const __closkellBranchContext = __closkellFresh && updateContext ? {{ ...updateContext, force: true, frames: updateContext.frames }} : updateContext; {component}?.update?.(dispatch, __closkellBranchContext); return {component}?.root ?? {placeholder}; }}, get root() {{ return {component}?.root ?? {placeholder}; }}, dispose() {{ __closkellDispose(); {component} = null; {branch} = null; }} }}; }})()"
        )
    }

    fn emit_component_call(&mut self, expr: &Expr, spec: ComponentSpec<'_>) -> String {
        let var = self.next_node_var();
        self.create_lines
            .push(format!("const {} = document.createTextNode(\"\");", var));

        let render = self.owner.emit_expr(expr);
        let args = spec
            .args
            .iter()
            .map(|arg| self.owner.emit_expr(arg))
            .collect::<Vec<_>>()
            .join(", ");
        let id = self.slots.len();
        self.slots.push(TemplateSlot {
            id,
            node_id: self.node_id_for_var(&var),
            kind: TemplateSlotKind::Component {
                name: spec.name.to_string(),
                render,
                args: format!("[{}]", args),
            },
            expr: format_expr(expr),
            reads: expand_reads(
                component_call_reads(&spec, &self.owner.read_summaries),
                self.read_aliases,
            ),
        });
        var
    }

    fn next_node_var(&mut self) -> String {
        let id = self.nodes.len();
        let var = format!("n{}_{}", self.template_id, id);
        self.nodes.push(var.clone());
        var
    }

    fn node_id_for_var(&self, var: &str) -> usize {
        self.nodes
            .iter()
            .position(|node| node == var)
            .expect("node variable should have been registered")
    }

    fn push_slot(&mut self, node_id: usize, kind: TemplateSlotKind, expr: &Expr) {
        let id = self.slots.len();
        let reads = match &kind {
            TemplateSlotKind::Event(_) => Vec::new(),
            TemplateSlotKind::Text
            | TemplateSlotKind::Attr(_)
            | TemplateSlotKind::Ref
            | TemplateSlotKind::Conditional { .. }
            | TemplateSlotKind::Component { .. }
            | TemplateSlotKind::KeyedList { .. } => expand_reads(
                collect_template_reads(expr, &self.owner.read_summaries),
                self.read_aliases,
            ),
        };
        let expr = self.owner.emit_expr(expr);
        self.slots.push(TemplateSlot {
            id,
            node_id,
            kind,
            expr,
            reads,
        });
    }

    fn push_static_slot(&mut self, node_id: usize, kind: TemplateSlotKind, expr: String) {
        let id = self.slots.len();
        self.slots.push(TemplateSlot {
            id,
            node_id,
            kind,
            expr,
            reads: Vec::new(),
        });
    }

    fn emit_update_body(&self) -> String {
        let mut lines = Vec::new();
        for slot in &self.slots {
            let update = match &slot.kind {
                TemplateSlotKind::Text => format!(
                    "__closkellSetText(instance, {}, instance.nodes[{}], {});",
                    slot.id, slot.node_id, slot.expr
                ),
                TemplateSlotKind::Attr(name) => format!(
                    "__closkellSetAttr(instance, {}, instance.nodes[{}], \"{}\", {});",
                    slot.id,
                    slot.node_id,
                    escape_js(name),
                    slot.expr
                ),
                TemplateSlotKind::Event(event) => format!(
                    "__closkellSetEvent(instance, {}, instance.nodes[{}], \"{}\", (event) => {}, dispatch);",
                    slot.id,
                    slot.node_id,
                    escape_js(event),
                    parenthesize_arrow_body(&slot.expr)
                ),
                TemplateSlotKind::Ref => format!(
                    "__closkellSetRef(instance, {}, instance.nodes[{}], {}, dispatch);",
                    slot.id, slot.node_id, slot.expr
                ),
                TemplateSlotKind::Conditional {
                    condition,
                    render_then,
                    render_else,
                } => format!(
                    "__closkellSetConditional(instance, {}, instance.nodes[{}], {}, {}, {}, dispatch, updateContext);",
                    slot.id, slot.node_id, condition, render_then, render_else
                ),
                TemplateSlotKind::Component { render, args, .. } => format!(
                    "__closkellSetComponent(instance, {}, instance.nodes[{}], () => {}, {}, dispatch, updateContext);",
                    slot.id, slot.node_id, render, args
                ),
                TemplateSlotKind::KeyedList {
                    collection,
                    item,
                    index,
                    key,
                    render,
                } => {
                    let key_params = if let Some(index) = index {
                        format!("{}, {}", item, index)
                    } else {
                        item.clone()
                    };
                    format!(
                        "__closkellSetKeyedList(instance, {}, instance.nodes[{}], {}, ({}) => {}, {}, dispatch, updateContext);",
                        slot.id, slot.node_id, collection, key_params, key, render
                    )
                }
            };
            lines.push(format!(
                "if (__closkellShouldUpdateSlot(instance, {}, updateContext)) {{ {} }}",
                slot.id, update
            ));
        }
        lines.join(" ")
    }

    fn emit_metadata(&self) -> String {
        let slots = self
            .slots
            .iter()
            .map(|slot| {
                let kind = match &slot.kind {
                    TemplateSlotKind::Text => "\"text\"".to_string(),
                    TemplateSlotKind::Attr(name) => format!("{{ attr: \"{}\" }}", escape_js(name)),
                    TemplateSlotKind::Event(name) => {
                        format!("{{ event: \"{}\" }}", escape_js(name))
                    }
                    TemplateSlotKind::Ref => "{ ref: true }".to_string(),
                    TemplateSlotKind::Conditional { .. } => "{ conditional: true }".to_string(),
                    TemplateSlotKind::Component { name, .. } => {
                        format!("{{ component: \"{}\" }}", escape_js(name))
                    }
                    TemplateSlotKind::KeyedList { item, index, .. } => match index {
                        Some(index) => format!(
                            "{{ keyed: \"{}\", index: \"{}\" }}",
                            escape_js(item),
                            escape_js(index)
                        ),
                        None => format!("{{ keyed: \"{}\" }}", escape_js(item)),
                    },
                };
                let reads = slot
                    .reads
                    .iter()
                    .map(|read| format!("\"{}\"", escape_js(read)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "{{ id: {}, node: {}, kind: {}, reads: [{}] }}",
                    slot.id, slot.node_id, kind, reads
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}]", slots)
    }
}

struct ForSpec<'a> {
    item: &'a str,
    index: Option<&'a str>,
    collection: &'a Expr,
    key: &'a Expr,
    template: &'a HtmlNode,
}

impl<'a> ForSpec<'a> {
    fn parse(expr: &'a Expr) -> Option<Self> {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        if items.len() != 3 || !matches_symbol(&items[0], "for") {
            return None;
        }

        let ExprKind::Vector(bindings) = &items[1].kind else {
            return None;
        };
        if bindings.len() != 4 && bindings.len() != 5 {
            return None;
        }

        let ExprKind::Symbol(item) = &bindings[0].kind else {
            return None;
        };
        let (index, key_marker, key) = if bindings.len() == 5 {
            let ExprKind::Symbol(index) = &bindings[2].kind else {
                return None;
            };
            (Some(index.as_str()), &bindings[3], &bindings[4])
        } else {
            (None, &bindings[2], &bindings[3])
        };
        if !matches!(&key_marker.kind, ExprKind::Keyword(name) if name == "key") {
            return None;
        }

        let ExprKind::HtmlTemplate(template) = &items[2].kind else {
            return None;
        };

        Some(Self {
            item,
            index,
            collection: &bindings[1],
            key,
            template,
        })
    }
}

struct IfSpec<'a> {
    condition: &'a Expr,
    then_branch: TemplateBranch<'a>,
    else_branch: TemplateBranch<'a>,
}

enum TemplateBranch<'a> {
    Html(&'a HtmlNode),
    If(Box<IfSpec<'a>>),
    Component {
        expr: &'a Expr,
        spec: ComponentSpec<'a>,
    },
}

impl<'a> TemplateBranch<'a> {
    fn parse(expr: &'a Expr, components: &BTreeSet<String>) -> Option<Self> {
        match &expr.kind {
            ExprKind::HtmlTemplate(template) => Some(Self::Html(template)),
            ExprKind::List(_) => IfSpec::parse(expr, components)
                .map(|spec| Self::If(Box::new(spec)))
                .or_else(|| {
                    ComponentSpec::parse(expr, components)
                        .map(|spec| Self::Component { expr, spec })
                }),
            _ => None,
        }
    }
}

impl<'a> IfSpec<'a> {
    fn parse(expr: &'a Expr, components: &BTreeSet<String>) -> Option<Self> {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        if items.len() != 4 || !matches_symbol(&items[0], "if") {
            return None;
        }

        let then_branch = TemplateBranch::parse(&items[2], components)?;
        let else_branch = TemplateBranch::parse(&items[3], components)?;

        Some(Self {
            condition: &items[1],
            then_branch,
            else_branch,
        })
    }
}

struct ComponentSpec<'a> {
    name: &'a str,
    args: &'a [Expr],
}

impl<'a> ComponentSpec<'a> {
    fn parse(expr: &'a Expr, components: &BTreeSet<String>) -> Option<Self> {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        let Some((head, args)) = items.split_first() else {
            return None;
        };
        let ExprKind::Symbol(name) = &head.kind else {
            return None;
        };
        if !components.contains(name) {
            return None;
        }

        Some(Self { name, args })
    }
}

fn matches_symbol(expr: &Expr, expected: &str) -> bool {
    matches!(&expr.kind, ExprKind::Symbol(name) if name == expected)
}

fn symbol_name(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Symbol(name) => Some(name),
        _ => None,
    }
}

fn is_data_constructor_pattern(name: &str) -> bool {
    matches!(name, "some" | "ok" | "err" | "list" | "cons")
}

fn let_template_parts(expr: &Expr) -> Option<(&[Expr], &HtmlNode)> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    if items.len() != 3 || !matches_symbol(&items[0], "let") {
        return None;
    }
    let ExprKind::Vector(bindings) = &items[1].kind else {
        return None;
    };
    let ExprKind::HtmlTemplate(node) = &items[2].kind else {
        return None;
    };
    Some((bindings, node))
}

fn is_template_component_expr(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::HtmlTemplate(_)) || let_template_parts(expr).is_some()
}

fn collect_template_defns(source: &SourceFile) -> BTreeSet<String> {
    source
        .forms
        .iter()
        .filter_map(|form| {
            let ExprKind::List(items) = &form.kind else {
                return None;
            };
            if items.len() != 4 || !matches_symbol(&items[0], "defn") {
                return None;
            }
            let ExprKind::Symbol(name) = &items[1].kind else {
                return None;
            };
            let Some(body) = items.last() else {
                return None;
            };
            if !is_template_component_expr(body) {
                return None;
            }

            Some(name.clone())
        })
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ReadSummary {
    params: Vec<String>,
    reads: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReadAlias {
    reads: Vec<String>,
    projectable: bool,
}

type ReadAliases = BTreeMap<String, ReadAlias>;

fn collect_read_summaries(source: &SourceFile) -> BTreeMap<String, ReadSummary> {
    let components = collect_template_defns(source);
    let defs = source
        .forms
        .iter()
        .filter_map(|form| {
            let ExprKind::List(items) = &form.kind else {
                return None;
            };
            if items.len() < 4 || !matches_symbol(&items[0], "defn") {
                return None;
            }
            let ExprKind::Symbol(name) = &items[1].kind else {
                return None;
            };
            let params = params_from_vector(&items[2])?;
            Some((name.clone(), params, items[3..].to_vec()))
        })
        .collect::<Vec<_>>();

    let mut summaries = BTreeMap::new();
    for (name, params, bodies) in &defs {
        let reads = bodies
            .iter()
            .flat_map(|body| collect_template_reads(body, &BTreeMap::new()))
            .collect::<Vec<_>>();
        summaries.insert(
            name.clone(),
            ReadSummary {
                params: params.clone(),
                reads: filter_param_reads(params, reads),
            },
        );
    }

    for (name, params, bodies) in defs {
        let mut reads = BTreeSet::new();
        for body in &bodies {
            reads.extend(collect_summary_body_reads(body, &components, &summaries));
        }
        summaries.insert(
            name,
            ReadSummary {
                params: params.clone(),
                reads: filter_param_reads(&params, reads.into_iter().collect()),
            },
        );
    }

    summaries
}

fn params_from_vector(expr: &Expr) -> Option<Vec<String>> {
    let ExprKind::Vector(params) = &expr.kind else {
        return None;
    };
    let mut names = Vec::new();
    for param in params {
        let ExprKind::Symbol(name) = &param.kind else {
            return None;
        };
        names.push(name.clone());
    }
    Some(names)
}

fn collect_summary_body_reads(
    body: &Expr,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    if let Some(template) = template_expr(body, read_summaries) {
        let mut reads = BTreeSet::new();
        collect_html_node_reads(template.node, &mut reads, components, read_summaries);
        return expand_reads(reads.into_iter().collect(), &template.read_aliases);
    }
    collect_template_reads(body, read_summaries)
}

fn filter_param_reads(params: &[String], reads: Vec<String>) -> Vec<String> {
    reads
        .into_iter()
        .filter(|read| {
            let (head, _) = split_read(read);
            params.iter().any(|param| param == head)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn collect_template_reads(
    expr: &Expr,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    collect_template_reads_inner(expr, &mut symbols, read_summaries);
    symbols.into_iter().collect()
}

fn expand_reads(reads: Vec<String>, aliases: &ReadAliases) -> Vec<String> {
    let mut expanded = BTreeSet::new();
    for read in reads {
        expand_read(&read, aliases, &mut BTreeSet::new(), &mut expanded);
    }
    expanded.into_iter().collect()
}

fn expand_read(
    read: &str,
    aliases: &ReadAliases,
    visiting: &mut BTreeSet<String>,
    expanded: &mut BTreeSet<String>,
) {
    let (head, suffix) = split_read(read);
    let Some(alias) = aliases.get(head) else {
        expanded.insert(read.to_string());
        return;
    };
    if !visiting.insert(head.to_string()) {
        return;
    }
    for alias_read in &alias.reads {
        let expanded_read = if alias.projectable {
            append_read_suffix_str(alias_read, suffix)
        } else {
            alias_read.clone()
        };
        expand_read(&expanded_read, aliases, visiting, expanded);
    }
    visiting.remove(head);
}

struct TemplateExpr<'a> {
    node: &'a HtmlNode,
    read_aliases: ReadAliases,
}

fn template_expr<'a>(
    expr: &'a Expr,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Option<TemplateExpr<'a>> {
    match &expr.kind {
        ExprKind::HtmlTemplate(node) => Some(TemplateExpr {
            node: node.as_ref(),
            read_aliases: ReadAliases::new(),
        }),
        ExprKind::List(items) if items.len() == 3 && matches_symbol(&items[0], "let") => {
            let ExprKind::Vector(bindings) = &items[1].kind else {
                return None;
            };
            let ExprKind::HtmlTemplate(node) = &items[2].kind else {
                return None;
            };
            Some(TemplateExpr {
                node: node.as_ref(),
                read_aliases: collect_read_aliases(bindings, read_summaries),
            })
        }
        _ => None,
    }
}

fn collect_read_aliases(
    bindings: &[Expr],
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> ReadAliases {
    let mut aliases = ReadAliases::new();
    for pair in bindings.chunks(2) {
        let [binding, value] = pair else {
            continue;
        };
        let value_reads = expand_reads(collect_template_reads(value, read_summaries), &aliases);
        let can_project = can_project_read_alias(value, &aliases);
        collect_pattern_read_aliases(binding, &value_reads, can_project, &mut aliases);
    }
    aliases
}

fn can_project_read_alias(value: &Expr, aliases: &ReadAliases) -> bool {
    projectable_read_path(value, aliases).is_some()
}

fn collect_pattern_read_aliases(
    pattern: &Expr,
    base_reads: &[String],
    can_project: bool,
    aliases: &mut ReadAliases,
) {
    collect_pattern_read_aliases_inner(pattern, base_reads, can_project, &[], aliases);
}

fn collect_pattern_read_aliases_inner(
    pattern: &Expr,
    base_reads: &[String],
    can_project: bool,
    suffix: &[String],
    aliases: &mut ReadAliases,
) {
    match &pattern.kind {
        ExprKind::Symbol(name) if name == "_" => {}
        ExprKind::Symbol(name) => {
            aliases.insert(
                name.clone(),
                read_alias_for_suffix(base_reads, can_project, suffix, can_project),
            );
        }
        ExprKind::List(items) if items.first().is_some_and(|head| matches_symbol(head, "as")) => {
            if items.len() == 3 {
                collect_pattern_read_aliases_inner(
                    &items[1],
                    base_reads,
                    can_project,
                    suffix,
                    aliases,
                );
                if let ExprKind::Symbol(name) = &items[2].kind {
                    if name != "_" {
                        aliases.insert(
                            name.clone(),
                            read_alias_for_suffix(base_reads, can_project, suffix, can_project),
                        );
                    }
                }
            }
        }
        ExprKind::List(items) => {
            let Some(name) = items.first().and_then(symbol_name) else {
                return;
            };
            match name {
                "list" => {
                    for (index, item) in items[1..].iter().enumerate() {
                        collect_pattern_read_aliases_inner(
                            item,
                            base_reads,
                            can_project,
                            &with_suffix(suffix, index.to_string()),
                            aliases,
                        );
                    }
                }
                "cons" if items.len() == 3 => {
                    collect_pattern_read_aliases_inner(
                        &items[1],
                        base_reads,
                        can_project,
                        &with_suffix(suffix, "0"),
                        aliases,
                    );
                    let tail_reads = alias_reads_for_suffix(base_reads, can_project, suffix);
                    collect_pattern_read_aliases_inner(&items[2], &tail_reads, false, &[], aliases);
                }
                "some" if items.len() == 2 => {
                    collect_pattern_read_aliases_inner(
                        &items[1],
                        base_reads,
                        can_project,
                        suffix,
                        aliases,
                    );
                }
                "ok" | "err" if items.len() == 2 => {
                    let field = if name == "ok" { "value" } else { "error" };
                    collect_pattern_read_aliases_inner(
                        &items[1],
                        base_reads,
                        can_project,
                        &with_suffix(suffix, field),
                        aliases,
                    );
                }
                _ => {}
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                let Some(name) = object_key_name(key) else {
                    continue;
                };
                collect_pattern_read_aliases_inner(
                    value,
                    base_reads,
                    can_project,
                    &with_suffix(suffix, name),
                    aliases,
                );
            }
        }
        ExprKind::Vector(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_pattern_read_aliases_inner(
                    item,
                    base_reads,
                    can_project,
                    &with_suffix(suffix, index.to_string()),
                    aliases,
                );
            }
        }
        _ => {}
    }
}

fn read_alias_for_suffix(
    base_reads: &[String],
    can_project: bool,
    suffix: &[String],
    projectable: bool,
) -> ReadAlias {
    ReadAlias {
        reads: alias_reads_for_suffix(base_reads, can_project, suffix),
        projectable,
    }
}

fn alias_reads_for_suffix(
    base_reads: &[String],
    can_project: bool,
    suffix: &[String],
) -> Vec<String> {
    if can_project && !suffix.is_empty() {
        return base_reads
            .iter()
            .map(|read| append_read_suffix(read, suffix))
            .collect();
    }
    base_reads.to_vec()
}

fn append_read_suffix(read: &str, suffix: &[String]) -> String {
    if suffix.is_empty() {
        return read.to_string();
    }
    format!("{}.{}", read, suffix.join("."))
}

fn append_read_suffix_str(read: &str, suffix: Option<&str>) -> String {
    suffix
        .map(|suffix| format!("{}.{}", read, suffix))
        .unwrap_or_else(|| read.to_string())
}

fn with_suffix(suffix: &[String], part: impl Into<String>) -> Vec<String> {
    let mut next = suffix.to_vec();
    next.push(part.into());
    next
}

fn collect_pattern_symbols(pattern: &Expr, symbols: &mut BTreeSet<String>) {
    match &pattern.kind {
        ExprKind::Symbol(name) if name == "_" => {}
        ExprKind::Symbol(name) => {
            symbols.insert(sanitize_identifier(name));
        }
        ExprKind::List(items) if items.first().is_some_and(|head| matches_symbol(head, "as")) => {
            if items.len() == 3 {
                collect_pattern_symbols(&items[1], symbols);
                if let ExprKind::Symbol(name) = &items[2].kind {
                    if name != "_" {
                        symbols.insert(sanitize_identifier(name));
                    }
                }
            }
        }
        ExprKind::List(items) => {
            let Some(name) = items.first().and_then(symbol_name) else {
                return;
            };
            match name {
                "list" => {
                    for item in &items[1..] {
                        collect_pattern_symbols(item, symbols);
                    }
                }
                "cons" if items.len() == 3 => {
                    collect_pattern_symbols(&items[1], symbols);
                    collect_pattern_symbols(&items[2], symbols);
                }
                "some" | "ok" | "err" if items.len() == 2 => {
                    collect_pattern_symbols(&items[1], symbols);
                }
                _ => {}
            }
        }
        ExprKind::Map(entries) => {
            for (_, value) in entries {
                collect_pattern_symbols(value, symbols);
            }
        }
        ExprKind::Vector(items) => {
            for item in items {
                collect_pattern_symbols(item, symbols);
            }
        }
        _ => {}
    }
}

fn const_bindings_to_assignments(bindings: &str) -> String {
    bindings
        .split(';')
        .filter_map(|statement| {
            let statement = statement.trim();
            if statement.is_empty() {
                None
            } else if let Some(assignment) = statement.strip_prefix("const ") {
                Some(format!("{};", assignment))
            } else {
                Some(format!("{};", statement))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_conditional_reads(
    spec: &IfSpec<'_>,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    collect_template_reads_inner(spec.condition, &mut symbols, read_summaries);
    collect_template_branch_reads(&spec.then_branch, &mut symbols, components, read_summaries);
    collect_template_branch_reads(&spec.else_branch, &mut symbols, components, read_summaries);
    symbols.into_iter().collect()
}

fn collect_keyed_reads(
    spec: &ForSpec<'_>,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    collect_template_reads_inner(spec.collection, &mut symbols, read_summaries);
    collect_template_reads_inner(spec.key, &mut symbols, read_summaries);
    collect_html_node_reads(spec.template, &mut symbols, components, read_summaries);
    let item_prefix = format!("{}.", spec.item);
    symbols.retain(|read| read != spec.item && !read.starts_with(&item_prefix));
    if let Some(index) = spec.index {
        let index_prefix = format!("{}.", index);
        symbols.retain(|read| read != index && !read.starts_with(&index_prefix));
    }
    symbols.into_iter().collect()
}

fn collect_template_branch_reads(
    branch: &TemplateBranch<'_>,
    symbols: &mut BTreeSet<String>,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) {
    match branch {
        TemplateBranch::Html(node) => {
            collect_html_node_reads(node, symbols, components, read_summaries)
        }
        TemplateBranch::If(spec) => {
            collect_template_reads_inner(spec.condition, symbols, read_summaries);
            collect_template_branch_reads(&spec.then_branch, symbols, components, read_summaries);
            collect_template_branch_reads(&spec.else_branch, symbols, components, read_summaries);
        }
        TemplateBranch::Component { spec, .. } => {
            symbols.extend(component_call_reads(spec, read_summaries));
        }
    }
}

fn collect_html_node_reads(
    node: &HtmlNode,
    symbols: &mut BTreeSet<String>,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) {
    match node {
        HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if attr.name.starts_with("on:") {
                    continue;
                }
                if let HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_template_reads_inner(expr, symbols, read_summaries);
                }
            }
            for child in &element.children {
                collect_html_node_reads(child, symbols, components, read_summaries);
            }
        }
        HtmlNode::Expr { expr, .. } => {
            if let Some(spec) = ForSpec::parse(expr) {
                symbols.extend(collect_keyed_reads(&spec, components, read_summaries));
                return;
            }
            if let Some(spec) = IfSpec::parse(expr, components) {
                symbols.extend(collect_conditional_reads(&spec, components, read_summaries));
                return;
            }
            collect_template_reads_inner(expr, symbols, read_summaries);
        }
        HtmlNode::Text { .. } => {}
    }
}

fn collect_template_reads_inner(
    expr: &Expr,
    symbols: &mut BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) {
    match &expr.kind {
        ExprKind::Symbol(name) => {
            symbols.insert(name.clone());
        }
        ExprKind::List(items) => {
            if let Some(read) = projectable_read_path(expr, &ReadAliases::new()) {
                symbols.insert(read);
                return;
            }
            if let Some((head, args)) = items.split_first() {
                if let ExprKind::Symbol(name) = &head.kind {
                    if let Some(summary) = read_summaries.get(name) {
                        symbols.extend(project_call_reads(summary, args, read_summaries));
                        return;
                    }
                    for item in args {
                        collect_template_reads_inner(item, symbols, read_summaries);
                    }
                    return;
                }
            }
            for item in items {
                collect_template_reads_inner(item, symbols, read_summaries);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_template_reads_inner(item, symbols, read_summaries);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_template_reads_inner(key, symbols, read_summaries);
                collect_template_reads_inner(value, symbols, read_summaries);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => {
            collect_template_reads_inner(inner, symbols, read_summaries)
        }
        ExprKind::HtmlTemplate(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn component_call_reads(
    spec: &ComponentSpec<'_>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    let Some(summary) = read_summaries.get(spec.name) else {
        return spec
            .args
            .iter()
            .flat_map(|arg| collect_template_reads(arg, read_summaries))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
    };
    project_call_reads(summary, spec.args, read_summaries)
}

fn project_call_reads(
    summary: &ReadSummary,
    args: &[Expr],
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    let mut reads = BTreeSet::new();
    for read in &summary.reads {
        let (head, suffix) = split_read(read);
        let Some(index) = summary.params.iter().position(|param| param == head) else {
            reads.insert(read.clone());
            continue;
        };
        let Some(arg) = args.get(index) else {
            continue;
        };
        if let Some(suffix) = suffix {
            if let Some(base) = projectable_read_path(arg, &ReadAliases::new()) {
                reads.insert(format!("{}.{}", base, suffix));
            } else {
                collect_template_reads_inner(arg, &mut reads, read_summaries);
            }
        } else {
            collect_template_reads_inner(arg, &mut reads, read_summaries);
        }
    }
    reads.into_iter().collect()
}

fn split_read(read: &str) -> (&str, Option<&str>) {
    read.split_once('.')
        .map(|(head, suffix)| (head, Some(suffix)))
        .unwrap_or((read, None))
}

fn projectable_read_path(expr: &Expr, aliases: &ReadAliases) -> Option<String> {
    match &expr.kind {
        ExprKind::Symbol(read) if read_path_projectable(read, aliases) => Some(read.clone()),
        ExprKind::List(items) => projectable_indexed_read_path(items, aliases),
        _ => None,
    }
}

fn projectable_indexed_read_path(items: &[Expr], aliases: &ReadAliases) -> Option<String> {
    let (head, args) = items.split_first()?;
    let name = symbol_name(head)?;
    let (collection, index) = match name {
        "first" if args.len() == 1 => (&args[0], 0),
        "second" if args.len() == 1 => (&args[0], 1),
        "nth" if args.len() == 2 => (&args[0], literal_vector_index(&args[1])?),
        _ => return None,
    };
    Some(format!(
        "{}.{}",
        projectable_read_path(collection, aliases)?,
        index
    ))
}

fn literal_vector_index(expr: &Expr) -> Option<usize> {
    let ExprKind::Number(value) = &expr.kind else {
        return None;
    };
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn read_path_projectable(read: &str, aliases: &ReadAliases) -> bool {
    let (head, _) = split_read(read);
    aliases.get(head).is_none_or(|alias| alias.projectable)
}

fn sanitize_identifier(name: &str) -> String {
    let mut output = String::new();
    for (index, ch) in name.chars().enumerate() {
        let valid = ch.is_ascii_alphanumeric() || ch == '_';
        if index == 0 && ch.is_ascii_digit() {
            output.push('_');
        }
        output.push(if valid { ch } else { '_' });
    }
    if output.is_empty() {
        "_".to_string()
    } else {
        output
    }
}

fn emit_symbol_read(name: &str) -> String {
    let parts = name.split('.').collect::<Vec<_>>();
    if parts.len() < 2 || parts.iter().any(|part| part.is_empty()) {
        return sanitize_identifier(name);
    }

    let mut output = sanitize_identifier(parts[0]);
    for part in &parts[1..] {
        output.push_str(&property_access(part));
    }
    output
}

fn object_key(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Keyword(name) | ExprKind::Symbol(name) | ExprKind::String(name) => {
            Some(property_key(name))
        }
        _ => None,
    }
}

fn object_key_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Keyword(name) | ExprKind::Symbol(name) | ExprKind::String(name) => {
            Some(name.clone())
        }
        _ => None,
    }
}

fn property_key(name: &str) -> String {
    if is_js_identifier(name) {
        name.to_string()
    } else {
        format!("\"{}\"", escape_js(name))
    }
}

fn property_access(name: &str) -> String {
    if is_js_identifier(name) {
        format!(".{}", name)
    } else {
        format!("[\"{}\"]", escape_js(name))
    }
}

fn optional_property_access(name: &str) -> String {
    if is_js_identifier(name) {
        format!("?.{}", name)
    } else {
        format!("?.[\"{}\"]", escape_js(name))
    }
}

fn parenthesize_member_base(value: String) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '[' | ']' | '"' | '?'))
    {
        value
    } else {
        format!("({})", value)
    }
}

fn parenthesize_expression(value: String) -> String {
    let trimmed = value.trim();
    if trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '[' | ']' | '"' | '?'))
    {
        value
    } else {
        format!("({})", value)
    }
}

fn parenthesize_arrow_body(value: &str) -> String {
    let trimmed = value.trim_start();
    if trimmed.starts_with('{') {
        format!("({})", value)
    } else {
        value.to_string()
    }
}

fn is_js_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_' || first == '$')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

fn escape_js(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

fn js_string_array(values: &[String]) -> String {
    let entries = values
        .iter()
        .map(|value| format!("\"{}\"", escape_js(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{}]", entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_definitions_as_esm_exports() {
        let source = syntax::parse_source("(def answer (+ 40 2))");
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert_eq!(emitted.code.trim(), "export const answer = 40 + 2;");
    }

    #[test]
    fn emits_self_tail_calls_as_loop() {
        let source = syntax::parse_source(
            "(defn sum-down [n total]\n  (if (<= n 0)\n      total\n      (sum-down (- n 1) (+ total n))))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("while (true)"));
        assert!(emitted.code.contains("continue;"));
        assert!(emitted.code.contains("n = __closkell_tail"));
        assert!(emitted.code.contains("total = __closkell_tail"));
        assert!(!emitted.code.contains("return sum_down("));
    }

    #[test]
    fn emits_source_mappings_for_top_level_forms() {
        let source =
            syntax::parse_source("(def answer (+ 40 2))\n(defn label [value] (str value))");
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert_eq!(emitted.source_mappings.len(), 2);
        assert_eq!(emitted.source_mappings[0].generated_line, 0);
        assert_eq!(
            emitted.source_mappings[0].source_offset,
            source.forms[0].span.start
        );
        assert_eq!(emitted.source_mappings[1].generated_line, 1);
        assert_eq!(
            emitted.source_mappings[1].source_offset,
            source.forms[1].span.start
        );
    }

    #[test]
    fn source_mappings_account_for_import_and_runtime_preamble() {
        let source =
            syntax::parse_source("(import \"./dep.clsk\" [value])\n#html <div>{value}</div>");
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert_eq!(emitted.source_mappings.len(), 2);
        assert_eq!(emitted.source_mappings[0].generated_line, 2);
        assert_eq!(emitted.source_mappings[1].generated_line, 4);
    }

    #[test]
    fn emits_imports_as_esm_imports() {
        let source = syntax::parse_source(
            "(import \"./hrweb_metrics.clsk\" [calculate-trimp matches-hrr-type?])\n\
             (defn summarize [entry] (calculate-trimp entry))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(
            "import { calculate_trimp, matches_hrr_type_ } from \"./hrweb_metrics.mjs\";"
        ));
        assert!(!emitted.code.contains("value0"));
        assert!(emitted.code.contains("calculate_trimp(entry)"));
    }

    #[test]
    fn erases_type_only_names_from_esm_imports() {
        let source = syntax::parse_source(
            "(import \"./chart.clsk\" [HeartReading HeartZone heart-chart-command])\n\
             (defn draw [state] (heart-chart-command state))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("import { heart_chart_command } from \"./chart.mjs\";")
        );
        assert!(!emitted.code.contains("HeartReading"));
        assert!(!emitted.code.contains("HeartZone"));
    }

    #[test]
    fn erases_type_only_esm_imports_entirely() {
        let source = syntax::parse_source(
            "(import \"./chart.clsk\" [HeartReading HeartZone])\n\
             (def answer 42)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(!emitted.code.contains("import"));
        assert_eq!(emitted.code.trim(), "export const answer = 42;");
    }

    #[test]
    fn erases_type_declarations_from_js_output() {
        let source = syntax::parse_source(
            "(type WorkoutMsg (Union {:kind :start} {:kind :heart-rate :bpm Number}))\n\
             (ann api-type-count Number)\n\
             (def api-type-count 1)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert_eq!(emitted.code.trim(), "export const api_type_count = 1;");
        assert_eq!(emitted.source_mappings.len(), 1);
        assert_eq!(
            emitted.source_mappings[0].source_offset,
            source.forms[2].span.start
        );
    }

    #[test]
    fn imports_runtime_for_templates() {
        let source = syntax::parse_source("#html <div>{label}</div>");
        let emitted = emit_module(&source);

        assert!(emitted.code.contains("@closkell/runtime"));
        assert!(emitted.code.contains("__closkellCreateTemplate"));
        assert!(emitted.code.contains("__closkellSetText"));
    }

    #[test]
    fn emits_template_slots_for_attrs_events_and_text() {
        let source = syntax::parse_source(
            "(defn status-view [state] #html <button class={state.buttonClass} disabled={not state.connected?} on:click={:start}>{state.label}</button>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("export function status_view(state)"));
        assert!(emitted.code.contains("document.createElement(\"button\")"));
        assert!(emitted.code.contains("__closkellSetAttr(instance, 0"));
        assert!(emitted.code.contains("__closkellSetAttr(instance, 1"));
        assert!(emitted.code.contains("__closkellSetEvent(instance, 2"));
        assert!(emitted.code.contains("__closkellSetText(instance, 3"));
        assert!(emitted.code.contains("state.buttonClass"));
        assert!(emitted.code.contains("!(state[\"connected?\"])"));
        assert!(emitted.code.contains("state.label"));
    }

    #[test]
    fn emits_indexed_keyed_template_loops() {
        let source = syntax::parse_source(
            "(defn view [state]\n  #html <section>{(for [zone state.zones index :key zone.id] #html <button data-index={index} on:click={{:kind :select :rank (+ index 1)}}>{(str index zone.name)}</button>)}</section>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetKeyedList"));
        assert!(emitted.code.contains("(zone, index) => zone.id"));
        assert!(
            emitted
                .code
                .contains("(__closkell_zone, __closkell_index) =>")
        );
        assert!(emitted.code.contains("let index = __closkell_index"));
        assert!(emitted.code.contains(
            "update(__closkell_next_zone, __closkell_next_index, dispatch, updateContext)"
        ));
        assert!(emitted.code.contains("index = __closkell_next_index"));
        assert!(
            emitted
                .code
                .contains("{ keyed: \"zone\", index: \"index\" }")
        );
    }

    #[test]
    fn emits_update_wrapper_for_let_wrapped_template_defns() {
        let source = syntax::parse_source(
            "(defn view [state] (let [label state.label] #html <p>{label}</p>))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("export function view(state)"));
        assert!(emitted.code.contains("let label;"));
        assert!(
            emitted
                .code
                .contains("const __closkellRefresh = () => { label = state.label; }")
        );
        assert!(emitted.code.contains(
            "update(next_state, dispatch, updateContext) { state = next_state; __closkellRefresh();"
        ));
        assert!(emitted.code.contains("__closkellSetText(instance, 0"));
        assert!(
            emitted
                .code
                .contains("__closkellShouldUpdateSlot(instance, 0, updateContext)")
        );
        assert!(!emitted.code.contains("return (() =>"));
    }

    #[test]
    fn emits_source_reads_for_let_wrapped_template_metadata() {
        let source = syntax::parse_source(
            "(defn stat-tile [label value] #html <strong>{value}</strong>)\n\
             (defn view [state]\n  (let [avg (average-bpm state.readings)\n        entry (selected-log state.entries state.selectedLogId)]\n    #html <section>\n            {(stat-tile \"Avg\" avg)}\n            <p>{entry.durationMs}</p>\n            {(for [item state.entries :key item.id]\n               #html <button on:click={{:kind :select :id item.id}}>{item.label}</button>)}\n          </section>))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("kind: { component: \"stat-tile\" }, reads: [\"state.readings\"]"),
            "let-derived component slot did not use source state reads:\n{}",
            emitted.code
        );
        assert!(
            emitted
                .code
                .contains("kind: \"text\", reads: [\"state.entries\", \"state.selectedLogId\"]"),
            "let-derived text slot did not use source state reads:\n{}",
            emitted.code
        );
        assert!(
            emitted
                .code
                .contains("kind: { keyed: \"item\" }, reads: [\"state.entries\"]"),
            "keyed list reads should ignore event payloads:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_pattern_let_wrapped_template_reads_and_refresh_assignments() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (let [{:reading {:bpm bpm}\n\
                      :samples (cons head rest)} state.payload]\n\
                 #html <section data-bpm={bpm} data-head={head} data-count={(count rest)}></section>))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("let bpm;"));
        assert!(emitted.code.contains("let head;"));
        assert!(emitted.code.contains("let rest;"));
        assert!(emitted.code.contains("const __closkell_template_let"));
        assert!(emitted.code.contains("let pattern did not match"));
        assert!(emitted.code.contains("bpm = "));
        assert!(emitted.code.contains("head = "));
        assert!(emitted.code.contains("rest = "));
        assert!(
            emitted
                .code
                .contains("reads: [\"state.payload.reading.bpm\"]"),
            "bpm alias did not project to the source field:\n{}",
            emitted.code
        );
        assert!(
            emitted
                .code
                .contains("reads: [\"state.payload.samples.0\"]"),
            "head alias did not project to the list head source:\n{}",
            emitted.code
        );
        assert!(
            emitted.code.contains("reads: [\"state.payload.samples\"]"),
            "rest alias did not preserve the source collection read:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_projected_template_reads_for_direct_let_alias_suffixes() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (let [payload state.payload\n\
                     reading payload.reading]\n\
                 #html <section data-bpm={payload.reading.bpm} data-zone={reading.zone}></section>))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("reads: [\"state.payload.reading.bpm\"]"),
            "direct dotted alias reads should project through payload:\n{}",
            emitted.code
        );
        assert!(
            emitted
                .code
                .contains("reads: [\"state.payload.reading.zone\"]"),
            "alias-of-alias dotted reads should preserve the source suffix:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_projected_template_reads_for_option_and_result_pattern_aliases() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (let [(ok {:entries entries}) state.importResult\n\
                     (some current) state.latest]\n\
                 #html <section data-count={(count entries)} data-bpm={current.bpm}></section>))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("reads: [\"state.importResult.value.entries\"]"),
            "result pattern alias did not project through the ok value field:\n{}",
            emitted.code
        );
        assert!(
            emitted.code.contains("reads: [\"state.latest.bpm\"]"),
            "option pattern alias did not project dotted reads through the source option:\n{}",
            emitted.code
        );
    }

    #[test]
    fn keeps_helper_derived_alias_suffix_reads_on_source_dependencies() {
        let source = syntax::parse_source(
            "(defn selected-log [entries selectedId] (if selectedId entries entries))\n\
             (defn view [state]\n\
               (let [entry (selected-log state.entries state.selectedLogId)\n\
                     details entry.details]\n\
                 #html <section data-duration={entry.durationMs} data-kind={details.kind}></section>))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("reads: [\"state.entries\", \"state.selectedLogId\"]"),
            "helper-derived aliases should remain tied to helper source reads:\n{}",
            emitted.code
        );
        assert!(
            !emitted.code.contains("state.entries.durationMs")
                && !emitted.code.contains("state.selectedLogId.kind"),
            "helper-derived aliases should not project dotted suffixes onto broad source reads:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_indexed_template_reads_for_static_collection_projections() {
        let source = syntax::parse_source(
            "(defn row [entry] #html <li>{entry.label}</li>)\n\
             (defn view [state]\n\
               (let [first-entry (first state.entries)\n\
                     second-entry (second state.entries)\n\
                     third-entry (nth state.entries 2)\n\
                     first-cell (first (nth state.matrix 0))]\n\
                 #html <section data-first={first-entry.label}\n\
                                data-second={second-entry.label}\n\
                                data-third={third-entry.durationMs}\n\
                                data-cell={first-cell.value}>\n\
                         {(row (first state.entries))}\n\
                       </section>))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted.code.contains("reads: [\"state.entries.0.label\"]"),
            "first alias/component reads should project to index 0:\n{}",
            emitted.code
        );
        assert!(
            emitted.code.contains("reads: [\"state.entries.1.label\"]"),
            "second alias reads should project to index 1:\n{}",
            emitted.code
        );
        assert!(
            emitted
                .code
                .contains("reads: [\"state.entries.2.durationMs\"]"),
            "literal nth alias reads should project to the numeric index:\n{}",
            emitted.code
        );
        assert!(
            emitted.code.contains("reads: [\"state.matrix.0.0.value\"]"),
            "nested static indexed aliases should keep the full projection path:\n{}",
            emitted.code
        );
        assert!(
            emitted
                .code
                .contains("kind: { component: \"row\" }, reads: [\"state.entries.0.label\"]"),
            "component args using static indexed projections should keep child suffixes:\n{}",
            emitted.code
        );
    }

    #[test]
    fn keeps_dynamic_nth_reads_on_collection_and_index_dependencies() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (let [entry (nth state.entries state.selectedIndex)]\n\
                 #html <section data-label={entry.label}></section>))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("reads: [\"state.entries\", \"state.selectedIndex\"]"),
            "dynamic nth aliases must keep both collection and index dependencies:\n{}",
            emitted.code
        );
        assert!(
            !emitted.code.contains("state.entries.label")
                && !emitted.code.contains("state.entries.state.selectedIndex"),
            "dynamic nth aliases should not invent a projected path:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_template_ref_slots() {
        let source =
            syntax::parse_source("(defn view [state] #html <canvas ref=\"heart-chart\"></canvas>)");
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetRef"));
        assert!(emitted.code.contains("\"heart-chart\""));
        assert!(emitted.code.contains("{ ref: true }"));
        assert!(!emitted.code.contains("setAttribute(\"ref\""));
    }

    #[test]
    fn emits_component_template_slots() {
        let source = syntax::parse_source(
            "(defn summary-card [summary] #html <article>{summary.label}</article>)\n(defn view [state] #html <section>{(summary-card state.summary)}</section>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetComponent"));
        assert!(emitted.code.contains("() => summary_card(state.summary)"));
        assert!(emitted.code.contains("[state.summary]"));
        assert!(emitted.code.contains("{ component: \"summary-card\" }"));
        assert!(
            emitted
                .code
                .contains("{ component: \"summary-card\" }, reads: [\"state.summary.label\"]")
        );
        assert!(
            emitted
                .code
                .contains("update(next_summary, dispatch, updateContext)")
        );
    }

    #[test]
    fn emits_helper_call_template_reads_as_state_paths() {
        let source = syntax::parse_source(
            "(defn connection-label [state]\n  (if state.connected? (if state.simulated? \"Simulated\" \"Bluetooth\") \"Disconnected\"))\n\
             (defn view [state] #html <h2>{(connection-label state)}</h2>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("kind: \"text\", reads: [\"state.connected?\", \"state.simulated?\"]"),
            "{}",
            emitted.code
        );
    }

    #[test]
    fn emits_component_reads_from_let_wrapped_template_bodies() {
        let source = syntax::parse_source(
            "(defn live-pane [state]\n  (let [avg (average-bpm state.readings)]\n    #html <section>\n            <strong>{state.latestBpm}</strong>\n            <button disabled={(= state.exerciseState \"running\")}>Start</button>\n            <em>{avg}</em>\n          </section>))\n\
             (defn view [state] #html <main>{(live-pane state)}</main>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted.code.contains(
                "kind: { component: \"live-pane\" }, reads: [\"state.exerciseState\", \"state.latestBpm\", \"state.readings\"]"
            ),
            "let-wrapped component summary did not include full template reads:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_defn_and_dotted_record_reads() {
        let source = syntax::parse_source(
            "(defn in-zone? [zone bpm] (and (>= bpm zone.min) (<= bpm zone.max)))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert_eq!(
            emitted.code.trim(),
            "export function in_zone_(zone, bpm) { return (bpm >= zone.min) && (bpm <= zone.max); }"
        );
    }

    #[test]
    fn preserves_nested_infix_precedence() {
        let source = syntax::parse_source(
            "(defn bar-width [plot-width gap count] (/ (- plot-width (* gap (+ count 1))) count))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("(plot_width - (gap * (count + 1))) / count"),
            "{}",
            emitted.code
        );
    }

    #[test]
    fn emits_keyword_maps_as_plain_objects() {
        let source =
            syntax::parse_source("(def sample {:durationMs 60000 :readings [{:bpm 120 :time 0}]})");
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert_eq!(
            emitted.code.trim(),
            "export const sample = { durationMs: 60000, readings: [{ bpm: 120, time: 0 }] };"
        );
    }

    #[test]
    fn emits_keyword_match() {
        let source =
            syntax::parse_source("(defn label [msg] (match msg :start \"Start\" _ \"Other\"))");
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("Symbol.for(\"start\")"));
        assert!(emitted.code.contains("return \"Start\";"));
        assert!(emitted.code.contains("return \"Other\";"));
    }

    #[test]
    fn emits_record_pattern_match_bindings() {
        let source =
            syntax::parse_source("(defn next [msg] (match msg {:kind :rate :bpm bpm} bpm _ 0))");
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".kind === Symbol.for(\"rate\")"));
        assert!(emitted.code.contains("const bpm ="));
        assert!(emitted.code.contains("return bpm;"));
    }

    #[test]
    fn emits_as_pattern_match_bindings() {
        let source = syntax::parse_source(
            "(defn normalize [msg]\n  (match msg\n    (as {:kind :rate :bpm bpm} whole) (assoc whole :bpm (+ bpm 1))\n    _ msg))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".kind === Symbol.for(\"rate\")"));
        assert!(emitted.code.contains("const bpm ="));
        assert!(emitted.code.contains("const whole ="));
        assert!(emitted.code.contains("bpm + 1"));
    }

    #[test]
    fn emits_let_destructuring_pattern_bindings() {
        let source = syntax::parse_source(
            "(defn summarize [payload]\n\
               (let [{:reading {:bpm bpm}\n\
                      :samples (cons first rest)} payload\n\
                     (cons second tail) rest]\n\
                 {:bpm bpm :delta (- second first) :tailCount (count tail)}))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("let pattern did not match"));
        assert!(emitted.code.contains("const bpm ="));
        assert!(emitted.code.contains("const first ="));
        assert!(emitted.code.contains("const rest ="));
        assert!(emitted.code.contains("const second ="));
        assert!(emitted.code.contains("const tail ="));
        assert!(emitted.code.contains(".slice(1)"));
    }

    #[test]
    fn emits_fn_parameter_destructuring_pattern_bindings() {
        let source = syntax::parse_source(
            "(def summaries\n\
               (map [{:reading {:bpm 142} :samples (list 100 136 150)}]\n\
                    (fn [{:reading {:bpm bpm}\n\
                          :samples (cons head rest)}]\n\
                      {:bpm (+ bpm 0)\n\
                       :delta (- (first rest) head)\n\
                       :tailCount (count rest)})))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("fn parameter pattern did not match"));
        assert!(emitted.code.contains("const bpm ="));
        assert!(emitted.code.contains("const head ="));
        assert!(emitted.code.contains("const rest ="));
        assert!(emitted.code.contains(".slice(1)"));
    }

    #[test]
    fn emits_defn_parameter_destructuring_pattern_bindings() {
        let source = syntax::parse_source(
            "(defn summarize [{:reading {:bpm bpm}\n\
                               :samples (cons head rest)}]\n\
               {:bpm (+ bpm 0)\n\
                :delta (- (first rest) head)\n\
                :tailCount (count rest)})",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("export function summarize(__closkell_arg")
        );
        assert!(
            emitted
                .code
                .contains("defn parameter pattern did not match")
        );
        assert!(emitted.code.contains("const bpm ="));
        assert!(emitted.code.contains("const head ="));
        assert!(emitted.code.contains("const rest ="));
        assert!(emitted.code.contains(".slice(1)"));
    }

    #[test]
    fn rejects_template_defn_parameter_destructuring_for_metadata() {
        let source = syntax::parse_source("(defn view [{:label label}] #html <p>{label}</p>)");
        let emitted = emit_module(&source);

        assert!(
            emitted.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("defn parameter must be a symbol")),
            "{:?}",
            emitted.diagnostics
        );
    }

    #[test]
    fn emits_record_event_messages_as_arrow_expressions() {
        let source = syntax::parse_source(
            "(defn view [state] #html <button on:click={{:kind :start}}>Go</button>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("(event) => ({ kind: Symbol.for(\"start\") })")
        );
    }

    #[test]
    fn emits_event_payload_reads() {
        let source = syntax::parse_source(
            "(defn view [state] #html <input value={state.draft} on:input={{:kind :draft :value event.currentTarget.value}}></input>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(
            "(event) => ({ kind: Symbol.for(\"draft\"), value: event.currentTarget.value })"
        ));
    }

    #[test]
    fn emits_str_as_string_conversion() {
        let source = syntax::parse_source("(defn label [bpm] (str bpm))");
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("return String(bpm);"));
    }

    #[test]
    fn emits_json_helpers() {
        let source = syntax::parse_source(
            "(defn export-log [entries]\n  (json-stringify {:version 2 :entries entries} 2))\n\
             (defn imported-count [text]\n  (count (let [parsed (json-parse text)] parsed.entries)))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("return JSON.stringify({ version: 2, entries: entries }, null, 2);")
        );
        assert!(emitted.code.contains("const parsed = JSON.parse(text);"));
    }

    #[test]
    fn emits_safe_get_and_type_predicates() {
        let source = syntax::parse_source(
            "(defn valid-entry? [entry]\n  (and (string? (get entry :id))\n       (number? (get entry :durationMs))\n       (vector? (get entry :readings))\n       (nil? (get entry :hiddenAt))))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("entry?.id ?? null"));
        assert!(emitted.code.contains("typeof"));
        assert!(emitted.code.contains("Number.isFinite"));
        assert!(emitted.code.contains("Array.isArray"));
        assert!(emitted.code.contains("== null"));
    }

    #[test]
    fn emits_date_helpers() {
        let source = syntax::parse_source(
            "(defn label [timestamp]\n  (let [start (date-start-of-week timestamp)]\n    (date-format start :month-day)))\n\
             (defn iso [timestamp] (date-format timestamp :iso-date))\n\
             (defn log-label [timestamp] (date-format timestamp :month-day-time))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("const __day = __date.getDay();"));
        assert!(emitted.code.contains("Symbol.keyFor(__style)"));
        assert!(emitted.code.contains("month: \"short\", day: \"numeric\""));
        assert!(
            emitted
                .code
                .contains("hour: \"2-digit\", minute: \"2-digit\"")
        );
        assert!(emitted.code.contains("toISOString().slice(0, 10)"));
    }

    #[test]
    fn emits_collection_primitives() {
        let source = syntax::parse_source(
            "(defn sample [items]\n  (let [found (find items (fn [item] (> item.value 0)))\n        first-item (first items)\n        second-item (second items)\n        indexed (nth items 0)\n        latest (last items)\n        leading (drop-last items)]\n    (reduce-indexed leading 0 (fn [sum item index] (+ sum item.value index latest.value first-item.value second-item.value indexed.value)))))\n\
             (def ticks (range 0 6))\n\
             (def descending (range 5 0 -2))\n\
             (def sample-list (cons 1 (list 2 3)))\n\
             (def sample-tail (rest sample-list))\n\
             (def list-summary {:list (list? sample-list) :tail-count (count sample-tail)})",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".find("));
        assert!(emitted.code.contains("[0] ?? null"));
        assert!(emitted.code.contains("[1] ?? null"));
        assert!(emitted.code.contains(".at(-1) ?? null"));
        assert!(emitted.code.contains(".slice(0, -(1))"));
        assert!(emitted.code.contains("Array.from({ length: __count }"));
        assert!(emitted.code.contains(".reduce((__acc, __item, __index)"));
        assert!(
            emitted
                .code
                .contains("[__item, ...(Array.isArray(__list) ? __list : [])]")
        );
        assert!(emitted.code.contains("__list.slice(1)"));
        assert!(emitted.code.contains("Array.isArray(sample_list)"));
    }

    #[test]
    fn emits_collection_transforms() {
        let source = syntax::parse_source(
            "(defn sample [entries]\n  {:visible (filter entries (fn [entry] (not (some? entry.hiddenAt))))\n   :bars (map (take-last (sort-by entries (fn [entry] entry.stoppedAt)) 2)\n              (fn [entry] {:label entry.id :value entry.durationMs}))\n   :ranked (map-indexed (sort-by-desc entries (fn [entry] entry.stoppedAt))\n              (fn [entry index] {:id entry.id :rank (+ index 1)}))\n   :custom (sort-with entries (fn [first second] (- second.stoppedAt first.stoppedAt)))\n   :types (sort-with [\"Strength\" \"LISS\"] (fn [first second] (locale-compare first second)))\n   :allTyped (every? entries (fn [entry] (some? entry.exerciseType)))\n   :page (slice entries 0 2)\n   :hasSelected (any? entries (fn [entry] (= entry.id \"warmup\")))\n   :appended (conj entries {:id \"next\"})})",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".filter((__item) =>"));
        assert!(emitted.code.contains(".map((__item) =>"));
        assert!(emitted.code.contains(".map((__item, __index) =>"));
        assert!(emitted.code.contains(".some((__item) =>"));
        assert!(emitted.code.contains(".every((__item) =>"));
        assert!(emitted.code.contains("[...entries].sort("));
        assert!(emitted.code.contains("(__left, __right))"));
        assert!(emitted.code.contains(".localeCompare("));
        assert!(emitted.code.contains(".slice(-(2))"));
        assert!(emitted.code.contains(".slice(0, 2)"));
        assert!(
            emitted
                .code
                .contains(": [...__collection, { id: \"next\" }]")
        );
    }

    #[test]
    fn emits_set_operations() {
        let source = syntax::parse_source(
            "(def tags (set \"steady\" \"zone2\" \"steady\"))\n\
             (def more (conj tags \"tempo\"))\n\
             (def fewer (disj more \"steady\"))\n\
             (def ordered (set-values fewer))\n\
             (def summary {:hasZone2 (contains? fewer \"zone2\")\n\
                           :count (count fewer)\n\
                           :empty (empty? fewer)\n\
                           :set (set? fewer)})",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("export const tags = new Set([\"steady\", \"zone2\", \"steady\"]);")
        );
        assert!(
            emitted
                .code
                .contains("new Set([...__collection, \"tempo\"])")
        );
        assert!(emitted.code.contains("__next.delete(__item);"));
        assert!(
            emitted
                .code
                .contains("export const ordered = Array.from(fewer);")
        );
        assert!(emitted.code.contains("__collection.has(__value)"));
        assert!(emitted.code.contains("__collection.size"));
        assert!(emitted.code.contains("fewer instanceof Set"));
    }

    #[test]
    fn emits_map_operations() {
        let source = syntax::parse_source(
            "(def registry (hash-map \"zone2\" {:id \"zone2\"} \"trimp\" {:id \"trimp\"}))\n\
             (def selected (map-get registry \"zone2\"))\n\
             (def updated (map-assoc registry \"hrr\" {:id \"hrr\"}))\n\
             (def trimmed (map-dissoc updated \"trimp\"))\n\
             (def summary {:hasZone2 (contains? trimmed \"zone2\")\n\
                           :count (count trimmed)\n\
                           :empty (empty? trimmed)\n\
                           :map (map? trimmed)})",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(
            "export const registry = new Map([[\"zone2\", { id: \"zone2\" }], [\"trimp\", { id: \"trimp\" }]]);"
        ));
        assert!(
            emitted
                .code
                .contains("__map.has(__key) ? __map.get(__key) : null")
        );
        assert!(emitted.code.contains(
            "const __next = new Map(__map); __next.set(\"hrr\", { id: \"hrr\" }); return __next;"
        ));
        assert!(emitted.code.contains("__next.delete(\"trimp\");"));
        assert!(emitted.code.contains("__collection instanceof Map"));
        assert!(emitted.code.contains("trimmed instanceof Map"));
    }

    #[test]
    fn emits_map_enumeration() {
        let source = syntax::parse_source(
            "(def durations (hash-map 1 0 2 45000 3 15000))\n\
             (def entries (map-entries durations))\n\
             (def keys (map-keys durations))\n\
             (def values (map-values durations))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(
            "Array.from(__map.entries(), ([__key, __value]) => ({ key: __key, value: __value }))"
        ));
        assert!(emitted.code.contains("Array.from(__map.keys())"));
        assert!(emitted.code.contains("Array.from(__map.values())"));
    }

    #[test]
    fn emits_result_helpers() {
        let source = syntax::parse_source(
            "(def parsed (ok [{:id \"warmup\"}]))\n\
             (def failed (err \"missing entries\"))\n\
             (def entries (unwrap-or parsed []))\n\
             (def message (result-error failed))\n\
             (def flags {:ok (ok? parsed) :err (err? failed) :value (result-value parsed)})",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("export const parsed = { ok: true, value: [{ id: \"warmup\" }] };")
        );
        assert!(
            emitted
                .code
                .contains("export const failed = { ok: false, error: \"missing entries\" };")
        );
        assert!(
            emitted
                .code
                .contains("__result?.ok === true ? __result.value")
        );
        assert!(
            emitted
                .code
                .contains("__result?.ok === false ? __result.error")
        );
        assert!(emitted.code.contains("parsed?.ok === true"));
        assert!(emitted.code.contains("failed?.ok === false"));
    }

    #[test]
    fn emits_option_and_result_match_patterns() {
        let source = syntax::parse_source(
            "(defn summarize [result]\n  (match result\n    (ok entries) (str \"Imported \" (count entries))\n    (err message) message))\n\
             (defn selected-id [entry]\n  (match entry\n    (some selected) selected.id\n    nil \"none\"))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".ok === true"));
        assert!(emitted.code.contains("const entries = __closkell_match"));
        assert!(emitted.code.contains(".value;"));
        assert!(emitted.code.contains(".ok === false"));
        assert!(emitted.code.contains("const message = __closkell_match"));
        assert!(emitted.code.contains(".error;"));
        assert!(emitted.code.contains("!= null"));
        assert!(emitted.code.contains("const selected = __closkell_match"));
    }

    #[test]
    fn emits_list_match_pattern_bindings() {
        let source = syntax::parse_source(
            "(defn delta [samples]\n  (match samples\n    (list first second _) (- second first)\n    _ 0))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("Array.isArray(__closkell_match"));
        assert!(emitted.code.contains(".length === 3"));
        assert!(emitted.code.contains("const first = __closkell_match"));
        assert!(emitted.code.contains("const second = __closkell_match"));
        assert!(emitted.code.contains("return second - first;"));
    }

    #[test]
    fn emits_cons_match_pattern_bindings() {
        let source = syntax::parse_source(
            "(defn head-plus-tail-count [samples]\n\
               (match samples\n\
                 (cons head tail) (+ head (count tail))\n\
                 (list) 0))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("Array.isArray(__closkell_match"));
        assert!(emitted.code.contains(".length > 0"));
        assert!(emitted.code.contains("const head = __closkell_match"));
        assert!(emitted.code.contains("const tail = __closkell_match"));
        assert!(emitted.code.contains(".slice(1);"));
    }

    #[test]
    fn emits_record_update_helpers() {
        let source = syntax::parse_source(
            "(defn update-entry [entry value] (assoc entry :exerciseType value :hiddenAt 42))\n\
             (defn import-complete [state entries] (merge state {:entries entries :message \"Imported\"}))\n\
             (defn clear-message [state] (dissoc state :message))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("{ ...(entry), exerciseType: value, hiddenAt: 42 }")
        );
        assert!(
            emitted
                .code
                .contains("Object.assign({}, state, { entries: entries, message: \"Imported\" })")
        );
        assert!(emitted.code.contains("delete __closkell_record_0.message;"));
    }

    #[test]
    fn emits_conditional_template_slots() {
        let source = syntax::parse_source(
            "(defn view [state]\n  #html <section>{(if state.connected? #html <strong>{state.label}</strong> #html <em>Offline</em>)}</section>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetConditional"));
        assert!(emitted.code.contains("{ conditional: true }"));
        assert!(emitted.code.contains("state[\"connected?\"]"));
    }

    #[test]
    fn emits_nested_conditional_template_slots() {
        let source = syntax::parse_source(
            "(defn pane [state]\n  #html <article>{state.label}</article>)\n\
             (defn view [state]\n  #html <main>{(if (= state.view \"metrics\") #html <section>{(pane state)}</section> (if (= state.view \"log\") #html <aside>Log</aside> #html <div>Live</div>))}</main>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetConditional"));
        assert!(emitted.code.contains("{ conditional: true }"));
        assert!(emitted.code.contains("__closkellBranch"));
        assert!(emitted.code.contains("__closkellFresh"));
        assert!(emitted.code.contains("force: true"));
        assert!(emitted.code.contains("{ component: \"pane\" }"));
        assert!(!emitted.code.contains(
            "__closkellSetText(instance, 0, instance.nodes[1], (state.view === \"metrics\""
        ));
    }

    #[test]
    fn emits_conditional_component_branches() {
        let source = syntax::parse_source(
            "(defn live-pane [state]\n  #html <section>{state.latestBpm}</section>)\n\
             (defn log-pane [state]\n  #html <section>{state.selectedLogId}</section>)\n\
             (defn view [state]\n  #html <main>{(if (= state.detailView \"live\") (live-pane state) (log-pane state))}</main>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetConditional"));
        assert!(emitted.code.contains("live_pane(state)"));
        assert!(emitted.code.contains("log_pane(state)"));
        assert!(
            emitted
                .code
                .contains(".update(state, dispatch, updateContext)")
        );
        assert!(emitted.code.contains(
            "kind: { conditional: true }, reads: [\"state.detailView\", \"state.latestBpm\", \"state.selectedLogId\"]"
        ));
        assert!(
            !emitted
                .code
                .contains("? live_pane(state) : log_pane(state)"),
            "component branches should not be lowered as text:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_conditional_reads_without_nested_keyed_loop_locals() {
        let source = syntax::parse_source(
            "(defn view [state]\n  #html <section>{(if state.show? #html <div>{(for [zone state.zones :key zone.id] #html <i>{zone.name}</i>)}</div> #html <p>None</p>)}</section>)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("kind: { conditional: true }, reads: [\"state.show?\", \"state.zones\"]"),
            "conditional reads should not include loop locals:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_string_primitives() {
        let source = syntax::parse_source(
            "(defn matches [exercise-type]\n  (regex-test? (trim exercise-type) \"liss|steady\" \"i\"))\n\
             (defn plain-match? [exercise-type]\n  (regex-test? exercise-type \"recovery\"))\n\
             (defn metric-visible? [enabled]\n  (includes? enabled \"zone2\"))\n\
             (defn id-suffix [roll]\n  (string-slice (to-radix roll 36) 2 9))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".trim()"));
        assert!(emitted.code.contains("new RegExp(\"liss|steady\", \"i\")"));
        assert!(emitted.code.contains(".test(exercise_type.trim())"));
        assert!(emitted.code.contains("new RegExp(\"recovery\", \"\")"));
        assert!(emitted.code.contains("enabled.includes(\"zone2\")"));
        assert!(emitted.code.contains(".toString(36)"));
        assert!(emitted.code.contains(".slice(2, 9)"));
    }

    #[test]
    fn emits_dev_environment_flag() {
        let source = syntax::parse_source(
            "(def dev-enabled? (env-dev?))\n(defn label [] (if (env-dev?) \"Dev\" \"Prod\"))",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("globalThis.__CLOSKELL_ENV__?.DEV"));
        assert!(
            emitted
                .code
                .contains("import.meta.env && import.meta.env.DEV")
        );
    }

    #[test]
    fn emits_numeric_vector_aggregates() {
        let source = syntax::parse_source(
            "(defn bounds [values]\n\
               {:min (min-of values 50)\n\
                :max (max-of values 170)\n\
                :total (sum values)})",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("Math.min(...values, 50)"));
        assert!(emitted.code.contains("Math.max(...values, 170)"));
        assert!(
            emitted
                .code
                .contains("__values.reduce((__sum, __value) => __sum + __value, 0)")
        );
    }

    #[test]
    fn emits_duration_formatting_primitives() {
        let source = syntax::parse_source(
            "(defn pad2 [value] (pad-start (str value) 2 \"0\"))\n\
             (defn seconds-part [ms]\n  (mod (floor (/ ms 1000)) 60))\n\
             (defn nested-mod [value]\n  (% (+ value 1) 60))\n\
             (defn trend-delta [current previous]\n  (abs (- current previous)))\n\
             (defn short-minute-label [minutes]\n  (to-fixed minutes 1))\n\
             (defn stored-number [value]\n  (to-number value))\n\
             (def recovery-ms 60_000)",
        );
        let emitted = emit_module(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".padStart(2, \"0\")"));
        assert!(emitted.code.contains("((Math.floor(ms / 1000)) % 60)"));
        assert!(emitted.code.contains("((value + 1) % 60)"));
        assert!(emitted.code.contains("Math.abs("));
        assert!(emitted.code.contains("(minutes).toFixed(1)"));
        assert!(emitted.code.contains("return Number(__value);"));
        assert!(emitted.code.contains("export const recovery_ms = 60_000;"));
    }
}
