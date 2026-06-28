use std::collections::{BTreeMap, BTreeSet};

use syntax::{
    Diagnostic, Expr, ExprKind, HtmlAttrValue, HtmlElement, HtmlNode, SourceFile, format_expr,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmitResult {
    pub code: String,
    pub diagnostics: Vec<Diagnostic>,
    pub source_mappings: Vec<SourceMapping>,
    pub runtime_effects: BTreeSet<String>,
    pub exports: BTreeMap<String, EmitExport>,
}

#[derive(Clone, Debug)]
pub struct EmitOptions {
    pub reachable_message_kinds: Option<BTreeSet<String>>,
    pub message_field_reads: BTreeMap<String, MessageFieldReads>,
    pub static_reads: BTreeMap<String, String>,
    pub direct_call_replacements: BTreeMap<String, String>,
    pub symbol_reads: Vec<SymbolReadEmitRule>,
    pub intrinsic_calls: Vec<IntrinsicCallEmitRule>,
    pub custom_calls: Vec<CustomCallEmitRule>,
    pub prelude_code: String,
    pub html_templates: HtmlTemplateEmitOptions,
    pub omit_replaced_defn_exports: bool,
    pub export_top_level: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            reachable_message_kinds: None,
            message_field_reads: BTreeMap::new(),
            static_reads: BTreeMap::new(),
            direct_call_replacements: BTreeMap::new(),
            symbol_reads: Vec::new(),
            intrinsic_calls: Vec::new(),
            custom_calls: Vec::new(),
            prelude_code: String::new(),
            html_templates: HtmlTemplateEmitOptions::default(),
            omit_replaced_defn_exports: false,
            export_top_level: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmitExport {
    pub js_name: String,
    pub arity: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntrinsicCallEmitRule {
    pub name: String,
    pub html_runtime: bool,
    pub runtime_imports: Vec<IntrinsicRuntimeImport>,
    pub runtime_effects: Vec<String>,
    pub fallback: String,
    pub forms: Vec<IntrinsicCallEmitForm>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolReadEmitRule {
    pub name: String,
    pub replacement: String,
    pub needs_none_const: bool,
}

#[derive(Clone, Debug)]
pub struct CustomCallEmitRule {
    pub name: String,
    pub emitter: CustomCallEmitter,
}

pub type CustomCallEmitter = for<'a> fn(&mut CustomCallEmitContext<'a>, &[Expr]) -> String;

pub struct CustomCallEmitContext<'a> {
    emitter: &'a mut Emitter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntrinsicRuntimeImport {
    pub imported: String,
    pub alias: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntrinsicCallEmitForm {
    pub arity: usize,
    pub template: String,
}

impl IntrinsicCallEmitRule {
    pub fn new(
        name: impl Into<String>,
        runtime_effects: Vec<String>,
        fallback: impl Into<String>,
        forms: Vec<IntrinsicCallEmitForm>,
    ) -> Self {
        Self {
            name: name.into(),
            html_runtime: false,
            runtime_imports: Vec::new(),
            runtime_effects,
            fallback: fallback.into(),
            forms,
        }
    }

    pub fn html_runtime(mut self) -> Self {
        self.html_runtime = true;
        self
    }

    pub fn runtime_import(mut self, imported: impl Into<String>, alias: impl Into<String>) -> Self {
        self.runtime_imports.push(IntrinsicRuntimeImport {
            imported: imported.into(),
            alias: alias.into(),
        });
        self
    }
}

impl SymbolReadEmitRule {
    pub fn new(name: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            replacement: replacement.into(),
            needs_none_const: false,
        }
    }

    pub fn none_const(mut self) -> Self {
        self.needs_none_const = true;
        self
    }
}

impl CustomCallEmitRule {
    pub fn new(name: impl Into<String>, emitter: CustomCallEmitter) -> Self {
        Self {
            name: name.into(),
            emitter,
        }
    }
}

impl CustomCallEmitContext<'_> {
    pub fn emit_expr(&mut self, expr: &Expr) -> String {
        self.emitter.emit_expr(expr)
    }

    pub fn add_runtime_effect(&mut self, effect: &str) {
        self.emitter.add_runtime_effect(effect);
    }

    pub fn message_field_reads(&self, kind: &str) -> Option<&MessageFieldReads> {
        self.emitter.message_field_reads.get(kind)
    }
}

impl IntrinsicCallEmitForm {
    pub fn new(arity: usize, template: impl Into<String>) -> Self {
        Self {
            arity,
            template: template.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlTemplateEmitOptions {
    pub enabled: bool,
    pub constructor: String,
    pub disabled_diagnostic: String,
}

impl Default for HtmlTemplateEmitOptions {
    fn default() -> Self {
        Self::disabled("#html templates are not enabled for this JS emission")
    }
}

impl HtmlTemplateEmitOptions {
    pub fn enabled(constructor: impl Into<String>) -> Self {
        Self {
            enabled: true,
            constructor: constructor.into(),
            disabled_diagnostic: "#html templates are not enabled for this JS emission".to_string(),
        }
    }

    pub fn disabled(message: impl Into<String>) -> Self {
        Self {
            enabled: false,
            constructor: String::new(),
            disabled_diagnostic: message.into(),
        }
    }
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
    names: Vec<ImportName>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ImportName {
    Named { imported: String, local: String },
    Default { local: String },
}

pub fn emit_module(source: &SourceFile) -> EmitResult {
    emit_module_with_types(source, BTreeMap::new())
}

pub fn emit_module_with_types(
    source: &SourceFile,
    expr_types: BTreeMap<usize, String>,
) -> EmitResult {
    emit_module_with_types_and_options(source, expr_types, EmitOptions::default())
}

pub fn emit_module_with_types_and_options(
    source: &SourceFile,
    expr_types: BTreeMap<usize, String>,
    options: EmitOptions,
) -> EmitResult {
    let prelude_code = options.prelude_code.clone();
    let mut emitter = Emitter::new(source, expr_types, options);
    let mut import_lines = Vec::new();
    let mut lines = Vec::new();
    let mut exports = BTreeMap::new();

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
            if is_type_form(form) || is_ann_form(form) || is_foreign_form(form) {
                continue;
            }

            if let [head, name, value] = items.as_slice() {
                if matches_symbol(head, "def") {
                    if let ExprKind::Symbol(name) = &name.kind {
                        let js_name = sanitize_identifier(name);
                        exports.insert(
                            name.clone(),
                            EmitExport {
                                js_name: js_name.clone(),
                                arity: None,
                            },
                        );
                        lines.push(EmittedLine {
                            code: format!(
                                "{}const {} = {};",
                                emitter.export_prefix(),
                                js_name,
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
                    if emitter.omit_replaced_defn_exports
                        && emitter.direct_call_replacements.contains_key(name)
                    {
                        continue;
                    }
                    exports.insert(
                        name.clone(),
                        EmitExport {
                            js_name: sanitize_identifier(name),
                            arity: defn_arity(&items[2..]),
                        },
                    );
                    lines.push(EmittedLine {
                        code: emitter.emit_defn(name, form, &items[2..]),
                        source_offset: form.span.start,
                    });
                    continue;
                }
            }
        }

        lines.push(EmittedLine {
            code: format!(
                "{}const value{} = {};",
                emitter.export_prefix(),
                index,
                emitter.emit_expr(form)
            ),
            source_offset: form.span.start,
        });
        exports.insert(
            format!("value{}", index),
            EmitExport {
                js_name: format!("value{}", index),
                arity: None,
            },
        );
    }

    let mut code = String::new();
    let mut source_mappings = Vec::new();
    let mut generated_line = 0;
    if emitter.needs_html_runtime || emitter.needs_cmd_map {
        code.push_str(&compiled_template_runtime_import(&emitter));
        generated_line += 2;
    }
    if emitter.needs_html_runtime {
        code.push_str(&format!(
            "const {document} = () => globalThis.document, {element} = (tag) => {document}().createElement(tag), {svg_element} = (tag) => {document}().createElementNS(\"http://www.w3.org/2000/svg\", tag), {text} = (value) => {document}().createTextNode(value), {attr} = (node, name, value) => node.setAttribute(name, value), {append} = (parent, child) => parent.appendChild(child);\n\n",
            document = DOCUMENT_ALIAS,
            element = CREATE_ELEMENT_ALIAS,
            svg_element = CREATE_SVG_ELEMENT_ALIAS,
            text = CREATE_TEXT_ALIAS,
            attr = SET_STATIC_ATTR_ALIAS,
            append = APPEND_CHILD_ALIAS
        ));
        generated_line += 2;
    }
    if emitter.needs_decoder_runtime {
        code.push_str(
            "import { Decoder as __closkellDecoder, decode as __closkellDecode } from \"@closkell/runtime\";\n\n",
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
    if !prelude_code.is_empty() {
        code.push_str(&prelude_code);
        if !prelude_code.ends_with('\n') {
            code.push('\n');
        }
        generated_line += prelude_code.lines().count().max(1);
    }
    push_emitter_helpers(&mut code, &mut generated_line, &emitter);
    push_emitted_lines(&mut code, &lines, &mut source_mappings, &mut generated_line);
    EmitResult {
        code,
        diagnostics: emitter.diagnostics,
        source_mappings,
        runtime_effects: emitter.runtime_effects,
        exports,
    }
}

fn defn_arity(args: &[Expr]) -> Option<usize> {
    match args.first().map(|expr| &expr.kind) {
        Some(ExprKind::Vector(params)) => Some(params.len()),
        _ => None,
    }
}

fn compiled_template_runtime_import(emitter: &Emitter) -> String {
    let mut imports = Vec::new();
    if emitter.needs_html_runtime {
        imports.push(format!("bindCompiledComponent as {}", BIND_COMPONENT_ALIAS));
    }
    if emitter.needs_cmd_map {
        imports.push(format!("mapCommand as {}", CMD_MAP_ALIAS));
    }
    if emitter.needs_template_skeleton {
        imports.push(format!(
            "{} as {}",
            emitter.html_templates.constructor, CREATE_HTML_TEMPLATE_ALIAS
        ));
    }
    if emitter.needs_render_to_string {
        imports.push(format!("renderToString as {}", RENDER_TO_STRING_ALIAS));
    }
    if emitter.needs_scope_subscriptions {
        imports.push(format!(
            "scopeSubscriptions as {}",
            SCOPE_SUBSCRIPTIONS_ALIAS
        ));
    }
    if emitter.needs_scope_update {
        imports.push(format!("scopeUpdate as {}", SCOPE_UPDATE_ALIAS));
    }
    if emitter.needs_scope_view {
        imports.push(format!("scopeView as {}", SCOPE_VIEW_ALIAS));
    }
    imports.extend(emitter.runtime_imports.iter().cloned());
    if emitter.needs_template_attr {
        imports.push(format!("setCompiledAttr as {}", SET_ATTR_ALIAS));
    }
    if emitter.needs_template_text_attr {
        imports.push(format!("setCompiledTextAttr as {}", SET_TEXT_ATTR_ALIAS));
    }
    if emitter.needs_template_nullable_text_attr {
        imports.push(format!(
            "setCompiledNullableTextAttr as {}",
            SET_NULLABLE_TEXT_ATTR_ALIAS
        ));
    }
    if emitter.needs_template_text_property {
        imports.push(format!(
            "setCompiledTextProperty as {}",
            SET_TEXT_PROPERTY_ALIAS
        ));
    }
    if emitter.needs_template_nullable_text_property {
        imports.push(format!(
            "setCompiledNullableTextProperty as {}",
            SET_NULLABLE_TEXT_PROPERTY_ALIAS
        ));
    }
    if emitter.needs_template_presence_attr {
        imports.push(format!(
            "setCompiledPresenceAttr as {}",
            SET_PRESENCE_ATTR_ALIAS
        ));
    }
    if emitter.needs_template_boolean_property {
        imports.push(format!(
            "setCompiledBooleanProperty as {}",
            SET_BOOLEAN_PROPERTY_ALIAS
        ));
    }
    if emitter.needs_template_class_name {
        imports.push(format!("setCompiledClassName as {}", SET_CLASS_NAME_ALIAS));
    }
    if emitter.needs_template_class {
        imports.push(format!("setCompiledClass as {}", SET_CLASS_ALIAS));
    }
    if emitter.needs_template_style {
        imports.push(format!("setCompiledStyle as {}", SET_STYLE_ALIAS));
    }
    if emitter.needs_template_style_record {
        imports.push(format!(
            "setCompiledStyleRecord as {}",
            SET_STYLE_RECORD_ALIAS
        ));
    }
    if emitter.needs_template_component {
        imports.push(format!("setCompiledComponent as {}", SET_COMPONENT_ALIAS));
    }
    if emitter.needs_template_conditional {
        imports.push(format!(
            "setCompiledConditional as {}",
            SET_CONDITIONAL_ALIAS
        ));
    }
    if emitter.needs_template_keyed_list {
        imports.push(format!("setCompiledKeyedList as {}", SET_KEYED_LIST_ALIAS));
    }
    if emitter.needs_template_event {
        imports.push(format!("setCompiledEvent as {}", SET_EVENT_ALIAS));
    }
    if emitter.needs_template_ref {
        imports.push(format!("setCompiledRef as {}", SET_REF_ALIAS));
    }
    if emitter.needs_template_text {
        imports.push(format!("setText as {}", SET_TEXT_ALIAS));
    }
    format!(
        "import {{ {} }} from \"@closkell/runtime\";\n\n",
        imports.join(", ")
    )
}

pub fn emit_function_specialization(
    source: &SourceFile,
    expr_types: BTreeMap<usize, String>,
    original_name: &str,
    specialized_name: &str,
    mut options: EmitOptions,
) -> EmitResult {
    options.export_top_level = false;
    let mut emitter = Emitter::new(source, expr_types, options);
    let mut code = String::new();
    let mut source_mappings = Vec::new();
    let mut generated_line = 0;
    let mut found = false;

    for form in &source.forms {
        let ExprKind::List(items) = &form.kind else {
            continue;
        };
        if items.len() < 4 || !matches_symbol(&items[0], "defn") {
            continue;
        }
        let ExprKind::Symbol(name) = &items[1].kind else {
            continue;
        };
        if name != original_name {
            continue;
        }

        found = true;
        let line = emitter.emit_defn(specialized_name, form, &items[2..]);
        push_emitter_helpers(&mut code, &mut generated_line, &emitter);
        source_mappings.push(SourceMapping {
            generated_line,
            generated_column: 0,
            source_offset: form.span.start,
        });
        code.push_str(&line);
        code.push('\n');
        break;
    }

    if !found {
        emitter.diagnostics.push(Diagnostic::error(
            syntax::Span::new(0, 0),
            format!("cannot specialize missing function `{}`", original_name),
        ));
    }

    EmitResult {
        code,
        diagnostics: emitter.diagnostics,
        source_mappings,
        runtime_effects: emitter.runtime_effects,
        exports: BTreeMap::new(),
    }
}

#[derive(Clone, Debug)]
struct EmittedLine {
    code: String,
    source_offset: usize,
}

fn push_emitter_helpers(code: &mut String, generated_line: &mut usize, emitter: &Emitter) {
    if emitter.needs_value_equal_helper && !code.contains("const __closkellValueEqual") {
        code.push_str(VALUE_EQUAL_HELPER);
        *generated_line += VALUE_EQUAL_HELPER.lines().count();
    }
    if emitter.needs_count_helper && !code.contains("const __closkellCount") {
        code.push_str(COUNT_HELPER);
        *generated_line += COUNT_HELPER.lines().count();
    }
    if emitter.needs_object_predicate_helper && !code.contains("const __closkellIsObject") {
        code.push_str(OBJECT_PREDICATE_HELPER);
        *generated_line += OBJECT_PREDICATE_HELPER.lines().count();
    }
    if emitter.needs_object_entries_helper && !code.contains("const __closkellObjectEntries") {
        code.push_str(OBJECT_ENTRIES_HELPER);
        *generated_line += OBJECT_ENTRIES_HELPER.lines().count();
    }
    if emitter.needs_none_const && !code.contains("const __closkellNone") {
        code.push_str("const __closkellNone = { kind: Symbol.for(\"none\") };\n\n");
        *generated_line += 2;
    }
    for metadata_const in &emitter.template_metadata_consts {
        code.push_str(metadata_const);
        *generated_line += metadata_const.lines().count();
    }
    if !emitter.template_metadata_consts.is_empty() {
        code.push('\n');
        *generated_line += 1;
    }
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
        match parse_import_name(name) {
            Ok(parsed) => imported.push(parsed),
            Err(diagnostic) => return Some(Err(diagnostic)),
        }
    }

    Some(Ok(ImportSpec {
        path: path.clone(),
        names: imported,
    }))
}

fn parse_import_name(expr: &Expr) -> Result<ImportName, Diagnostic> {
    match &expr.kind {
        ExprKind::Symbol(symbol) => Ok(ImportName::Named {
            imported: symbol.clone(),
            local: symbol.clone(),
        }),
        ExprKind::List(items)
            if items.len() == 2
                && matches!(&items[0].kind, ExprKind::Symbol(name) if name == "default") =>
        {
            let ExprKind::Symbol(local) = &items[1].kind else {
                return Err(Diagnostic::error(
                    items[1].span,
                    "default import local name must be a symbol",
                ));
            };
            Ok(ImportName::Default {
                local: local.clone(),
            })
        }
        ExprKind::List(items)
            if items.len() == 3
                && matches!(&items[1].kind, ExprKind::Symbol(name) if name == "as") =>
        {
            let ExprKind::Symbol(imported) = &items[0].kind else {
                return Err(Diagnostic::error(
                    items[0].span,
                    "aliased import name must be a symbol",
                ));
            };
            let ExprKind::Symbol(local) = &items[2].kind else {
                return Err(Diagnostic::error(
                    items[2].span,
                    "aliased import local name must be a symbol",
                ));
            };
            Ok(ImportName::Named {
                imported: imported.clone(),
                local: local.clone(),
            })
        }
        _ => Err(Diagnostic::error(
            expr.span,
            "imported name must be a symbol, (default local), or (name as local)",
        )),
    }
}

fn is_type_form(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::List(items) if items.first().is_some_and(|head| matches_symbol(head, "type")))
}

fn is_ann_form(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::List(items) if items.first().is_some_and(|head| matches_symbol(head, "ann")))
}

fn is_foreign_form(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::List(items) if items.first().is_some_and(|head| matches_symbol(head, "foreign")))
}

fn emit_import(spec: &ImportSpec) -> Option<String> {
    let default_name = spec.names.iter().find_map(|name| match name {
        ImportName::Default { local } if is_runtime_import_name(local) => {
            Some(sanitize_identifier(local))
        }
        _ => None,
    });
    let names = spec
        .names
        .iter()
        .filter_map(|name| match name {
            ImportName::Named { imported, local } if is_runtime_import_name(local) => {
                let imported = sanitize_identifier(imported);
                let local = sanitize_identifier(local);
                if imported == local {
                    Some(imported)
                } else {
                    Some(format!("{} as {}", imported, local))
                }
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(", ");
    if names.is_empty() && default_name.is_none() {
        return None;
    }
    let path = escape_js(&js_import_path(&spec.path));
    match (default_name, names.is_empty()) {
        (Some(default_name), true) => Some(format!("import {} from \"{}\";", default_name, path)),
        (Some(default_name), false) => Some(format!(
            "import {}, {{ {} }} from \"{}\";",
            default_name, names, path
        )),
        (None, false) => Some(format!("import {{ {} }} from \"{}\";", names, path)),
        (None, true) => None,
    }
}

pub fn is_runtime_import_name(name: &str) -> bool {
    !name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
}

fn js_import_path(path: &str) -> String {
    if path == "closkell/test" {
        return "@closkell/runtime".to_string();
    }
    path.strip_suffix(".clsk")
        .map(|prefix| format!("{}.mjs", prefix))
        .unwrap_or_else(|| path.to_string())
}

struct Emitter {
    diagnostics: Vec<Diagnostic>,
    needs_html_runtime: bool,
    needs_decoder_runtime: bool,
    needs_value_equal_helper: bool,
    needs_count_helper: bool,
    needs_object_predicate_helper: bool,
    needs_object_entries_helper: bool,
    needs_none_const: bool,
    needs_render_to_string: bool,
    needs_scope_update: bool,
    needs_scope_subscriptions: bool,
    needs_scope_view: bool,
    needs_cmd_map: bool,
    needs_template_attr: bool,
    needs_template_text_attr: bool,
    needs_template_nullable_text_attr: bool,
    needs_template_text_property: bool,
    needs_template_nullable_text_property: bool,
    needs_template_presence_attr: bool,
    needs_template_boolean_property: bool,
    needs_template_class_name: bool,
    needs_template_class: bool,
    needs_template_style: bool,
    needs_template_style_record: bool,
    needs_template_component: bool,
    needs_template_conditional: bool,
    needs_template_keyed_list: bool,
    needs_template_event: bool,
    needs_template_ref: bool,
    needs_template_text: bool,
    needs_template_skeleton: bool,
    runtime_imports: BTreeSet<String>,
    runtime_effects: BTreeSet<String>,
    expr_types: BTreeMap<usize, String>,
    local_types: Vec<BTreeMap<String, String>>,
    reachable_message_kinds: Option<BTreeSet<String>>,
    static_reads: BTreeMap<String, String>,
    direct_call_replacements: BTreeMap<String, String>,
    symbol_reads: Vec<SymbolReadEmitRule>,
    intrinsic_calls: Vec<IntrinsicCallEmitRule>,
    custom_calls: Vec<CustomCallEmitRule>,
    html_templates: HtmlTemplateEmitOptions,
    omit_replaced_defn_exports: bool,
    export_top_level: bool,
    current_update_message_param: Option<String>,
    component_fns: BTreeSet<String>,
    function_defs: BTreeMap<String, FunctionDef>,
    read_summaries: BTreeMap<String, ReadSummary>,
    message_field_reads: BTreeMap<String, MessageFieldReads>,
    template_metadata_consts: Vec<String>,
    next_template_id: usize,
    next_temp_id: usize,
}

impl Emitter {
    fn new(source: &SourceFile, expr_types: BTreeMap<usize, String>, options: EmitOptions) -> Self {
        Self {
            diagnostics: Vec::new(),
            needs_html_runtime: false,
            needs_decoder_runtime: false,
            needs_value_equal_helper: false,
            needs_count_helper: false,
            needs_object_predicate_helper: false,
            needs_object_entries_helper: false,
            needs_none_const: false,
            needs_render_to_string: false,
            needs_scope_update: false,
            needs_scope_subscriptions: false,
            needs_scope_view: false,
            needs_cmd_map: false,
            needs_template_attr: false,
            needs_template_text_attr: false,
            needs_template_nullable_text_attr: false,
            needs_template_text_property: false,
            needs_template_nullable_text_property: false,
            needs_template_presence_attr: false,
            needs_template_boolean_property: false,
            needs_template_class_name: false,
            needs_template_class: false,
            needs_template_style: false,
            needs_template_style_record: false,
            needs_template_component: false,
            needs_template_conditional: false,
            needs_template_keyed_list: false,
            needs_template_event: false,
            needs_template_ref: false,
            needs_template_text: false,
            needs_template_skeleton: false,
            runtime_imports: BTreeSet::new(),
            runtime_effects: BTreeSet::new(),
            expr_types,
            local_types: Vec::new(),
            reachable_message_kinds: options.reachable_message_kinds,
            static_reads: options.static_reads,
            direct_call_replacements: options.direct_call_replacements,
            symbol_reads: options.symbol_reads,
            intrinsic_calls: options.intrinsic_calls,
            custom_calls: options.custom_calls,
            html_templates: options.html_templates,
            omit_replaced_defn_exports: options.omit_replaced_defn_exports,
            export_top_level: options.export_top_level,
            current_update_message_param: None,
            component_fns: collect_template_defns(source),
            function_defs: collect_function_defs(source),
            read_summaries: collect_read_summaries(source),
            message_field_reads: merged_message_field_reads(
                options.message_field_reads,
                collect_message_field_reads(source),
            ),
            template_metadata_consts: Vec::new(),
            next_template_id: 0,
            next_temp_id: 0,
        }
    }

    fn export_prefix(&self) -> &'static str {
        if self.export_top_level { "export " } else { "" }
    }
}

const VALUE_EQUAL_HELPER: &str = "const __closkellValueEqual = (__left, __right) => { const __plain = (__value) => __value !== null && typeof __value === \"object\" && !Array.isArray(__value) && !(__value instanceof Map) && !(__value instanceof Set); const __eq = (__left, __right) => { if (Object.is(__left, __right)) return true; if (Array.isArray(__left) || Array.isArray(__right)) return Array.isArray(__left) && Array.isArray(__right) && __left.length === __right.length && __left.every((__value, __index) => __eq(__value, __right[__index])); if (__left instanceof Set || __right instanceof Set) { if (!(__left instanceof Set) || !(__right instanceof Set) || __left.size !== __right.size) return false; const __remaining = Array.from(__right); return Array.from(__left).every((__item) => { const __match = __remaining.findIndex((__candidate) => __eq(__item, __candidate)); if (__match < 0) return false; __remaining.splice(__match, 1); return true; }); } if (__left instanceof Map || __right instanceof Map) { if (!(__left instanceof Map) || !(__right instanceof Map) || __left.size !== __right.size) return false; const __remaining = Array.from(__right); return Array.from(__left).every(([__key, __value]) => { const __match = __remaining.findIndex(([__rightKey, __rightValue]) => __eq(__key, __rightKey) && __eq(__value, __rightValue)); if (__match < 0) return false; __remaining.splice(__match, 1); return true; }); } if (__plain(__left) || __plain(__right)) { if (!__plain(__left) || !__plain(__right)) return false; const __leftKeys = Object.keys(__left).sort(); const __rightKeys = Object.keys(__right).sort(); return __eq(__leftKeys, __rightKeys) && __leftKeys.every((__key) => __eq(__left[__key], __right[__key])); } return false; }; return __eq(__left, __right); };\n\n";

const COUNT_HELPER: &str = "const __closkellCount = (__collection) => __collection instanceof Set || __collection instanceof Map ? __collection.size : (Array.isArray(__collection) || typeof __collection === \"string\" ? __collection.length : (__collection == null ? 0 : Object.keys(__collection).length));\n\n";
const OBJECT_PREDICATE_HELPER: &str = "const __closkellIsObject = (__value) => __value != null && typeof __value === \"object\" && !Array.isArray(__value) && !(__value instanceof Map) && !(__value instanceof Set);\n\n";
const OBJECT_ENTRIES_HELPER: &str = "const __closkellObjectEntries = (__value) => __value instanceof Map ? Array.from(__value.entries(), ([__key, __value]) => ({ key: __key, value: __value })) : (__value != null && typeof __value === \"object\" ? Object.entries(__value).map(([__key, __value]) => ({ key: __key, value: __value })) : []);\n\n";
const DOCUMENT_ALIAS: &str = "__closkellDocument";
const CREATE_ELEMENT_ALIAS: &str = "__closkellCreateElement";
const CREATE_SVG_ELEMENT_ALIAS: &str = "__closkellCreateSvgElement";
const CREATE_TEXT_ALIAS: &str = "__closkellCreateText";
const SET_STATIC_ATTR_ALIAS: &str = "__closkellSetStaticAttr";
const APPEND_CHILD_ALIAS: &str = "__closkellAppendChild";
const BIND_COMPONENT_ALIAS: &str = "__closkellBindComponent";
const CREATE_HTML_TEMPLATE_ALIAS: &str = "__closkellCreateHtmlTemplate";
const RENDER_TO_STRING_ALIAS: &str = "__closkellRenderToString";
const SCOPE_SUBSCRIPTIONS_ALIAS: &str = "__closkellScopeSubscriptions";
const SCOPE_UPDATE_ALIAS: &str = "__closkellScopeUpdate";
const SCOPE_VIEW_ALIAS: &str = "__closkellScopeView";
const CMD_MAP_ALIAS: &str = "__closkellMapCommand";
const SET_TEXT_ALIAS: &str = "__closkellSetText";
const SET_ATTR_ALIAS: &str = "__closkellSetAttr";
const SET_TEXT_ATTR_ALIAS: &str = "__closkellSetTextAttr";
const SET_NULLABLE_TEXT_ATTR_ALIAS: &str = "__closkellSetNullableTextAttr";
const SET_TEXT_PROPERTY_ALIAS: &str = "__closkellSetTextProperty";
const SET_NULLABLE_TEXT_PROPERTY_ALIAS: &str = "__closkellSetNullableTextProperty";
const SET_PRESENCE_ATTR_ALIAS: &str = "__closkellSetPresenceAttr";
const SET_BOOLEAN_PROPERTY_ALIAS: &str = "__closkellSetBooleanProperty";
const SET_CLASS_NAME_ALIAS: &str = "__closkellSetClassName";
const SET_CLASS_ALIAS: &str = "__closkellSetClass";
const SET_STYLE_ALIAS: &str = "__closkellSetStyle";
const SET_STYLE_RECORD_ALIAS: &str = "__closkellSetStyleRecord";
const SET_COMPONENT_ALIAS: &str = "__closkellSetComponent";
const SET_CONDITIONAL_ALIAS: &str = "__closkellSetConditional";
const SET_KEYED_LIST_ALIAS: &str = "__closkellSetKeyedList";
const SET_EVENT_ALIAS: &str = "__closkellSetEvent";
const SET_REF_ALIAS: &str = "__closkellSetRef";

struct PrimitiveEqualityLiteral {
    code: String,
}

fn primitive_equality_literal(expr: &Expr) -> Option<PrimitiveEqualityLiteral> {
    let code = match &expr.kind {
        ExprKind::Nil => "null".to_string(),
        ExprKind::Bool(value) => value.to_string(),
        ExprKind::Number(value) => value.clone(),
        ExprKind::String(value) => format!("\"{}\"", escape_js(value)),
        ExprKind::Keyword(name) => keyword_literal(name),
        _ => return None,
    };
    Some(PrimitiveEqualityLiteral { code })
}

fn literal_string_value(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::String(value) | ExprKind::Keyword(value) => Some(value),
        _ => None,
    }
}

fn keyword_literal(name: &str) -> String {
    format!("Symbol.for(\"{}\")", escape_js(name))
}

fn apply_intrinsic_template(template: &str, args: &[String]) -> String {
    let mut code = template.to_string();
    for (index, arg) in args.iter().enumerate() {
        code = code.replace(&format!("{{{}}}", index), arg);
    }
    code
}

fn is_strict_comparable_type(ty: &str) -> bool {
    if let Some(inner) = ty
        .strip_prefix("(Option ")
        .and_then(|value| value.strip_suffix(')'))
    {
        return is_strict_comparable_type(inner);
    }
    ty == "Number" || ty == "String" || ty == "Bool" || ty == "Nil" || ty.starts_with(':')
}

fn is_simple_attr_value_type(ty: &str) -> bool {
    if let Some(inner) = ty
        .strip_prefix("(Option ")
        .and_then(|value| value.strip_suffix(')'))
    {
        return is_simple_attr_value_type(inner);
    }
    is_strict_comparable_type(ty)
}

fn attr_type_inner(ty: &str) -> (&str, bool) {
    if let Some(inner) = type_option_inner(ty) {
        (inner, true)
    } else {
        (ty, false)
    }
}

fn is_text_attr_type(ty: &str) -> bool {
    ty == "String" || ty == "Number" || type_is_keyword(ty)
}

fn is_html_text_property_attr(name: &str) -> bool {
    matches!(name, "value")
}

fn is_html_boolean_property_attr(name: &str) -> bool {
    matches!(
        name,
        "checked" | "disabled" | "hidden" | "multiple" | "readonly" | "required" | "selected"
    )
}

fn type_counts_by_length(ty: &str) -> bool {
    ty == "String" || type_is_vector_like(ty)
}

fn type_counts_by_size(ty: &str) -> bool {
    type_is_set(ty) || type_is_map(ty)
}

fn type_is_vector_like(ty: &str) -> bool {
    ty.starts_with("(Vector ") || ty.starts_with("(List ") || ty.starts_with('[')
}

fn type_is_set(ty: &str) -> bool {
    ty.starts_with("(Set ")
}

fn type_is_map(ty: &str) -> bool {
    ty.starts_with("(Map ")
}

fn type_is_record(ty: &str) -> bool {
    ty.starts_with('{')
}

fn type_is_keyword(ty: &str) -> bool {
    ty == "Keyword" || ty.starts_with(':')
}

fn split_top_level_terms(input: &str) -> Vec<&str> {
    let mut terms = Vec::new();
    let mut depth = 0i32;
    let mut start = None;

    for (index, ch) in input.char_indices() {
        if ch.is_whitespace() && depth == 0 {
            if let Some(term_start) = start.take() {
                if term_start < index {
                    terms.push(&input[term_start..index]);
                }
            }
            continue;
        }

        if start.is_none() {
            start = Some(index);
        }

        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
    }

    if let Some(term_start) = start {
        if term_start < input.len() {
            terms.push(&input[term_start..]);
        }
    }

    terms
}

fn type_app_args<'a>(ty: &'a str, name: &str) -> Option<Vec<&'a str>> {
    let prefix = format!("({} ", name);
    if !ty.starts_with(&prefix) || !ty.ends_with(')') {
        return None;
    }
    Some(split_top_level_terms(&ty[prefix.len()..ty.len() - 1]))
}

fn type_option_inner(ty: &str) -> Option<&str> {
    type_app_args(ty, "Option").and_then(|args| args.first().copied())
}

fn type_result_parts(ty: &str) -> Option<(&str, &str)> {
    let args = type_app_args(ty, "Result")?;
    match args.as_slice() {
        [ok, err] => Some((ok, err)),
        _ => None,
    }
}

fn type_sequence_element(ty: &str) -> Option<&str> {
    type_app_args(ty, "Vector")
        .or_else(|| type_app_args(ty, "List"))
        .and_then(|args| args.first().copied())
}

fn type_tuple_items(ty: &str) -> Option<Vec<&str>> {
    if !ty.starts_with('[') || !ty.ends_with(']') {
        return None;
    }
    Some(split_top_level_terms(&ty[1..ty.len() - 1]))
}

fn type_vector_item_type(ty: &str, index: usize) -> Option<&str> {
    type_tuple_items(ty)
        .and_then(|items| items.get(index).copied())
        .or_else(|| type_sequence_element(ty))
}

fn type_record_field_type<'a>(ty: &'a str, key: &str) -> Option<&'a str> {
    if !ty.starts_with('{') || !ty.ends_with('}') {
        return None;
    }
    let terms = split_top_level_terms(&ty[1..ty.len() - 1]);
    let field = format!(":{}", key);
    let mut iter = terms.chunks_exact(2);
    iter.find_map(|pair| {
        if pair[0] == field {
            Some(pair[1])
        } else {
            None
        }
    })
}

fn type_has_required_record_field(ty: &str, key: &str) -> bool {
    type_record_field_type(ty, key).is_some()
}

fn type_union_variants(ty: &str) -> Option<Vec<&str>> {
    type_app_args(ty, "Union")
}

fn type_has_kind_property(ty: &str) -> bool {
    if type_record_field_type(ty, "kind").is_some() {
        return true;
    }
    type_union_variants(ty).is_some_and(|variants| {
        !variants.is_empty()
            && variants
                .iter()
                .all(|variant| type_record_field_type(variant, "kind").is_some())
    })
}

fn type_fn_param_types(ty: &str) -> Option<Vec<&str>> {
    let args = type_app_args(ty, "Fn")?;
    let params = args.first()?;
    if !params.starts_with('[') || !params.ends_with(']') {
        return None;
    }
    Some(split_top_level_terms(&params[1..params.len() - 1]))
}

fn type_fn_return_type(ty: &str) -> Option<&str> {
    let args = type_app_args(ty, "Fn")?;
    match args.as_slice() {
        [_, ret] => Some(*ret),
        _ => None,
    }
}

fn type_fn_returns_html(ty: &str) -> bool {
    type_fn_return_type(ty).is_some_and(|ret| ret == "Html")
}

fn type_is_html(ty: &str) -> bool {
    ty.trim() == "Html"
}

fn join_pattern_tests(tests: Vec<String>) -> String {
    let tests = tests
        .into_iter()
        .filter(|test| test != "true")
        .collect::<Vec<_>>();
    if tests.is_empty() {
        "true".to_string()
    } else {
        tests.join(" && ")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeTypePredicate {
    Nil,
    Number,
    String,
    Bool,
    Keyword,
    Vector,
    Set,
    Map,
    Object,
}

fn date_format_options(style: &str) -> Option<&'static str> {
    match style {
        "month-year" => Some("{ month: \"short\", year: \"2-digit\" }"),
        "month-day-time" => {
            Some("{ month: \"short\", day: \"numeric\", hour: \"2-digit\", minute: \"2-digit\" }")
        }
        "month-day" => Some("{ month: \"short\", day: \"numeric\" }"),
        "month" => Some("{ month: \"short\" }"),
        "day" => Some("{ day: \"numeric\" }"),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CollectionShape {
    VectorLike,
    Set,
    Map,
    String,
    Unknown,
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
    fn add_runtime_effect(&mut self, kind: &str) {
        self.runtime_effects.insert(kind.to_string());
    }

    fn mark_template_slot_runtime(&mut self, kind: &TemplateSlotKind) {
        match kind {
            TemplateSlotKind::Text => self.needs_template_text = true,
            TemplateSlotKind::Attr { setter, .. } => match setter {
                TemplateAttrSetter::Simple => self.needs_template_attr = true,
                TemplateAttrSetter::Text => self.needs_template_text_attr = true,
                TemplateAttrSetter::NullableText => self.needs_template_nullable_text_attr = true,
                TemplateAttrSetter::TextProperty => self.needs_template_text_property = true,
                TemplateAttrSetter::NullableTextProperty => {
                    self.needs_template_nullable_text_property = true
                }
                TemplateAttrSetter::Presence => self.needs_template_presence_attr = true,
                TemplateAttrSetter::BooleanProperty => self.needs_template_boolean_property = true,
                TemplateAttrSetter::ClassName => self.needs_template_class_name = true,
                TemplateAttrSetter::Class => self.needs_template_class = true,
                TemplateAttrSetter::Style => self.needs_template_style = true,
                TemplateAttrSetter::StyleRecord => self.needs_template_style_record = true,
            },
            TemplateSlotKind::Event(_) => self.needs_template_event = true,
            TemplateSlotKind::Ref => self.needs_template_ref = true,
            TemplateSlotKind::Conditional { .. } => self.needs_template_conditional = true,
            TemplateSlotKind::Component { .. } => self.needs_template_component = true,
            TemplateSlotKind::KeyedList { .. } => self.needs_template_keyed_list = true,
        }
    }

    fn expr_type(&self, expr: &Expr) -> Option<&str> {
        if let Some(ty) = self.expr_types.get(&expr.span.start) {
            return Some(ty);
        }
        if let ExprKind::Symbol(name) = &expr.kind {
            return self.local_type(name);
        }
        None
    }

    fn call_callee_type(&self, expr: &Expr) -> Option<&str> {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        let head = items.first()?;
        self.expr_type(head)
    }

    fn expr_is_typed_component_call(&self, expr: &Expr) -> bool {
        if !matches!(&expr.kind, ExprKind::List(_)) {
            return false;
        }
        self.call_callee_type(expr)
            .is_some_and(type_fn_returns_html)
            || self.expr_type(expr).is_some_and(type_is_html)
    }

    fn local_type(&self, name: &str) -> Option<&str> {
        self.local_types
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).map(String::as_str))
    }

    fn expr_is_strict_comparable(&self, expr: &Expr) -> bool {
        self.expr_type(expr).is_some_and(is_strict_comparable_type)
    }

    fn expr_is_countable_by_length(&self, expr: &Expr) -> bool {
        !self.expr_has_projected_collection_type(expr)
            && self.expr_type(expr).is_some_and(type_counts_by_length)
    }

    fn expr_is_countable_by_size(&self, expr: &Expr) -> bool {
        !self.expr_has_projected_collection_type(expr)
            && self.expr_type(expr).is_some_and(type_counts_by_size)
    }

    fn expr_is_vector_like(&self, expr: &Expr) -> bool {
        !self.expr_has_projected_collection_type(expr)
            && self.expr_type(expr).is_some_and(type_is_vector_like)
    }

    fn expr_is_set(&self, expr: &Expr) -> bool {
        !self.expr_has_projected_collection_type(expr)
            && self.expr_type(expr).is_some_and(type_is_set)
    }

    fn expr_is_map(&self, expr: &Expr) -> bool {
        !self.expr_has_projected_collection_type(expr)
            && self.expr_type(expr).is_some_and(type_is_map)
    }

    fn expr_is_record(&self, expr: &Expr) -> bool {
        !self.expr_has_projected_collection_type(expr)
            && self.expr_type(expr).is_some_and(type_is_record)
    }

    fn expr_is_projected_field(&self, expr: &Expr) -> bool {
        matches!(&expr.kind, ExprKind::Symbol(name) if name.contains('.'))
    }

    fn expr_has_projected_collection_type(&self, expr: &Expr) -> bool {
        self.expr_is_projected_field(expr)
            && self.expr_type(expr).is_some_and(|ty| {
                type_counts_by_length(ty) || type_counts_by_size(ty) || type_is_record(ty)
            })
    }

    fn expr_runtime_type_predicate(
        &self,
        expr: &Expr,
        predicate: RuntimeTypePredicate,
    ) -> Option<bool> {
        self.literal_runtime_type_predicate(expr, predicate)
    }

    fn literal_runtime_type_predicate(
        &self,
        expr: &Expr,
        predicate: RuntimeTypePredicate,
    ) -> Option<bool> {
        let actual = match &expr.kind {
            ExprKind::Nil => RuntimeTypePredicate::Nil,
            ExprKind::Bool(_) => RuntimeTypePredicate::Bool,
            ExprKind::Number(_) => RuntimeTypePredicate::Number,
            ExprKind::String(_) => RuntimeTypePredicate::String,
            ExprKind::Keyword(_) => RuntimeTypePredicate::Keyword,
            ExprKind::Vector(_) => RuntimeTypePredicate::Vector,
            ExprKind::Set(_) => RuntimeTypePredicate::Set,
            _ => return None,
        };
        Some(actual == predicate)
    }

    fn expr_is_str_raw_string(&self, expr: &Expr) -> bool {
        matches!(expr.kind, ExprKind::String(_) | ExprKind::Keyword(_))
            || self
                .expr_type(expr)
                .is_some_and(|ty| ty == "String" || type_is_keyword(ty))
    }

    fn expr_is_str_concat_safe(&self, expr: &Expr) -> bool {
        if self.expr_is_str_raw_string(expr) {
            return true;
        }
        matches!(
            expr.kind,
            ExprKind::Nil | ExprKind::Bool(_) | ExprKind::Number(_)
        ) || self
            .expr_type(expr)
            .is_some_and(|ty| ty == "Number" || ty == "Bool" || ty == "Nil")
    }

    fn expr_collection_shape(&self, expr: &Expr) -> CollectionShape {
        if self.expr_has_projected_collection_type(expr) {
            return CollectionShape::Unknown;
        }
        let Some(ty) = self.expr_type(expr) else {
            return CollectionShape::Unknown;
        };
        if type_is_vector_like(ty) {
            CollectionShape::VectorLike
        } else if type_is_set(ty) {
            CollectionShape::Set
        } else if type_is_map(ty) {
            CollectionShape::Map
        } else if ty == "String" {
            CollectionShape::String
        } else {
            CollectionShape::Unknown
        }
    }

    fn emit_expr(&mut self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Nil => "null".to_string(),
            ExprKind::Bool(value) => value.to_string(),
            ExprKind::Number(value) => value.clone(),
            ExprKind::String(value) => format!("\"{}\"", escape_js(value)),
            ExprKind::Keyword(name) => keyword_literal(name),
            ExprKind::Symbol(name) => self.emit_symbol_read(name),
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
                if !self.html_templates.enabled {
                    self.diagnostics.push(Diagnostic::error(
                        expr.span,
                        self.html_templates.disabled_diagnostic.clone(),
                    ));
                }
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

    fn emit_symbol_read(&mut self, name: &str) -> String {
        if let Some(rule) = self
            .symbol_reads
            .iter()
            .find(|rule| rule.name == name)
            .cloned()
        {
            if rule.needs_none_const {
                self.needs_none_const = true;
            }
            return rule.replacement;
        }
        if name == "Cmd.none" {
            self.needs_none_const = true;
            return "__closkellNone".to_string();
        }
        if let Some(value) = self.static_reads.get(name) {
            return value.clone();
        }
        if let Some(property) = primitive_decoder_runtime_property(name) {
            self.needs_decoder_runtime = true;
            return format!("__closkellDecoder.{}", property);
        }
        emit_symbol_read(name)
    }

    fn emit_list(&mut self, expr: &Expr, items: &[Expr]) -> String {
        let Some((head, args)) = items.split_first() else {
            return "[]".to_string();
        };

        if let ExprKind::Symbol(name) = &head.kind {
            if let Some(rule) = self
                .custom_calls
                .iter()
                .find(|rule| rule.name == *name)
                .cloned()
            {
                let mut context = CustomCallEmitContext { emitter: self };
                return (rule.emitter)(&mut context, args);
            }
            if let Some(rule) = self
                .intrinsic_calls
                .iter()
                .find(|rule| rule.name == *name)
                .cloned()
            {
                return self.emit_intrinsic_call(&rule, args);
            }
            match name.as_str() {
                "fn" => return self.emit_fn(expr, args),
                "let" => return self.emit_let(expr, args),
                "if" => return self.emit_if(expr, args),
                "match" => return self.emit_match(expr, args),
                "do" => return self.emit_do(args),
                "unsafe-cast" => return self.emit_unsafe_cast(args),
                "Msg.of" => return self.emit_msg_of(args),
                "Msg.with" => return self.emit_msg_with(args),
                "Msg.with2" => return self.emit_msg_with2(args),
                "Msg.mapper" => return self.emit_msg_mapper(args),
                "Event.prevent" => return self.emit_event_control(args, true, false),
                "Event.stop" => return self.emit_event_control(args, false, true),
                "Event.prevent-stop" => return self.emit_event_control(args, true, true),
                "Cmd.map" => return self.emit_cmd_map(args),
                "Cmd.batch" => return self.emit_cmd_batch(args),
                "Cmd.time/now" => return self.emit_cmd_time_now(args),
                "Cmd.random/number" => return self.emit_cmd_random_number(args),
                "Cmd.timer/every" => return self.emit_cmd_timer(args, "timer/every"),
                "Cmd.timer/after" => return self.emit_cmd_timer(args, "timer/after"),
                "Cmd.timer/cancel" => return self.emit_cmd_timer_cancel(args),
                "Cmd.animation/frame" => return self.emit_cmd_animation_frame(args),
                "Cmd.animation/cancel" => return self.emit_cmd_animation_cancel(args),
                "Task.succeed" => return self.emit_task_succeed(args),
                "Task.fail" => return self.emit_task_fail(args),
                "Task.map" => return self.emit_task_combinator(args, "task/map", "mapper"),
                "Task.map-error" => {
                    return self.emit_task_combinator(args, "task/map-error", "mapper");
                }
                "Task.and-then" => return self.emit_task_combinator(args, "task/and-then", "next"),
                "Task.perform" => return self.emit_task_perform(args),
                "Http.get-text" => return self.emit_http_task(args, "task/http/get-text"),
                "Http.get-json" => return self.emit_http_task(args, "task/http/get-json"),
                "scope-update" => return self.emit_scope_update(args),
                "scope-subscriptions" => return self.emit_scope_subscriptions(args),
                "+" | "-" | "*" | "/" | "<" | ">" | "<=" | ">=" => {
                    return self.emit_infix(name, args);
                }
                "%" | "mod" => return self.emit_mod(args),
                "=" => return self.emit_value_equal(args),
                "identical?" => return self.emit_identical(args),
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
                "object?" => return self.emit_object_predicate(args),
                "list?" => return self.emit_vector_predicate(args),
                "vector?" => return self.emit_vector_predicate(args),
                "set?" => return self.emit_set_predicate(args),
                "get" => return self.emit_get(args),
                "object-get" => return self.emit_object_get(args),
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
                "split" => return self.emit_split(args),
                "join" => return self.emit_join(args),
                "starts-with?" => return self.emit_string_predicate_method("startsWith", args),
                "ends-with?" => return self.emit_string_predicate_method("endsWith", args),
                "to-radix" => return self.emit_to_radix(args),
                "string-slice" => return self.emit_string_slice(args),
                "pad-start" => return self.emit_pad_start(args),
                "regex-test?" => return self.emit_regex_test(args),
                "includes?" => return self.emit_includes(args),
                "contains?" => return self.emit_contains(args),
                "locale-compare" => return self.emit_locale_compare(args),
                "json-stringify" => return self.emit_json_stringify(args),
                "json-parse" => return self.emit_json_parse(args),
                "json-parse-result" => return self.emit_json_parse_result(args),
                "decoder-string" => return self.emit_zero_arg_decoder(args, "string"),
                "decoder-number" => return self.emit_zero_arg_decoder(args, "number"),
                "decoder-bool" => return self.emit_zero_arg_decoder(args, "bool"),
                "decoder-keyword" => return self.emit_zero_arg_decoder(args, "keyword"),
                "decoder-literal" => return self.emit_decoder_literal(args),
                "decoder-optional" => return self.emit_decoder_unary(args, "optional"),
                "decoder-vector" => return self.emit_decoder_unary(args, "vector"),
                "decoder-record" => return self.emit_decoder_record(args),
                "decode" => return self.emit_decode(args),
                "object-entries" => return self.emit_object_entries(args),
                "object-keys" => return self.emit_object_projection(args, "keys"),
                "object-values" => return self.emit_object_projection(args, "values"),
                "object-assoc" => return self.emit_object_assoc(args),
                "object-dissoc" => return self.emit_object_dissoc(args),
                "encode-uri-component" => {
                    return self.emit_global_string_call("encodeURIComponent", args);
                }
                "decode-uri-component" => {
                    return self.emit_global_string_call("decodeURIComponent", args);
                }
                "url-resolve" => return self.emit_url_resolve(args),
                "url-without-hash" => return self.emit_url_part(args, "href", true),
                "url-origin" => return self.emit_url_part(args, "origin", false),
                "url-hostname" => return self.emit_url_part(args, "hostname", false),
                "url-pathname" => return self.emit_url_part(args, "pathname", false),
                "url-search-param" => return self.emit_url_search_param(args),
                "url-set-search-param" => return self.emit_url_set_search_param(args),
                "url-set-deep-object-param" => return self.emit_url_set_deep_object_param(args),
                "resolve-token-expiry" => return self.emit_resolve_token_expiry(args),
                "path-fill-params" => return self.emit_path_fill_params(args),
                "path-fill-param" => return self.emit_path_fill_param(args),
                "regex-capture" => return self.emit_regex_capture(args),
                "regex-capture-all" => return self.emit_regex_capture_all(args),
                "base64-encode" => return self.emit_base64(true, args),
                "base64-decode" => return self.emit_base64(false, args),
                "fail" => return self.emit_fail(args),
                "env-dev?" => return self.emit_env_dev(args),
                "env-mode" => return self.emit_env_mode(args),
                "not" => return self.emit_prefix("!", args),
                "and" => return self.emit_logical(args, true),
                "or" => return self.emit_logical(args, false),
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
                "get-in" => return self.emit_get_in(args),
                "assoc" => return self.emit_assoc(expr, args),
                "merge" => return self.emit_merge(args),
                "dissoc" => return self.emit_dissoc(expr, args),
                "update-in" => return self.emit_update_in(expr, args),
                "str" => return self.emit_str(args),
                "list" | "vector" => return self.emit_array(args),
                "set" => return format!("new Set({})", self.emit_array(args)),
                _ => {}
            }
        }

        let callee = if let ExprKind::Symbol(name) = &head.kind {
            self.direct_call_replacements
                .get(name)
                .map(|replacement| sanitize_identifier(replacement))
                .unwrap_or_else(|| self.emit_expr(head))
        } else {
            self.emit_expr(head)
        };
        let args = args
            .iter()
            .map(|arg| self.emit_expr(arg))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}({})", callee, args)
    }

    fn emit_intrinsic_call(&mut self, rule: &IntrinsicCallEmitRule, args: &[Expr]) -> String {
        if rule.html_runtime {
            self.needs_html_runtime = true;
        }
        for import in &rule.runtime_imports {
            self.runtime_imports
                .insert(format!("{} as {}", import.imported, import.alias));
        }
        for effect in &rule.runtime_effects {
            self.add_runtime_effect(effect);
        }
        let Some(form) = rule.forms.iter().find(|form| form.arity == args.len()) else {
            return rule.fallback.clone();
        };
        let arg_codes = args
            .iter()
            .map(|arg| self.emit_expr(arg))
            .collect::<Vec<_>>();
        apply_intrinsic_template(&form.template, &arg_codes)
    }

    fn emit_str(&mut self, args: &[Expr]) -> String {
        if args.is_empty() {
            return "\"\"".to_string();
        }
        if args.len() == 1 {
            return self.emit_str_part(&args[0], true);
        }
        args.iter()
            .enumerate()
            .map(|(index, arg)| self.emit_str_part(arg, index == 0))
            .collect::<Vec<_>>()
            .join(" + ")
    }

    fn emit_str_part(&mut self, expr: &Expr, first: bool) -> String {
        let code = self.emit_expr(expr);
        if matches!(expr.kind, ExprKind::String(_)) {
            return code;
        }
        if let ExprKind::Keyword(name) = &expr.kind {
            return format!("\"{}\"", escape_js(name));
        }
        if self.expr_type(expr).is_some_and(type_is_keyword) {
            return format!(
                "((__value) => typeof __value === \"symbol\" ? (Symbol.keyFor(__value) ?? __value.description ?? String(__value)) : String(__value))({})",
                code
            );
        }
        if self.expr_is_str_raw_string(expr) || (!first && self.expr_is_str_concat_safe(expr)) {
            return parenthesize_expression(code);
        }
        format!("String({})", code)
    }

    fn emit_unsafe_cast(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        self.emit_expr(&args[1])
    }

    fn emit_event_control(
        &mut self,
        args: &[Expr],
        prevent_default: bool,
        stop_propagation: bool,
    ) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "{{ __ce: 1, p: {}, s: {}, m: {} }}",
            prevent_default,
            stop_propagation,
            self.emit_expr(&args[0])
        )
    }

    fn emit_task_succeed(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "{ kind: Symbol.for(\"task/succeed\"), value: undefined }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"task/succeed\"), value: {} }}",
            self.emit_expr(&args[0])
        )
    }

    fn emit_task_fail(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "{ kind: Symbol.for(\"task/fail\"), error: undefined }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"task/fail\"), error: {} }}",
            self.emit_expr(&args[0])
        )
    }

    fn emit_task_combinator(&mut self, args: &[Expr], kind: &str, fn_field: &str) -> String {
        if args.len() != 2 {
            return "{ kind: Symbol.for(\"task/fail\"), error: \"invalid task\" }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"{}\"), task: {}, {}: {} }}",
            kind,
            self.emit_expr(&args[0]),
            fn_field,
            self.emit_expr(&args[1])
        )
    }

    fn emit_task_perform(&mut self, args: &[Expr]) -> String {
        self.add_runtime_effect("task/perform");
        match args.len() {
            3 => format!(
                "{{ kind: Symbol.for(\"task/perform\"), task: {}, onSuccess: {}, onError: {} }}",
                self.emit_expr(&args[0]),
                self.emit_expr(&args[1]),
                self.emit_expr(&args[2])
            ),
            4 => format!(
                "{{ kind: Symbol.for(\"task/perform\"), task: {}({}), onSuccess: {}, onError: {} }}",
                self.emit_expr(&args[0]),
                self.emit_expr(&args[1]),
                self.emit_expr(&args[2]),
                self.emit_expr(&args[3])
            ),
            _ => "{ kind: Symbol.for(\"none\") }".to_string(),
        }
    }

    fn emit_http_task(&mut self, args: &[Expr], kind: &str) -> String {
        if args.len() != 1 {
            return "{ kind: Symbol.for(\"task/fail\"), error: \"invalid HTTP task\" }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"{}\"), url: {} }}",
            kind,
            self.emit_expr(&args[0])
        )
    }

    fn emit_cmd_batch(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "{ kind: Symbol.for(\"none\") }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"batch\"), commands: {} }}",
            self.emit_expr(&args[0])
        )
    }

    fn emit_cmd_time_now(&mut self, args: &[Expr]) -> String {
        self.add_runtime_effect("time/now");
        if args.len() != 1 {
            return "{ kind: Symbol.for(\"none\") }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"time/now\"), toMessage: {} }}",
            self.emit_expr(&args[0])
        )
    }

    fn emit_cmd_random_number(&mut self, args: &[Expr]) -> String {
        self.add_runtime_effect("random/number");
        if args.len() != 3 {
            return "{ kind: Symbol.for(\"none\") }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"random/number\"), min: {}, max: {}, toMessage: {} }}",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            self.emit_expr(&args[2])
        )
    }

    fn emit_cmd_timer(&mut self, args: &[Expr], kind: &str) -> String {
        self.add_runtime_effect(kind);
        if args.len() != 3 {
            return "{ kind: Symbol.for(\"none\") }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"{}\"), id: {}, ms: {}, msg: {} }}",
            kind,
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            self.emit_expr(&args[2])
        )
    }

    fn emit_cmd_timer_cancel(&mut self, args: &[Expr]) -> String {
        self.add_runtime_effect("timer/cancel");
        if args.len() != 1 {
            return "{ kind: Symbol.for(\"none\") }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"timer/cancel\"), id: {} }}",
            self.emit_expr(&args[0])
        )
    }

    fn emit_cmd_animation_frame(&mut self, args: &[Expr]) -> String {
        self.add_runtime_effect("animation/frame");
        if args.len() != 2 {
            return "{ kind: Symbol.for(\"none\") }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"animation/frame\"), id: {}, onFrame: {} }}",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_cmd_animation_cancel(&mut self, args: &[Expr]) -> String {
        self.add_runtime_effect("animation/cancel");
        if args.len() != 2 {
            return "{ kind: Symbol.for(\"none\") }".to_string();
        }
        format!(
            "{{ kind: Symbol.for(\"animation/cancel\"), id: {}, msg: {} }}",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_msg_of(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "{ kind: undefined }".to_string();
        }
        format!("{{ kind: {} }}", self.emit_expr(&args[0]))
    }

    fn emit_msg_with(&mut self, args: &[Expr]) -> String {
        if args.len() != 3 {
            return "{ kind: undefined }".to_string();
        }
        let field = object_key(&args[1]).unwrap_or_else(|| "value".to_string());
        format!(
            "{{ kind: {}, {}: {} }}",
            self.emit_expr(&args[0]),
            field,
            self.emit_expr(&args[2])
        )
    }

    fn emit_msg_with2(&mut self, args: &[Expr]) -> String {
        if args.len() != 5 {
            return "{ kind: undefined }".to_string();
        }
        let first = object_key(&args[1]).unwrap_or_else(|| "first".to_string());
        let second = object_key(&args[3]).unwrap_or_else(|| "second".to_string());
        format!(
            "{{ kind: {}, {}: {}, {}: {} }}",
            self.emit_expr(&args[0]),
            first,
            self.emit_expr(&args[2]),
            second,
            self.emit_expr(&args[4])
        )
    }

    fn emit_msg_mapper(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "(value) => ({ kind: undefined, value })".to_string();
        }
        let field = object_key(&args[1]).unwrap_or_else(|| "value".to_string());
        format!(
            "(value) => ({{ kind: {}, {}: value }})",
            self.emit_expr(&args[0]),
            field
        )
    }

    fn emit_cmd_map(&mut self, args: &[Expr]) -> String {
        self.needs_cmd_map = true;
        if args.len() != 2 {
            return "{ kind: Symbol.for(\"none\") }".to_string();
        }
        format!(
            "{}({}, {})",
            CMD_MAP_ALIAS,
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_scope_update(&mut self, args: &[Expr]) -> String {
        self.needs_html_runtime = true;
        self.needs_scope_update = true;
        if args.len() != 5 {
            return "[undefined, { kind: Symbol.for(\"none\") }]".to_string();
        }
        format!(
            "{}({}, {}, {}, {}, {})",
            SCOPE_UPDATE_ALIAS,
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            self.emit_expr(&args[2]),
            self.emit_expr(&args[3]),
            self.emit_expr(&args[4])
        )
    }

    fn emit_scope_subscriptions(&mut self, args: &[Expr]) -> String {
        self.needs_html_runtime = true;
        self.needs_scope_subscriptions = true;
        if args.len() != 3 {
            return "{ kind: Symbol.for(\"none\") }".to_string();
        }
        format!(
            "{}({}, {}, {})",
            SCOPE_SUBSCRIPTIONS_ALIAS,
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            self.emit_expr(&args[2])
        )
    }

    fn emit_defn(&mut self, name: &str, expr: &Expr, args: &[Expr]) -> String {
        let ExprKind::Vector(params) = &args[0].kind else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "defn params must be a vector",
            ));
            return format!(
                "{}const {} = undefined;",
                self.export_prefix(),
                sanitize_identifier(name)
            );
        };
        let param_types = self
            .expr_type(expr)
            .and_then(type_fn_param_types)
            .map(|types| types.into_iter().map(str::to_string).collect::<Vec<_>>())
            .unwrap_or_default();

        if let [template] = &args[1..] {
            if let ExprKind::HtmlTemplate(node) = &template.kind {
                let (param_names, param_idents) = self.emit_defn_symbol_params(params);
                return self.emit_template_defn(
                    name,
                    &param_idents,
                    &param_names,
                    &param_types,
                    node,
                );
            }
            if let Some((bindings, node)) = let_template_parts(template) {
                let (param_names, param_idents) = self.emit_defn_symbol_params(params);
                return self.emit_let_template_defn(
                    name,
                    &param_idents,
                    &param_names,
                    &param_types,
                    bindings,
                    node,
                );
            }
        }

        let mut js_params = Vec::new();
        let mut simple_param_idents = Vec::new();
        let mut pattern_statements = Vec::new();
        let mut has_pattern_params = false;
        let mut local_type_bindings = BTreeMap::new();
        for (index, param) in params.iter().enumerate() {
            match &param.kind {
                ExprKind::Symbol(name) if name != "_" => {
                    let ident = sanitize_identifier(name);
                    js_params.push(ident.clone());
                    simple_param_idents.push(ident);
                    if let Some(param_type) = param_types.get(index) {
                        local_type_bindings.insert(name.clone(), param_type.clone());
                    }
                }
                _ => {
                    has_pattern_params = true;
                    let value_name = self.next_temp("__closkell_arg");
                    js_params.push(value_name.clone());
                    let param_type = param_types.get(index).map(String::as_str);
                    collect_pattern_type_bindings(param, param_type, &mut local_type_bindings);
                    self.emit_pattern_value_statement(
                        param,
                        &value_name,
                        param_type,
                        "defn parameter pattern did not match",
                        &mut pattern_statements,
                    );
                }
            }
        }

        let previous_update_message_param = self.current_update_message_param.clone();
        if name == "update" {
            self.current_update_message_param = simple_param_idents.get(1).cloned();
        }

        self.local_types.push(local_type_bindings);
        let body = if has_pattern_params {
            let function_body = self.emit_function_body(&args[1..]);
            [pattern_statements.join(" "), function_body]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            self.emit_tail_recursive_function_body(name, &simple_param_idents, &args[1..])
                .unwrap_or_else(|| self.emit_function_body(&args[1..]))
        };
        self.local_types.pop();
        let params = js_params.join(", ");
        let result = format!(
            "{}function {}({}) {{ {} }}",
            self.export_prefix(),
            sanitize_identifier(name),
            params,
            body
        );
        self.current_update_message_param = previous_update_message_param;
        result
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
        param_types: &[String],
        node: &HtmlNode,
    ) -> String {
        let param_list = params.join(", ");
        let local_type_bindings = param_names
            .iter()
            .zip(param_types.iter())
            .map(|(name, ty)| (name.clone(), ty.clone()))
            .collect::<BTreeMap<_, _>>();
        self.local_types.push(local_type_bindings);
        let component_expr =
            self.emit_template_component_with_params(node, &ReadAliases::new(), param_names);
        self.local_types.pop();
        let update_params = params
            .iter()
            .map(|param| format!("next_{}", param))
            .collect::<Vec<_>>();
        let reassign = params
            .iter()
            .zip(update_params.iter())
            .map(|(param, update_param)| format!("{} = {};", param, update_param))
            .collect::<Vec<_>>()
            .join(" ");
        let bind = if update_params.is_empty() {
            "null".to_string()
        } else {
            format!("({}) => {{ {} }}", update_params.join(", "), reassign)
        };

        format!(
            "{}function {}({}) {{ const __closkellComponent = {}; return {}(__closkellComponent, {}, {}); }}",
            self.export_prefix(),
            sanitize_identifier(name),
            param_list,
            component_expr,
            BIND_COMPONENT_ALIAS,
            params.len(),
            bind
        )
    }

    fn emit_let_template_defn(
        &mut self,
        name: &str,
        params: &[String],
        param_names: &[String],
        param_types: &[String],
        bindings: &[Expr],
        node: &HtmlNode,
    ) -> String {
        let param_list = params.join(", ");
        let mut locals = BTreeSet::new();
        let mut refresh_lines = Vec::new();
        let mut read_aliases = ReadAliases::new();
        let local_type_bindings = param_names
            .iter()
            .zip(param_types.iter())
            .map(|(name, ty)| (name.clone(), ty.clone()))
            .collect::<BTreeMap<_, _>>();
        self.local_types.push(local_type_bindings);

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
            let value_type = self.expr_type(value).map(str::to_string);

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
                        value_type.as_deref(),
                        "let pattern did not match",
                        &mut refresh_lines,
                    );
                }
            }
            if let Some(scope) = self.local_types.last_mut() {
                collect_pattern_type_bindings(binding, value_type.as_deref(), scope);
            }
        }

        let declarations = locals
            .iter()
            .map(|local| format!("let {};", local))
            .collect::<Vec<_>>()
            .join(" ");
        let refresh_body = refresh_lines.join(" ");
        let component_expr =
            self.emit_template_component_with_params(node, &read_aliases, param_names);
        self.local_types.pop();
        let update_params = params
            .iter()
            .map(|param| format!("next_{}", param))
            .collect::<Vec<_>>();
        let reassign = params
            .iter()
            .zip(update_params.iter())
            .map(|(param, update_param)| format!("{} = {};", param, update_param))
            .collect::<Vec<_>>()
            .join(" ");
        let bind = if update_params.is_empty() {
            format!("() => {{ __closkellRefresh(); }}")
        } else {
            format!(
                "({}) => {{ {} __closkellRefresh(); }}",
                update_params.join(", "),
                reassign
            )
        };

        format!(
            "{}function {}({}) {{ {} const __closkellRefresh = () => {{ {} }}; __closkellRefresh(); const __closkellComponent = {}; return {}(__closkellComponent, {}, {}); }}",
            self.export_prefix(),
            sanitize_identifier(name),
            param_list,
            declarations,
            refresh_body,
            component_expr,
            BIND_COMPONENT_ALIAS,
            params.len(),
            bind
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
        let mut local_type_bindings = BTreeMap::new();
        let param_types = self
            .expr_type(expr)
            .and_then(type_fn_param_types)
            .map(|types| types.into_iter().map(str::to_string).collect::<Vec<_>>())
            .unwrap_or_default();
        for (index, param) in params.iter().enumerate() {
            match &param.kind {
                ExprKind::Symbol(name) if name != "_" => {
                    js_params.push(sanitize_identifier(name));
                    if let Some(param_type) = param_types.get(index) {
                        local_type_bindings.insert(name.clone(), param_type.clone());
                    }
                }
                _ => {
                    needs_block = true;
                    let value_name = self.next_temp("__closkell_arg");
                    js_params.push(value_name.clone());
                    let param_type = param_types.get(index).map(String::as_str);
                    collect_pattern_type_bindings(param, param_type, &mut local_type_bindings);
                    self.emit_pattern_value_statement(
                        param,
                        &value_name,
                        param_type,
                        "fn parameter pattern did not match",
                        &mut statements,
                    );
                }
            }
        }
        let params = js_params.join(", ");
        self.local_types.push(local_type_bindings);
        let body = self.emit_do(&args[1..]);
        self.local_types.pop();
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
        self.local_types.push(BTreeMap::new());
        for pair in bindings.chunks(2) {
            let [pattern, value] = pair else {
                self.diagnostics.push(Diagnostic::error(
                    args[0].span,
                    "let emission requires complete binding pairs",
                ));
                continue;
            };
            let value_type = self.expr_type(value).map(str::to_string);
            self.emit_let_pattern_statement(pattern, value, &mut statements);
            if let Some(scope) = self.local_types.last_mut() {
                collect_pattern_type_bindings(pattern, value_type.as_deref(), scope);
            }
        }
        statements.push(format!("return {};", self.emit_do(&args[1..])));
        self.local_types.pop();
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
        let value_type = self.expr_type(value).map(str::to_string);
        self.emit_pattern_value_statement(
            pattern,
            &value_name,
            value_type.as_deref(),
            "let pattern did not match",
            statements,
        );
    }

    fn emit_pattern_value_statement(
        &mut self,
        pattern: &Expr,
        value_name: &str,
        value_type: Option<&str>,
        error_message: &str,
        statements: &mut Vec<String>,
    ) {
        let compiled = self.emit_pattern(pattern, value_name, value_type);
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
        value_type: Option<&str>,
        error_message: &str,
        statements: &mut Vec<String>,
    ) {
        let compiled = self.emit_pattern(pattern, value_name, value_type);
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
        if let Some(condition) = self.static_bool(&args[0]) {
            return self.emit_expr(if condition { &args[1] } else { &args[2] });
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
        if let Some(code) = self.emit_kind_switch_match(args) {
            return code;
        }

        let value_name = self.next_temp("__closkell_match");
        let value_type = self.expr_type(&args[0]).map(str::to_string);
        let value = self.emit_expr(&args[0]);
        let mut lines = vec![format!("const {} = {};", value_name, value)];

        for (index, arm) in args[1..].chunks(2).enumerate() {
            let [pattern, body] = arm else {
                continue;
            };
            let compiled = self.emit_pattern(pattern, &value_name, value_type.as_deref());
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

    fn emit_kind_switch_match(&mut self, args: &[Expr]) -> Option<String> {
        let plan = kind_match_plan(args)?;
        let value_name = self.next_temp("__closkell_match");
        let kind_name = self.next_temp("__closkell_kind");
        let value = self.emit_expr(&args[0]);
        let value_type = self.expr_type(&args[0]).map(str::to_string);
        let kind_read = if value_type.as_deref().is_some_and(type_has_kind_property) {
            format!("{}.kind", value_name)
        } else {
            format!("{}?.kind", value_name)
        };
        let reachable_update_messages = self.reachable_update_messages_for_match(&args[0]).cloned();
        let mut lines = vec![
            format!("const {} = {};", value_name, value),
            format!("const {} = {};", kind_name, kind_read),
            format!("switch ({}) {{", kind_name),
        ];

        for arm in &plan.arms {
            if reachable_update_messages
                .as_ref()
                .is_some_and(|reachable| !reachable.contains(&arm.kind))
            {
                continue;
            }
            let bindings = self.emit_kind_pattern_bindings(arm.pattern, &value_name)?;
            let body = self.emit_expr(arm.body);
            lines.push(format!(
                "case {}: {{ {} return {}; }}",
                keyword_literal(&arm.kind),
                bindings,
                body
            ));
        }

        if let Some(default) = &plan.default {
            let compiled = self.emit_pattern(default.pattern, &value_name, value_type.as_deref());
            if compiled.test != "true" {
                return None;
            }
            let body = self.emit_expr(default.body);
            lines.push(format!(
                "default: {{ {} return {}; }}",
                compiled.bindings, body
            ));
        }

        lines.push("}".to_string());
        lines.push("throw new Error(\"non-exhaustive match\");".to_string());
        Some(format!("(() => {{ {} }})()", lines.join(" ")))
    }

    fn reachable_update_messages_for_match(&self, value: &Expr) -> Option<&BTreeSet<String>> {
        let message_param = self.current_update_message_param.as_ref()?;
        let ExprKind::Symbol(name) = &value.kind else {
            return None;
        };
        if name == message_param {
            self.reachable_message_kinds.as_ref()
        } else {
            None
        }
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

    fn emit_logical(&mut self, args: &[Expr], is_and: bool) -> String {
        if args.is_empty() {
            return if is_and { "true" } else { "false" }.to_string();
        }
        let mut parts = Vec::new();
        for arg in args {
            match self.static_bool(arg) {
                Some(value) if value == is_and => {}
                Some(value) => return value.to_string(),
                None => parts.push(parenthesize_expression(self.emit_expr(arg))),
            }
        }
        if parts.is_empty() {
            return if is_and { "true" } else { "false" }.to_string();
        }
        let op = if is_and { " && " } else { " || " };
        parts.join(op)
    }

    fn emit_value_equal(&mut self, args: &[Expr]) -> String {
        if args.len() < 2 {
            return "undefined".to_string();
        }
        if let Some(code) = self.emit_primitive_value_equal(args) {
            return code;
        }
        if args.len() == 2
            && self.expr_is_strict_comparable(&args[0])
            && self.expr_is_strict_comparable(&args[1])
        {
            return format!(
                "{} === {}",
                parenthesize_expression(self.emit_expr(&args[0])),
                parenthesize_expression(self.emit_expr(&args[1]))
            );
        }
        self.needs_value_equal_helper = true;
        if args.len() == 2 {
            return format!(
                "__closkellValueEqual({}, {})",
                self.emit_expr(&args[0]),
                self.emit_expr(&args[1])
            );
        }
        let values = args
            .iter()
            .map(|arg| self.emit_expr(arg))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "((__values) => {{ for (let __index = 1; __index < __values.length; __index += 1) if (!__closkellValueEqual(__values[__index - 1], __values[__index])) return false; return true; }})([{}])",
            values
        )
    }

    fn emit_primitive_value_equal(&mut self, args: &[Expr]) -> Option<String> {
        if args.len() != 2 {
            return None;
        }
        let left_literal = primitive_equality_literal(&args[0]);
        let right_literal = primitive_equality_literal(&args[1]);
        match (left_literal, right_literal) {
            (Some(left), Some(right)) => Some(format!("{} === {}", left.code, right.code)),
            (Some(left), None) => Some(format!(
                "{} === {}",
                parenthesize_expression(self.emit_expr(&args[1])),
                left.code
            )),
            (None, Some(right)) => Some(format!(
                "{} === {}",
                parenthesize_expression(self.emit_expr(&args[0])),
                right.code
            )),
            (None, None) => None,
        }
    }

    fn emit_identical(&mut self, args: &[Expr]) -> String {
        if args.len() < 2 {
            return "undefined".to_string();
        }
        let values = args
            .iter()
            .map(|arg| self.emit_expr(arg))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "((...__values) => {{ for (let __index = 1; __index < __values.length; __index += 1) if (!Object.is(__values[__index - 1], __values[__index])) return false; return true; }})({})",
            values
        )
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
        if let Some((collection, mapper, indexed)) = map_call_parts(values) {
            return self
                .emit_mapped_numeric_aggregate(method, collection, mapper, indexed, fallbacks);
        }

        let values = self.emit_expr(values);
        let identity = if method == "max" {
            "-Infinity"
        } else {
            "Infinity"
        };
        let comparison = if method == "max" { ">" } else { "<" };
        let fallback_terms = fallbacks
            .iter()
            .map(|fallback| self.emit_expr(fallback))
            .collect::<Vec<_>>();
        let fallback_update = fallback_terms
            .iter()
            .map(|fallback| {
                format!(
                    "if ({} {} __result) __result = {};",
                    fallback, comparison, fallback
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "((__values) => {{ let __result = {}; for (const __value of __values) if (__value {} __result) __result = __value; {} return __result; }})({})",
            identity, comparison, fallback_update, values
        )
    }

    fn emit_sum(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        if let Some((collection, mapper, indexed)) = map_call_parts(&args[0]) {
            return self.emit_mapped_sum(collection, mapper, indexed);
        }
        format!(
            "((__values) => {{ let __sum = 0; for (const __value of __values) __sum += __value; return __sum; }})({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_mapped_sum(&mut self, collection: &Expr, mapper: &Expr, indexed: bool) -> String {
        let collection = self.emit_expr(collection);
        let mapper = self.emit_expr(mapper);
        if indexed {
            format!(
                "((__collection) => {{ const __items = Array.isArray(__collection) ? __collection : Array.from(__collection); let __sum = 0; for (let __index = 0; __index < __items.length; __index += 1) {{ const __item = __items[__index]; __sum += {}(__item, __index); }} return __sum; }})({})",
                mapper, collection
            )
        } else {
            format!(
                "((__collection) => {{ let __sum = 0; for (const __item of __collection) __sum += {}(__item); return __sum; }})({})",
                mapper, collection
            )
        }
    }

    fn emit_mapped_numeric_aggregate(
        &mut self,
        method: &str,
        collection: &Expr,
        mapper: &Expr,
        indexed: bool,
        fallbacks: &[Expr],
    ) -> String {
        let collection = self.emit_expr(collection);
        let mapper = self.emit_expr(mapper);
        let identity = if method == "max" {
            "-Infinity"
        } else {
            "Infinity"
        };
        let comparison = if method == "max" { ">" } else { "<" };
        let fallback_update = fallbacks
            .iter()
            .map(|fallback| {
                let fallback = self.emit_expr(fallback);
                format!(
                    "if ({} {} __result) __result = {};",
                    fallback, comparison, fallback
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        if indexed {
            format!(
                "((__collection) => {{ const __items = Array.isArray(__collection) ? __collection : Array.from(__collection); let __result = {}; for (let __index = 0; __index < __items.length; __index += 1) {{ const __item = __items[__index]; const __value = {}(__item, __index); if (__value {} __result) __result = __value; }} {} return __result; }})({})",
                identity, mapper, comparison, fallback_update, collection
            )
        } else {
            format!(
                "((__collection) => {{ let __result = {}; for (const __item of __collection) {{ const __value = {}(__item); if (__value {} __result) __result = __value; }} {} return __result; }})({})",
                identity, mapper, comparison, fallback_update, collection
            )
        }
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
            "((__value) => {{ try {{ const __number = Number(__value); return Number.isFinite(__number) ? __number : 0; }} catch {{ return 0; }} }})({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_date_format(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        if let Some(style) = literal_string_value(&args[1]) {
            let timestamp = self.emit_expr(&args[0]);
            if style == "iso-date" {
                return format!("new Date({}).toISOString().slice(0, 10)", timestamp);
            }
            return match date_format_options(style) {
                Some(options) => format!(
                    "new Intl.DateTimeFormat(undefined, {}).format({})",
                    options, timestamp
                ),
                None => format!("new Intl.DateTimeFormat(undefined).format({})", timestamp),
            };
        }
        format!(
            "((__timestamp, __key) => {{ const __date = new Date(__timestamp); const __options = __key === \"month-year\" ? {{ month: \"short\", year: \"2-digit\" }} : __key === \"month-day-time\" ? {{ month: \"short\", day: \"numeric\", hour: \"2-digit\", minute: \"2-digit\" }} : __key === \"month-day\" ? {{ month: \"short\", day: \"numeric\" }} : __key === \"month\" ? {{ month: \"short\" }} : __key === \"day\" ? {{ day: \"numeric\" }} : undefined; return __key === \"iso-date\" ? __date.toISOString().slice(0, 10) : new Intl.DateTimeFormat(undefined, __options).format(__timestamp); }})({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_env_dev(&mut self, args: &[Expr]) -> String {
        if !args.is_empty() {
            return "undefined".to_string();
        }
        "Boolean(import.meta.env && import.meta.env.DEV)".to_string()
    }

    fn emit_env_mode(&mut self, args: &[Expr]) -> String {
        if !args.is_empty() {
            return "undefined".to_string();
        }
        "String(import.meta.env && import.meta.env.MODE || \"\")".to_string()
    }

    fn static_bool(&self, expr: &Expr) -> Option<bool> {
        match &expr.kind {
            ExprKind::Bool(value) => Some(*value),
            ExprKind::Symbol(name) => match self.static_reads.get(name).map(String::as_str) {
                Some("true") => Some(true),
                Some("false") => Some(false),
                _ => None,
            },
            ExprKind::List(items) => {
                let (head, args) = items.split_first()?;
                let ExprKind::Symbol(name) = &head.kind else {
                    return None;
                };
                match name.as_str() {
                    "some?" if args.len() == 1 => self
                        .expr_runtime_type_predicate(&args[0], RuntimeTypePredicate::Nil)
                        .map(|value| !value),
                    "nil?" if args.len() == 1 => {
                        self.expr_runtime_type_predicate(&args[0], RuntimeTypePredicate::Nil)
                    }
                    "number?" if args.len() == 1 => {
                        self.expr_runtime_type_predicate(&args[0], RuntimeTypePredicate::Number)
                    }
                    "string?" if args.len() == 1 => {
                        self.expr_runtime_type_predicate(&args[0], RuntimeTypePredicate::String)
                    }
                    "bool?" if args.len() == 1 => {
                        self.expr_runtime_type_predicate(&args[0], RuntimeTypePredicate::Bool)
                    }
                    "keyword?" if args.len() == 1 => {
                        self.expr_runtime_type_predicate(&args[0], RuntimeTypePredicate::Keyword)
                    }
                    "list?" | "vector?" if args.len() == 1 => {
                        self.literal_runtime_type_predicate(&args[0], RuntimeTypePredicate::Vector)
                    }
                    "set?" if args.len() == 1 => {
                        self.literal_runtime_type_predicate(&args[0], RuntimeTypePredicate::Set)
                    }
                    "map?" if args.len() == 1 => {
                        self.literal_runtime_type_predicate(&args[0], RuntimeTypePredicate::Map)
                    }
                    "object?" if args.len() == 1 => {
                        self.literal_runtime_type_predicate(&args[0], RuntimeTypePredicate::Object)
                    }
                    "not" if args.len() == 1 => self.static_bool(&args[0]).map(|value| !value),
                    "and" => {
                        let mut all_true = true;
                        for arg in args {
                            match self.static_bool(arg) {
                                Some(false) => return Some(false),
                                Some(true) => {}
                                None => all_true = false,
                            }
                        }
                        all_true.then_some(true)
                    }
                    "or" => {
                        let mut all_false = true;
                        for arg in args {
                            match self.static_bool(arg) {
                                Some(true) => return Some(true),
                                Some(false) => {}
                                None => all_false = false,
                            }
                        }
                        all_false.then_some(false)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn emit_count(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        if self.expr_is_countable_by_length(&args[0]) {
            return format!(
                "{}.length",
                parenthesize_member_base(self.emit_expr(&args[0]))
            );
        }
        if self.expr_is_countable_by_size(&args[0]) {
            return format!(
                "{}.size",
                parenthesize_member_base(self.emit_expr(&args[0]))
            );
        }
        if self.expr_is_record(&args[0]) {
            return format!("Object.keys({}).length", self.emit_expr(&args[0]));
        }
        self.needs_count_helper = true;
        format!("__closkellCount({})", self.emit_expr(&args[0]))
    }

    fn emit_empty(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        if self.expr_is_countable_by_length(&args[0]) {
            return format!(
                "{}.length === 0",
                parenthesize_member_base(self.emit_expr(&args[0]))
            );
        }
        if self.expr_is_countable_by_size(&args[0]) {
            return format!(
                "{}.size === 0",
                parenthesize_member_base(self.emit_expr(&args[0]))
            );
        }
        if self.expr_is_record(&args[0]) {
            return format!("Object.keys({}).length === 0", self.emit_expr(&args[0]));
        }
        self.needs_count_helper = true;
        format!("__closkellCount({}) === 0", self.emit_expr(&args[0]))
    }

    fn emit_some(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        if let Some(result) = self.expr_runtime_type_predicate(&args[0], RuntimeTypePredicate::Nil)
        {
            return (!result).to_string();
        }
        format!("{} != null", self.emit_expr(&args[0]))
    }

    fn emit_nil(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        if let Some(result) = self.expr_runtime_type_predicate(&args[0], RuntimeTypePredicate::Nil)
        {
            return result.to_string();
        }
        format!("({}) == null", self.emit_expr(&args[0]))
    }

    fn emit_type_predicate(&mut self, js_type: &str, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        let predicate = match js_type {
            "number" => RuntimeTypePredicate::Number,
            "string" => RuntimeTypePredicate::String,
            "boolean" => RuntimeTypePredicate::Bool,
            "symbol" => RuntimeTypePredicate::Keyword,
            _ => return "undefined".to_string(),
        };
        if let Some(result) = self.expr_runtime_type_predicate(&args[0], predicate) {
            return result.to_string();
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
        if let Some(result) =
            self.literal_runtime_type_predicate(&args[0], RuntimeTypePredicate::Vector)
        {
            return result.to_string();
        }
        format!("Array.isArray({})", self.emit_expr(&args[0]))
    }

    fn emit_set_predicate(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        if let Some(result) =
            self.literal_runtime_type_predicate(&args[0], RuntimeTypePredicate::Set)
        {
            return result.to_string();
        }
        format!("{} instanceof Set", self.emit_expr(&args[0]))
    }

    fn emit_map_predicate(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        if let Some(result) =
            self.literal_runtime_type_predicate(&args[0], RuntimeTypePredicate::Map)
        {
            return result.to_string();
        }
        format!("{} instanceof Map", self.emit_expr(&args[0]))
    }

    fn emit_object_predicate(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        if let Some(result) =
            self.literal_runtime_type_predicate(&args[0], RuntimeTypePredicate::Object)
        {
            return result.to_string();
        }
        self.needs_object_predicate_helper = true;
        format!("__closkellIsObject({})", self.emit_expr(&args[0]))
    }

    fn emit_get(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        let base = parenthesize_member_base(self.emit_expr(&args[0]));
        match &args[1].kind {
            ExprKind::Keyword(name) | ExprKind::Symbol(name) | ExprKind::String(name) => {
                if self
                    .expr_type(&args[0])
                    .is_some_and(|ty| type_has_required_record_field(ty, name))
                {
                    return format!("{}{}", base, property_access(name));
                }
                format!("({}{} ?? null)", base, optional_property_access(name))
            }
            _ => format!("({}?.[{}] ?? null)", base, self.emit_expr(&args[1])),
        }
    }

    fn emit_object_get(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }

        format!(
            "((__value, __key) => __value instanceof Map ? (__value.has(__key) ? __value.get(__key) : null) : (__value?.[__key] ?? null))({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_get_in(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }

        format!(
            "((__value, __path) => {{ const __keyName = (__key) => typeof __key === \"symbol\" ? (Symbol.keyFor(__key) ?? __key.description ?? String(__key)) : __key; const __get = (__current, __key) => {{ if (__current == null) return null; const __name = __keyName(__key); if (__current instanceof Map) return __current.has(__key) ? __current.get(__key) : (__current.has(__name) ? __current.get(__name) : null); return __current?.[__name] ?? null; }}; return Array.isArray(__path) ? __path.reduce((__current, __key) => __get(__current, __key), __value) : null; }})({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
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

    fn emit_object_entries(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }

        self.needs_object_entries_helper = true;
        format!("__closkellObjectEntries({})", self.emit_expr(&args[0]))
    }

    fn emit_object_projection(&mut self, args: &[Expr], method: &str) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }

        format!(
            "((__value) => __value instanceof Map ? Array.from(__value.{}()) : (__value != null && typeof __value === \"object\" ? Object.{}(__value) : []))({})",
            method,
            method,
            self.emit_expr(&args[0])
        )
    }

    fn emit_object_assoc(&mut self, args: &[Expr]) -> String {
        if args.len() < 3 || args[1..].len() % 2 != 0 {
            return "undefined".to_string();
        }

        let statements = args[1..]
            .chunks(2)
            .filter_map(|pair| match pair {
                [key, value] => Some(format!(
                    "__next[{}] = {};",
                    self.emit_expr(key),
                    self.emit_expr(value)
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "((__object) => {{ const __next = {{ ...(__object ?? {{}}) }}; {} return __next; }})({})",
            statements,
            self.emit_expr(&args[0])
        )
    }

    fn emit_object_dissoc(&mut self, args: &[Expr]) -> String {
        if args.len() < 2 {
            return "undefined".to_string();
        }

        let statements = args[1..]
            .iter()
            .map(|key| format!("delete __next[{}];", self.emit_expr(key)))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "((__object) => {{ const __next = {{ ...(__object ?? {{}}) }}; {} return __next; }})({})",
            statements,
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
        if let Some(emitted) = self.emit_inline_range_map_transform(&args[0], &args[1], indexed) {
            return emitted;
        }
        if let Some(emitted) = self.emit_native_literal_callback_transform(
            &args[0],
            &args[1],
            ".map",
            if indexed {
                &["__item", "__index"]
            } else {
                &["__item"]
            },
        ) {
            return emitted;
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
        if let Some(emitted) =
            self.emit_native_literal_callback_transform(&args[0], &args[1], ".filter", &["__item"])
        {
            return emitted;
        }
        format!(
            "{}.filter((__item) => {}(__item))",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_native_literal_callback_transform(
        &mut self,
        collection: &Expr,
        callback: &Expr,
        method: &str,
        fallback_params: &[&str],
    ) -> Option<String> {
        let (params, body) = literal_fn_parts(callback)?;
        if params.len() != fallback_params.len()
            || params.iter().any(|param| !simple_fn_param(param))
        {
            return None;
        }

        let collection = parenthesize_member_base(self.emit_expr(collection));
        let param_types = self
            .expr_type(callback)
            .and_then(type_fn_param_types)
            .map(|types| types.into_iter().map(str::to_string).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut local_type_bindings = BTreeMap::new();
        let params = params
            .iter()
            .zip(fallback_params.iter())
            .enumerate()
            .map(|(index, (param, fallback))| {
                let name = symbol_name(param)
                    .filter(|name| !name.is_empty())
                    .map(sanitize_identifier)
                    .unwrap_or_else(|| fallback.to_string());
                if let Some(param_type) = param_types.get(index) {
                    local_type_bindings.insert(name.clone(), param_type.clone());
                }
                name
            })
            .collect::<Vec<_>>();

        self.local_types.push(local_type_bindings);
        let body = self.emit_arrow_callback_body(body);
        self.local_types.pop();

        Some(format!(
            "{}{}(({}) => {})",
            collection,
            method,
            params.join(", "),
            body
        ))
    }

    fn emit_arrow_callback_body(&mut self, body: &[Expr]) -> String {
        if let [single] = body {
            if let Some((bindings, body)) = let_parts(single) {
                let mut statements = Vec::new();
                self.local_types.push(BTreeMap::new());
                for pair in bindings.chunks(2) {
                    let [pattern, value] = pair else {
                        self.diagnostics.push(Diagnostic::error(
                            single.span,
                            "let emission requires complete binding pairs",
                        ));
                        continue;
                    };
                    let value_type = self.expr_type(value).map(str::to_string);
                    self.emit_let_pattern_statement(pattern, value, &mut statements);
                    if let Some(scope) = self.local_types.last_mut() {
                        collect_pattern_type_bindings(pattern, value_type.as_deref(), scope);
                    }
                }
                statements.push(format!("return {};", self.emit_do(body)));
                self.local_types.pop();
                return format!("{{ {} }}", statements.join(" "));
            }
        }

        if body.len() > 1 {
            let mut statements = Vec::new();
            for expr in &body[..body.len() - 1] {
                statements.push(format!("{};", self.emit_expr(expr)));
            }
            statements.push(format!("return {};", self.emit_expr(&body[body.len() - 1])));
            return format!("{{ {} }}", statements.join(" "));
        }

        parenthesize_arrow_body(&self.emit_do(body))
    }

    fn emit_inline_range_map_transform(
        &mut self,
        collection: &Expr,
        mapper: &Expr,
        indexed: bool,
    ) -> Option<String> {
        let range_args = range_call_args(collection)?;
        let (params, body) = literal_fn_parts(mapper)?;
        let expected = if indexed { 2 } else { 1 };
        if params.len() != expected || params.iter().any(|param| !simple_fn_param(param)) {
            return None;
        }

        let (start, end, step) = match range_args {
            [end] => ("0".to_string(), self.emit_expr(end), "1".to_string()),
            [start, end] => (self.emit_expr(start), self.emit_expr(end), "1".to_string()),
            [start, end, step] => (
                self.emit_expr(start),
                self.emit_expr(end),
                self.emit_expr(step),
            ),
            _ => return None,
        };

        let start_name = self.next_temp("__closkell_range_start");
        let end_name = self.next_temp("__closkell_range_end");
        let step_name = self.next_temp("__closkell_range_step");
        let count_name = self.next_temp("__closkell_range_count");
        let result_name = self.next_temp("__closkell_range_result");
        let index_name = self.next_temp("__closkell_range_index");
        let value_name = self.next_temp("__closkell_range_value");
        let mut statements = Vec::new();
        let mut local_type_bindings = BTreeMap::new();
        let param_types = self
            .expr_type(mapper)
            .and_then(type_fn_param_types)
            .map(|types| types.into_iter().map(str::to_string).collect::<Vec<_>>())
            .unwrap_or_default();

        if let Some(name) = symbol_name(&params[0]).filter(|name| *name != "_") {
            let name = sanitize_identifier(name);
            if let Some(param_type) = param_types.first() {
                local_type_bindings.insert(name.clone(), param_type.clone());
            }
            statements.push(format!("const {} = {};", name, value_name));
        }
        if indexed {
            if let Some(name) = symbol_name(&params[1]).filter(|name| *name != "_") {
                let name = sanitize_identifier(name);
                if let Some(param_type) = param_types.get(1) {
                    local_type_bindings.insert(name.clone(), param_type.clone());
                }
                statements.push(format!("const {} = {};", name, index_name));
            }
        }

        self.local_types.push(local_type_bindings);
        let body = self.emit_do(body);
        self.local_types.pop();
        statements.push(format!("{}[{}] = {};", result_name, index_name, body));

        Some(format!(
            "(({}, {}, {}) => {{ if ({} === 0) return []; const {} = Math.max(0, Math.ceil(({} - {}) / {})); const {} = new Array({}); for (let {} = 0, {} = {}; {} < {}; {} += 1, {} += {}) {{ {} }} return {}; }})({}, {}, {})",
            start_name,
            end_name,
            step_name,
            step_name,
            count_name,
            end_name,
            start_name,
            step_name,
            result_name,
            count_name,
            index_name,
            value_name,
            start_name,
            index_name,
            count_name,
            index_name,
            value_name,
            step_name,
            statements.join(" "),
            result_name,
            start,
            end,
            step
        ))
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
        match self.expr_collection_shape(&args[0]) {
            CollectionShape::Set => {
                return format!(
                    "new Set([...{}, {}])",
                    parenthesize_expression(collection),
                    items
                );
            }
            CollectionShape::VectorLike => {
                return format!("[...{}, {}]", parenthesize_expression(collection), items);
            }
            _ => {}
        }
        format!(
            "((__collection, ...__items) => {{ if (__collection instanceof Set) return new Set([...__collection, ...__items]); const __next = Array.isArray(__collection) ? __collection.slice() : Array.from(__collection); __next.push(...__items); return __next; }})({}, {})",
            collection, items
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
        if let Some(optimized) = self.emit_append_reduce(args, indexed) {
            return optimized;
        }
        if let Some(optimized) = self.emit_helper_append_reduce(args, indexed) {
            return optimized;
        }
        if let Some(optimized) = self.emit_inline_reduce(args, indexed) {
            return optimized;
        }
        let collection = self.emit_expr(&args[0]);
        let initial = self.emit_expr(&args[1]);
        let reducer = self.emit_expr(&args[2]);
        let collection_name = self.next_temp("__closkell_reduce_collection");
        let initial_name = self.next_temp("__closkell_reduce_initial");
        let reducer_name = self.next_temp("__closkell_reduce_fn");
        let items_name = self.next_temp("__closkell_reduce_items");
        let acc_name = self.next_temp("__closkell_reduce_acc");
        let index_name = self.next_temp("__closkell_reduce_index");
        let item_name = self.next_temp("__closkell_reduce_item");
        let items_binding = self.emit_reduce_items_binding(
            &items_name,
            &collection_name,
            self.expr_collection_shape(&args[0]),
        );
        let call = if indexed {
            format!(
                "{}({}, {}, {})",
                reducer_name, acc_name, item_name, index_name
            )
        } else {
            format!("{}({}, {})", reducer_name, acc_name, item_name)
        };
        format!(
            "(({}, {}, {}) => {{ {} let {} = {}; for (let {} = 0; {} < {}.length; {} += 1) {{ const {} = {}[{}]; {} = {}; }} return {}; }})({}, {}, {})",
            collection_name,
            initial_name,
            reducer_name,
            items_binding,
            acc_name,
            initial_name,
            index_name,
            index_name,
            items_name,
            index_name,
            item_name,
            items_name,
            index_name,
            acc_name,
            call,
            acc_name,
            collection,
            initial,
            reducer
        )
    }

    fn emit_append_reduce(&mut self, args: &[Expr], indexed: bool) -> Option<String> {
        let reducer = AppendReducer::parse(&args[2], indexed)?;
        let collection = self.emit_expr(&args[0]);
        let initial = self.emit_expr(&args[1]);
        let collection_shape = self.expr_collection_shape(&args[0]);
        let initial_shape = self.expr_collection_shape(&args[1]);

        if !indexed
            && collection_shape == CollectionShape::VectorLike
            && initial_shape == CollectionShape::VectorLike
            && reducer.bindings.is_empty()
            && reducer.items.len() == 1
            && matches_symbol(&reducer.items[0], reducer.item)
        {
            return Some(format!(
                "((__items, __acc) => __acc.concat(__items))({}, {})",
                collection, initial
            ));
        }

        let collection_name = self.next_temp("__closkell_reduce_collection");
        let initial_name = self.next_temp("__closkell_reduce_initial");
        let items_name = self.next_temp("__closkell_reduce_items");
        let acc_name = self.next_temp("__closkell_reduce_acc");
        let index_name = self.next_temp("__closkell_reduce_index");
        let item_name = self.next_temp("__closkell_reduce_item");

        let mut body = Vec::new();
        body.push(format!(
            "const {} = {};",
            sanitize_identifier(reducer.acc),
            acc_name
        ));
        if reducer.item != "_" {
            body.push(format!(
                "const {} = {};",
                sanitize_identifier(reducer.item),
                item_name
            ));
        }
        if let Some(index) = reducer.index {
            if index != "_" {
                body.push(format!(
                    "const {} = {};",
                    sanitize_identifier(index),
                    index_name
                ));
            }
        }
        for (binding, value) in reducer.bindings {
            match &binding.kind {
                ExprKind::Symbol(name) if name == "_" => {
                    body.push(format!("{};", self.emit_expr(value)));
                }
                ExprKind::Symbol(name) => {
                    body.push(format!(
                        "const {} = {};",
                        sanitize_identifier(name),
                        self.emit_expr(value)
                    ));
                }
                _ => {
                    let value_name = self.next_temp("__closkell_reduce_let");
                    body.push(format!("const {} = {};", value_name, self.emit_expr(value)));
                    let value_type = self.expr_type(value).map(str::to_string);
                    self.emit_pattern_assignment_statement(
                        binding,
                        &value_name,
                        value_type.as_deref(),
                        "reduce append let pattern did not match",
                        &mut body,
                    );
                }
            }
        }

        self.push_append_items(&mut body, &acc_name, reducer.items, initial_shape);

        let items_binding =
            self.emit_reduce_items_binding(&items_name, &collection_name, collection_shape);
        let acc_binding = self.emit_reduce_acc_binding(&acc_name, &initial_name, initial_shape);
        Some(format!(
            "(({}, {}) => {{ {} {} for (let {} = 0; {} < {}.length; {} += 1) {{ const {} = {}[{}]; {} }} return {}; }})({}, {})",
            collection_name,
            initial_name,
            items_binding,
            acc_binding,
            index_name,
            index_name,
            items_name,
            index_name,
            item_name,
            items_name,
            index_name,
            body.join(" "),
            acc_name,
            collection,
            initial
        ))
    }

    fn emit_helper_append_reduce(&mut self, args: &[Expr], indexed: bool) -> Option<String> {
        let reducer = InlineReducer::parse(&args[2], indexed)?;
        let [body] = reducer.body else {
            return None;
        };
        let ExprKind::List(call_items) = &body.kind else {
            return None;
        };
        let (helper_head, helper_args) = call_items.split_first()?;
        let ExprKind::Symbol(helper_name) = &helper_head.kind else {
            return None;
        };
        if self.direct_call_replacements.contains_key(helper_name) {
            return None;
        }
        let helper = self.function_defs.get(helper_name)?.clone();
        if helper.params.len() != helper_args.len() || helper.params.is_empty() {
            return None;
        }
        if !matches_symbol(helper_args.first()?, reducer.acc) {
            return None;
        }
        let [helper_body] = helper.body.as_slice() else {
            return None;
        };
        let helper_acc = helper.params.first()?;
        let append_body = AppendHelperBody::parse(helper_body, helper_acc)?;

        let collection = self.emit_expr(&args[0]);
        let initial = self.emit_expr(&args[1]);
        let collection_shape = self.expr_collection_shape(&args[0]);
        let initial_shape = self.expr_collection_shape(&args[1]);
        let collection_name = self.next_temp("__closkell_reduce_collection");
        let initial_name = self.next_temp("__closkell_reduce_initial");
        let items_name = self.next_temp("__closkell_reduce_items");
        let acc_name = self.next_temp("__closkell_reduce_acc");
        let index_name = self.next_temp("__closkell_reduce_index");
        let item_name = self.next_temp("__closkell_reduce_item");

        let mut body =
            self.emit_reduce_iteration_bindings(&reducer, &acc_name, &item_name, &index_name);
        let helper_params = helper
            .params
            .iter()
            .map(|param| sanitize_identifier(param))
            .collect::<Vec<_>>()
            .join(", ");
        let helper_call_args = helper_args
            .iter()
            .map(|arg| self.emit_expr(arg))
            .collect::<Vec<_>>()
            .join(", ");
        let helper_statements =
            self.emit_append_helper_statements(&append_body, helper_acc, initial_shape);
        body.push(format!(
            "(({}) => {{ {} }})({});",
            helper_params, helper_statements, helper_call_args
        ));

        let items_binding =
            self.emit_reduce_items_binding(&items_name, &collection_name, collection_shape);
        let acc_binding = self.emit_reduce_acc_binding(&acc_name, &initial_name, initial_shape);
        Some(format!(
            "(({}, {}) => {{ {} {} for (let {} = 0; {} < {}.length; {} += 1) {{ const {} = {}[{}]; {} }} return {}; }})({}, {})",
            collection_name,
            initial_name,
            items_binding,
            acc_binding,
            index_name,
            index_name,
            items_name,
            index_name,
            item_name,
            items_name,
            index_name,
            body.join(" "),
            acc_name,
            collection,
            initial
        ))
    }

    fn emit_inline_reduce(&mut self, args: &[Expr], indexed: bool) -> Option<String> {
        let reducer = InlineReducer::parse(&args[2], indexed)?;
        let collection = self.emit_expr(&args[0]);
        let initial = self.emit_expr(&args[1]);
        let collection_shape = self.expr_collection_shape(&args[0]);
        let collection_name = self.next_temp("__closkell_reduce_collection");
        let initial_name = self.next_temp("__closkell_reduce_initial");
        let items_name = self.next_temp("__closkell_reduce_items");
        let acc_name = self.next_temp("__closkell_reduce_acc");
        let index_name = self.next_temp("__closkell_reduce_index");
        let item_name = self.next_temp("__closkell_reduce_item");

        let mut body =
            self.emit_reduce_iteration_bindings(&reducer, &acc_name, &item_name, &index_name);
        body.push(format!("{} = {};", acc_name, self.emit_do(reducer.body)));

        let items_binding =
            self.emit_reduce_items_binding(&items_name, &collection_name, collection_shape);
        Some(format!(
            "(({}, {}) => {{ {} let {} = {}; for (let {} = 0; {} < {}.length; {} += 1) {{ const {} = {}[{}]; {} }} return {}; }})({}, {})",
            collection_name,
            initial_name,
            items_binding,
            acc_name,
            initial_name,
            index_name,
            index_name,
            items_name,
            index_name,
            item_name,
            items_name,
            index_name,
            body.join(" "),
            acc_name,
            collection,
            initial
        ))
    }

    fn emit_reduce_iteration_bindings(
        &self,
        reducer: &InlineReducer<'_>,
        acc_name: &str,
        item_name: &str,
        index_name: &str,
    ) -> Vec<String> {
        let mut body = Vec::new();
        if reducer.acc != "_" {
            body.push(format!(
                "const {} = {};",
                sanitize_identifier(reducer.acc),
                acc_name
            ));
        }
        if reducer.item != "_" {
            body.push(format!(
                "const {} = {};",
                sanitize_identifier(reducer.item),
                item_name
            ));
        }
        if let Some(index) = reducer.index {
            if index != "_" {
                body.push(format!(
                    "const {} = {};",
                    sanitize_identifier(index),
                    index_name
                ));
            }
        }
        body
    }

    fn emit_reduce_items_binding(
        &self,
        items_name: &str,
        collection_name: &str,
        shape: CollectionShape,
    ) -> String {
        match shape {
            CollectionShape::VectorLike => {
                format!("const {} = {};", items_name, collection_name)
            }
            CollectionShape::Set | CollectionShape::Map | CollectionShape::String => {
                format!("const {} = Array.from({});", items_name, collection_name)
            }
            CollectionShape::Unknown => format!(
                "const {} = Array.isArray({}) ? {} : Array.from({});",
                items_name, collection_name, collection_name, collection_name
            ),
        }
    }

    fn emit_reduce_acc_binding(
        &self,
        acc_name: &str,
        initial_name: &str,
        shape: CollectionShape,
    ) -> String {
        match shape {
            CollectionShape::VectorLike => {
                format!("const {} = {}.slice();", acc_name, initial_name)
            }
            CollectionShape::Set => format!("const {} = new Set({});", acc_name, initial_name),
            _ => format!(
                "const {} = {} instanceof Set ? new Set({}) : Array.from({});",
                acc_name, initial_name, initial_name, initial_name
            ),
        }
    }

    fn emit_append_helper_statements(
        &mut self,
        append_body: &AppendHelperBody<'_>,
        acc: &str,
        target_shape: CollectionShape,
    ) -> String {
        let mut append_statements = Vec::new();
        self.push_append_statements(
            &mut append_statements,
            &sanitize_identifier(acc),
            &append_body.bindings,
            append_body.items,
            target_shape,
        );
        let append_code = append_statements.join(" ");
        match append_body.condition {
            None => append_code,
            Some(AppendCondition::When(condition)) => {
                format!("if ({}) {{ {} }}", self.emit_expr(condition), append_code)
            }
            Some(AppendCondition::Unless(condition)) => {
                format!(
                    "if (!({})) {{ {} }}",
                    self.emit_expr(condition),
                    append_code
                )
            }
        }
    }

    fn push_append_statements(
        &mut self,
        body: &mut Vec<String>,
        acc_name: &str,
        bindings: &[(&Expr, &Expr)],
        items: &[Expr],
        target_shape: CollectionShape,
    ) {
        for (binding, value) in bindings {
            match &binding.kind {
                ExprKind::Symbol(name) if name == "_" => {
                    body.push(format!("{};", self.emit_expr(value)));
                }
                ExprKind::Symbol(name) => {
                    body.push(format!(
                        "const {} = {};",
                        sanitize_identifier(name),
                        self.emit_expr(value)
                    ));
                }
                _ => {
                    let value_name = self.next_temp("__closkell_reduce_let");
                    body.push(format!("const {} = {};", value_name, self.emit_expr(value)));
                    let value_type = self.expr_type(value).map(str::to_string);
                    self.emit_pattern_assignment_statement(
                        binding,
                        &value_name,
                        value_type.as_deref(),
                        "reduce append let pattern did not match",
                        body,
                    );
                }
            }
        }

        self.push_append_items(body, acc_name, items, target_shape);
    }

    fn push_append_items(
        &mut self,
        body: &mut Vec<String>,
        acc_name: &str,
        items: &[Expr],
        target_shape: CollectionShape,
    ) {
        if items.is_empty() {
            return;
        }
        match target_shape {
            CollectionShape::VectorLike => {
                let append_items = items
                    .iter()
                    .map(|item| self.emit_expr(item))
                    .collect::<Vec<_>>()
                    .join(", ");
                body.push(format!("{}.push({});", acc_name, append_items));
            }
            CollectionShape::Set => {
                for item in items {
                    body.push(format!("{}.add({});", acc_name, self.emit_expr(item)));
                }
            }
            _ => {
                let append_name = self.next_temp("__closkell_append_items");
                let append_value_name = self.next_temp("__closkell_append_value");
                let append_items = items
                    .iter()
                    .map(|item| self.emit_expr(item))
                    .collect::<Vec<_>>()
                    .join(", ");
                body.push(format!("const {} = [{}];", append_name, append_items));
                body.push(format!(
            "if ({} instanceof Set) {{ for (const {} of {}) {}.add({}); }} else {{ {}.push(...{}); }}",
            acc_name,
            append_value_name,
            append_name,
            acc_name,
            append_value_name,
            acc_name,
            append_name
        ));
            }
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

    fn emit_split(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "{}.split({})",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_join(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "{}.join({})",
            parenthesize_member_base(self.emit_expr(&args[0])),
            self.emit_expr(&args[1])
        )
    }

    fn emit_string_predicate_method(&mut self, method: &str, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "{}.{}({})",
            parenthesize_member_base(self.emit_expr(&args[0])),
            method,
            self.emit_expr(&args[1])
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
        let collection = self.emit_expr(&args[0]);
        let value = self.emit_expr(&args[1]);
        if self.expr_is_set(&args[0]) || self.expr_is_map(&args[0]) {
            return format!("{}.has({})", parenthesize_member_base(collection), value);
        }
        if self.expr_is_vector_like(&args[0])
            || self.expr_type(&args[0]).is_some_and(|ty| ty == "String")
        {
            return format!(
                "{}.includes({})",
                parenthesize_member_base(collection),
                value
            );
        }
        if self.expr_is_record(&args[0]) {
            return format!(
                "Object.prototype.hasOwnProperty.call({}, {})",
                collection, value
            );
        }
        format!(
            "((__collection, __value) => {{ if (__collection instanceof Set || __collection instanceof Map) return __collection.has(__value); if (Array.isArray(__collection) || typeof __collection === \"string\") return __collection.includes(__value); return __collection != null && Object.prototype.hasOwnProperty.call(__collection, __value); }})({}, {})",
            collection, value
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

    fn emit_json_parse_result(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }

        format!(
            "((__text) => {{ try {{ return {{ ok: true, value: JSON.parse(__text) }}; }} catch (__error) {{ return {{ ok: false, error: __error?.message ?? String(__error) }}; }} }})({})",
            self.emit_expr(&args[0])
        )
    }

    fn emit_zero_arg_decoder(&mut self, args: &[Expr], name: &str) -> String {
        self.needs_decoder_runtime = true;
        if !args.is_empty() {
            return "undefined".to_string();
        }
        format!("__closkellDecoder.{}", name)
    }

    fn emit_decoder_literal(&mut self, args: &[Expr]) -> String {
        self.needs_decoder_runtime = true;
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("__closkellDecoder.literal({})", self.emit_expr(&args[0]))
    }

    fn emit_decoder_unary(&mut self, args: &[Expr], name: &str) -> String {
        self.needs_decoder_runtime = true;
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("__closkellDecoder.{}({})", name, self.emit_expr(&args[0]))
    }

    fn emit_decoder_record(&mut self, args: &[Expr]) -> String {
        self.needs_decoder_runtime = true;
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("__closkellDecoder.record({})", self.emit_expr(&args[0]))
    }

    fn emit_decode(&mut self, args: &[Expr]) -> String {
        self.needs_decoder_runtime = true;
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "__closkellDecode({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_global_string_call(&mut self, callee: &str, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!("{}({})", callee, self.emit_expr(&args[0]))
    }

    fn emit_url_resolve(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }

        format!(
            "((__value, __base) => {{ try {{ return new URL(__value, __base).href; }} catch {{ return null; }} }})({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_url_part(&mut self, args: &[Expr], part: &str, strip_hash: bool) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }

        let hash_statement = if strip_hash {
            "__url.hash = \"\"; "
        } else {
            ""
        };
        format!(
            "((__value) => {{ try {{ const __url = new URL(__value); {}return __url.{}; }} catch {{ return null; }} }})({})",
            hash_statement,
            part,
            self.emit_expr(&args[0])
        )
    }

    fn emit_url_search_param(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "((__url, __name) => {{ try {{ return new URL(__url, globalThis.location?.href ?? undefined).searchParams.get(__name) ?? \"\"; }} catch {{ return \"\"; }} }})({}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_url_set_search_param(&mut self, args: &[Expr]) -> String {
        if args.len() != 3 {
            return "undefined".to_string();
        }
        format!(
            "((__url, __name, __value) => {{ try {{ const __next = new URL(__url, globalThis.location?.href ?? undefined); __next.searchParams.set(__name, String(__value)); return __next.href; }} catch {{ return String(__url); }} }})({}, {}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            self.emit_expr(&args[2])
        )
    }

    fn emit_url_set_deep_object_param(&mut self, args: &[Expr]) -> String {
        if args.len() != 3 {
            return "undefined".to_string();
        }
        format!(
            "((__url, __name, __jsonText) => {{ try {{ const __next = new URL(__url, globalThis.location?.href ?? undefined); const __parsed = JSON.parse(String(__jsonText || \"{{}}\")); if (!__parsed || typeof __parsed !== \"object\" || Array.isArray(__parsed)) return __next.href; for (const [__key, __value] of Object.entries(__parsed)) {{ if (__value !== undefined && __value !== null && String(__value) !== \"\") __next.searchParams.set(`${{__name}}[${{__key}}]`, String(__value)); }} return __next.href; }} catch {{ try {{ return new URL(__url, globalThis.location?.href ?? undefined).href; }} catch {{ return String(__url); }} }} }})({}, {}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            self.emit_expr(&args[2])
        )
    }

    fn emit_resolve_token_expiry(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            r#"((__expiresIn, __accessToken) => {{
  const __fromResponse = Number(__expiresIn) > 0 ? Date.now() + Number(__expiresIn) * 1000 : null;
  let __fromJwt = null;
  try {{
    const __payload = String(__accessToken).split(".")[1];
    if (__payload) {{
      const __normalized = __payload.replace(/-/g, "+").replace(/_/g, "/");
      const __decoded = JSON.parse(globalThis.atob(__normalized));
      if (typeof __decoded?.exp === "number") __fromJwt = __decoded.exp * 1000;
    }}
  }} catch {{}}
  if (__fromResponse && __fromJwt) return Math.min(__fromResponse, __fromJwt);
  return __fromResponse ?? __fromJwt;
}})({}, {})"#,
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_path_fill_params(&mut self, args: &[Expr]) -> String {
        if args.len() != 2 {
            return "undefined".to_string();
        }
        format!(
            "String({}).replace(/\\{{[^}}]+\\}}/g, encodeURIComponent(String({})))",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1])
        )
    }

    fn emit_path_fill_param(&mut self, args: &[Expr]) -> String {
        if args.len() != 3 {
            return "undefined".to_string();
        }
        format!(
            "((__path, __name, __value) => String(__path).replace(new RegExp(`\\\\{{${{String(__name).replace(/[.*+?^${{}}()|[\\]\\\\]/g, \"\\\\$&\")}}\\\\}}`, \"g\"), encodeURIComponent(String(__value))))({}, {}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            self.emit_expr(&args[2])
        )
    }

    fn emit_regex_capture(&mut self, args: &[Expr]) -> String {
        if !(2..=3).contains(&args.len()) {
            return "undefined".to_string();
        }
        let flags = args
            .get(2)
            .map(|flags| self.emit_expr(flags))
            .unwrap_or_else(|| "\"\"".to_string());
        format!(
            "((__text, __pattern, __flags) => {{ try {{ return new RegExp(__pattern, __flags).exec(String(__text))?.[1] ?? \"\"; }} catch {{ return \"\"; }} }})({}, {}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            flags
        )
    }

    fn emit_regex_capture_all(&mut self, args: &[Expr]) -> String {
        if !(2..=3).contains(&args.len()) {
            return "undefined".to_string();
        }
        let flags = args
            .get(2)
            .map(|flags| self.emit_expr(flags))
            .unwrap_or_else(|| "\"\"".to_string());
        format!(
            "((__text, __pattern, __flags) => {{ try {{ const __flagText = String(__flags || \"\"); const __allFlags = __flagText.includes(\"g\") ? __flagText : `${{__flagText}}g`; return Array.from(String(__text).matchAll(new RegExp(__pattern, __allFlags)), (__match) => __match.slice(1).map((__value) => __value ?? \"\")); }} catch {{ return []; }} }})({}, {}, {})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            flags
        )
    }

    fn emit_base64(&mut self, encode: bool, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }

        let body = if encode {
            "if (typeof btoa === \"function\") return btoa(__value); return Buffer.from(__value, \"utf8\").toString(\"base64\");"
        } else {
            "if (typeof atob === \"function\") return atob(__value); return Buffer.from(__value, \"base64\").toString(\"utf8\");"
        };
        format!(
            "((__value) => {{ try {{ {} }} catch {{ return \"\"; }} }})({})",
            body,
            self.emit_expr(&args[0])
        )
    }

    fn emit_fail(&mut self, args: &[Expr]) -> String {
        if args.len() != 1 {
            return "undefined".to_string();
        }
        format!(
            "(() => {{ throw new Error({}); }})()",
            self.emit_expr(&args[0])
        )
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
            .map(|arg| match &arg.kind {
                ExprKind::Map(entries) => self
                    .emit_record_literal_fields(entries)
                    .unwrap_or_else(|| format!("...{}", self.emit_expr(arg))),
                _ => format!("...{}", self.emit_expr(arg)),
            })
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        format!("({{ {} }})", args)
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

    fn emit_update_in(&mut self, expr: &Expr, args: &[Expr]) -> String {
        if args.len() < 3 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "update-in emission expects a record, path, updater, and optional arguments",
            ));
            return "undefined".to_string();
        }

        let extra_args = args
            .iter()
            .skip(3)
            .map(|arg| self.emit_expr(arg))
            .collect::<Vec<_>>();
        let extra_args = if extra_args.is_empty() {
            String::new()
        } else {
            format!(", {}", extra_args.join(", "))
        };
        format!(
            "((__value, __path, __updater, ...__args) => {{ const __keyName = (__key) => typeof __key === \"symbol\" ? (Symbol.keyFor(__key) ?? __key.description ?? String(__key)) : __key; const __get = (__current, __key) => {{ if (__current == null) return null; const __name = __keyName(__key); if (__current instanceof Map) return __current.has(__key) ? __current.get(__key) : (__current.has(__name) ? __current.get(__name) : null); return __current?.[__name] ?? null; }}; const __set = (__current, __key, __next) => {{ const __name = __keyName(__key); if (__current instanceof Map) {{ const __map = new Map(__current); __map.set(__map.has(__key) ? __key : (__map.has(__name) ? __name : __key), __next); return __map; }} if (Array.isArray(__current)) {{ const __array = __current.slice(); __array[__name] = __next; return __array; }} return {{ ...(__current ?? {{}}), [__name]: __next }}; }}; const __update = (__current, __index) => __index >= __path.length ? __updater(__current, ...__args) : __set(__current, __path[__index], __update(__get(__current, __path[__index]), __index + 1)); return Array.isArray(__path) ? __update(__value, 0) : __value; }})({}, {}, {}{})",
            self.emit_expr(&args[0]),
            self.emit_expr(&args[1]),
            self.emit_expr(&args[2]),
            extra_args
        )
    }

    fn emit_map_or_record(&mut self, entries: &[(Expr, Expr)]) -> String {
        self.record_map_runtime_effects(entries);

        if let Some(fields) = self.emit_record_literal_fields(entries) {
            return format!("{{ {} }}", fields);
        }

        let entries = entries
            .iter()
            .map(|(key, value)| format!("[{}, {}]", self.emit_expr(key), self.emit_expr(value)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("new Map([{}])", entries)
    }

    fn emit_record_literal_fields(&mut self, entries: &[(Expr, Expr)]) -> Option<String> {
        self.record_map_runtime_effects(entries);
        let mut fields = Vec::new();
        for (key, value) in entries {
            let key = object_key(key)?;
            fields.push(format!("{}: {}", key, self.emit_expr(value)));
        }
        Some(fields.join(", "))
    }

    fn record_map_runtime_effects(&mut self, entries: &[(Expr, Expr)]) {
        for (key, value) in entries {
            if object_key_name(key).is_some_and(|name| name == "kind") {
                if let Some(kind) = runtime_effect_kind_name(value) {
                    self.add_runtime_effect(kind);
                }
            }
        }
    }

    fn emit_function_body(&mut self, body: &[Expr]) -> String {
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
        self.local_types.push(BTreeMap::new());
        for pair in bindings.chunks(2) {
            let [pattern, value] = pair else {
                self.diagnostics.push(Diagnostic::error(
                    args[0].span,
                    "let emission requires complete binding pairs",
                ));
                continue;
            };
            let value_type = self.expr_type(value).map(str::to_string);
            self.emit_let_pattern_statement(pattern, value, &mut statements);
            if let Some(scope) = self.local_types.last_mut() {
                collect_pattern_type_bindings(pattern, value_type.as_deref(), scope);
            }
        }

        let tail = self.emit_tail_sequence(self_name, params, &args[1..]);
        self.local_types.pop();
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
        if let Some(emitted) = self.emit_tail_kind_switch_match(self_name, params, args) {
            return emitted;
        }

        let value_name = self.next_temp("__closkell_match");
        let value_type = self.expr_type(&args[0]).map(str::to_string);
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
            let compiled = self.emit_pattern(pattern, &value_name, value_type.as_deref());
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

    fn emit_tail_kind_switch_match(
        &mut self,
        self_name: &str,
        params: &[String],
        args: &[Expr],
    ) -> Option<TailEmission> {
        let plan = kind_match_plan(args)?;
        let value_name = self.next_temp("__closkell_match");
        let kind_name = self.next_temp("__closkell_kind");
        let value_type = self.expr_type(&args[0]).map(str::to_string);
        let kind_read = if value_type.as_deref().is_some_and(type_has_kind_property) {
            format!("{}.kind", value_name)
        } else {
            format!("{}?.kind", value_name)
        };
        let mut lines = vec![
            format!("const {} = {};", value_name, self.emit_expr(&args[0])),
            format!("const {} = {};", kind_name, kind_read),
            format!("switch ({}) {{", kind_name),
        ];
        let mut has_tail_call = false;

        for arm in &plan.arms {
            let bindings = self.emit_kind_pattern_bindings(arm.pattern, &value_name)?;
            let body = self.emit_tail_expr(self_name, params, arm.body);
            has_tail_call |= body.has_tail_call;
            lines.push(format!(
                "case {}: {{ {} {} }}",
                keyword_literal(&arm.kind),
                bindings,
                body.code
            ));
        }

        if let Some(default) = &plan.default {
            let compiled = self.emit_pattern(default.pattern, &value_name, value_type.as_deref());
            if compiled.test != "true" {
                return None;
            }
            let body = self.emit_tail_expr(self_name, params, default.body);
            has_tail_call |= body.has_tail_call;
            lines.push(format!(
                "default: {{ {} {} }}",
                compiled.bindings, body.code
            ));
        }

        lines.push("}".to_string());
        lines.push("throw new Error(\"non-exhaustive match\");".to_string());
        Some(TailEmission {
            code: format!("{{ {} }}", lines.join(" ")),
            has_tail_call,
        })
    }

    fn next_temp(&mut self, prefix: &str) -> String {
        let id = self.next_temp_id;
        self.next_temp_id += 1;
        format!("{}_{}", prefix, id)
    }

    fn emit_pattern(
        &mut self,
        pattern: &Expr,
        value: &str,
        value_type: Option<&str>,
    ) -> CompiledPattern {
        match &pattern.kind {
            ExprKind::Symbol(name) if name == "_" => CompiledPattern::always(),
            ExprKind::Symbol(name) => CompiledPattern {
                test: "true".to_string(),
                bindings: format!("const {} = {};", sanitize_identifier(name), value),
            },
            ExprKind::List(items)
                if items.first().is_some_and(|head| matches_symbol(head, "as")) =>
            {
                self.emit_as_pattern(pattern, items, value, value_type)
            }
            ExprKind::List(items)
                if items
                    .first()
                    .and_then(symbol_name)
                    .is_some_and(is_data_constructor_pattern) =>
            {
                self.emit_data_constructor_pattern(pattern, items, value, value_type)
            }
            ExprKind::Keyword(name) => CompiledPattern {
                test: format!("{} === {}", value, keyword_literal(name)),
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
            ExprKind::Map(entries) => self.emit_record_pattern(entries, value, value_type),
            ExprKind::Vector(items) => self.emit_vector_pattern(items, value, value_type),
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

    fn emit_kind_pattern_bindings(&mut self, pattern: &Expr, value: &str) -> Option<String> {
        let info = kind_pattern_info(pattern)?;
        let mut bindings = Vec::new();
        if let Some(alias) = info.alias {
            if alias != "_" {
                bindings.push(format!("const {} = {};", sanitize_identifier(alias), value));
            }
        }
        for (key, field_pattern) in info.entries {
            let key = object_key_name(key)?;
            if key == "kind" {
                continue;
            }
            let field = format!("{}{}", value, property_access(&key));
            bindings.push(simple_field_pattern_binding(field_pattern, &field)?);
        }
        Some(
            bindings
                .into_iter()
                .filter(|binding| !binding.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    fn emit_data_constructor_pattern(
        &mut self,
        pattern: &Expr,
        items: &[Expr],
        value: &str,
        value_type: Option<&str>,
    ) -> CompiledPattern {
        let Some(name) = items.first().and_then(symbol_name) else {
            return CompiledPattern {
                test: "false".to_string(),
                bindings: String::new(),
            };
        };
        if name == "list" {
            return self.emit_vector_pattern(&items[1..], value, value_type);
        }
        if name == "cons" {
            return self.emit_cons_pattern(pattern, &items[1..], value, value_type);
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
                let inner_type = value_type.and_then(type_option_inner);
                let inner = self.emit_pattern(&items[1], value, inner_type);
                CompiledPattern {
                    test: join_pattern_tests(vec![format!("{} != null", value), inner.test]),
                    bindings: inner.bindings,
                }
            }
            "ok" | "err" => {
                let expected = if name == "ok" { "true" } else { "false" };
                let field = if name == "ok" { "value" } else { "error" };
                let field_value = format!("{}{}", value, property_access(field));
                let inner_type = value_type
                    .and_then(type_result_parts)
                    .map(|(ok, err)| if name == "ok" { ok } else { err });
                let inner = self.emit_pattern(&items[1], &field_value, inner_type);
                let mut tests = Vec::new();
                if !value_type.is_some_and(|ty| type_result_parts(ty).is_some()) {
                    tests.push(format!(
                        "{} !== null && typeof {} === \"object\"",
                        value, value
                    ));
                }
                tests.push(format!("{}.ok === {}", value, expected));
                tests.push(inner.test);
                CompiledPattern {
                    test: join_pattern_tests(tests),
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
        value_type: Option<&str>,
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
        let head_type = value_type.and_then(type_sequence_element);
        let head = self.emit_pattern(&items[0], &head_value, head_type);
        let tail = self.emit_pattern(&items[1], &tail_value, value_type);
        let bindings = [head.bindings, tail.bindings]
            .into_iter()
            .filter(|binding| !binding.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let mut tests = Vec::new();
        if !value_type.is_some_and(type_is_vector_like) {
            tests.push(format!("Array.isArray({})", value));
        }
        tests.push(format!("{}.length > 0", value));
        tests.push(head.test);
        tests.push(tail.test);
        CompiledPattern {
            test: join_pattern_tests(tests),
            bindings,
        }
    }

    fn emit_as_pattern(
        &mut self,
        pattern: &Expr,
        items: &[Expr],
        value: &str,
        value_type: Option<&str>,
    ) -> CompiledPattern {
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
        let inner = self.emit_pattern(&items[1], value, value_type);
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

    fn emit_record_pattern(
        &mut self,
        entries: &[(Expr, Expr)],
        value: &str,
        value_type: Option<&str>,
    ) -> CompiledPattern {
        let mut tests = Vec::new();
        if !value_type.is_some_and(type_is_record) {
            tests.push(format!(
                "{} !== null && typeof {} === \"object\"",
                value, value
            ));
        }
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
            let field_type = value_type.and_then(|ty| type_record_field_type(ty, &key));
            let compiled = self.emit_pattern(pattern, &field, field_type);
            tests.push(compiled.test);
            if !compiled.bindings.is_empty() {
                bindings.push(compiled.bindings);
            }
        }

        CompiledPattern {
            test: join_pattern_tests(tests),
            bindings: bindings.join(" "),
        }
    }

    fn emit_vector_pattern(
        &mut self,
        items: &[Expr],
        value: &str,
        value_type: Option<&str>,
    ) -> CompiledPattern {
        let mut tests = Vec::new();
        if !value_type.is_some_and(type_is_vector_like) {
            tests.push(format!("Array.isArray({})", value));
        }
        let tuple_len_matches = value_type
            .and_then(type_tuple_items)
            .is_some_and(|tuple_items| tuple_items.len() == items.len());
        if !tuple_len_matches {
            tests.push(format!("{}.length === {}", value, items.len()));
        }
        let mut bindings = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let field = format!("{}[{}]", value, index);
            let item_type = value_type.and_then(|ty| type_vector_item_type(ty, index));
            let compiled = self.emit_pattern(item, &field, item_type);
            tests.push(compiled.test);
            if !compiled.bindings.is_empty() {
                bindings.push(compiled.bindings);
            }
        }
        CompiledPattern {
            test: join_pattern_tests(tests),
            bindings: bindings.join(" "),
        }
    }

    fn emit_template_component(&mut self, root: &HtmlNode) -> String {
        self.emit_template_component_with_params(root, &ReadAliases::new(), &[])
    }

    fn emit_template_component_with_params(
        &mut self,
        root: &HtmlNode,
        read_aliases: &ReadAliases,
        params: &[String],
    ) -> String {
        self.needs_html_runtime = true;
        let template_id = self.next_template_id;
        self.next_template_id += 1;
        let mut template = TemplateEmitter {
            owner: self,
            template_id,
            svg_depth: 0,
            nodes: Vec::new(),
            slots: Vec::new(),
            create_lines: Vec::new(),
            read_aliases: read_aliases.clone(),
            params: params.to_vec(),
        };
        template.emit_node(root);
        let skeleton = build_template_skeleton(root, template.owner);
        let mut used_node_ids = BTreeSet::new();
        for slot in &template.slots {
            used_node_ids.insert(slot.node_id);
        }
        let mut node_id_remap = BTreeMap::new();
        for (next_id, old_id) in used_node_ids.iter().enumerate() {
            node_id_remap.insert(*old_id, next_id);
        }
        let skeleton_paths = used_node_ids
            .iter()
            .map(|old_id| {
                skeleton_path_token(
                    skeleton
                        .node_paths
                        .get(*old_id)
                        .expect("skeleton path should exist for emitted node"),
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        for slot in &mut template.slots {
            slot.node_id = *node_id_remap
                .get(&slot.node_id)
                .expect("slot node should be retained in compact node table");
        }
        let slot_kinds = template
            .slots
            .iter()
            .map(|slot| slot.kind.clone())
            .collect::<Vec<_>>();
        for kind in &slot_kinds {
            template.owner.mark_template_slot_runtime(kind);
        }
        let update_body = template.emit_update_body();

        let metadata_const = format!("__closkellTemplateMetadata{}", template_id);
        let metadata = template.metadata_expr();
        template
            .owner
            .template_metadata_consts
            .push(format!("const {} = {};\n", metadata_const, metadata));
        let skeleton_expr = format!(
            "{}(\"{}\", \"{}\", (i, d, c) => {{ {} }}, {})",
            CREATE_HTML_TEMPLATE_ALIAS,
            escape_js(&skeleton.html),
            escape_js(&skeleton_paths),
            update_body,
            metadata_const
        );
        template.owner.needs_template_skeleton = true;
        skeleton_expr
    }
}

struct TemplateEmitter<'a> {
    owner: &'a mut Emitter,
    template_id: usize,
    svg_depth: usize,
    nodes: Vec<String>,
    slots: Vec<TemplateSlot>,
    create_lines: Vec<String>,
    read_aliases: ReadAliases,
    params: Vec<String>,
}

struct CompiledPattern {
    test: String,
    bindings: String,
}

struct KindPatternInfo<'a> {
    kind: String,
    entries: &'a [(Expr, Expr)],
    alias: Option<&'a str>,
}

struct KindMatchArm<'a> {
    kind: String,
    pattern: &'a Expr,
    body: &'a Expr,
}

struct KindMatchDefault<'a> {
    pattern: &'a Expr,
    body: &'a Expr,
}

struct KindMatchPlan<'a> {
    arms: Vec<KindMatchArm<'a>>,
    default: Option<KindMatchDefault<'a>>,
}

#[derive(Clone, Debug)]
struct FunctionDef {
    params: Vec<String>,
    body: Vec<Expr>,
}

struct AppendReducer<'a> {
    acc: &'a str,
    item: &'a str,
    index: Option<&'a str>,
    bindings: Vec<(&'a Expr, &'a Expr)>,
    items: &'a [Expr],
}

impl<'a> AppendReducer<'a> {
    fn parse(expr: &'a Expr, indexed: bool) -> Option<Self> {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        let [head, params, body] = items.as_slice() else {
            return None;
        };
        if !matches_symbol(head, "fn") {
            return None;
        }
        let ExprKind::Vector(params) = &params.kind else {
            return None;
        };
        let expected_params = if indexed { 3 } else { 2 };
        if params.len() != expected_params {
            return None;
        }
        let acc = symbol_name(&params[0])?;
        let item = symbol_name(&params[1])?;
        let index = if indexed {
            Some(symbol_name(&params[2])?)
        } else {
            None
        };
        let (bindings, items) = append_conj_parts(body, acc)?;
        Some(Self {
            acc,
            item,
            index,
            bindings,
            items,
        })
    }
}

struct InlineReducer<'a> {
    acc: &'a str,
    item: &'a str,
    index: Option<&'a str>,
    body: &'a [Expr],
}

impl<'a> InlineReducer<'a> {
    fn parse(expr: &'a Expr, indexed: bool) -> Option<Self> {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        let Some((head, args)) = items.split_first() else {
            return None;
        };
        if !matches_symbol(head, "fn") {
            return None;
        }
        let (params, body) = args.split_first()?;
        let ExprKind::Vector(params) = &params.kind else {
            return None;
        };
        let expected_params = if indexed { 3 } else { 2 };
        if params.len() != expected_params || body.is_empty() {
            return None;
        }
        let acc = symbol_name(&params[0])?;
        let item = symbol_name(&params[1])?;
        let index = if indexed {
            Some(symbol_name(&params[2])?)
        } else {
            None
        };
        Some(Self {
            acc,
            item,
            index,
            body,
        })
    }
}

struct AppendHelperBody<'a> {
    condition: Option<AppendCondition<'a>>,
    bindings: Vec<(&'a Expr, &'a Expr)>,
    items: &'a [Expr],
}

enum AppendCondition<'a> {
    When(&'a Expr),
    Unless(&'a Expr),
}

impl<'a> AppendHelperBody<'a> {
    fn parse(expr: &'a Expr, acc: &str) -> Option<Self> {
        if let Some((bindings, items)) = append_conj_parts(expr, acc) {
            return Some(Self {
                condition: None,
                bindings,
                items,
            });
        }

        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        let [head, condition, then_branch, else_branch] = items.as_slice() else {
            return None;
        };
        if !matches_symbol(head, "if") {
            return None;
        }

        if matches_symbol(then_branch, acc) {
            let (bindings, items) = append_conj_parts(else_branch, acc)?;
            return Some(Self {
                condition: Some(AppendCondition::Unless(condition)),
                bindings,
                items,
            });
        }
        if matches_symbol(else_branch, acc) {
            let (bindings, items) = append_conj_parts(then_branch, acc)?;
            return Some(Self {
                condition: Some(AppendCondition::When(condition)),
                bindings,
                items,
            });
        }

        None
    }
}

fn append_conj_parts<'a>(
    expr: &'a Expr,
    acc: &str,
) -> Option<(Vec<(&'a Expr, &'a Expr)>, &'a [Expr])> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    let Some((head, args)) = items.split_first() else {
        return None;
    };
    if matches_symbol(head, "conj") {
        let Some((target, append_items)) = args.split_first() else {
            return None;
        };
        if !matches_symbol(target, acc) || append_items.is_empty() {
            return None;
        }
        return Some((Vec::new(), append_items));
    }
    if matches_symbol(head, "let") {
        let [bindings_expr, body] = args else {
            return None;
        };
        let ExprKind::Vector(bindings) = &bindings_expr.kind else {
            return None;
        };
        if bindings.len() % 2 != 0 {
            return None;
        }
        let (mut inner_bindings, items) = append_conj_parts(body, acc)?;
        let mut all_bindings = bindings
            .chunks(2)
            .filter_map(|pair| match pair {
                [binding, value] => Some((binding, value)),
                _ => None,
            })
            .collect::<Vec<_>>();
        all_bindings.append(&mut inner_bindings);
        return Some((all_bindings, items));
    }
    None
}

impl CompiledPattern {
    fn always() -> Self {
        Self {
            test: "true".to_string(),
            bindings: String::new(),
        }
    }
}

fn kind_match_plan(args: &[Expr]) -> Option<KindMatchPlan<'_>> {
    let mut arms = Vec::new();
    let mut default = None;
    let mut seen = BTreeSet::new();
    let pair_count = args[1..].len() / 2;

    for (index, arm) in args[1..].chunks(2).enumerate() {
        let [pattern, body] = arm else {
            return None;
        };
        if let Some(info) = kind_pattern_info(pattern) {
            if default.is_some() || !seen.insert(info.kind.clone()) {
                return None;
            }
            arms.push(KindMatchArm {
                kind: info.kind,
                pattern,
                body,
            });
            continue;
        }
        if index + 1 == pair_count && is_always_pattern(pattern) {
            default = Some(KindMatchDefault { pattern, body });
            continue;
        }
        return None;
    }

    if arms.is_empty() {
        return None;
    }
    Some(KindMatchPlan { arms, default })
}

fn kind_pattern_info(pattern: &Expr) -> Option<KindPatternInfo<'_>> {
    match &pattern.kind {
        ExprKind::Map(entries) => Some(KindPatternInfo {
            kind: record_kind_pattern(entries)?,
            entries: entries.as_slice(),
            alias: None,
        }),
        ExprKind::List(items) if items.len() == 3 && matches_symbol(&items[0], "as") => {
            let alias = symbol_name(&items[2])?;
            let mut info = kind_pattern_info(&items[1])?;
            info.alias = Some(alias);
            Some(info)
        }
        _ => None,
    }
}

fn record_kind_pattern(entries: &[(Expr, Expr)]) -> Option<String> {
    for (key, pattern) in entries {
        if object_key_name(key).is_some_and(|name| name == "kind") {
            return literal_kind_pattern(pattern);
        }
    }
    None
}

fn literal_kind_pattern(pattern: &Expr) -> Option<String> {
    match &pattern.kind {
        ExprKind::Keyword(name) | ExprKind::String(name) => Some(name.clone()),
        _ => None,
    }
}

fn is_always_pattern(pattern: &Expr) -> bool {
    matches!(&pattern.kind, ExprKind::Symbol(_))
}

fn simple_field_pattern_binding(pattern: &Expr, value: &str) -> Option<String> {
    match &pattern.kind {
        ExprKind::Symbol(name) if name == "_" => Some(String::new()),
        ExprKind::Symbol(name) => Some(format!("const {} = {};", sanitize_identifier(name), value)),
        ExprKind::List(items) if items.len() == 3 && matches_symbol(&items[0], "as") => {
            let inner = simple_field_pattern_binding(&items[1], value)?;
            let alias = symbol_name(&items[2])?;
            let alias = if alias == "_" {
                String::new()
            } else {
                format!("const {} = {};", sanitize_identifier(alias), value)
            };
            Some(
                [inner, alias]
                    .into_iter()
                    .filter(|binding| !binding.is_empty())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        }
        _ => None,
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
    Attr {
        name: String,
        setter: TemplateAttrSetter,
    },
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
        stable_item_update: bool,
    },
}

fn template_slot_metadata_expr(slot: &TemplateSlot) -> String {
    let reads = slot
        .reads
        .iter()
        .map(|read| format!("\"{}\"", escape_js(read)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{ id: {}, kind: {}, reads: [{}] }}",
        slot.id,
        template_slot_kind_metadata_expr(&slot.kind),
        reads
    )
}

fn template_slot_kind_metadata_expr(kind: &TemplateSlotKind) -> String {
    match kind {
        TemplateSlotKind::Text => "\"text\"".to_string(),
        TemplateSlotKind::Attr { name, .. } => {
            format!("{{ attr: \"{}\" }}", escape_js(name))
        }
        TemplateSlotKind::Event(event) => {
            format!("{{ event: \"{}\" }}", escape_js(event))
        }
        TemplateSlotKind::Ref => "\"ref\"".to_string(),
        TemplateSlotKind::Conditional { .. } => "{ conditional: true }".to_string(),
        TemplateSlotKind::Component { name, .. } => {
            format!("{{ component: \"{}\" }}", escape_js(name))
        }
        TemplateSlotKind::KeyedList { item, index, .. } => {
            let index = index
                .as_ref()
                .map(|index| format!(", index: \"{}\"", escape_js(index)))
                .unwrap_or_default();
            format!("{{ keyed: \"{}\"{} }}", escape_js(item), index)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TemplateAttrSetter {
    Simple,
    Text,
    NullableText,
    TextProperty,
    NullableTextProperty,
    Presence,
    BooleanProperty,
    ClassName,
    Class,
    Style,
    StyleRecord,
}

struct TemplateSkeleton {
    html: String,
    node_paths: Vec<Vec<usize>>,
}

fn build_template_skeleton(root: &HtmlNode, owner: &Emitter) -> TemplateSkeleton {
    let mut skeleton = TemplateSkeleton {
        html: String::new(),
        node_paths: Vec::new(),
    };
    let _ = emit_skeleton_node(root, owner, &mut Vec::new(), &mut skeleton);
    skeleton
}

fn emit_skeleton_node(
    node: &HtmlNode,
    owner: &Emitter,
    path: &mut Vec<usize>,
    skeleton: &mut TemplateSkeleton,
) -> usize {
    match node {
        HtmlNode::Element(element) => {
            skeleton.node_paths.push(path.clone());
            skeleton.html.push('<');
            skeleton.html.push_str(&element.tag);
            for attr in &element.attrs {
                match &attr.value {
                    HtmlAttrValue::Bool(true) if attr.name != "ref" => {
                        skeleton.html.push(' ');
                        skeleton.html.push_str(&attr.name);
                    }
                    HtmlAttrValue::Static(value) if attr.name != "ref" => {
                        skeleton.html.push(' ');
                        skeleton.html.push_str(&attr.name);
                        skeleton.html.push_str("=\"");
                        skeleton.html.push_str(&escape_html_attr(value));
                        skeleton.html.push('"');
                    }
                    HtmlAttrValue::Dynamic { expr, .. }
                        if attr.name != "ref" && owner.static_bool(expr) == Some(true) =>
                    {
                        skeleton.html.push(' ');
                        skeleton.html.push_str(&attr.name);
                    }
                    _ => {}
                }
            }
            skeleton.html.push('>');

            let mut child_index = 0usize;
            for child in &element.children {
                if is_indentation_text_node(child) {
                    continue;
                }
                path.push(child_index);
                let emitted_nodes = emit_skeleton_node(child, owner, path, skeleton);
                path.pop();
                child_index += emitted_nodes;
            }

            skeleton.html.push_str("</");
            skeleton.html.push_str(&element.tag);
            skeleton.html.push('>');
            1
        }
        HtmlNode::Text { text, .. } => {
            skeleton.node_paths.push(path.clone());
            skeleton.html.push_str(&escape_html_text(text));
            1
        }
        HtmlNode::Expr { expr, .. } => {
            if template_expr_uses_structural_marker(expr, owner) {
                skeleton.node_paths.push(path.clone());
                skeleton.html.push_str("<!---->");
                1
            } else {
                let mut text_path = path.clone();
                if let Some(last) = text_path.last_mut() {
                    *last += 1;
                }
                skeleton.node_paths.push(text_path);
                skeleton.html.push_str("<!----> <!---->");
                3
            }
        }
    }
}

fn template_expr_uses_structural_marker(expr: &Expr, owner: &Emitter) -> bool {
    let is_typed_component_call = |expr: &Expr| owner.expr_is_typed_component_call(expr);
    ForSpec::parse(expr).is_some()
        || IfSpec::parse(expr, &owner.component_fns, &is_typed_component_call).is_some()
        || ComponentSpec::parse(expr, &owner.component_fns, is_typed_component_call(expr)).is_some()
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_attr(value: &str) -> String {
    escape_html_text(value).replace('"', "&quot;")
}

fn skeleton_path_token(path: &[usize]) -> String {
    if path.is_empty() {
        return "-".to_string();
    }
    let mut encoded = String::new();
    for index in path {
        encoded.push(
            char::from_digit(*index as u32, 36).expect("template child index should fit in base36"),
        );
    }
    encoded
}

impl TemplateEmitter<'_> {
    fn emit_node(&mut self, node: &HtmlNode) -> String {
        match node {
            HtmlNode::Element(element) => self.emit_element(element),
            HtmlNode::Text { text, .. } => {
                let var = self.next_node_var();
                self.create_lines.push(format!(
                    "const {} = {}(\"{}\");",
                    var,
                    CREATE_TEXT_ALIAS,
                    escape_js(text)
                ));
                var
            }
            HtmlNode::Expr { expr, .. } => {
                if let Some(spec) = ForSpec::parse(expr) {
                    return self.emit_keyed_for(expr, spec);
                }
                let is_typed_component_call =
                    |expr: &Expr| self.owner.expr_is_typed_component_call(expr);
                if let Some(spec) =
                    IfSpec::parse(expr, &self.owner.component_fns, &is_typed_component_call)
                {
                    return self.emit_conditional(expr, spec);
                }
                if let Some(spec) = ComponentSpec::parse(
                    expr,
                    &self.owner.component_fns,
                    is_typed_component_call(expr),
                ) {
                    return self.emit_component_call(expr, spec);
                }

                let var = self.next_node_var();
                self.create_lines
                    .push(format!("const {} = {}(\"\");", var, CREATE_TEXT_ALIAS));
                self.push_slot(self.node_id_for_var(&var), TemplateSlotKind::Text, expr);
                var
            }
        }
    }

    fn emit_element(&mut self, element: &HtmlElement) -> String {
        let var = self.next_node_var();
        let node_id = self.node_id_for_var(&var);
        let is_svg_element = element.tag.eq_ignore_ascii_case("svg");
        let in_svg_tree = self.svg_depth > 0 || is_svg_element;
        if in_svg_tree {
            self.create_lines.push(format!(
                "const {} = {}(\"{}\");",
                var,
                CREATE_SVG_ELEMENT_ALIAS,
                escape_js(&element.tag)
            ));
        } else {
            self.create_lines.push(format!(
                "const {} = {}(\"{}\");",
                var,
                CREATE_ELEMENT_ALIAS,
                escape_js(&element.tag)
            ));
        }

        let statically_disabled = self.element_statically_disabled(element);
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
                    "{}({}, \"{}\", \"\");",
                    SET_STATIC_ATTR_ALIAS,
                    var,
                    escape_js(&attr.name)
                )),
                HtmlAttrValue::Bool(false) => {}
                HtmlAttrValue::Static(value) => self.create_lines.push(format!(
                    "{}({}, \"{}\", \"{}\");",
                    SET_STATIC_ATTR_ALIAS,
                    var,
                    escape_js(&attr.name),
                    escape_js(value)
                )),
                HtmlAttrValue::Dynamic { expr, .. } => {
                    if let Some(value) = self.owner.static_bool(expr) {
                        if value {
                            self.create_lines.push(format!(
                                "{}({}, \"{}\", \"\");",
                                SET_STATIC_ATTR_ALIAS,
                                var,
                                escape_js(&attr.name)
                            ));
                        }
                        continue;
                    }
                    if statically_disabled && attr.name.starts_with("on:") {
                        continue;
                    }
                    let kind = if let Some(event) = attr.name.strip_prefix("on:") {
                        TemplateSlotKind::Event(event.to_string())
                    } else {
                        TemplateSlotKind::Attr {
                            setter: self.attr_setter(&attr.name, expr, in_svg_tree),
                            name: attr.name.clone(),
                        }
                    };
                    self.push_slot(node_id, kind, expr);
                }
            }
        }

        if in_svg_tree {
            self.svg_depth += 1;
        }
        for child in &element.children {
            if is_indentation_text_node(child) {
                continue;
            }
            let child_var = self.emit_node(child);
            self.create_lines
                .push(format!("{}({}, {});", APPEND_CHILD_ALIAS, var, child_var));
        }
        if in_svg_tree {
            self.svg_depth -= 1;
        }
        var
    }

    fn element_statically_disabled(&self, element: &HtmlElement) -> bool {
        element.attrs.iter().any(|attr| {
            attr.name == "disabled"
                && match &attr.value {
                    HtmlAttrValue::Bool(value) => *value,
                    HtmlAttrValue::Static(_) => true,
                    HtmlAttrValue::Dynamic { expr, .. } => {
                        self.owner.static_bool(expr).is_some_and(|value| value)
                    }
                }
        })
    }

    fn emit_keyed_for(&mut self, expr: &Expr, spec: ForSpec<'_>) -> String {
        let var = self.next_node_var();
        self.create_lines
            .push(format!("const {} = {}(\"\");", var, CREATE_TEXT_ALIAS));

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
        let stable_item_update =
            keyed_item_update_reads(&spec, &self.owner.component_fns, &self.owner.read_summaries)
                .into_iter()
                .all(|read| keyed_item_local_read(&read, spec.item, spec.index));
        let reads = self.expand_reads(collect_keyed_reads(
            &spec,
            &self.owner.component_fns,
            &self.owner.read_summaries,
        ));
        let arity = if index.is_some() { 2 } else { 1 };
        let bind = format!(
            "({}) => {{ {} = {};{} }}",
            update_params, item, item_update_param, index_update
        );
        let render = format!(
            "({}) => {{ let {} = {};{} const __closkellItemComponent = {}; return {}(__closkellItemComponent, {}, {}); }}",
            render_params,
            item,
            item_param,
            index_binding,
            component_expr,
            BIND_COMPONENT_ALIAS,
            arity,
            bind
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
                stable_item_update,
            },
            expr: format_expr(expr),
            reads,
        });
        var
    }

    fn emit_conditional(&mut self, expr: &Expr, spec: IfSpec<'_>) -> String {
        let var = self.next_node_var();
        self.create_lines
            .push(format!("const {} = {}(\"\");", var, CREATE_TEXT_ALIAS));

        let condition = self.owner.emit_expr(spec.condition);
        let then_component = self.emit_branch_component(&spec.then_branch);
        let else_component = self.emit_branch_component(&spec.else_branch);
        let render_then = format!("() => {}", then_component);
        let render_else = format!("() => {}", else_component);
        let reads = self.expand_reads(collect_conditional_reads(
            &spec,
            &self.owner.component_fns,
            &self.owner.read_summaries,
        ));
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
            reads,
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
            "(() => {{ const {component} = {render}; return {{ mount(parent, dispatch) {{ return {component}.mount(parent, dispatch); }}, update(dispatch, updateContext = null) {{ return {component}.update({update_args}dispatch, updateContext); }}, dispose() {{ {component}.dispose?.(); }}, get root() {{ return {component}.root; }}, get definition() {{ return {component}.definition; }} }}; }})()"
        )
    }

    fn emit_inline_conditional_component(&mut self, spec: &IfSpec<'_>) -> String {
        let branch = self.owner.next_temp("__closkellBranch");
        let component = self.owner.next_temp("__closkellComponent");
        let placeholder = self.owner.next_temp("__closkellPlaceholder");
        let fresh = self.owner.next_temp("__closkellFresh");
        let condition = self.owner.emit_expr(spec.condition);
        let then_component = self.emit_branch_component(&spec.then_branch);
        let else_component = self.emit_branch_component(&spec.else_branch);

        format!(
            "(() => {{ let {branch} = null; let {component} = null; let {fresh} = false; const {placeholder} = {text}(\"\"); const __closkellDispose = () => {{ if ({component}?.root?.parentNode) {component}.root.parentNode.removeChild({component}.root); {component}?.dispose?.(); }}; const __closkellForceContext = (context) => context ? {{ ...context, force: true }} : null; return {{ update(dispatch, updateContext = null) {{ const __closkellCondition = {condition}; const __closkellNextBranch = __closkellCondition ? \"then\" : \"else\"; if ({branch} !== __closkellNextBranch) {{ __closkellDispose(); {component} = __closkellCondition ? {then_component} : {else_component}; {branch} = __closkellNextBranch; {fresh} = true; }} {component}?.update?.(dispatch, {fresh} ? __closkellForceContext(updateContext) : updateContext); {fresh} = false; return {component}?.root ?? {placeholder}; }}, get root() {{ return {component}?.root ?? {placeholder}; }}, dispose() {{ __closkellDispose(); {component} = null; {branch} = null; {fresh} = false; }} }}; }})()",
            text = CREATE_TEXT_ALIAS
        )
    }

    fn emit_component_call(&mut self, expr: &Expr, spec: ComponentSpec<'_>) -> String {
        let var = self.next_node_var();
        self.create_lines
            .push(format!("const {} = {}(\"\");", var, CREATE_TEXT_ALIAS));

        let render = self.owner.emit_expr(expr);
        let args = spec
            .args
            .iter()
            .map(|arg| self.owner.emit_expr(arg))
            .collect::<Vec<_>>()
            .join(", ");
        let reads = self.expand_reads(component_call_reads(&spec, &self.owner.read_summaries));
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
            reads,
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
        let reads = self.expr_reads(expr);
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

    fn expr_reads(&self, expr: &Expr) -> Vec<String> {
        self.expand_reads(collect_template_reads(expr, &self.owner.read_summaries))
    }

    fn expand_reads(&self, reads: Vec<String>) -> Vec<String> {
        expand_reads(reads, &self.read_aliases)
    }

    fn metadata_expr(&self) -> String {
        let slots = self
            .slots
            .iter()
            .map(template_slot_metadata_expr)
            .collect::<Vec<_>>()
            .join(", ");
        let params = self
            .params
            .iter()
            .map(|param| format!("\"{}\"", escape_js(param)))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{{ name: \"template{}\", params: [{}], slots: [{}] }}",
            self.template_id, params, slots
        )
    }

    fn attr_setter(&self, name: &str, expr: &Expr, in_svg_tree: bool) -> TemplateAttrSetter {
        let expr_type = self.owner.expr_type(expr);
        if let Some(ty) = expr_type {
            let (inner, nullable) = attr_type_inner(ty);
            if name == "class" && !in_svg_tree && is_text_attr_type(inner) {
                return if nullable {
                    TemplateAttrSetter::Class
                } else {
                    TemplateAttrSetter::ClassName
                };
            }
            if inner == "Bool" {
                return if !in_svg_tree && is_html_boolean_property_attr(name) {
                    TemplateAttrSetter::BooleanProperty
                } else {
                    TemplateAttrSetter::Presence
                };
            }
            if is_text_attr_type(inner) {
                return match (!in_svg_tree && is_html_text_property_attr(name), nullable) {
                    (true, true) => TemplateAttrSetter::NullableTextProperty,
                    (true, false) => TemplateAttrSetter::TextProperty,
                    (false, true) => TemplateAttrSetter::NullableText,
                    (false, false) => TemplateAttrSetter::Text,
                };
            }
        }
        match name {
            "style" if self.owner.expr_is_record(expr) => TemplateAttrSetter::StyleRecord,
            "style" => TemplateAttrSetter::Style,
            "class" if expr_type.is_some_and(is_simple_attr_value_type) => {
                TemplateAttrSetter::Simple
            }
            "class" => TemplateAttrSetter::Class,
            _ => TemplateAttrSetter::Simple,
        }
    }

    fn emit_update_body(&self) -> String {
        let mut lines = Vec::new();
        for slot in &self.slots {
            let update = match &slot.kind {
                TemplateSlotKind::Text => format!(
                    "{}(i, {}, i.nodes[{}], {}, c);",
                    SET_TEXT_ALIAS, slot.id, slot.node_id, slot.expr
                ),
                TemplateSlotKind::Attr { name, setter } => match setter {
                    TemplateAttrSetter::Simple => format!(
                        "{}(i, {}, i.nodes[{}], \"{}\", {}, c);",
                        SET_ATTR_ALIAS,
                        slot.id,
                        slot.node_id,
                        escape_js(name),
                        slot.expr
                    ),
                    TemplateAttrSetter::Text => format!(
                        "{}(i, {}, i.nodes[{}], \"{}\", {}, c);",
                        SET_TEXT_ATTR_ALIAS,
                        slot.id,
                        slot.node_id,
                        escape_js(name),
                        slot.expr
                    ),
                    TemplateAttrSetter::NullableText => format!(
                        "{}(i, {}, i.nodes[{}], \"{}\", {}, c);",
                        SET_NULLABLE_TEXT_ATTR_ALIAS,
                        slot.id,
                        slot.node_id,
                        escape_js(name),
                        slot.expr
                    ),
                    TemplateAttrSetter::TextProperty => format!(
                        "{}(i, {}, i.nodes[{}], \"{}\", {}, c);",
                        SET_TEXT_PROPERTY_ALIAS,
                        slot.id,
                        slot.node_id,
                        escape_js(name),
                        slot.expr
                    ),
                    TemplateAttrSetter::NullableTextProperty => format!(
                        "{}(i, {}, i.nodes[{}], \"{}\", {}, c);",
                        SET_NULLABLE_TEXT_PROPERTY_ALIAS,
                        slot.id,
                        slot.node_id,
                        escape_js(name),
                        slot.expr
                    ),
                    TemplateAttrSetter::Presence => format!(
                        "{}(i, {}, i.nodes[{}], \"{}\", {}, c);",
                        SET_PRESENCE_ATTR_ALIAS,
                        slot.id,
                        slot.node_id,
                        escape_js(name),
                        slot.expr
                    ),
                    TemplateAttrSetter::BooleanProperty => format!(
                        "{}(i, {}, i.nodes[{}], \"{}\", {}, c);",
                        SET_BOOLEAN_PROPERTY_ALIAS,
                        slot.id,
                        slot.node_id,
                        escape_js(name),
                        slot.expr
                    ),
                    TemplateAttrSetter::ClassName => format!(
                        "{}(i, {}, i.nodes[{}], {}, c);",
                        SET_CLASS_NAME_ALIAS, slot.id, slot.node_id, slot.expr
                    ),
                    TemplateAttrSetter::Class => format!(
                        "{}(i, {}, i.nodes[{}], {}, c);",
                        SET_CLASS_ALIAS, slot.id, slot.node_id, slot.expr
                    ),
                    TemplateAttrSetter::Style => format!(
                        "{}(i, {}, i.nodes[{}], {}, c);",
                        SET_STYLE_ALIAS, slot.id, slot.node_id, slot.expr
                    ),
                    TemplateAttrSetter::StyleRecord => format!(
                        "{}(i, {}, i.nodes[{}], {}, c);",
                        SET_STYLE_RECORD_ALIAS, slot.id, slot.node_id, slot.expr
                    ),
                },
                TemplateSlotKind::Event(event) => format!(
                    "{}(i, {}, i.nodes[{}], \"{}\", (event) => {}, d, c);",
                    SET_EVENT_ALIAS,
                    slot.id,
                    slot.node_id,
                    escape_js(event),
                    parenthesize_arrow_body(&slot.expr)
                ),
                TemplateSlotKind::Ref => format!(
                    "{}(i, {}, i.nodes[{}], {}, d, c);",
                    SET_REF_ALIAS, slot.id, slot.node_id, slot.expr
                ),
                TemplateSlotKind::Conditional {
                    condition,
                    render_then,
                    render_else,
                } => format!(
                    "{}(i, {}, i.nodes[{}], {}, {}, {}, d, c);",
                    SET_CONDITIONAL_ALIAS,
                    slot.id,
                    slot.node_id,
                    condition,
                    render_then,
                    render_else
                ),
                TemplateSlotKind::Component { name, render, args } => format!(
                    "{}(i, {}, i.nodes[{}], () => {}, {}, d, c, \"{}\");",
                    SET_COMPONENT_ALIAS,
                    slot.id,
                    slot.node_id,
                    render,
                    args,
                    escape_js(name)
                ),
                TemplateSlotKind::KeyedList {
                    collection,
                    item,
                    index,
                    key,
                    render,
                    stable_item_update,
                } => {
                    let key_params = if let Some(index) = index {
                        format!("{}, {}", item, index)
                    } else {
                        item.clone()
                    };
                    format!(
                        "{}(i, {}, i.nodes[{}], {}, ({}) => {}, {}, d, c, {});",
                        SET_KEYED_LIST_ALIAS,
                        slot.id,
                        slot.node_id,
                        collection,
                        key_params,
                        key,
                        render,
                        stable_item_update
                    )
                }
            };
            lines.push(update);
        }
        lines.join(" ")
    }
}

fn is_indentation_text_node(node: &HtmlNode) -> bool {
    matches!(node, HtmlNode::Text { text, .. } if text.contains('\n') && text.trim().is_empty())
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
    fn parse<F>(
        expr: &'a Expr,
        components: &BTreeSet<String>,
        is_typed_component_call: &F,
    ) -> Option<Self>
    where
        F: Fn(&Expr) -> bool,
    {
        match &expr.kind {
            ExprKind::HtmlTemplate(template) => Some(Self::Html(template)),
            ExprKind::List(_) => IfSpec::parse(expr, components, is_typed_component_call)
                .map(|spec| Self::If(Box::new(spec)))
                .or_else(|| {
                    ComponentSpec::parse(expr, components, is_typed_component_call(expr))
                        .map(|spec| Self::Component { expr, spec })
                }),
            _ => None,
        }
    }
}

impl<'a> IfSpec<'a> {
    fn parse<F>(
        expr: &'a Expr,
        components: &BTreeSet<String>,
        is_typed_component_call: &F,
    ) -> Option<Self>
    where
        F: Fn(&Expr) -> bool,
    {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        if items.len() != 4 || !matches_symbol(&items[0], "if") {
            return None;
        }

        let then_branch = TemplateBranch::parse(&items[2], components, is_typed_component_call)?;
        let else_branch = TemplateBranch::parse(&items[3], components, is_typed_component_call)?;

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
    fn parse(
        expr: &'a Expr,
        components: &BTreeSet<String>,
        is_typed_component_call: bool,
    ) -> Option<Self> {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        let Some((head, args)) = items.split_first() else {
            return None;
        };
        let ExprKind::Symbol(name) = &head.kind else {
            return None;
        };
        if name != "scope-view" && !components.contains(name) && !is_typed_component_call {
            return None;
        }

        Some(Self { name, args })
    }
}

fn expr_is_never_typed_component_call(_expr: &Expr) -> bool {
    false
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

fn literal_fn_parts(expr: &Expr) -> Option<(&[Expr], &[Expr])> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    let [head, params, body @ ..] = items.as_slice() else {
        return None;
    };
    if !matches_symbol(head, "fn") || body.is_empty() {
        return None;
    }
    let ExprKind::Vector(params) = &params.kind else {
        return None;
    };
    Some((params, body))
}

fn let_parts(expr: &Expr) -> Option<(&[Expr], &[Expr])> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    let [head, bindings, body @ ..] = items.as_slice() else {
        return None;
    };
    if !matches_symbol(head, "let") || body.is_empty() {
        return None;
    }
    let ExprKind::Vector(bindings) = &bindings.kind else {
        return None;
    };
    Some((bindings, body))
}

fn range_call_args(expr: &Expr) -> Option<&[Expr]> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    let [head, args @ ..] = items.as_slice() else {
        return None;
    };
    matches_symbol(head, "range").then_some(args)
}

fn simple_fn_param(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::Symbol(_))
}

fn map_call_parts(expr: &Expr) -> Option<(&Expr, &Expr, bool)> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    let [head, collection, mapper] = items.as_slice() else {
        return None;
    };
    if matches_symbol(head, "map") {
        Some((collection, mapper, false))
    } else if matches_symbol(head, "map-indexed") {
        Some((collection, mapper, true))
    } else {
        None
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

fn is_template_component_expr(expr: &Expr, components: &BTreeSet<String>) -> bool {
    match &expr.kind {
        ExprKind::HtmlTemplate(_) => true,
        ExprKind::List(items) => {
            if let_template_parts(expr).is_some() {
                return true;
            }
            if items.len() == 4 && matches_symbol(&items[0], "if") {
                return is_template_component_expr(&items[2], components)
                    && is_template_component_expr(&items[3], components);
            }
            ComponentSpec::parse(expr, components, false).is_some()
        }
        _ => false,
    }
}

fn collect_template_defns(source: &SourceFile) -> BTreeSet<String> {
    let defs = source
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
            Some((name.clone(), body))
        })
        .collect::<Vec<_>>();

    let mut components = BTreeSet::new();
    loop {
        let mut changed = false;
        for (name, body) in &defs {
            if components.contains(name) {
                continue;
            }
            if is_template_component_expr(body, &components) {
                changed |= components.insert(name.clone());
            }
        }
        if !changed {
            break;
        }
    }

    components
}

fn collect_function_defs(source: &SourceFile) -> BTreeMap<String, FunctionDef> {
    source
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
            let ExprKind::Vector(params) = &items[2].kind else {
                return None;
            };
            let params = params
                .iter()
                .map(symbol_name)
                .collect::<Option<Vec<_>>>()?
                .into_iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            Some((
                name.clone(),
                FunctionDef {
                    params,
                    body: items[3..].to_vec(),
                },
            ))
        })
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ReadSummary {
    params: Vec<String>,
    reads: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageFieldReads {
    pub top_fields: BTreeSet<String>,
    pub value_fields: BTreeSet<String>,
    pub value_escapes: bool,
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

pub fn collect_message_field_reads(source: &SourceFile) -> BTreeMap<String, MessageFieldReads> {
    let mut reads = BTreeMap::new();
    for form in &source.forms {
        let ExprKind::List(items) = &form.kind else {
            continue;
        };
        if items.len() < 4 || !matches_symbol(&items[0], "defn") {
            continue;
        }
        if !matches_symbol(&items[1], "update") {
            continue;
        }
        let ExprKind::Vector(params) = &items[2].kind else {
            continue;
        };
        let Some(message_param) = params.get(1).and_then(symbol_name) else {
            continue;
        };
        for body in &items[3..] {
            collect_message_field_reads_expr(body, message_param, &mut reads);
        }
    }
    reads
}

fn merged_message_field_reads(
    mut base: BTreeMap<String, MessageFieldReads>,
    local: BTreeMap<String, MessageFieldReads>,
) -> BTreeMap<String, MessageFieldReads> {
    for (kind, reads) in local {
        let entry = base.entry(kind).or_default();
        entry.top_fields.extend(reads.top_fields);
        entry.value_fields.extend(reads.value_fields);
        entry.value_escapes |= reads.value_escapes;
    }
    base
}

fn collect_message_field_reads_expr(
    expr: &Expr,
    message_param: &str,
    reads: &mut BTreeMap<String, MessageFieldReads>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            if items.len() >= 3
                && matches_symbol(&items[0], "match")
                && matches_symbol(&items[1], message_param)
            {
                for arm in items[2..].chunks(2) {
                    let [pattern, body] = arm else {
                        continue;
                    };
                    let Some(info) = kind_pattern_info(pattern) else {
                        collect_message_field_reads_expr(body, message_param, reads);
                        continue;
                    };
                    let entry = reads.entry(info.kind).or_default();
                    let mut value_aliases = BTreeSet::new();
                    collect_message_pattern_reads(info.entries, entry, &mut value_aliases);
                    collect_message_value_alias_reads(body, &value_aliases, entry);
                    collect_message_field_reads_expr(body, message_param, reads);
                }
                return;
            }
            for item in items {
                collect_message_field_reads_expr(item, message_param, reads);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_message_field_reads_expr(item, message_param, reads);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_message_field_reads_expr(key, message_param, reads);
                collect_message_field_reads_expr(value, message_param, reads);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => {
            collect_message_field_reads_expr(inner, message_param, reads)
        }
        ExprKind::HtmlTemplate(_)
        | ExprKind::Symbol(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn collect_message_pattern_reads(
    entries: &[(Expr, Expr)],
    reads: &mut MessageFieldReads,
    value_aliases: &mut BTreeSet<String>,
) {
    for (key, pattern) in entries {
        let Some(key) = object_key_name(key) else {
            reads.value_escapes = true;
            continue;
        };
        if key == "kind" {
            continue;
        }
        if key == "value" {
            collect_message_value_pattern_reads(pattern, reads, value_aliases);
        } else if !matches!(pattern.kind, ExprKind::Symbol(ref name) if name == "_") {
            reads.top_fields.insert(key);
        }
    }
}

fn collect_message_value_pattern_reads(
    pattern: &Expr,
    reads: &mut MessageFieldReads,
    value_aliases: &mut BTreeSet<String>,
) {
    match &pattern.kind {
        ExprKind::Symbol(name) if name == "_" => {}
        ExprKind::Symbol(name) => {
            value_aliases.insert(name.clone());
        }
        ExprKind::Map(entries) => {
            for (key, _) in entries {
                let Some(key) = object_key_name(key) else {
                    reads.value_escapes = true;
                    continue;
                };
                reads.value_fields.insert(key);
            }
        }
        _ => reads.value_escapes = true,
    }
}

fn collect_message_value_alias_reads(
    expr: &Expr,
    value_aliases: &BTreeSet<String>,
    reads: &mut MessageFieldReads,
) {
    match &expr.kind {
        ExprKind::Symbol(name) => {
            for alias in value_aliases {
                if name == alias {
                    reads.value_escapes = true;
                    continue;
                }
                let Some(suffix) = name.strip_prefix(&format!("{}.", alias)) else {
                    continue;
                };
                if let Some(field) = suffix.split('.').next().filter(|field| !field.is_empty()) {
                    reads.value_fields.insert(field.to_string());
                }
            }
        }
        ExprKind::List(items) => {
            if matches_symbol(items.first().unwrap_or(expr), "fn") {
                if let Some(params) = items.get(1).and_then(|params| match &params.kind {
                    ExprKind::Vector(params) => Some(params),
                    _ => None,
                }) {
                    let shadows_alias = params
                        .iter()
                        .filter_map(symbol_name)
                        .any(|name| value_aliases.contains(name));
                    if shadows_alias {
                        return;
                    }
                }
            }
            for item in items {
                collect_message_value_alias_reads(item, value_aliases, reads);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_message_value_alias_reads(item, value_aliases, reads);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_message_value_alias_reads(key, value_aliases, reads);
                collect_message_value_alias_reads(value, value_aliases, reads);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => {
            collect_message_value_alias_reads(inner, value_aliases, reads)
        }
        ExprKind::HtmlTemplate(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
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

fn collect_pattern_type_bindings(
    pattern: &Expr,
    value_type: Option<&str>,
    bindings: &mut BTreeMap<String, String>,
) {
    match &pattern.kind {
        ExprKind::Symbol(name) if name == "_" => {}
        ExprKind::Symbol(name) => {
            if let Some(ty) = value_type {
                bindings.insert(name.clone(), ty.to_string());
            }
        }
        ExprKind::List(items) => {
            let Some(name) = items.first().and_then(symbol_name) else {
                return;
            };
            match name {
                "list" => {
                    for (index, item) in items[1..].iter().enumerate() {
                        let item_type = value_type.and_then(|ty| type_vector_item_type(ty, index));
                        collect_pattern_type_bindings(item, item_type, bindings);
                    }
                }
                "cons" if items.len() == 3 => {
                    let head_type = value_type.and_then(type_sequence_element);
                    collect_pattern_type_bindings(&items[1], head_type, bindings);
                    collect_pattern_type_bindings(&items[2], value_type, bindings);
                }
                "some" if items.len() == 2 => {
                    collect_pattern_type_bindings(
                        &items[1],
                        value_type.and_then(type_option_inner),
                        bindings,
                    );
                }
                "ok" | "err" if items.len() == 2 => {
                    let inner_type = value_type
                        .and_then(type_result_parts)
                        .map(|(ok, err)| if name == "ok" { ok } else { err });
                    collect_pattern_type_bindings(&items[1], inner_type, bindings);
                }
                "as" if items.len() == 3 => {
                    collect_pattern_type_bindings(&items[1], value_type, bindings);
                    if let ExprKind::Symbol(name) = &items[2].kind {
                        if name != "_" {
                            if let Some(ty) = value_type {
                                bindings.insert(name.clone(), ty.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                let Some(key) = object_key_name(key) else {
                    continue;
                };
                let field_type = value_type.and_then(|ty| type_record_field_type(ty, &key));
                collect_pattern_type_bindings(value, field_type, bindings);
            }
        }
        ExprKind::Vector(items) => {
            for (index, item) in items.iter().enumerate() {
                let item_type = value_type.and_then(|ty| type_vector_item_type(ty, index));
                collect_pattern_type_bindings(item, item_type, bindings);
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

fn keyed_item_update_reads(
    spec: &ForSpec<'_>,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    collect_keyed_item_html_reads(spec.template, &mut symbols, components, read_summaries);
    symbols.into_iter().collect()
}

fn keyed_item_local_read(read: &str, item: &str, index: Option<&str>) -> bool {
    let item_prefix = format!("{}.", item);
    if read == item || read.starts_with(&item_prefix) {
        return true;
    }
    if let Some(index) = index {
        let index_prefix = format!("{}.", index);
        return read == index || read.starts_with(&index_prefix);
    }
    false
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

fn collect_keyed_item_html_reads(
    node: &HtmlNode,
    symbols: &mut BTreeSet<String>,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) {
    match node {
        HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_template_reads_inner(expr, symbols, read_summaries);
                }
            }
            for child in &element.children {
                collect_keyed_item_html_reads(child, symbols, components, read_summaries);
            }
        }
        HtmlNode::Expr { expr, .. } => {
            if let Some(spec) = ForSpec::parse(expr) {
                symbols.extend(collect_keyed_reads(&spec, components, read_summaries));
                return;
            }
            if let Some(spec) = IfSpec::parse(expr, components, &expr_is_never_typed_component_call)
            {
                symbols.extend(collect_conditional_reads(&spec, components, read_summaries));
                return;
            }
            if let Some(spec) = ComponentSpec::parse(expr, components, false) {
                symbols.extend(component_call_reads(&spec, read_summaries));
                return;
            }
            collect_template_reads_inner(expr, symbols, read_summaries);
        }
        HtmlNode::Text { .. } => {}
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
            if let Some(spec) = IfSpec::parse(expr, components, &expr_is_never_typed_component_call)
            {
                symbols.extend(collect_conditional_reads(&spec, components, read_summaries));
                return;
            }
            if let Some(spec) = ComponentSpec::parse(expr, components, false) {
                symbols.extend(component_call_reads(&spec, read_summaries));
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
    if spec.name == "scope-view" {
        return scope_view_reads(spec, read_summaries);
    }
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

fn scope_view_reads(
    spec: &ComponentSpec<'_>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    let Some(view_expr) = spec.args.get(1) else {
        return Vec::new();
    };
    let Some(state_expr) = spec.args.get(2) else {
        return Vec::new();
    };
    let Some(view_name) = symbol_name(view_expr) else {
        return collect_template_reads(state_expr, read_summaries);
    };
    let Some(summary) = read_summaries.get(view_name) else {
        return collect_template_reads(state_expr, read_summaries);
    };
    project_call_reads(summary, std::slice::from_ref(state_expr), read_summaries)
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
    if name == "Cmd.none" {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }

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

fn primitive_decoder_runtime_property(name: &str) -> Option<&'static str> {
    match name {
        "decoder-string" => Some("string"),
        "decoder-number" => Some("number"),
        "decoder-bool" => Some("bool"),
        "decoder-keyword" => Some("keyword"),
        _ => None,
    }
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

fn runtime_effect_kind_name(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Keyword(name) | ExprKind::String(name) if name.contains('/') => Some(name),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn html_template_emit_options() -> HtmlTemplateEmitOptions {
        HtmlTemplateEmitOptions::enabled("createCompiledHtmlTemplateComponent")
    }

    fn emit_module_with_html(source: &SourceFile) -> EmitResult {
        let options = EmitOptions {
            html_templates: html_template_emit_options(),
            ..EmitOptions::default()
        };
        emit_module_with_types_and_options(source, BTreeMap::new(), options)
    }

    fn emit_module_with_html_types(
        source: &SourceFile,
        expr_types: BTreeMap<usize, String>,
    ) -> EmitResult {
        let options = EmitOptions {
            html_templates: html_template_emit_options(),
            ..EmitOptions::default()
        };
        emit_module_with_types_and_options(source, expr_types, options)
    }

    fn first_call_span(expr: &Expr, name: &str) -> usize {
        if let ExprKind::List(items) = &expr.kind {
            if items.first().is_some_and(|head| matches_symbol(head, name)) {
                return expr.span.start;
            }
            for item in items {
                if let Some(span) = find_call_span(item, name) {
                    return span;
                }
            }
        }
        if let Some(span) = find_call_span(expr, name) {
            return span;
        }
        panic!("missing `{}` call", name);
    }

    fn find_call_span(expr: &Expr, name: &str) -> Option<usize> {
        match &expr.kind {
            ExprKind::List(items) => {
                if items.first().is_some_and(|head| matches_symbol(head, name)) {
                    return Some(expr.span.start);
                }
                items.iter().find_map(|item| find_call_span(item, name))
            }
            ExprKind::Vector(items) | ExprKind::Set(items) => {
                items.iter().find_map(|item| find_call_span(item, name))
            }
            ExprKind::Map(entries) => entries.iter().find_map(|(key, value)| {
                find_call_span(key, name).or_else(|| find_call_span(value, name))
            }),
            ExprKind::Quote(inner)
            | ExprKind::QuasiQuote(inner)
            | ExprKind::Unquote(inner)
            | ExprKind::UnquoteSplicing(inner) => find_call_span(inner, name),
            ExprKind::HtmlTemplate(node) => find_call_span_in_html(node, name),
            ExprKind::Symbol(_)
            | ExprKind::Nil
            | ExprKind::Bool(_)
            | ExprKind::Number(_)
            | ExprKind::String(_)
            | ExprKind::Keyword(_) => None,
        }
    }

    fn find_call_span_in_html(node: &HtmlNode, name: &str) -> Option<usize> {
        match node {
            HtmlNode::Element(element) => {
                for attr in &element.attrs {
                    if let HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                        if let Some(span) = find_call_span(expr, name) {
                            return Some(span);
                        }
                    }
                }
                element
                    .children
                    .iter()
                    .find_map(|child| find_call_span_in_html(child, name))
            }
            HtmlNode::Expr { expr, .. } => find_call_span(expr, name),
            HtmlNode::Text { .. } => None,
        }
    }

    fn first_call_head_span(expr: &Expr, name: &str) -> usize {
        find_call_head_span(expr, name).unwrap_or_else(|| panic!("missing `{}` call head", name))
    }

    fn find_call_head_span(expr: &Expr, name: &str) -> Option<usize> {
        match &expr.kind {
            ExprKind::List(items) => {
                if let Some(head) = items.first() {
                    if matches_symbol(head, name) {
                        return Some(head.span.start);
                    }
                }
                items
                    .iter()
                    .find_map(|item| find_call_head_span(item, name))
            }
            ExprKind::Vector(items) | ExprKind::Set(items) => items
                .iter()
                .find_map(|item| find_call_head_span(item, name)),
            ExprKind::Map(entries) => entries.iter().find_map(|(key, value)| {
                find_call_head_span(key, name).or_else(|| find_call_head_span(value, name))
            }),
            ExprKind::Quote(inner)
            | ExprKind::QuasiQuote(inner)
            | ExprKind::Unquote(inner)
            | ExprKind::UnquoteSplicing(inner) => find_call_head_span(inner, name),
            ExprKind::HtmlTemplate(node) => find_call_head_span_in_html(node, name),
            ExprKind::Symbol(_)
            | ExprKind::Nil
            | ExprKind::Bool(_)
            | ExprKind::Number(_)
            | ExprKind::String(_)
            | ExprKind::Keyword(_) => None,
        }
    }

    fn find_call_head_span_in_html(node: &HtmlNode, name: &str) -> Option<usize> {
        match node {
            HtmlNode::Element(element) => {
                for attr in &element.attrs {
                    if let HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                        if let Some(span) = find_call_head_span(expr, name) {
                            return Some(span);
                        }
                    }
                }
                element
                    .children
                    .iter()
                    .find_map(|child| find_call_head_span_in_html(child, name))
            }
            HtmlNode::Expr { expr, .. } => find_call_head_span(expr, name),
            HtmlNode::Text { .. } => None,
        }
    }

    #[test]
    fn emits_definitions_as_esm_exports() {
        let source = syntax::parse_source("(def answer (+ 40 2))");
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert_eq!(emitted.code.trim(), "export const answer = 40 + 2;");
        assert_eq!(
            emitted.exports.get("answer").map(|export| export.arity),
            Some(None)
        );
    }

    #[test]
    fn emits_private_top_level_bindings_with_export_metadata() {
        let source = syntax::parse_source(
            "(def banner \"export function text should stay literal\")\n\
             (defn init [boot] [{:boot boot} Cmd.none])\n\
             (defn subscriptions [state] Sub.none)",
        );
        let mut options = EmitOptions::default();
        options.export_top_level = false;
        let emitted = emit_module_with_types_and_options(&source, BTreeMap::new(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("const banner = \"export function text should stay literal\";")
        );
        assert!(emitted.code.contains("function init(boot)"));
        assert!(emitted.code.contains("function subscriptions(state)"));
        assert!(!emitted.code.contains("export const banner"));
        assert!(!emitted.code.contains("export function init"));
        assert_eq!(
            emitted.exports.get("init").map(|export| export.arity),
            Some(Some(1))
        );
        assert!(emitted.exports.contains_key("subscriptions"));
    }

    #[test]
    fn emits_self_tail_calls_as_loop() {
        let source = syntax::parse_source(
            "(defn sum-down [n total]\n  (if (<= n 0)\n      total\n      (sum-down (- n 1) (+ total n))))",
        );
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert_eq!(emitted.source_mappings.len(), 2);
        assert_eq!(emitted.source_mappings[0].generated_line, 4);
        assert_eq!(emitted.source_mappings[1].generated_line, 8);
    }

    #[test]
    fn emits_imports_as_esm_imports() {
        let source = syntax::parse_source(
            "(import \"./hrweb_metrics.clsk\" [calculate-trimp matches-hrr-type?])\n\
             (defn summarize [entry] (calculate-trimp entry))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(
            "import { calculate_trimp, matches_hrr_type_ } from \"./hrweb_metrics.mjs\";"
        ));
        assert!(!emitted.code.contains("value0"));
        assert!(emitted.code.contains("calculate_trimp(entry)"));
    }

    #[test]
    fn emits_closkell_test_import_from_runtime() {
        let source = syntax::parse_source(
            "(import \"closkell/test\" [describe test expect= expect-not= expect-match expect-throws])\n\
             (describe \"math\" (test \"adds\" (expect= (+ 1 1) 2) (expect-not= 2 3) (expect-match {:kind :ok :value 2} {:kind :ok}) (expect-throws (fn [] (fail \"boom\")) \"boom\")))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("import { describe, test, expect_, expect_not_, expect_match, expect_throws } from \"@closkell/runtime\";"));
        assert!(emitted.code.contains("expect_(1 + 1, 2)"));
        assert!(emitted.code.contains("expect_not_(2, 3)"));
        assert!(emitted.code.contains("expect_match("));
        assert!(emitted.code.contains("expect_throws("));
    }

    #[test]
    fn erases_type_only_names_from_esm_imports() {
        let source = syntax::parse_source(
            "(import \"./chart.clsk\" [HeartReading HeartZone heart-chart-command])\n\
             (defn draw [state] (heart-chart-command state))",
        );
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert_eq!(emitted.code.trim(), "export const api_type_count = 1;");
        assert_eq!(emitted.source_mappings.len(), 1);
        assert_eq!(
            emitted.source_mappings[0].source_offset,
            source.forms[2].span.start
        );
    }

    #[test]
    fn emits_command_helpers_as_plain_data() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state (Cmd.batch [Cmd.none\n                         (Cmd.time/now (Msg.mapper :started :timestamp))\n                         (Cmd.random/number 1 10 (Msg.mapper :rolled :value))\n                         {:kind :timer/after :id \"tick\" :ms 1000 :msg {:kind :tick}}])])",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("kind: Symbol.for(\"batch\")"));
        assert!(emitted.code.contains("commands: ["));
        assert!(emitted.code.contains("kind: Symbol.for(\"none\")"));
        assert!(emitted.code.contains("kind: Symbol.for(\"time/now\")"));
        assert!(emitted.code.contains("kind: Symbol.for(\"random/number\")"));
        assert!(emitted.code.contains("kind: Symbol.for(\"timer/after\")"));
        assert!(!emitted.code.contains("Cmd."));
        assert!(!emitted.code.contains("Msg."));
    }

    #[test]
    fn emits_task_helpers_as_plain_data() {
        let source = syntax::parse_source(
            "(defn decode [text]\n  (Task.succeed {:title text}))\n\
             (defn load [url]\n  (Task.perform\n    (Task.and-then (Http.get-text url) decode)\n    (fn [spec] {:kind :loaded :value spec})\n    (fn [error] {:kind :failed :error error})))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("kind: Symbol.for(\"task/perform\")"));
        assert!(emitted.code.contains("kind: Symbol.for(\"task/and-then\")"));
        assert!(
            emitted
                .code
                .contains("kind: Symbol.for(\"task/http/get-text\")")
        );
        assert!(emitted.code.contains("kind: Symbol.for(\"task/succeed\")"));
        assert!(!emitted.code.contains("Task."));
        assert!(!emitted.code.contains("Http.get"));
    }

    #[test]
    fn emits_scoped_composition_helpers() {
        let source = syntax::parse_source(
            "(defn child-view [state]\n  #html <button>{state.count}</button>)\n\
             (defn child-update [state msg]\n  [state {:kind :none}])\n\
             (defn child-subscriptions [state]\n  Sub.none)\n\
             (defn update [state msg]\n  (scope-update state :log msg child-update :log))\n\
             (defn subscriptions [state]\n  (scope-subscriptions state.log child-subscriptions :log))\n\
             (defn view [state]\n  #html <main>{state.log.count}</main>)",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellScopeUpdate"));
        assert!(emitted.code.contains("__closkellScopeSubscriptions"));
    }

    #[test]
    fn emits_decoder_runtime_helpers() {
        let source = syntax::parse_source(
            "(def spec-decoder\n\
               (decoder-record {:title decoder-string\n\
                                :tags (decoder-vector decoder-string)\n\
                                :draft (decoder-optional decoder-number)}))\n\
             (def decoded\n\
               (decode spec-decoder (json-parse \"{\\\"title\\\":\\\"Pulse\\\",\\\"tags\\\":[\\\"zone\\\"]}\")))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(
            "import { Decoder as __closkellDecoder, decode as __closkellDecode } from \"@closkell/runtime\";"
        ));
        assert!(emitted.code.contains("__closkellDecoder.record({"));
        assert!(emitted.code.contains("title: __closkellDecoder.string"));
        assert!(
            emitted
                .code
                .contains("tags: __closkellDecoder.vector(__closkellDecoder.string)")
        );
        assert!(
            emitted
                .code
                .contains("draft: __closkellDecoder.optional(__closkellDecoder.number)")
        );
        assert!(
            emitted
                .code
                .contains("__closkellDecode(spec_decoder, JSON.parse(")
        );
    }

    #[test]
    fn emits_typed_imported_html_calls_as_component_slots() {
        let input = "(import \"./child.clsk\" [child-view])\n\
             (defn view [state]\n  #html <main>{(child-view state.child)}</main>)";
        let source = syntax::parse_source(input);
        let mut expr_types = BTreeMap::new();
        expr_types.insert(
            first_call_head_span(&source.forms[1], "child-view"),
            "(Fn [{:count Number}] Html)".to_string(),
        );
        let emitted = emit_module_with_html_types(&source, expr_types);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted.code.contains("__closkellSetComponent"),
            "typed Html call should mount as a component:\n{}",
            emitted.code
        );
        assert!(
            !emitted.code.contains("__closkellSetText(i, 0"),
            "typed Html call must not be emitted as text:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_typed_imported_html_calls_inside_conditionals_as_conditional_slots() {
        let input = "(import \"./session.clsk\" [live-pane])\n\
             (import \"./logbook.clsk\" [log-pane])\n\
             (defn view [state]\n  #html <main>{(if (= state.detailView \"live\") (live-pane state) (log-pane state))}</main>)";
        let source = syntax::parse_source(input);
        let mut expr_types = BTreeMap::new();
        expr_types.insert(
            first_call_head_span(&source.forms[2], "live-pane"),
            "(Fn [{:detailView String}] Html)".to_string(),
        );
        expr_types.insert(
            first_call_head_span(&source.forms[2], "log-pane"),
            "(Fn [{:detailView String}] Html)".to_string(),
        );
        let emitted = emit_module_with_html_types(&source, expr_types);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted.code.contains("__closkellSetConditional"),
            "typed Html branches should lower as a conditional slot:\n{}",
            emitted.code
        );
        assert!(emitted.code.contains("live_pane(state)"));
        assert!(emitted.code.contains("log_pane(state)"));
        assert!(
            !emitted
                .code
                .contains("state.detailView === \"live\", live_pane(state)"),
            "conditional Html branches must not be emitted as eager component args:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_typed_html_result_calls_inside_keyed_lists_as_component_slots() {
        let input = "(import \"./operation.clsk\" [operationPanel])\n\
             (defn view [state]\n  #html <section>{(for [item state.items :key item.id] #html <div>{(operationPanel state item)}</div>)}</section>)";
        let source = syntax::parse_source(input);
        let mut expr_types = BTreeMap::new();
        expr_types.insert(
            first_call_span(&source.forms[1], "operationPanel"),
            "Html".to_string(),
        );
        let emitted = emit_module_with_html_types(&source, expr_types);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted.code.contains("__closkellSetKeyedList"),
            "keyed list should still lower as a list slot:\n{}",
            emitted.code
        );
        assert!(
            emitted.code.contains("__closkellSetComponent")
                && emitted.code.contains("operationPanel(state, item)"),
            "typed Html call in a keyed item should mount as a component:\n{}",
            emitted.code
        );
        assert!(
            !emitted
                .code
                .contains("__closkellSetText(i, 0, i.nodes[0], operationPanel(state, item)"),
            "typed Html call in a keyed item must not be emitted as text:\n{}",
            emitted.code
        );
    }

    #[test]
    fn imports_runtime_for_templates() {
        let source = syntax::parse_source("#html <div>{label}</div>");
        let emitted = emit_module_with_html(&source);

        assert!(emitted.code.contains("@closkell/runtime"));
        assert!(emitted.code.contains("__closkellCreateHtmlTemplate"));
        assert!(emitted.code.contains("__closkellSetText"));
    }

    #[test]
    fn default_emission_rejects_html_templates() {
        let source = syntax::parse_source("#html <div>{label}</div>");
        let emitted = emit_module(&source);

        assert!(
            emitted.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("#html templates are not enabled")),
            "{:?}",
            emitted.diagnostics
        );
    }

    #[test]
    fn templates_import_configured_runtime_constructor() {
        let source = syntax::parse_source("#html <div>{label}</div>");
        let options = EmitOptions {
            html_templates: HtmlTemplateEmitOptions::enabled(
                "createCustomCompiledHtmlTemplateComponent",
            ),
            ..EmitOptions::default()
        };
        let emitted = emit_module_with_types_and_options(&source, BTreeMap::new(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted.code.contains(
                "createCustomCompiledHtmlTemplateComponent as __closkellCreateHtmlTemplate"
            )
        );
        assert!(
            !emitted
                .code
                .contains("createCompiledHtmlTemplateComponent as __closkellCreateHtmlTemplate")
        );
    }

    #[test]
    fn emits_template_slots_for_attrs_events_and_text() {
        let source = syntax::parse_source(
            "(defn status-view [state] #html <button class={state.buttonClass} disabled={not state.connected?} on:click={:start}>{state.label}</button>)",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("export function status_view(state)"));
        assert!(emitted.code.contains("__closkellCreateHtmlTemplate"));
        assert!(emitted.code.contains("__closkellSetClass"));
        assert!(emitted.code.contains("state[\"connected?\"]"));
        assert!(emitted.code.contains("__closkellSetEvent(i, 2"));
        assert!(emitted.code.contains("__closkellSetText(i, 3"));
        assert!(emitted.code.contains("state.buttonClass"));
        assert!(emitted.code.contains("!(state[\"connected?\"])"));
        assert!(emitted.code.contains("state.label"));
    }

    #[test]
    fn emits_svg_templates_with_svg_namespace() {
        let source = syntax::parse_source(
            "(defn icon [] #html <span><svg viewBox=\"0 0 24 24\"><path d=\"M12 3v18\"></path></svg></span>)",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellCreateHtmlTemplate"));
        assert!(emitted.code.contains("svg"));
        assert!(emitted.code.contains("viewBox"));
        assert!(emitted.code.contains("path"));
    }

    #[test]
    fn emits_indexed_keyed_template_loops() {
        let source = syntax::parse_source(
            "(defn view [state]\n  #html <section>{(for [zone state.zones index :key zone.id] #html <button data-index={index} on:click={{:kind :select :rank (+ index 1)}}>{(str index zone.name)}</button>)}</section>)",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetKeyedList"));
        assert!(emitted.code.contains("(zone, index) => zone.id"));
        assert!(
            emitted
                .code
                .contains("(__closkell_zone, __closkell_index) =>")
        );
        assert!(emitted.code.contains("let index = __closkell_index"));
        assert!(
            emitted
                .code
                .contains("(__closkell_next_zone, __closkell_next_index) =>")
        );
        assert!(emitted.code.contains("index = __closkell_next_index"));
    }

    #[test]
    fn emits_update_wrapper_for_let_wrapped_template_defns() {
        let source = syntax::parse_source(
            "(defn view [state] (let [label state.label] #html <p>{label}</p>))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("export function view(state)"));
        assert!(emitted.code.contains("let label;"));
        assert!(
            emitted
                .code
                .contains("const __closkellRefresh = () => { label = state.label; }")
        );
        assert!(
            emitted
                .code
                .contains("(next_state) => { state = next_state; __closkellRefresh(); }")
        );
        assert!(emitted.code.contains("__closkellSetText(i, 0"));
        assert!(!emitted.code.contains("return (() =>"));
    }

    #[test]
    fn emits_source_reads_for_let_wrapped_template_metadata() {
        let source = syntax::parse_source(
            "(defn stat-tile [label value] #html <strong>{value}</strong>)\n\
             (defn view [state]\n  (let [avg (average-bpm state.readings)\n        entry (selected-log state.entries state.selectedLogId)]\n    #html <section>\n            {(stat-tile \"Avg\" avg)}\n            <p>{entry.durationMs}</p>\n            {(for [item state.entries :key item.id]\n               #html <button on:click={{:kind :select :id item.id}}>{item.label}</button>)}\n          </section>))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("avg = average_bpm(state.readings)"));
        assert!(
            emitted
                .code
                .contains("entry = selected_log(state.entries, state.selectedLogId)")
        );
        assert!(emitted.code.contains("() => stat_tile(\"Avg\", avg)"));
        assert!(emitted.code.contains("entry.durationMs"));
        assert!(emitted.code.contains("state.entries, (item) => item.id"));
    }

    #[test]
    fn emits_pattern_let_wrapped_template_reads_and_refresh_assignments() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (let [{:reading {:bpm bpm}\n\
                      :samples (cons head rest)} state.payload]\n\
                 #html <section data-bpm={bpm} data-head={head} data-count={(count rest)}></section>))",
        );
        let emitted = emit_module_with_html(&source);

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
                .contains("bpm = __closkell_template_let_0.reading.bpm")
        );
        assert!(
            emitted
                .code
                .contains("head = __closkell_template_let_0.samples[0]")
        );
        assert!(
            emitted
                .code
                .contains("rest = __closkell_template_let_0.samples.slice(1)")
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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("payload = state.payload"));
        assert!(emitted.code.contains("reading = payload.reading"));
        assert!(emitted.code.contains("payload.reading.bpm"));
        assert!(emitted.code.contains("reading.zone"));
    }

    #[test]
    fn emits_projected_template_reads_for_option_and_result_pattern_aliases() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (let [(ok {:entries entries}) state.importResult\n\
                     (some current) state.latest]\n\
                 #html <section data-count={(count entries)} data-bpm={current.bpm}></section>))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("entries = __closkell_template_let_0.value.entries")
        );
        assert!(emitted.code.contains("current = __closkell_template_let_1"));
        assert!(emitted.code.contains("__closkellCount(entries)"));
        assert!(emitted.code.contains("current.bpm"));
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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("entry = selected_log(state.entries, state.selectedLogId)")
        );
        assert!(emitted.code.contains("details = entry.details"));
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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("first_entry = (state.entries[0] ?? null)")
        );
        assert!(
            emitted
                .code
                .contains("second_entry = (state.entries[1] ?? null)")
        );
        assert!(
            emitted
                .code
                .contains("third_entry = (state.entries[2] ?? null)")
        );
        assert!(
            emitted
                .code
                .contains("first_cell = (((state.matrix[0] ?? null))[0] ?? null)")
        );
        assert!(
            emitted
                .code
                .contains("() => row((state.entries[0] ?? null))")
        );
    }

    #[test]
    fn keeps_dynamic_nth_reads_on_collection_and_index_dependencies() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (let [entry (nth state.entries state.selectedIndex)]\n\
                 #html <section data-label={entry.label}></section>))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("entry = (state.entries[state.selectedIndex] ?? null)")
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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetRef"));
        assert!(emitted.code.contains("\"heart-chart\""));
        assert!(emitted.code.contains("__closkellSetRef(i, 0"));
        assert!(!emitted.code.contains("setAttribute(\"ref\""));
    }

    #[test]
    fn emits_component_template_slots() {
        let source = syntax::parse_source(
            "(defn summary-card [summary] #html <article>{summary.label}</article>)\n(defn view [state] #html <section>{(summary-card state.summary)}</section>)",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetComponent"));
        assert!(emitted.code.contains("() => summary_card(state.summary)"));
        assert!(emitted.code.contains("[state.summary]"));
        assert!(emitted.code.contains("\"summary-card\""));
        assert!(emitted.code.contains("() => summary_card(state.summary)"));
        assert!(
            emitted
                .code
                .contains("(next_summary) => { summary = next_summary; }")
        );
    }

    #[test]
    fn emits_conditional_component_returning_defns_as_components() {
        let source = syntax::parse_source(
            "(defn password-card [scheme] #html <article>{scheme.id}<span>Password</span></article>)\n\
             (defn api-key-card [scheme] #html <article>{scheme.id}<span>API key</span></article>)\n\
             (defn scheme-card [scheme]\n\
               (if (= scheme.kind \"password\")\n\
                   (password-card scheme)\n\
                   (api-key-card scheme)))\n\
             (defn view [state]\n\
               #html <section>{(for [scheme state.schemes :key scheme.id]\n\
                                #html <div>{(scheme-card scheme)}</div>)}</section>)",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetComponent"));
        assert!(emitted.code.contains("() => scheme_card(scheme)"));
        assert!(emitted.code.contains("\"scheme-card\""));
        assert!(emitted.code.contains("scheme.kind === \"password\""));
    }

    #[test]
    fn emits_helper_call_template_reads_as_state_paths() {
        let source = syntax::parse_source(
            "(defn connection-label [state]\n  (if state.connected? (if state.simulated? \"Simulated\" \"Bluetooth\") \"Disconnected\"))\n\
             (defn view [state] #html <h2>{(connection-label state)}</h2>)",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted.code.contains("connection_label(state)"),
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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("avg = average_bpm(state.readings)"));
        assert!(emitted.code.contains("state.latestBpm"));
        assert!(emitted.code.contains("state.exerciseState === \"running\""));
        assert!(emitted.code.contains("() => live_pane(state)"));
    }

    #[test]
    fn emits_defn_and_dotted_record_reads() {
        let source = syntax::parse_source(
            "(defn in-zone? [zone bpm] (and (>= bpm zone.min) (<= bpm zone.max)))",
        );
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("\"start\""));
        assert!(emitted.code.contains("return \"Start\";"));
        assert!(emitted.code.contains("return \"Other\";"));
    }

    #[test]
    fn emits_record_pattern_match_bindings() {
        let source =
            syntax::parse_source("(defn next [msg] (match msg {:kind :rate :bpm bpm} bpm _ 0))");
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("kind"));
        assert!(emitted.code.contains("\"rate\""));
        assert!(emitted.code.contains("const bpm ="));
        assert!(emitted.code.contains("return bpm;"));
    }

    #[test]
    fn emits_as_pattern_match_bindings() {
        let source = syntax::parse_source(
            "(defn normalize [msg]\n  (match msg\n    (as {:kind :rate :bpm bpm} whole) (assoc whole :bpm (+ bpm 1))\n    _ msg))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("kind"));
        assert!(emitted.code.contains("\"rate\""));
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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(
            "(event) => ({ kind: Symbol.for(\"draft\"), value: event.currentTarget.value })"
        ));
    }

    #[test]
    fn emits_str_as_string_conversion() {
        let source = syntax::parse_source("(defn label [bpm] (str bpm))");
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("return String(bpm);"));
    }

    #[test]
    fn emits_json_helpers() {
        let source = syntax::parse_source(
            "(defn export-log [entries]\n  (json-stringify {:version 2 :entries entries} 2))\n\
             (defn imported-count [text]\n  (count (let [parsed (json-parse text)] parsed.entries)))",
        );
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("entry?.id ?? null"));
        assert!(emitted.code.contains("typeof"));
        assert!(emitted.code.contains("Number.isFinite"));
        assert!(emitted.code.contains("Array.isArray"));
        assert!(emitted.code.contains("== null"));
    }

    #[test]
    fn emits_direct_get_for_typed_record_fields() {
        let source = syntax::parse_source("(defn entries [state] (get state :entries))");
        let mut expr_types = BTreeMap::new();
        expr_types.insert(
            source.forms[0].span.start,
            "(Fn [{:entries (Vector Number)}] (Vector Number))".to_string(),
        );
        let emitted = emit_module_with_html_types(&source, expr_types);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("return state.entries;"));
        assert!(!emitted.code.contains("state?.entries ?? null"));
    }

    #[test]
    fn keeps_some_check_for_typed_nullable_sequence_access() {
        let source = syntax::parse_source(
            "(defn next-max [items]\n\
               (let [previous (last items)]\n\
                 (if (some? previous)\n\
                     (+ previous.max 1)\n\
                     0)))",
        );
        let mut expr_types = BTreeMap::new();
        expr_types.insert(
            source.forms[0].span.start,
            "(Fn [(Vector {:max Number})] Number)".to_string(),
        );
        expr_types.insert(
            first_call_span(&source.forms[0], "last"),
            "{:max Number}".to_string(),
        );
        let emitted = emit_module_with_html_types(&source, expr_types);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted.code.contains("previous != null"),
            "{}",
            emitted.code
        );
        assert!(
            emitted.code.contains("previous.max + 1"),
            "{}",
            emitted.code
        );
        assert!(emitted.code.contains(": 0"), "{}", emitted.code);
    }

    #[test]
    fn keeps_some_check_for_projected_state_fields_in_templates() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (if (some? state.spec)\n\
                   #html <main>{state.spec.title}</main>\n\
                   #html <p>empty</p>))",
        );
        let mut expr_types = BTreeMap::new();
        expr_types.insert(
            source.forms[0].span.start,
            "(Fn [{:spec Nil}] Html)".to_string(),
        );
        let emitted = emit_module_with_html_types(&source, expr_types);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted.code.contains("state.spec != null"),
            "{}",
            emitted.code
        );
        assert!(!emitted.code.contains("false ?"), "{}", emitted.code);
    }

    #[test]
    fn emits_safe_get_for_optional_record_fields() {
        let source = syntax::parse_source("(defn maybe-max [zone] (get zone :max))");
        let mut expr_types = BTreeMap::new();
        expr_types.insert(
            source.forms[0].span.start,
            "(Fn [(Option {:max Number})] (Option Number))".to_string(),
        );
        let emitted = emit_module_with_html_types(&source, expr_types);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("return (zone?.max ?? null);"));
        assert!(!emitted.code.contains("return zone.max;"));
    }

    #[test]
    fn keeps_runtime_predicates_for_typed_dynamic_values() {
        let source = syntax::parse_source(
            "(defn already-number? [value] (number? value))\n\
             (defn guarded-inc [value]\n  (if (number? value) (+ value 1) 0))\n\
             (defn nonzero-number? [value]\n  (and (number? value) (not (= value 0))))",
        );
        let mut expr_types = BTreeMap::new();
        expr_types.insert(source.forms[0].span.start, "(Fn [Number] Bool)".to_string());
        expr_types.insert(
            source.forms[1].span.start,
            "(Fn [Number] Number)".to_string(),
        );
        expr_types.insert(source.forms[2].span.start, "(Fn [Number] Bool)".to_string());
        let emitted = emit_module_with_html_types(&source, expr_types);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("Number.isFinite(value)"));
        assert!(emitted.code.contains("value + 1"));
        assert!(!emitted.code.contains("return true;"));
    }

    #[test]
    fn emits_date_helpers() {
        let source = syntax::parse_source(
            "(defn label [timestamp]\n  (let [start (date-start-of-week timestamp)]\n    (date-format start :month-day)))\n\
             (defn iso [timestamp] (date-format timestamp :iso-date))\n\
             (defn log-label [timestamp] (date-format timestamp :month-day-time))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("const __day = __date.getDay();"));
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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".find("));
        assert!(emitted.code.contains("[0] ?? null"));
        assert!(emitted.code.contains("[1] ?? null"));
        assert!(emitted.code.contains(".at(-1) ?? null"));
        assert!(emitted.code.contains(".slice(0, -(1))"));
        assert!(emitted.code.contains("Array.from({ length: __count }"));
        assert!(emitted.code.contains("for (let __closkell_reduce_index"));
        assert!(emitted.code.contains("let __closkell_reduce_acc"));
        assert!(
            emitted
                .code
                .contains("[__item, ...(Array.isArray(__list) ? __list : [])]")
        );
        assert!(emitted.code.contains("__list.slice(1)"));
        assert!(emitted.code.contains("Array.isArray(sample_list)"));
    }

    #[test]
    fn emits_value_equality_and_identity_predicates() {
        let source = syntax::parse_source(
            "(def equal-records (= {:items [1 2] :tag :ok} {:tag :ok :items [1 2]}))\n\
             (def shared-identity (let [value {:items [1 2]}] (identical? value value)))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("const __eq = (__left, __right)"));
        assert!(emitted.code.contains("__left instanceof Set"));
        assert!(emitted.code.contains("__left instanceof Map"));
        assert!(emitted.code.contains("Object.is(__values[__index - 1]"));
    }

    #[test]
    fn emits_collection_transforms() {
        let source = syntax::parse_source(
            "(defn sample [entries]\n  {:visible (filter entries (fn [entry] (not (some? entry.hiddenAt))))\n   :bars (map (take-last (sort-by entries (fn [entry] entry.stoppedAt)) 2)\n              (fn [entry] {:label entry.id :value entry.durationMs}))\n   :ranked (map-indexed (sort-by-desc entries (fn [entry] entry.stoppedAt))\n              (fn [entry index] {:id entry.id :rank (+ index 1)}))\n   :custom (sort-with entries (fn [first second] (- second.stoppedAt first.stoppedAt)))\n   :types (sort-with [\"Strength\" \"LISS\"] (fn [first second] (locale-compare first second)))\n   :allTyped (every? entries (fn [entry] (some? entry.exerciseType)))\n   :page (slice entries 0 2)\n   :hasSelected (any? entries (fn [entry] (= entry.id \"warmup\")))\n   :appended (conj entries {:id \"next\"})})",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".filter((entry) =>"));
        assert!(emitted.code.contains(".map((entry) =>"));
        assert!(emitted.code.contains(".map((entry) => ({"));
        assert!(emitted.code.contains(".map((entry, index) =>"));
        assert!(emitted.code.contains(".some((__item) =>"));
        assert!(emitted.code.contains(".every((__item) =>"));
        assert!(emitted.code.contains("[...entries].sort("));
        assert!(emitted.code.contains("(__left, __right))"));
        assert!(emitted.code.contains(".localeCompare("));
        assert!(emitted.code.contains(".slice(-(2))"));
        assert!(emitted.code.contains(".slice(0, 2)"));
        assert!(emitted.code.contains("__next.push(...__items);"));
        assert!(emitted.code.contains("entries, { id: \"next\" }"));
    }

    #[test]
    fn emits_range_map_as_single_loop() {
        let source = syntax::parse_source(
            "(defn rows [start count]\n\
               (map (range start (+ start count))\n\
                    (fn [id] {:id id :label (str \"row \" id)})))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("const __closkell_range_result"));
        assert!(emitted.code.contains("for (let __closkell_range_index"));
        assert!(!emitted.code.contains("Array.from({ length: __count }"));
        assert!(!emitted.code.contains(".map((__item) =>"));
    }

    #[test]
    fn emits_literal_callback_let_as_arrow_block() {
        let source = syntax::parse_source(
            "(defn select [rows id]\n\
               (map rows\n\
                    (fn [row]\n\
                      (let [next-selected (= row.id id)]\n\
                        (if (= row.selected next-selected)\n\
                            row\n\
                            (assoc row :selected next-selected))))))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".map((row) => { const next_selected"));
        assert!(emitted.code.contains("return"));
        assert!(emitted.code.contains("row.selected"));
        assert!(emitted.code.contains("next_selected"));
        assert!(!emitted.code.contains("(() => { const next_selected"));
    }

    #[test]
    fn emits_simple_vector_append_reduce_as_concat() {
        let source = syntax::parse_source(
            "(defn append-all [rows more]\n\
               (reduce more rows\n\
                 (fn [acc row]\n\
                   (conj acc row))))",
        );
        let mut expr_types = BTreeMap::new();
        expr_types.insert(
            source.forms[0].span.start,
            "(Fn [(Vector Number) (Vector Number)] (Vector Number))".to_string(),
        );
        let emitted = emit_module_with_html_types(&source, expr_types);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__acc.concat(__items)"));
        assert!(!emitted.code.contains("for (let __closkell_reduce_index"));
    }

    #[test]
    fn function_specialization_emits_required_helpers() {
        let source = syntax::parse_source(
            "(defn summarize [value]\n\
               {:object? (object? value)\n\
                :entries (object-entries value)\n\
                :count (count value)\n\
                 :none Cmd.none})",
        );
        let emitted = emit_function_specialization(
            &source,
            BTreeMap::new(),
            "summarize",
            "summarize_specialized",
            EmitOptions::default(),
        );

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("const __closkellIsObject"));
        assert!(emitted.code.contains("const __closkellObjectEntries"));
        assert!(emitted.code.contains("const __closkellCount"));
        assert!(emitted.code.contains("const __closkellNone"));
        assert!(emitted.code.contains("function summarize_specialized"));
    }

    #[test]
    fn emits_append_reduce_as_push_loop() {
        let source = syntax::parse_source(
            "(defn append-ops [items]\n\
               (reduce-indexed (rest items)\n\
                 []\n\
                 (fn [ops item index]\n\
                   (let [previous (nth items index)\n\
                         total (+ previous item)]\n\
                     (conj ops {:index index :total total} item)))))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("for (let __closkell_reduce_index"));
        assert!(emitted.code.contains(".push(...__closkell_append_items"));
        assert!(
            emitted
                .code
                .contains("Array.from(__closkell_reduce_initial")
        );
        assert!(
            !emitted.code.contains(".reduce((__acc"),
            "append-only reducer should not emit native reduce:\n{}",
            emitted.code
        );
    }

    #[test]
    fn emits_helper_append_reduce_as_push_loop() {
        let source = syntax::parse_source(
            "(defn append-segment [ops items item index]\n\
               (if (= index 0)\n\
                   ops\n\
                   (let [previous (nth items (- index 1))]\n\
                     (conj ops previous item))))\n\
             (defn append-all [items]\n\
               (reduce-indexed items []\n\
                 (fn [ops item index]\n\
                   (append-segment ops items item index))))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("for (let __closkell_reduce_index"));
        assert!(emitted.code.contains(".push(...__closkell_append_items"));
        assert!(
            !emitted.code.contains(".reduce((__acc"),
            "helper append reducer should not emit native reduce:\n{}",
            emitted.code
        );
    }

    #[test]
    fn specialized_helper_append_reduce_keeps_replacement_call() {
        let source = syntax::parse_source(
            "(defn append-segment [ops items item index]\n\
               (if (= index 0)\n\
                   ops\n\
                   (let [previous (nth items (- index 1))]\n\
                     (conj ops previous item))))\n\
             (defn append-all [items]\n\
               (reduce-indexed items []\n\
                 (fn [ops item index]\n\
                   (append-segment ops items item index))))",
        );
        let mut options = EmitOptions::default();
        options.direct_call_replacements.insert(
            "append-segment".to_string(),
            "append_segment_fast".to_string(),
        );
        options.omit_replaced_defn_exports = true;
        let emitted = emit_module_with_types_and_options(&source, BTreeMap::new(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("for (let __closkell_reduce_index"));
        assert!(
            emitted
                .code
                .contains("append_segment_fast(ops, items, item, index)"),
            "specialized reducer should call the replacement helper:\n{}",
            emitted.code
        );
        assert!(
            !emitted.code.contains(".push(...__closkell_append_items"),
            "specialized reducer must not inline the stale generic helper body:\n{}",
            emitted.code
        );
        assert!(
            !emitted.code.contains("export function append_segment"),
            "replaced generic helper should not be emitted as dead export:\n{}",
            emitted.code
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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("export const tags = new Set([\"steady\", \"zone2\", \"steady\"]);")
        );
        assert!(
            emitted
                .code
                .contains("new Set([...__collection, ...__items])")
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
    fn projected_collection_fields_use_runtime_shape_for_collection_ops() {
        let source = syntax::parse_source(
            "(def init {:tags #{\"steady\"} :tick 0})\n\
             (defn update [state]\n\
               (assoc state :tags (conj state.tags \"tempo\")))\n\
             (defn view [state]\n\
               #html <button>{(str \"Tags \" (count state.tags) \" \" (if (contains? state.tags \"tempo\") \"tempo\" \"steady\") \" \" (set? state.tags))}</button>)",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("new Set([...__collection, ...__items])"),
            "projected field conj should preserve runtime Set values:\n{}",
            emitted.code
        );
        assert!(
            emitted.code.contains("__closkellCount(state.tags)"),
            "projected field count should use the generic count helper:\n{}",
            emitted.code
        );
        assert!(
            emitted
                .code
                .contains("__collection instanceof Set || __collection instanceof Map"),
            "projected field contains? should use runtime collection dispatch:\n{}",
            emitted.code
        );
        assert!(
            emitted.code.contains("state.tags instanceof Set"),
            "projected field predicates should stay runtime checks:\n{}",
            emitted.code
        );
        assert!(
            !emitted.code.contains("state.tags.length")
                && !emitted.code.contains("state.tags.includes")
                && !emitted.code.contains("[...state.tags"),
            "projected field collection ops must not assume Vector shape:\n{}",
            emitted.code
        );
    }

    #[test]
    fn collection_predicates_do_not_fold_dynamic_payload_branches() {
        let source = syntax::parse_source(
            "(defn payload-entries [payload]\n\
               (if (vector? payload)\n\
                   payload\n\
                   (get payload :entries)))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted.code.contains("Array.isArray(payload)"),
            "dynamic payload branch should keep its runtime vector? check:\n{}",
            emitted.code
        );
        assert!(
            emitted.code.contains("payload?.entries"),
            "dynamic payload fallback should still be emitted:\n{}",
            emitted.code
        );
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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(
            "Array.from(__map.entries(), ([__key, __value]) => ({ key: __key, value: __value }))"
        ));
        assert!(emitted.code.contains("Array.from(__map.keys())"));
        assert!(emitted.code.contains("Array.from(__map.values())"));
    }

    #[test]
    fn emits_dynamic_object_get() {
        let source = syntax::parse_source(
            "(def method \"get\")\n\
             (def operation (object-get {:get {:id \"listPets\"}} method))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("__value instanceof Map ? (__value.has(__key) ? __value.get(__key) : null) : (__value?.[__key] ?? null)")
        );
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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

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
             (defn clear-message [state] (dissoc state :message))\n\
             (defn summary-value [state] (get-in state [:summary :value]))\n\
             (defn bump-summary [state] (update-in state [:summary :value] (fn [value] (+ value 1))))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("{ ...(entry), exerciseType: value, hiddenAt: 42 }")
        );
        assert!(emitted.code.contains("message: \"Imported\""));
        assert!(emitted.code.contains("delete __closkell_record_0.message;"));
        assert!(emitted.code.contains("Array.isArray(__path)"));
        assert!(
            emitted
                .code
                .contains("const __update = (__current, __index)")
        );
    }

    #[test]
    fn emits_conditional_template_slots() {
        let source = syntax::parse_source(
            "(defn view [state]\n  #html <section>{(if state.connected? #html <strong>{state.label}</strong> #html <em>Offline</em>)}</section>)",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetConditional"));
        assert!(emitted.code.contains("__closkellSetConditional(i, 0"));
        assert!(emitted.code.contains("state[\"connected?\"]"));
    }

    #[test]
    fn emits_nested_conditional_template_slots() {
        let source = syntax::parse_source(
            "(defn pane [state]\n  #html <article>{state.label}</article>)\n\
             (defn view [state]\n  #html <main>{(if (= state.view \"metrics\") #html <section>{(pane state)}</section> (if (= state.view \"log\") #html <aside>Log</aside> #html <div>Live</div>))}</main>)",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetConditional"));
        assert!(emitted.code.contains("__closkellSetConditional(i, 0"));
        assert!(emitted.code.contains("() => __closkellCreateHtmlTemplate"));
        assert!(emitted.code.contains("() => pane(state)"));
        assert!(emitted.code.contains("\"pane\""));
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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("__closkellSetConditional"));
        assert!(emitted.code.contains("live_pane(state)"));
        assert!(emitted.code.contains("log_pane(state)"));
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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("state[\"show?\"]"));
        assert!(emitted.code.contains("state.zones, (zone) => zone.id"));
        assert!(
            !emitted.code.contains("state.zone."),
            "conditional output should not invent loop-local state reads:\n{}",
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
        let emitted = emit_module_with_html(&source);

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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("import.meta.env && import.meta.env.DEV")
        );
    }

    #[test]
    fn emits_env_mode_and_regex_capture_all() {
        let source = syntax::parse_source(
            "(def mode (env-mode))\n(defn pairs [text]\n  (regex-capture-all text \"name=([^;]+);url=([^;]+)\" \"g\"))",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("import.meta.env && import.meta.env.MODE")
        );
        assert!(emitted.code.contains(".matchAll(new RegExp"));
        assert!(emitted.code.contains("__match.slice(1)"));
    }

    #[test]
    fn emits_numeric_vector_aggregates() {
        let source = syntax::parse_source(
            "(defn bounds [values]\n\
               {:min (min-of values 50)\n\
                :max (max-of values 170)\n\
                :total (sum values)})",
        );
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("let __result = Infinity"));
        assert!(emitted.code.contains("let __result = -Infinity"));
        assert!(emitted.code.contains("if (50 < __result) __result = 50;"));
        assert!(emitted.code.contains("if (170 > __result) __result = 170;"));
        assert!(emitted.code.contains("let __sum = 0;"));
        assert!(emitted.code.contains("for (const __value of __values)"));
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
        let emitted = emit_module_with_html(&source);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains(".padStart(2, \"0\")"));
        assert!(emitted.code.contains("((Math.floor(ms / 1000)) % 60)"));
        assert!(emitted.code.contains("((value + 1) % 60)"));
        assert!(emitted.code.contains("Math.abs("));
        assert!(emitted.code.contains("(minutes).toFixed(1)"));
        assert!(emitted.code.contains("Number.isFinite(__number)"));
        assert!(emitted.code.contains("catch { return 0; }"));
        assert!(emitted.code.contains("export const recovery_ms = 60_000;"));
    }
}
