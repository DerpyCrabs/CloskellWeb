use std::collections::{BTreeMap, BTreeSet, HashMap};

use syntax::{Diagnostic, Expr, ExprKind, HtmlAttrValue, HtmlNode, SourceFile, Span, format_expr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedForm {
    pub source: String,
    pub ty: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckResult {
    pub forms: Vec<TypedForm>,
    pub expr_types: BTreeMap<usize, String>,
    pub type_declarations: Vec<TypeDeclaration>,
    pub type_annotations: Vec<TypeAnnotation>,
    pub foreign_declarations: Vec<ForeignDeclaration>,
    pub bindings: Vec<ExportedBinding>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDeclaration {
    pub name: String,
    pub params: Vec<String>,
    pub schema: String,
    syntax: TypeSyntax,
    span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDeclarationReport {
    pub declarations: Vec<TypeDeclaration>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeAnnotation {
    pub name: String,
    pub schema: String,
    syntax: TypeSyntax,
    span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeAnnotationReport {
    pub annotations: Vec<TypeAnnotation>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignDeclaration {
    pub mode: String,
    pub name: String,
    pub schema: String,
    syntax: TypeSyntax,
    span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignDeclarationReport {
    pub declarations: Vec<ForeignDeclaration>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedTypeDeclaration {
    name: String,
    params: Vec<String>,
    syntax: TypeSyntax,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedBinding {
    pub name: String,
    ty: Type,
    annotated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedBinding {
    pub name: String,
    ty: Type,
    annotated: bool,
}

impl TypeDeclaration {
    pub fn import_as(&self, name: impl Into<String>) -> ImportedTypeDeclaration {
        ImportedTypeDeclaration {
            name: name.into(),
            params: self.params.clone(),
            syntax: self.syntax.clone(),
        }
    }
}

impl ExportedBinding {
    pub fn import_as(&self, name: impl Into<String>) -> ImportedBinding {
        ImportedBinding {
            name: name.into(),
            ty: self.ty.clone(),
            annotated: self.annotated,
        }
    }

    pub fn is_annotated(&self) -> bool {
        self.annotated
    }

    pub fn is_annotated_or_value(&self) -> bool {
        self.annotated || !matches!(self.ty, Type::Fn(_, _))
    }

    pub fn schema(&self) -> String {
        format_type_inner(&self.ty)
    }
}

impl ImportedBinding {
    pub fn returns_cmd(&self) -> bool {
        type_returns_cmd(&self.ty)
    }

    pub fn returns_sub(&self) -> bool {
        type_returns_sub(&self.ty)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Type {
    Var(u32),
    Number,
    String,
    Bool,
    Nil,
    Keyword(Option<String>),
    Syntax,
    Js,
    Html,
    TrustedHtml,
    Decoder(Box<Type>),
    Option(Box<Type>),
    List(Box<Type>),
    Vector(Box<Type>),
    Tuple(Vec<Type>),
    Set(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Result(Box<Type>, Box<Type>),
    Cmd(Box<Type>),
    Task(Box<Type>, Box<Type>),
    Sub(Box<Type>),
    Event(Box<Type>),
    Union(Vec<Type>),
    Record(BTreeMap<String, Type>),
    Fn(Vec<Type>, Box<Type>),
}

#[derive(Default)]
struct Inferencer {
    next_var: u32,
    subst: HashMap<u32, Type>,
    type_aliases: HashMap<String, TypeAlias>,
    html_event_msg: Option<Type>,
    expr_types: BTreeMap<usize, Type>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TypeAlias {
    params: Vec<String>,
    syntax: TypeSyntax,
}

pub fn check_source(source: &SourceFile) -> CheckResult {
    check_source_with_imports(source, &[])
}

pub fn check_source_with_imports(source: &SourceFile, imports: &[ImportedBinding]) -> CheckResult {
    check_source_with_module_imports(source, imports, &[])
}

pub fn check_source_with_module_imports(
    source: &SourceFile,
    imports: &[ImportedBinding],
    imported_types: &[ImportedTypeDeclaration],
) -> CheckResult {
    let mut inferencer = Inferencer::default();
    let mut env = HashMap::new();
    let mut forms = Vec::new();
    let mut bindings = Vec::new();
    let type_report = collect_type_declarations(source);
    let annotation_report = collect_type_annotations(source);
    let foreign_report = collect_foreign_declarations(source);
    let mut type_aliases = imported_types
        .iter()
        .map(|declaration| {
            (
                declaration.name.clone(),
                TypeAlias {
                    params: declaration.params.clone(),
                    syntax: declaration.syntax.clone(),
                },
            )
        })
        .collect::<HashMap<_, _>>();
    type_aliases.extend(type_report.declarations.iter().map(|declaration| {
        (
            declaration.name.clone(),
            TypeAlias {
                params: declaration.params.clone(),
                syntax: declaration.syntax.clone(),
            },
        )
    }));
    let annotations_by_name = annotation_report
        .annotations
        .iter()
        .map(|annotation| (annotation.name.clone(), annotation))
        .collect::<HashMap<_, _>>();
    let mut defined_names = BTreeSet::new();

    inferencer
        .diagnostics
        .extend(type_report.diagnostics.iter().cloned());
    inferencer
        .diagnostics
        .extend(annotation_report.diagnostics.iter().cloned());
    inferencer
        .diagnostics
        .extend(foreign_report.diagnostics.iter().cloned());
    inferencer.type_aliases = type_aliases.clone();
    inferencer.html_event_msg = annotations_by_name.get("update").and_then(|annotation| {
        update_message_type_syntax(&annotation.syntax).and_then(|syntax| {
            inferencer.type_syntax_to_type(syntax, &type_aliases, annotation.span)
        })
    });

    for declaration in &type_report.declarations {
        if declaration.params.is_empty() {
            continue;
        }
        let substituted = substitute_type_syntax(
            &declaration.syntax,
            &type_parameter_validation_bindings(&declaration.params),
        );
        inferencer.type_syntax_to_type(&substituted, &type_aliases, declaration.span);
    }

    for import in imports {
        let ty = inferencer.instantiate_imported_type(&import.ty);
        env.insert(import.name.clone(), ty);
    }

    for form in &source.forms {
        if let Some(import) = parse_import_form(form) {
            match import {
                Ok(import) => {
                    let is_js_import = !is_closkell_import_path(&import.path);
                    for name in import.names {
                        env.entry(name).or_insert_with(|| {
                            if is_js_import {
                                Type::Js
                            } else {
                                inferencer.fresh()
                            }
                        });
                    }
                }
                Err(diagnostic) => inferencer.diagnostics.push(diagnostic),
            }
        }
    }

    for declaration in &foreign_report.declarations {
        if let Some(ty) =
            inferencer.type_syntax_to_type(&declaration.syntax, &type_aliases, declaration.span)
        {
            env.insert(declaration.name.clone(), ty);
        }
    }

    for form in &source.forms {
        if parse_import_form(form).is_some()
            || is_type_form(form)
            || is_ann_form(form)
            || is_foreign_form(form)
        {
            continue;
        }

        let form_name = definition_name(form);
        let ty = if let Some(name) = form_name {
            defined_names.insert(name.to_string());
            if let Some(annotation) = annotations_by_name.get(name) {
                if let Some(expected) = inferencer.type_syntax_to_type(
                    &annotation.syntax,
                    &type_aliases,
                    annotation.span,
                ) {
                    inferencer.infer_definition_with_expected(form, &mut env, expected)
                } else {
                    inferencer.infer_expr(form, &mut env)
                }
            } else {
                inferencer.infer_expr(form, &mut env)
            }
        } else {
            inferencer.infer_expr(form, &mut env)
        };
        if let Some(name) = definition_name(form) {
            bindings.push(ExportedBinding {
                name: name.to_string(),
                ty: inferencer.resolve(ty.clone()),
                annotated: annotations_by_name.contains_key(name),
            });
        }
        forms.push(TypedForm {
            source: format_expr(form),
            ty: inferencer.format_type(&ty),
        });
    }

    for annotation in &annotation_report.annotations {
        if !defined_names.contains(&annotation.name) {
            inferencer.diagnostics.push(Diagnostic::error(
                annotation.span,
                format!(
                    "type annotation `{}` does not match any def or defn",
                    annotation.name
                ),
            ));
        }
    }

    let expr_type_entries = inferencer
        .expr_types
        .iter()
        .map(|(offset, ty)| (*offset, ty.clone()))
        .collect::<Vec<_>>();
    let expr_types = expr_type_entries
        .iter()
        .map(|(offset, ty)| (*offset, inferencer.format_type(ty)))
        .collect();

    CheckResult {
        forms,
        expr_types,
        type_declarations: type_report.declarations,
        type_annotations: annotation_report.annotations,
        foreign_declarations: foreign_report.declarations,
        bindings,
        diagnostics: inferencer.diagnostics,
    }
}

pub fn collect_type_declarations(source: &SourceFile) -> TypeDeclarationReport {
    let mut declarations = Vec::new();
    let mut diagnostics = Vec::new();
    let mut names = BTreeSet::new();

    for form in &source.forms {
        let Some(parsed) = parse_type_declaration_form(form) else {
            continue;
        };
        match parsed {
            Ok(declaration) => {
                if !names.insert(declaration.name.clone()) {
                    diagnostics.push(Diagnostic::error(
                        form.span,
                        format!("duplicate type declaration `{}`", declaration.name),
                    ));
                }
                declarations.push(declaration);
            }
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    TypeDeclarationReport {
        declarations,
        diagnostics,
    }
}

pub fn collect_type_annotations(source: &SourceFile) -> TypeAnnotationReport {
    let mut annotations = Vec::new();
    let mut diagnostics = Vec::new();
    let mut names = BTreeSet::new();

    for form in &source.forms {
        let Some(parsed) = parse_type_annotation_form(form) else {
            continue;
        };
        match parsed {
            Ok(annotation) => {
                if !names.insert(annotation.name.clone()) {
                    diagnostics.push(Diagnostic::error(
                        form.span,
                        format!("duplicate type annotation `{}`", annotation.name),
                    ));
                }
                annotations.push(annotation);
            }
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    TypeAnnotationReport {
        annotations,
        diagnostics,
    }
}

pub fn collect_foreign_declarations(source: &SourceFile) -> ForeignDeclarationReport {
    let mut declarations = Vec::new();
    let mut diagnostics = Vec::new();
    let mut names = BTreeSet::new();

    for form in &source.forms {
        let Some(parsed) = parse_foreign_declaration_form(form) else {
            continue;
        };
        match parsed {
            Ok(declaration) => {
                if !names.insert(declaration.name.clone()) {
                    diagnostics.push(Diagnostic::error(
                        form.span,
                        format!("duplicate foreign declaration `{}`", declaration.name),
                    ));
                }
                declarations.push(declaration);
            }
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    ForeignDeclarationReport {
        declarations,
        diagnostics,
    }
}

impl Inferencer {
    fn instantiate_imported_type(&mut self, ty: &Type) -> Type {
        let mut vars = HashMap::new();
        self.instantiate_imported_type_inner(ty, &mut vars)
    }

    fn instantiate_imported_type_inner(
        &mut self,
        ty: &Type,
        vars: &mut HashMap<u32, Type>,
    ) -> Type {
        match ty {
            Type::Var(id) => {
                if let Some(existing) = vars.get(id) {
                    existing.clone()
                } else {
                    let fresh = self.fresh();
                    vars.insert(*id, fresh.clone());
                    fresh
                }
            }
            Type::Option(inner) => {
                Type::Option(Box::new(self.instantiate_imported_type_inner(inner, vars)))
            }
            Type::Decoder(inner) => {
                Type::Decoder(Box::new(self.instantiate_imported_type_inner(inner, vars)))
            }
            Type::List(inner) => {
                Type::List(Box::new(self.instantiate_imported_type_inner(inner, vars)))
            }
            Type::Vector(inner) => {
                Type::Vector(Box::new(self.instantiate_imported_type_inner(inner, vars)))
            }
            Type::Set(inner) => {
                Type::Set(Box::new(self.instantiate_imported_type_inner(inner, vars)))
            }
            Type::Map(key, value) => Type::Map(
                Box::new(self.instantiate_imported_type_inner(key, vars)),
                Box::new(self.instantiate_imported_type_inner(value, vars)),
            ),
            Type::Result(ok, err) => Type::Result(
                Box::new(self.instantiate_imported_type_inner(ok, vars)),
                Box::new(self.instantiate_imported_type_inner(err, vars)),
            ),
            Type::Cmd(msg) => Type::Cmd(Box::new(self.instantiate_imported_type_inner(msg, vars))),
            Type::Task(err, ok) => Type::Task(
                Box::new(self.instantiate_imported_type_inner(err, vars)),
                Box::new(self.instantiate_imported_type_inner(ok, vars)),
            ),
            Type::Sub(msg) => Type::Sub(Box::new(self.instantiate_imported_type_inner(msg, vars))),
            Type::Event(msg) => {
                Type::Event(Box::new(self.instantiate_imported_type_inner(msg, vars)))
            }
            Type::Tuple(items) => Type::Tuple(
                items
                    .iter()
                    .map(|item| self.instantiate_imported_type_inner(item, vars))
                    .collect(),
            ),
            Type::Union(variants) => Type::Union(
                variants
                    .iter()
                    .map(|variant| self.instantiate_imported_type_inner(variant, vars))
                    .collect(),
            ),
            Type::Record(fields) => Type::Record(
                fields
                    .iter()
                    .map(|(name, field)| {
                        (
                            name.clone(),
                            self.instantiate_imported_type_inner(field, vars),
                        )
                    })
                    .collect(),
            ),
            Type::Fn(args, ret) => Type::Fn(
                args.iter()
                    .map(|arg| self.instantiate_imported_type_inner(arg, vars))
                    .collect(),
                Box::new(self.instantiate_imported_type_inner(ret, vars)),
            ),
            Type::Number => Type::Number,
            Type::String => Type::String,
            Type::Bool => Type::Bool,
            Type::Nil => Type::Nil,
            Type::Keyword(name) => Type::Keyword(name.clone()),
            Type::Syntax => Type::Syntax,
            Type::Js => Type::Js,
            Type::Html => Type::Html,
            Type::TrustedHtml => Type::TrustedHtml,
        }
    }

    fn infer_expr(&mut self, expr: &Expr, env: &mut HashMap<String, Type>) -> Type {
        let ty = match &expr.kind {
            ExprKind::Nil => Type::Nil,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::Number(_) => Type::Number,
            ExprKind::String(_) => Type::String,
            ExprKind::Keyword(name) => Type::Keyword(Some(name.clone())),
            ExprKind::Symbol(name) => self.infer_symbol(expr.span, name, env),
            ExprKind::Vector(items) => self.infer_vector(items, env),
            ExprKind::Set(items) => self.infer_collection(items, env, CollectionKind::Set),
            ExprKind::Map(entries) => self.infer_map(entries, env),
            ExprKind::Quote(_)
            | ExprKind::QuasiQuote(_)
            | ExprKind::Unquote(_)
            | ExprKind::UnquoteSplicing(_) => Type::Syntax,
            ExprKind::HtmlTemplate(node) => self.infer_html_template(node, env),
            ExprKind::List(items) => self.infer_list(expr.span, items, env),
        };
        self.expr_types.insert(expr.span.start, ty.clone());
        ty
    }

    fn infer_list(&mut self, span: Span, items: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        let Some((head, args)) = items.split_first() else {
            return Type::List(Box::new(self.fresh()));
        };

        if let ExprKind::Symbol(name) = &head.kind {
            match name.as_str() {
                "def" => return self.infer_def(span, args, env),
                "defn" => return self.infer_defn(span, args, env),
                "import" => return Type::Nil,
                "fn" => return self.infer_fn(span, args, env),
                "let" => return self.infer_let(span, args, env),
                "if" => return self.infer_if(span, args, env),
                "match" => return self.infer_match(span, args, env),
                "do" => return self.infer_do(args, env),
                "unsafe-cast" => return self.infer_unsafe_cast(name, args, env),
                "Msg.of" => return self.infer_msg_of(name, args, env),
                "Msg.with" => return self.infer_msg_with(name, args, env),
                "Msg.with2" => return self.infer_msg_with2(name, args, env),
                "Msg.mapper" => return self.infer_msg_mapper(name, args, env),
                "Event.prevent" | "Event.stop" | "Event.prevent-stop" => {
                    return self.infer_event_control(name, args, env);
                }
                "Cmd.batch" => return self.infer_cmd_batch(name, args, env),
                "Cmd.storage/get" => return self.infer_cmd_storage_get(name, args, env),
                "Cmd.storage/set" => return self.infer_cmd_storage_set(name, args, env),
                "Cmd.storage/set-silent" => {
                    return self.infer_cmd_storage_set_silent(name, args, env);
                }
                "Cmd.time/now" => {
                    return self.infer_cmd_payload_mapper(
                        name,
                        args,
                        env,
                        "time/now",
                        Type::Number,
                    );
                }
                "Cmd.random/number" => return self.infer_cmd_random_number(name, args, env),
                "Cmd.timer/every" => return self.infer_cmd_timer(name, args, env, "timer/every"),
                "Cmd.timer/after" => return self.infer_cmd_timer(name, args, env, "timer/after"),
                "Cmd.timer/cancel" => return self.infer_cmd_timer_cancel(name, args, env),
                "Cmd.animation/frame" => return self.infer_cmd_animation_frame(name, args, env),
                "Cmd.animation/cancel" => return self.infer_cmd_animation_cancel(name, args, env),
                "Cmd.dom-ref/click" => {
                    return self.infer_cmd_dom_ref_action(name, args, env, "dom-ref/click");
                }
                "Cmd.dom-ref/focus" => {
                    return self.infer_cmd_dom_ref_action(name, args, env, "dom-ref/focus");
                }
                "Cmd.file/read-selected" => {
                    return self.infer_cmd_file_read_selected(name, args, env);
                }
                "Cmd.file/download" => return self.infer_cmd_file_download(name, args, env),
                "Cmd.canvas/draw" => return self.infer_cmd_canvas_draw(name, args, env),
                "Cmd.dom-ref/measure" => return self.infer_cmd_dom_ref_measure(name, args, env),
                "Cmd.dom-ref/resize-watch" => return self.infer_cmd_resize_watch(name, args, env),
                "Cmd.bluetooth/connect-heart-rate" => {
                    return self.infer_cmd_bluetooth_connect_heart_rate(name, args, env);
                }
                "Cmd.bluetooth/disconnect" => {
                    return self.infer_cmd_bluetooth_disconnect(name, args, env);
                }
                "Cmd.simulation/heart-rate" => {
                    return self.infer_cmd_simulation_heart_rate(name, args, env);
                }
                "Cmd.simulation/stop" => return self.infer_cmd_simulation_stop(name, args, env),
                "Task.succeed" => return self.infer_task_succeed(name, args, env),
                "Task.fail" => return self.infer_task_fail(name, args, env),
                "Task.map" => return self.infer_task_map(name, args, env),
                "Task.map-error" => return self.infer_task_map_error(name, args, env),
                "Task.and-then" => return self.infer_task_and_then(name, args, env),
                "Task.perform" => return self.infer_task_perform(name, args, env),
                "Http.get-text" => return self.infer_http_get_text(name, args, env),
                "Http.get-json" => return self.infer_http_get_json(name, args, env),
                "scope-update" => return self.infer_scope_update(name, args, env),
                "scope-subscriptions" => {
                    return self.infer_scope_subscriptions(name, args, env);
                }
                "scope-view" => return self.infer_scope_view(name, args, env),
                "Sub.batch" => return self.infer_sub_batch(name, args, env),
                "Sub.timer/every" => return self.infer_sub_timer_every(name, args, env),
                "Sub.media-query" => {
                    return self.infer_sub_change(name, args, env, "sub/media-query");
                }
                "Sub.window/event" => return self.infer_sub_window_event(name, args, env),
                "Sub.window/event-with" => {
                    return self.infer_sub_window_event_with(name, args, env);
                }
                "Sub.dom-ref/resize" => {
                    return self.infer_sub_change(name, args, env, "sub/dom-ref/resize");
                }
                "describe" => return self.infer_test_group(name, args, env),
                "test" => return self.infer_test_case(name, args, env),
                "expect=" => return self.infer_expect_equal(name, args, env, false),
                "expect-not=" => return self.infer_expect_equal(name, args, env, true),
                "expect-ok" => return self.infer_expect_ok(name, args, env),
                "expect-err" => return self.infer_expect_err(name, args, env),
                "expect-some" => return self.infer_expect_some(name, args, env),
                "expect-nil" => return self.infer_expect_nil(name, args, env),
                "expect-match" => return self.infer_expect_match(name, args, env),
                "expect-throws" => return self.infer_expect_throws(name, args, env),
                "render-to-string" => return self.infer_render_to_string(name, args, env),
                "render" => return self.infer_render_harness(name, args, env),
                "rerender" => return self.infer_rerender_harness(name, args, env),
                "dispose" => return self.infer_dispose_harness(name, args, env),
                "find-all" => return self.infer_query_helper(name, args, env, true),
                "text" | "html" => return self.infer_harness_string_query(name, args, env),
                "attr" | "style" => return self.infer_harness_attr_query(name, args, env),
                "class?" => return self.infer_harness_class_query(name, args, env),
                "messages" | "commands" => return self.infer_harness_records(name, args, env),
                "fire.event" => return self.infer_fire_event(name, args, env),
                "fire.click" | "fire.keydown" | "fire.pointerdown" => {
                    return self.infer_fire_selector_event(name, args, env);
                }
                "fire.input" | "fire.change" => {
                    return self.infer_fire_value_event(name, args, env);
                }
                "type" => return Type::Nil,
                "foreign" => return Type::Nil,
                "+" | "-" | "*" | "/" | "max" | "min" => {
                    return self.infer_numeric_call(name, args, env);
                }
                "min-of" | "max-of" => {
                    return self.infer_numeric_vector_aggregate(name, args, env, true);
                }
                "sum" => return self.infer_numeric_vector_aggregate(name, args, env, false),
                "%" | "mod" => return self.infer_binary_number(name, args, env),
                "abs" | "round" | "floor" | "ceil" => {
                    return self.infer_unary_number(name, args, env);
                }
                "date-start-of-week" | "date-start-of-month" | "date-month" | "date-day" => {
                    return self.infer_unary_number(name, args, env);
                }
                "date-add-days" => return self.infer_binary_number(name, args, env),
                "to-number" => return self.infer_to_number(name, args, env),
                "to-fixed" => return self.infer_to_fixed(name, args, env),
                "date-format" => return self.infer_date_format(name, args, env),
                "=" | "<" | ">" | "<=" | ">=" => {
                    return self.infer_comparison_call(name, args, env);
                }
                "identical?" => return self.infer_identity_call(name, args, env),
                "not" => return self.infer_unary_bool(name, args, env),
                "and" | "or" => return self.infer_bool_call(name, args, env),
                "env-dev?" => return self.infer_zero_arg_bool(name, args),
                "env-mode" => return self.infer_zero_arg_string(name, args),
                "fail" => return self.infer_fail(name, args, env),
                "ok" => return self.infer_ok(name, args, env),
                "err" => return self.infer_err(name, args, env),
                "ok?" | "err?" => return self.infer_result_predicate(name, args, env),
                "result-value" => return self.infer_result_projection(name, args, env, false),
                "result-error" => return self.infer_result_projection(name, args, env, true),
                "unwrap-or" => return self.infer_unwrap_or(name, args, env),
                "hash-map" => return self.infer_hash_map(name, args, env),
                "map-get" => return self.infer_map_get(name, args, env),
                "map-assoc" => return self.infer_map_assoc(name, args, env),
                "map-dissoc" => return self.infer_map_dissoc(name, args, env),
                "map-entries" => return self.infer_map_entries(name, args, env),
                "map-keys" => return self.infer_map_keys(name, args, env),
                "map-values" => return self.infer_map_values(name, args, env),
                "get-in" => return self.infer_get_in(name, args, env),
                "assoc" => return self.infer_assoc(name, args, env),
                "merge" => return self.infer_merge(name, args, env),
                "dissoc" => return self.infer_dissoc(name, args, env),
                "update-in" => return self.infer_update_in(name, args, env),
                "count" => return self.infer_count(name, args, env),
                "empty?" => return self.infer_empty(name, args, env),
                "some?" => return self.infer_some(name, args, env),
                "nil?" | "number?" | "string?" | "bool?" | "keyword?" | "list?" | "vector?"
                | "set?" | "map?" | "object?" => return self.infer_predicate(name, args, env),
                "get" => return self.infer_get(name, args, env),
                "object-get" => return self.infer_object_get(name, args, env),
                "first" | "second" => return self.infer_ordered_access(name, args, env),
                "nth" => return self.infer_nth(name, args, env),
                "last" => return self.infer_last(name, args, env),
                "cons" => return self.infer_cons(name, args, env),
                "rest" => return self.infer_rest(name, args, env),
                "find" => return self.infer_find_overloaded(name, args, env),
                "map" => return self.infer_map_transform(name, args, env, false),
                "map-indexed" => return self.infer_map_transform(name, args, env, true),
                "filter" => return self.infer_filter(name, args, env),
                "any?" | "every?" => return self.infer_vector_predicate_call(name, args, env),
                "range" => return self.infer_range(name, args, env),
                "conj" => return self.infer_conj(name, args, env),
                "disj" => return self.infer_disj(name, args, env),
                "set-values" => return self.infer_set_values(name, args, env),
                "sort-by" | "sort-by-desc" => return self.infer_sort_by(name, args, env),
                "sort-with" => return self.infer_sort_with(name, args, env),
                "slice" => return self.infer_slice(name, args, env),
                "drop-last" => return self.infer_drop_last(name, args, env),
                "take-last" => return self.infer_take_last(name, args, env),
                "reduce" => return self.infer_reduce(name, args, env),
                "reduce-indexed" => return self.infer_reduce_indexed(name, args, env),
                "trim" | "lower-case" => return self.infer_unary_string(name, args, env),
                "split" => return self.infer_split(name, args, env),
                "join" => return self.infer_join(name, args, env),
                "starts-with?" | "ends-with?" => {
                    return self.infer_binary_string_predicate(name, args, env);
                }
                "to-radix" => return self.infer_to_radix(name, args, env),
                "string-slice" => return self.infer_string_slice(name, args, env),
                "pad-start" => return self.infer_pad_start(name, args, env),
                "regex-test?" => return self.infer_regex_test(name, args, env),
                "includes?" => return self.infer_includes(name, args, env),
                "contains?" => return self.infer_contains(name, args, env),
                "locale-compare" => return self.infer_locale_compare(name, args, env),
                "json-stringify" => return self.infer_json_stringify(name, args, env),
                "json-parse" => return self.infer_json_parse(name, args, env),
                "json-parse-result" => return self.infer_json_parse_result(name, args, env),
                "decoder-string" => {
                    return self.infer_zero_arg_decoder(name, args, Type::String);
                }
                "decoder-number" => {
                    return self.infer_zero_arg_decoder(name, args, Type::Number);
                }
                "decoder-bool" => return self.infer_zero_arg_decoder(name, args, Type::Bool),
                "decoder-keyword" => {
                    return self.infer_zero_arg_decoder(name, args, Type::Keyword(None));
                }
                "decoder-literal" => return self.infer_decoder_literal(name, args, env),
                "decoder-optional" => return self.infer_decoder_optional(name, args, env),
                "decoder-vector" => return self.infer_decoder_vector(name, args, env),
                "decoder-record" => return self.infer_decoder_record(name, args, env),
                "decode" => return self.infer_decode(name, args, env),
                "object-entries" => return self.infer_object_entries(name, args, env),
                "object-keys" => return self.infer_object_keys(name, args, env),
                "object-values" => return self.infer_object_values(name, args, env),
                "object-assoc" => return self.infer_object_assoc(name, args, env),
                "object-dissoc" => return self.infer_object_dissoc(name, args, env),
                "encode-uri-component"
                | "decode-uri-component"
                | "base64-encode"
                | "base64-decode" => return self.infer_unary_string(name, args, env),
                "url-resolve" => return self.infer_url_resolve(name, args, env),
                "url-without-hash" | "url-origin" | "url-hostname" | "url-pathname" => {
                    return self.infer_url_part(name, args, env);
                }
                "browser-current-url" => return self.infer_zero_arg_string(name, args),
                "url-search-param" => {
                    return self.infer_fixed_string_args(name, args, env, 2, Type::String);
                }
                "url-set-search-param" | "url-set-deep-object-param" => {
                    return self.infer_fixed_string_args(name, args, env, 3, Type::String);
                }
                "history-replace-search-param" | "browser-set-cookie" => {
                    return self.infer_fixed_string_args(name, args, env, 2, Type::Nil);
                }
                "history-write-route" => {
                    if args.len() != 3 {
                        self.diagnostics.push(Diagnostic::error(
                            args.first().map_or(Span::default(), |arg| arg.span),
                            format!("{} expects 3 arguments, got {}", name, args.len()),
                        ));
                    }
                    for arg in args {
                        self.infer_expr(arg, env);
                    }
                    return Type::Nil;
                }
                "browser-theme-initial" => {
                    return self.infer_fixed_string_args(name, args, env, 1, Type::String);
                }
                "browser-theme-toggle" => {
                    return self.infer_fixed_string_args(name, args, env, 2, Type::String);
                }
                "auth-storage-load" => {
                    let result = self.fresh();
                    return self.infer_fixed_string_args(name, args, env, 1, result);
                }
                "auth-storage-persist" => {
                    if args.len() != 2 {
                        self.diagnostics.push(Diagnostic::error(
                            args.first().map_or(Span::default(), |arg| arg.span),
                            format!("{} expects 2 arguments, got {}", name, args.len()),
                        ));
                    }
                    for arg in args {
                        self.infer_expr(arg, env);
                    }
                    return Type::Nil;
                }
                "resolve-token-expiry" => {
                    if args.len() != 2 {
                        self.diagnostics.push(Diagnostic::error(
                            args.first().map_or(Span::default(), |arg| arg.span),
                            format!("{} expects 2 arguments, got {}", name, args.len()),
                        ));
                    }
                    for arg in args {
                        self.infer_expr(arg, env);
                    }
                    return Type::Option(Box::new(Type::Number));
                }
                "clipboard-text" => return self.infer_clipboard_text(name, args, env),
                "clipboard-write" => {
                    return self.infer_fixed_string_args(name, args, env, 1, Type::Nil);
                }
                "path-fill-params" => {
                    return self.infer_fixed_string_args(name, args, env, 2, Type::String);
                }
                "path-fill-param" => {
                    return self.infer_fixed_string_args(name, args, env, 3, Type::String);
                }
                "selected-file-or-blob" => {
                    let result = self.fresh();
                    return self.infer_fixed_string_args(name, args, env, 4, result);
                }
                "selected-file-by-test-id" => {
                    let result = self.fresh();
                    return self.infer_fixed_string_args(name, args, env, 1, result);
                }
                "has-selected-file" => {
                    return self.infer_fixed_string_args(name, args, env, 1, Type::Bool);
                }
                "multipart-form-body" => {
                    if args.len() != 2 {
                        self.diagnostics.push(Diagnostic::error(
                            args.first().map_or(Span::default(), |arg| arg.span),
                            format!("{} expects 2 arguments, got {}", name, args.len()),
                        ));
                    }
                    for arg in args {
                        self.infer_expr(arg, env);
                    }
                    return self.fresh();
                }
                "urlencoded-form-body" => {
                    if args.len() != 2 {
                        self.diagnostics.push(Diagnostic::error(
                            args.first().map_or(Span::default(), |arg| arg.span),
                            format!("{} expects 2 arguments, got {}", name, args.len()),
                        ));
                    }
                    for arg in args {
                        self.infer_expr(arg, env);
                    }
                    return Type::String;
                }
                "regex-capture" => return self.infer_regex_capture(name, args, env),
                "regex-capture-all" => return self.infer_regex_capture_all(name, args, env),
                "install-virtual-json-viewer" => return self.infer_zero_arg_nil(name, args),
                "str" => {
                    for arg in args {
                        self.infer_expr(arg, env);
                    }
                    return Type::String;
                }
                "list" => return self.infer_collection(args, env, CollectionKind::List),
                "vector" => return self.infer_collection(args, env, CollectionKind::Vector),
                "set" => return self.infer_collection(args, env, CollectionKind::Set),
                _ => {}
            }
        }

        let callee = self.infer_expr(head, env);
        let arg_types = args
            .iter()
            .map(|arg| self.infer_expr(arg, env))
            .collect::<Vec<_>>();
        if matches!(self.resolve(callee.clone()), Type::Js) {
            return Type::Js;
        }
        let ret = self.fresh();
        self.unify(
            callee,
            Type::Fn(arg_types, Box::new(ret.clone())),
            head.span,
        );
        self.resolve(ret)
    }

    fn infer_html_template(&mut self, node: &HtmlNode, env: &mut HashMap<String, Type>) -> Type {
        self.infer_html_node(node, env);
        Type::Html
    }

    fn infer_html_node(&mut self, node: &HtmlNode, env: &mut HashMap<String, Type>) {
        match node {
            HtmlNode::Element(element) => {
                for attr in &element.attrs {
                    match &attr.value {
                        HtmlAttrValue::Dynamic { expr, .. } => {
                            if attr.name.starts_with("on:") {
                                let mut event_env = env.clone();
                                event_env.insert("event".to_string(), self.dom_event_type());
                                let message_ty = self.infer_expr(expr, &mut event_env);
                                if let Some(expected_msg) = self.html_event_msg.clone() {
                                    self.require_html_event_message_matches(
                                        expected_msg,
                                        message_ty,
                                        expr.span,
                                        &attr.name,
                                    );
                                }
                            } else {
                                let attr_ty = self.infer_expr(expr, env);
                                if attr.name == "ref" {
                                    self.require_html_ref_attr(attr_ty, expr.span);
                                } else if attr.name == "class" {
                                    self.require_html_class_attr(attr_ty, expr.span);
                                } else if attr.name == "style" {
                                    self.require_html_style_attr(attr_ty, expr.span);
                                } else if attr.name == "innerHTML" {
                                    self.require_html_inner_html_attr(attr_ty, expr.span);
                                } else if is_boolean_html_attr(&attr.name) {
                                    self.unify(attr_ty, Type::Bool, expr.span);
                                }
                            }
                        }
                        HtmlAttrValue::Bool(_) | HtmlAttrValue::Static(_) => {
                            self.validate_static_html_attr(&attr.name, &attr.value, attr.span);
                        }
                    }
                }
                for child in &element.children {
                    self.infer_html_node(child, env);
                }
            }
            HtmlNode::Expr { expr, .. } => self.infer_html_expr(expr, env),
            HtmlNode::Text { .. } => {}
        }
    }

    fn validate_static_html_attr(&mut self, name: &str, value: &HtmlAttrValue, span: Span) {
        match value {
            HtmlAttrValue::Bool(true) => match name {
                "ref" => self.diagnostics.push(Diagnostic::error(
                    span,
                    "ref attribute requires a value; use ref=\"name\" or ref={...}",
                )),
                "class" => self.diagnostics.push(Diagnostic::error(
                    span,
                    "class attribute requires a value; use class=\"...\" or class={...}",
                )),
                "style" => self.diagnostics.push(Diagnostic::error(
                    span,
                    "style attribute requires a value; use style=\"...\" or style={...}",
                )),
                _ => {}
            },
            HtmlAttrValue::Static(static_value) => {
                if name == "innerHTML" {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        "innerHTML requires TrustedHtml; use innerHTML={...} with an explicit sanitizer or unsafe-cast boundary",
                    ));
                }

                if name == "ref" && static_value.is_empty() {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        "ref attribute requires a non-empty ref name",
                    ));
                }

                if is_boolean_html_attr(name)
                    && !static_value.is_empty()
                    && !static_value.eq_ignore_ascii_case(name)
                {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!(
                            "boolean attribute {} ignores string value {:?}; use bare {} or {}={{...}}",
                            name, static_value, name, name
                        ),
                    ));
                }
            }
            HtmlAttrValue::Bool(false) | HtmlAttrValue::Dynamic { .. } => {}
        }
    }

    fn infer_html_expr(&mut self, expr: &Expr, env: &mut HashMap<String, Type>) {
        if let Some(spec) = HtmlForSpec::parse(expr) {
            let collection_ty = self.infer_expr(spec.collection, env);
            let item_ty = self.infer_iterable_element(collection_ty, spec.collection.span);
            let mut loop_env = env.clone();
            loop_env.insert(spec.item.to_string(), item_ty);
            if let Some(index) = spec.index {
                loop_env.insert(index.to_string(), Type::Number);
            }
            self.infer_expr(spec.key, &mut loop_env);
            self.infer_html_template(spec.template, &mut loop_env);
            return;
        }

        if let Some(spec) = HtmlIfSpec::parse(expr) {
            let condition_ty = self.infer_expr(spec.condition, env);
            self.unify(condition_ty, Type::Bool, spec.condition.span);
            self.infer_html_template(spec.then_template, env);
            self.infer_html_template(spec.else_template, env);
            return;
        }

        self.infer_expr(expr, env);
    }

    fn require_html_ref_attr(&mut self, ty: Type, span: Span) {
        let resolved = self.resolve(ty);
        if self.html_ref_attr_type_matches(resolved.clone(), span) {
            return;
        }

        let found = self.format_type(&resolved);
        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "ref attribute expects a string, keyword, or nil optional ref name, found {}",
                found
            ),
        ));
    }

    fn html_ref_attr_type_matches(&mut self, ty: Type, span: Span) -> bool {
        match self.resolve(ty) {
            Type::Var(id) => {
                self.unify(Type::Var(id), Type::String, span);
                true
            }
            Type::String | Type::Keyword(_) | Type::Nil => true,
            Type::Option(inner) => self.html_ref_attr_type_matches(*inner, span),
            _ => false,
        }
    }

    fn require_html_class_attr(&mut self, ty: Type, span: Span) {
        let resolved = self.resolve(ty);
        if self.html_class_attr_type_matches(resolved.clone(), span) {
            return;
        }

        let found = self.format_type(&resolved);
        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "class attribute expects a CSS class string, keyword, nil, bool, structured collection, or class flag map, found {}",
                found
            ),
        ));
    }

    fn html_class_attr_type_matches(&mut self, ty: Type, span: Span) -> bool {
        match self.resolve(ty) {
            Type::Var(_) | Type::String | Type::Keyword(_) | Type::Bool | Type::Nil => true,
            Type::Option(inner) => self.html_class_attr_type_matches(*inner, span),
            Type::List(inner) | Type::Vector(inner) | Type::Set(inner) => {
                self.html_class_attr_type_matches(*inner, span)
            }
            Type::Tuple(items) => items
                .into_iter()
                .all(|item| self.html_class_attr_type_matches(item, span)),
            Type::Record(fields) => fields
                .into_values()
                .all(|field_ty| self.html_class_flag_type_matches(field_ty, span)),
            Type::Map(key, value) => {
                self.html_class_key_type_matches(*key)
                    && self.html_class_flag_type_matches(*value, span)
            }
            _ => false,
        }
    }

    fn html_class_key_type_matches(&mut self, ty: Type) -> bool {
        match self.resolve(ty) {
            Type::Var(_) | Type::String | Type::Keyword(_) => true,
            _ => false,
        }
    }

    fn html_class_flag_type_matches(&mut self, ty: Type, span: Span) -> bool {
        match self.resolve(ty) {
            Type::Var(id) => {
                self.unify(Type::Var(id), Type::Bool, span);
                true
            }
            Type::Bool | Type::Nil => true,
            Type::Option(inner) => self.html_class_flag_type_matches(*inner, span),
            _ => false,
        }
    }

    fn require_html_inner_html_attr(&mut self, ty: Type, span: Span) {
        let resolved = self.resolve(ty);
        if self.html_inner_html_attr_type_matches(resolved.clone(), span) {
            return;
        }

        let found = self.format_type(&resolved);
        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "innerHTML expects TrustedHtml from an explicit sanitizer or unsafe-cast boundary, found {}",
                found
            ),
        ));
    }

    fn html_inner_html_attr_type_matches(&mut self, ty: Type, span: Span) -> bool {
        match self.resolve(ty) {
            Type::Var(id) => {
                self.unify(Type::Var(id), Type::TrustedHtml, span);
                true
            }
            Type::TrustedHtml | Type::Nil => true,
            Type::Option(inner) => self.html_inner_html_attr_type_matches(*inner, span),
            _ => false,
        }
    }

    fn require_html_style_attr(&mut self, ty: Type, span: Span) {
        let resolved = self.resolve(ty);
        if self.html_style_attr_type_matches(resolved.clone()) {
            return;
        }

        let found = self.format_type(&resolved);
        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "style attribute expects a CSS string, nil, record, or map with style property values, found {}",
                found
            ),
        ));
    }

    fn html_style_attr_type_matches(&mut self, ty: Type) -> bool {
        match self.resolve(ty) {
            Type::Var(_) | Type::String | Type::Nil => true,
            Type::Option(inner) => self.html_style_attr_type_matches(*inner),
            Type::Record(fields) => fields
                .into_values()
                .all(|field_ty| self.html_style_value_type_matches(field_ty)),
            Type::Map(key, value) => {
                self.html_style_key_type_matches(*key) && self.html_style_value_type_matches(*value)
            }
            _ => false,
        }
    }

    fn html_style_key_type_matches(&mut self, ty: Type) -> bool {
        match self.resolve(ty) {
            Type::Var(_) | Type::String | Type::Keyword(_) => true,
            Type::Option(inner) => self.html_style_key_type_matches(*inner),
            _ => false,
        }
    }

    fn html_style_value_type_matches(&mut self, ty: Type) -> bool {
        match self.resolve(ty) {
            Type::Var(_) | Type::String | Type::Number | Type::Bool | Type::Nil => true,
            Type::Option(inner) => self.html_style_value_type_matches(*inner),
            _ => false,
        }
    }

    fn infer_iterable_element(&mut self, collection_ty: Type, span: Span) -> Type {
        match self.shallow_resolve(collection_ty.clone()) {
            Type::List(inner) | Type::Vector(inner) | Type::Set(inner) => self.resolve(*inner),
            Type::Tuple(items) => {
                let Some(first) = items.first().cloned() else {
                    return self.fresh();
                };
                for item in items.into_iter().skip(1) {
                    self.unify(item, first.clone(), span);
                }
                self.resolve(first)
            }
            _ => {
                let element_ty = self.fresh();
                self.unify(
                    collection_ty,
                    Type::Vector(Box::new(element_ty.clone())),
                    span,
                );
                self.resolve(element_ty)
            }
        }
    }

    fn dom_event_type(&self) -> Type {
        let form_target = Type::Record(BTreeMap::from([
            ("checked".to_string(), Type::Bool),
            ("value".to_string(), Type::String),
            ("valueAsNumber".to_string(), Type::Number),
        ]));
        Type::Record(BTreeMap::from([
            ("altKey".to_string(), Type::Bool),
            ("clientX".to_string(), Type::Number),
            ("clientY".to_string(), Type::Number),
            ("ctrlKey".to_string(), Type::Bool),
            ("currentTarget".to_string(), form_target.clone()),
            ("key".to_string(), Type::String),
            ("metaKey".to_string(), Type::Bool),
            ("shiftKey".to_string(), Type::Bool),
            ("target".to_string(), form_target),
        ]))
    }

    fn infer_symbol(&mut self, span: Span, name: &str, env: &mut HashMap<String, Type>) -> Type {
        if let Some(ty) = env.get(name) {
            return ty.clone();
        }

        if name == "Cmd.none" {
            return Type::Cmd(Box::new(self.fresh()));
        }

        if name == "Sub.none" {
            return Type::Sub(Box::new(self.fresh()));
        }

        if let Some(ty) = primitive_decoder_type(name) {
            return ty;
        }

        if let Some((base, fields)) = split_path(name) {
            let Some(base_ty) = env.get(base).cloned() else {
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!("unknown symbol `{}`", base),
                ));
                return self.fresh();
            };

            let root_is_var = matches!(base_ty, Type::Var(_));
            let mut current_ty = base_ty.clone();
            for (index, field) in fields.iter().enumerate() {
                let (field_ty, updated_current) = self.infer_field_read(current_ty, field, span);
                current_ty = field_ty;
                if index == 0 && !root_is_var {
                    env.insert(base.to_string(), self.resolve(updated_current));
                }
            }
            return current_ty;
        }

        self.diagnostics.push(Diagnostic::error(
            span,
            format!("unknown symbol `{}`", name),
        ));
        self.fresh()
    }

    fn infer_field_read(&mut self, base_ty: Type, field: &str, span: Span) -> (Type, Type) {
        if let Type::Var(id) = base_ty {
            return match self.resolve(Type::Var(id)) {
                Type::Var(root_id) => {
                    let field_ty = self.fresh();
                    let fields = BTreeMap::from([(field.to_string(), field_ty.clone())]);
                    let record = Type::Record(fields);
                    self.bind(root_id, record.clone(), span);
                    if root_id != id {
                        self.subst.insert(id, Type::Var(root_id));
                    }
                    (field_ty, record)
                }
                Type::Record(mut fields) => {
                    if let Some(field_ty) = fields.remove(field) {
                        fields.insert(field.to_string(), field_ty.clone());
                        let record = Type::Record(fields);
                        self.subst.insert(id, record.clone());
                        (field_ty, record)
                    } else {
                        let field_ty = self.fresh();
                        fields.insert(field.to_string(), field_ty.clone());
                        let record = Type::Record(fields);
                        self.subst.insert(id, record.clone());
                        (field_ty, record)
                    }
                }
                Type::Js => (Type::Js, Type::Js),
                other => {
                    let type_name = self.format_type(&other);
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!("cannot read field `{}` from {}", field, type_name),
                    ));
                    (self.fresh(), other)
                }
            };
        }

        match self.resolve(base_ty) {
            Type::Record(mut fields) => {
                if let Some(field_ty) = fields.remove(field) {
                    fields.insert(field.to_string(), field_ty.clone());
                    (field_ty, Type::Record(fields))
                } else {
                    let field_ty = self.fresh();
                    fields.insert(field.to_string(), field_ty.clone());
                    (field_ty, Type::Record(fields))
                }
            }
            Type::Union(variants) => {
                let field_ty = self.union_field_type(&variants, field, span);
                (field_ty, Type::Union(variants))
            }
            Type::Js => (Type::Js, Type::Js),
            other => {
                let type_name = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!("cannot read field `{}` from {}", field, type_name),
                ));
                (self.fresh(), other)
            }
        }
    }

    fn union_field_type(&mut self, variants: &[Type], field: &str, span: Span) -> Type {
        let mut merged = None;
        for variant in variants {
            match self.resolve(variant.clone()) {
                Type::Record(fields) => {
                    let Some(field_ty) = fields.get(field).cloned() else {
                        self.diagnostics.push(Diagnostic::error(
                            span,
                            format!(
                                "cannot read field `{}` because not every union variant has it",
                                field
                            ),
                        ));
                        return self.fresh();
                    };
                    merged = Some(match merged {
                        Some(existing) => self.unify(existing, field_ty, span),
                        None => field_ty,
                    });
                }
                other => {
                    let type_name = self.format_type(&other);
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!(
                            "cannot read field `{}` from union variant {}",
                            field, type_name
                        ),
                    ));
                    return self.fresh();
                }
            }
        }
        merged.unwrap_or_else(|| self.fresh())
    }

    fn infer_definition_with_expected(
        &mut self,
        expr: &Expr,
        env: &mut HashMap<String, Type>,
        expected: Type,
    ) -> Type {
        let ExprKind::List(items) = &expr.kind else {
            let inferred = self.infer_expr(expr, env);
            return self.unify(expected, inferred, expr.span);
        };

        if let [head, name, value] = items.as_slice() {
            if matches_symbol(head, "def") {
                let inferred = self.infer_def(expr.span, &[name.clone(), value.clone()], env);
                let checked = self.unify(expected, inferred, expr.span);
                if let ExprKind::Symbol(name) = &name.kind {
                    env.insert(name.clone(), checked.clone());
                }
                return checked;
            }
        }

        if items.len() >= 4 && matches_symbol(&items[0], "defn") {
            let expected_fn = match expected.clone() {
                Type::Fn(args, ret) => Some((args, *ret)),
                _ => None,
            };
            let inferred = self.infer_defn_with_expected(expr.span, &items[1..], env, expected_fn);
            return self.unify(expected, inferred, expr.span);
        }

        let inferred = self.infer_expr(expr, env);
        self.unify(expected, inferred, expr.span)
    }

    fn infer_def(&mut self, span: Span, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 2 {
            self.arity_error(span, "def", 2, args.len());
            return self.fresh();
        }

        let ExprKind::Symbol(name) = &args[0].kind else {
            self.diagnostics
                .push(Diagnostic::error(args[0].span, "def name must be a symbol"));
            return self.fresh();
        };

        let value_ty = self.infer_expr(&args[1], env);
        env.insert(name.clone(), value_ty.clone());
        value_ty
    }

    fn infer_defn(&mut self, span: Span, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() < 3 {
            self.diagnostics.push(Diagnostic::error(
                span,
                "defn expects a name, parameter vector, and body",
            ));
            return self.fresh();
        }

        let ExprKind::Symbol(name) = &args[0].kind else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "defn name must be a symbol",
            ));
            return self.fresh();
        };

        let fn_args = &args[1..];
        let allow_pattern_params = !Self::has_single_template_body(fn_args);
        let fn_ty = self.infer_recursive_defn(span, name, fn_args, env, None, allow_pattern_params);
        env.insert(name.clone(), fn_ty.clone());
        fn_ty
    }

    fn infer_defn_with_expected(
        &mut self,
        span: Span,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        expected_fn: Option<(Vec<Type>, Type)>,
    ) -> Type {
        if args.len() < 3 {
            self.diagnostics.push(Diagnostic::error(
                span,
                "defn expects a name, parameter vector, and body",
            ));
            return self.fresh();
        }

        let ExprKind::Symbol(name) = &args[0].kind else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "defn name must be a symbol",
            ));
            return self.fresh();
        };

        let fn_args = &args[1..];
        let allow_pattern_params = !Self::has_single_template_body(fn_args);
        let fn_ty =
            self.infer_recursive_defn(span, name, fn_args, env, expected_fn, allow_pattern_params);
        env.insert(name.clone(), fn_ty.clone());
        fn_ty
    }

    fn infer_recursive_defn(
        &mut self,
        span: Span,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        expected_fn: Option<(Vec<Type>, Type)>,
        allow_pattern_params: bool,
    ) -> Type {
        if !Self::fn_body_references_name(name, args) {
            return self.infer_fn_with_expected(span, args, env, expected_fn, allow_pattern_params);
        }

        let expected_fn = expected_fn.or_else(|| self.fresh_signature_for_fn(args));
        if let Some((param_types, return_ty)) = &expected_fn {
            env.insert(
                name.to_string(),
                Type::Fn(param_types.clone(), Box::new(return_ty.clone())),
            );
        }
        self.infer_fn_with_expected(span, args, env, expected_fn, allow_pattern_params)
    }

    fn has_single_template_body(args: &[Expr]) -> bool {
        matches!(args.get(1..), Some([body]) if Self::is_template_component_body(body))
    }

    fn is_template_component_body(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::HtmlTemplate(_) => true,
            ExprKind::List(items) if items.len() == 3 && matches_symbol(&items[0], "let") => {
                matches!(&items[2].kind, ExprKind::HtmlTemplate(_))
            }
            _ => false,
        }
    }

    fn fresh_signature_for_fn(&mut self, args: &[Expr]) -> Option<(Vec<Type>, Type)> {
        let ExprKind::Vector(params) = args.first()?.kind.clone() else {
            return None;
        };
        let param_types = params.iter().map(|_| self.fresh()).collect::<Vec<_>>();
        Some((param_types, self.fresh()))
    }

    fn fn_body_references_name(name: &str, args: &[Expr]) -> bool {
        args.get(1..).is_some_and(|body| {
            body.iter()
                .any(|expr| Self::expr_references_name(name, expr))
        })
    }

    fn expr_references_name(name: &str, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Symbol(symbol) => symbol == name,
            ExprKind::List(items) | ExprKind::Vector(items) | ExprKind::Set(items) => items
                .iter()
                .any(|item| Self::expr_references_name(name, item)),
            ExprKind::Map(entries) => entries.iter().any(|(key, value)| {
                Self::expr_references_name(name, key) || Self::expr_references_name(name, value)
            }),
            ExprKind::Quote(inner)
            | ExprKind::QuasiQuote(inner)
            | ExprKind::Unquote(inner)
            | ExprKind::UnquoteSplicing(inner) => Self::expr_references_name(name, inner),
            ExprKind::HtmlTemplate(node) => Self::html_node_references_name(name, node),
            ExprKind::Nil
            | ExprKind::Bool(_)
            | ExprKind::Number(_)
            | ExprKind::String(_)
            | ExprKind::Keyword(_) => false,
        }
    }

    fn html_node_references_name(name: &str, node: &HtmlNode) -> bool {
        match node {
            HtmlNode::Element(element) => {
                element.attrs.iter().any(|attr| match &attr.value {
                    HtmlAttrValue::Dynamic { expr, .. } => Self::expr_references_name(name, expr),
                    HtmlAttrValue::Bool(_) | HtmlAttrValue::Static(_) => false,
                }) || element
                    .children
                    .iter()
                    .any(|child| Self::html_node_references_name(name, child))
            }
            HtmlNode::Expr { expr, .. } => Self::expr_references_name(name, expr),
            HtmlNode::Text { .. } => false,
        }
    }

    fn infer_fn(&mut self, span: Span, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        self.infer_fn_with_expected(span, args, env, None, true)
    }

    fn infer_fn_with_expected(
        &mut self,
        span: Span,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        expected_fn: Option<(Vec<Type>, Type)>,
        allow_pattern_params: bool,
    ) -> Type {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                span,
                "fn expects a parameter vector and a body",
            ));
            return self.fresh();
        }

        let ExprKind::Vector(params) = &args[0].kind else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "fn parameters must be a vector",
            ));
            return self.fresh();
        };

        let mut inner = env.clone();
        let mut param_types = Vec::new();
        let expected_args = expected_fn.as_ref().map(|(args, _)| args);
        for (index, param) in params.iter().enumerate() {
            let ty = expected_args
                .and_then(|args| args.get(index).cloned())
                .unwrap_or_else(|| self.fresh());

            if allow_pattern_params {
                let pattern_ty = self.infer_pattern(param, ty.clone(), &mut inner);
                let arg_ty = self.unify(ty, pattern_ty, param.span);
                param_types.push(self.resolve(arg_ty));
                continue;
            }

            let ExprKind::Symbol(name) = &param.kind else {
                self.diagnostics.push(Diagnostic::error(
                    param.span,
                    "fn parameter must be a symbol",
                ));
                continue;
            };
            inner.insert(name.clone(), ty.clone());
            param_types.push(self.resolve(ty));
        }

        let mut body_ty = self.infer_do(&args[1..], &mut inner);
        if let Some((_, expected_ret)) = expected_fn {
            body_ty = self.unify(expected_ret, body_ty, span);
        }
        Type::Fn(param_types, Box::new(body_ty))
    }

    fn infer_fn_expr_with_expected(
        &mut self,
        expr: &Expr,
        env: &mut HashMap<String, Type>,
        arg_types: Vec<Type>,
        return_ty: Type,
    ) -> Type {
        if let ExprKind::List(items) = &expr.kind {
            if let Some((head, args)) = items.split_first() {
                if matches!(&head.kind, ExprKind::Symbol(name) if name == "fn") {
                    return self.infer_fn_with_expected(
                        expr.span,
                        args,
                        env,
                        Some((arg_types, return_ty)),
                        true,
                    );
                }
            }
        }

        let expected = Type::Fn(arg_types, Box::new(return_ty));
        let inferred = self.infer_expr(expr, env);
        self.unify(inferred, expected, expr.span)
    }

    fn infer_let(&mut self, span: Span, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                span,
                "let expects a binding vector and a body",
            ));
            return self.fresh();
        }

        let ExprKind::Vector(bindings) = &args[0].kind else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "let bindings must be a vector",
            ));
            return self.fresh();
        };

        if bindings.len() % 2 != 0 {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "let bindings must contain name/value pairs",
            ));
        }

        let mut inner = env.clone();
        for pair in bindings.chunks(2) {
            let [pattern_expr, value_expr] = pair else {
                break;
            };
            let ty = self.infer_expr(value_expr, &mut inner);
            let pattern_ty = self.infer_pattern(pattern_expr, ty.clone(), &mut inner);
            self.unify(ty, pattern_ty, pattern_expr.span);
        }

        self.infer_do(&args[1..], &mut inner)
    }

    fn infer_if(&mut self, span: Span, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 3 {
            self.arity_error(span, "if", 3, args.len());
            return self.fresh();
        }

        let cond_ty = self.infer_expr(&args[0], env);
        self.unify(cond_ty, Type::Bool, args[0].span);
        let then_ty = self.infer_expr(&args[1], env);
        let else_ty = self.infer_expr(&args[2], env);
        self.join_types(then_ty, else_ty, span)
    }

    fn infer_match(&mut self, span: Span, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() < 3 || args.len() % 2 == 0 {
            self.diagnostics.push(Diagnostic::error(
                span,
                "match expects a value followed by pattern/body pairs",
            ));
            return self.fresh();
        }

        let scrutinee_ty = self.infer_expr(&args[0], env);
        let union_variants = match self.resolve(scrutinee_ty.clone()) {
            Type::Union(variants) => Some(variants),
            _ => None,
        };
        let mut covered_variants = BTreeSet::new();
        let mut result_ty: Option<Type> = None;

        for arm in args[1..].chunks(2) {
            let [pattern, body] = arm else {
                continue;
            };
            if let Some(variants) = &union_variants {
                covered_variants.extend(self.union_variants_matching_pattern(pattern, variants));
            }
            let mut arm_env = env.clone();
            let pattern_ty = self.infer_pattern(pattern, scrutinee_ty.clone(), &mut arm_env);
            if !(union_variants.is_some() && matches!(pattern_ty, Type::Union(_))) {
                self.unify(scrutinee_ty.clone(), pattern_ty, pattern.span);
            }
            let body_ty = self.infer_expr(body, &mut arm_env);
            result_ty = Some(match result_ty {
                Some(existing) => self.join_types(existing, body_ty, body.span),
                None => body_ty,
            });
        }

        if let Some(variants) = union_variants {
            self.report_non_exhaustive_union_match(span, &variants, &covered_variants);
        }

        result_ty.unwrap_or_else(|| self.fresh())
    }

    fn infer_pattern(
        &mut self,
        pattern: &Expr,
        expected: Type,
        env: &mut HashMap<String, Type>,
    ) -> Type {
        match &pattern.kind {
            ExprKind::Symbol(name) if name == "_" => expected,
            ExprKind::Symbol(name) => {
                env.insert(name.clone(), expected.clone());
                expected
            }
            ExprKind::List(items)
                if items.first().is_some_and(|head| matches_symbol(head, "as")) =>
            {
                let Some((inner, alias)) = self.parse_as_pattern(pattern.span, items) else {
                    return expected;
                };
                let pattern_ty = self.infer_pattern(inner, expected, env);
                if alias != "_" {
                    env.insert(alias.to_string(), pattern_ty.clone());
                }
                pattern_ty
            }
            ExprKind::List(items)
                if items
                    .first()
                    .and_then(symbol_name)
                    .is_some_and(is_data_constructor_pattern) =>
            {
                self.infer_data_constructor_pattern(pattern.span, items, expected, env)
            }
            ExprKind::Keyword(name) => Type::Keyword(Some(name.clone())),
            ExprKind::String(_) => Type::String,
            ExprKind::Number(_) => Type::Number,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::Nil => Type::Nil,
            ExprKind::Map(entries) => {
                if let Type::Union(variants) = self.resolve(expected.clone()) {
                    return self.infer_union_record_pattern(pattern.span, entries, variants, env);
                }
                let mut fields = BTreeMap::new();
                for (key, value) in entries {
                    let Some(name) = record_key_name(key) else {
                        self.diagnostics.push(Diagnostic::error(
                            key.span,
                            "record pattern keys must be keywords, strings, or symbols",
                        ));
                        continue;
                    };
                    let field_expected = self.fresh();
                    let field_ty = self.infer_pattern(value, field_expected, env);
                    fields.insert(name, field_ty);
                }
                Type::Record(fields)
            }
            ExprKind::Vector(items) => Type::Tuple(
                items
                    .iter()
                    .map(|item| {
                        let expected = self.fresh();
                        self.infer_pattern(item, expected, env)
                    })
                    .collect(),
            ),
            _ => {
                self.diagnostics
                    .push(Diagnostic::error(pattern.span, "unsupported match pattern"));
                expected
            }
        }
    }

    fn parse_as_pattern<'a>(
        &mut self,
        span: Span,
        items: &'a [Expr],
    ) -> Option<(&'a Expr, &'a str)> {
        if items.len() != 3 {
            self.diagnostics.push(Diagnostic::error(
                span,
                "as pattern expects `(as pattern name)`",
            ));
            return None;
        }
        let ExprKind::Symbol(name) = &items[2].kind else {
            self.diagnostics.push(Diagnostic::error(
                items[2].span,
                "as pattern name must be a symbol",
            ));
            return None;
        };
        Some((&items[1], name))
    }

    fn infer_data_constructor_pattern(
        &mut self,
        span: Span,
        items: &[Expr],
        expected: Type,
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let Some(name) = items.first().and_then(symbol_name) else {
            return expected;
        };
        if name == "list" {
            return self.infer_list_pattern(span, &items[1..], expected, env);
        }
        if name == "cons" {
            return self.infer_cons_pattern(span, &items[1..], expected, env);
        }
        if items.len() != 2 {
            self.diagnostics.push(Diagnostic::error(
                span,
                format!("{} pattern expects `({} pattern)`", name, name),
            ));
            return expected;
        }

        match name {
            "some" => {
                let inner_expected = match self.resolve(expected.clone()) {
                    Type::Option(inner) => *inner,
                    Type::Var(_) => self.fresh(),
                    other => {
                        let found = self.format_type(&other);
                        self.diagnostics.push(Diagnostic::error(
                            span,
                            format!("some pattern expects an Option value, found {}", found),
                        ));
                        self.fresh()
                    }
                };
                let inner_ty = self.infer_pattern(&items[1], inner_expected, env);
                Type::Option(Box::new(inner_ty))
            }
            "ok" | "err" => {
                let (ok_expected, err_expected) = match self.resolve(expected.clone()) {
                    Type::Result(ok, err) => (*ok, *err),
                    Type::Var(_) => (self.fresh(), self.fresh()),
                    other => {
                        let found = self.format_type(&other);
                        self.diagnostics.push(Diagnostic::error(
                            span,
                            format!("{} pattern expects a Result value, found {}", name, found),
                        ));
                        (self.fresh(), self.fresh())
                    }
                };
                if name == "ok" {
                    let ok_ty = self.infer_pattern(&items[1], ok_expected, env);
                    Type::Result(Box::new(ok_ty), Box::new(err_expected))
                } else {
                    let err_ty = self.infer_pattern(&items[1], err_expected, env);
                    Type::Result(Box::new(ok_expected), Box::new(err_ty))
                }
            }
            _ => expected,
        }
    }

    fn infer_cons_pattern(
        &mut self,
        span: Span,
        items: &[Expr],
        expected: Type,
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if items.len() != 2 {
            self.diagnostics.push(Diagnostic::error(
                span,
                "cons pattern expects `(cons head tail)`",
            ));
            return expected;
        }

        let (element_expected, matches_list) = match self.resolve(expected.clone()) {
            Type::List(inner) => (*inner, true),
            Type::Var(_) => (self.fresh(), true),
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!("cons pattern expects a List value, found {}", found),
                ));
                (self.fresh(), false)
            }
        };

        let head_ty = self.infer_pattern(&items[0], element_expected.clone(), env);
        let element_ty = self.unify(element_expected, head_ty, items[0].span);
        let tail_ty = self.infer_pattern(&items[1], Type::List(Box::new(element_ty.clone())), env);
        self.unify(
            Type::List(Box::new(element_ty.clone())),
            tail_ty,
            items[1].span,
        );

        if matches_list {
            Type::List(Box::new(self.resolve(element_ty)))
        } else {
            expected
        }
    }

    fn infer_list_pattern(
        &mut self,
        span: Span,
        items: &[Expr],
        expected: Type,
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let (mut element_expected, matches_list) = match self.resolve(expected.clone()) {
            Type::List(inner) => (*inner, true),
            Type::Var(_) => (self.fresh(), true),
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!("list pattern expects a List value, found {}", found),
                ));
                (self.fresh(), false)
            }
        };

        for item in items {
            let item_ty = self.infer_pattern(item, element_expected.clone(), env);
            element_expected = self.unify(element_expected, item_ty, item.span);
        }

        if matches_list {
            Type::List(Box::new(self.resolve(element_expected)))
        } else {
            expected
        }
    }

    fn infer_union_record_pattern(
        &mut self,
        span: Span,
        entries: &[(Expr, Expr)],
        variants: Vec<Type>,
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let mut pattern_fields = Vec::new();
        for (key, value) in entries {
            let Some(name) = record_key_name(key) else {
                self.diagnostics.push(Diagnostic::error(
                    key.span,
                    "record pattern keys must be keywords, strings, or symbols",
                ));
                continue;
            };
            pattern_fields.push((name, value));
        }

        let pattern_tag = pattern_record_kind_literal(&pattern_fields);
        let mut matched_fields: BTreeMap<String, Type> = BTreeMap::new();
        let mut matched_any = false;
        for variant in &variants {
            let Type::Record(fields) = self.resolve(variant.clone()) else {
                continue;
            };
            if pattern_tag.is_some_and(|tag| record_fields_kind_literal(&fields) != Some(tag)) {
                continue;
            }
            if !self.record_pattern_matches_fields(&pattern_fields, &fields) {
                continue;
            }
            matched_any = true;
            for (field, _) in &pattern_fields {
                let Some(field_ty) = fields.get(field).cloned() else {
                    continue;
                };
                let merged = match matched_fields.remove(field) {
                    Some(existing) => self.unify(existing, field_ty, span),
                    None => field_ty,
                };
                matched_fields.insert(field.clone(), merged);
            }
        }

        if !matched_any {
            self.diagnostics.push(Diagnostic::error(
                span,
                "record pattern does not match any union variant",
            ));
            return Type::Union(variants);
        }

        for (field, value) in pattern_fields {
            let field_ty = matched_fields
                .remove(&field)
                .unwrap_or_else(|| self.fresh());
            self.infer_pattern(value, field_ty, env);
        }

        Type::Union(variants)
    }

    fn union_variants_matching_pattern(&mut self, pattern: &Expr, variants: &[Type]) -> Vec<usize> {
        if let Some(tag) = pattern_kind_literal(pattern) {
            return variants
                .iter()
                .enumerate()
                .filter_map(|(index, variant)| {
                    if tagged_record_literal(variant) == Some(tag)
                        && self.pattern_matches_type(pattern, variant.clone())
                    {
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect();
        }

        variants
            .iter()
            .enumerate()
            .filter_map(|(index, variant)| {
                if self.pattern_matches_type(pattern, variant.clone()) {
                    Some(index)
                } else {
                    None
                }
            })
            .collect()
    }

    fn pattern_matches_type(&mut self, pattern: &Expr, ty: Type) -> bool {
        let ty = self.resolve(ty);
        match &pattern.kind {
            ExprKind::Symbol(_) => true,
            ExprKind::List(items)
                if items.first().is_some_and(|head| matches_symbol(head, "as")) =>
            {
                if items.len() == 3 {
                    self.pattern_matches_type(&items[1], ty)
                } else {
                    false
                }
            }
            ExprKind::List(items)
                if items
                    .first()
                    .and_then(symbol_name)
                    .is_some_and(is_data_constructor_pattern) =>
            {
                let Some(name) = items.first().and_then(symbol_name) else {
                    return false;
                };
                if name == "list" {
                    return match ty {
                        Type::List(item_ty) => items[1..]
                            .iter()
                            .all(|pattern| self.pattern_matches_type(pattern, (*item_ty).clone())),
                        _ => false,
                    };
                }
                if name == "cons" {
                    if items.len() != 3 {
                        return false;
                    }
                    return match ty {
                        Type::List(item_ty) => {
                            self.pattern_matches_type(&items[1], (*item_ty).clone())
                                && self.pattern_matches_type(&items[2], Type::List(item_ty))
                        }
                        _ => false,
                    };
                }
                if items.len() != 2 {
                    return false;
                }
                match (name, ty) {
                    ("some", Type::Option(inner)) => self.pattern_matches_type(&items[1], *inner),
                    ("ok", Type::Result(ok, _)) => self.pattern_matches_type(&items[1], *ok),
                    ("err", Type::Result(_, err)) => self.pattern_matches_type(&items[1], *err),
                    _ => false,
                }
            }
            ExprKind::Keyword(name) => match ty {
                Type::Keyword(expected) => keyword_literal_matches(&expected, name),
                _ => false,
            },
            ExprKind::String(_) => matches!(ty, Type::String),
            ExprKind::Number(_) => matches!(ty, Type::Number),
            ExprKind::Bool(_) => matches!(ty, Type::Bool),
            ExprKind::Nil => matches!(ty, Type::Nil | Type::Option(_)),
            ExprKind::Map(entries) => {
                let mut pattern_fields = Vec::new();
                for (key, value) in entries {
                    let Some(name) = record_key_name(key) else {
                        return false;
                    };
                    pattern_fields.push((name, value));
                }
                match ty {
                    Type::Record(fields) => {
                        self.record_pattern_matches_fields(pattern_fields.as_slice(), &fields)
                    }
                    Type::Union(variants) => variants
                        .into_iter()
                        .any(|variant| self.pattern_matches_type(pattern, variant)),
                    _ => false,
                }
            }
            ExprKind::Vector(pattern_items) => match ty {
                Type::Tuple(items) => {
                    pattern_items.len() == items.len()
                        && pattern_items
                            .iter()
                            .zip(items)
                            .all(|(pattern, ty)| self.pattern_matches_type(pattern, ty))
                }
                Type::Vector(item_ty) => pattern_items
                    .iter()
                    .all(|pattern| self.pattern_matches_type(pattern, (*item_ty).clone())),
                _ => false,
            },
            _ => false,
        }
    }

    fn record_pattern_matches_fields(
        &mut self,
        pattern_fields: &[(String, &Expr)],
        fields: &BTreeMap<String, Type>,
    ) -> bool {
        for (field, value) in pattern_fields {
            let Some(field_ty) = fields.get(field).cloned() else {
                return false;
            };
            if !self.pattern_matches_type(value, field_ty) {
                return false;
            }
        }
        true
    }

    fn report_non_exhaustive_union_match(
        &mut self,
        span: Span,
        variants: &[Type],
        covered: &BTreeSet<usize>,
    ) {
        if covered.len() >= variants.len() {
            return;
        }

        let missing = variants
            .iter()
            .enumerate()
            .filter_map(|(index, variant)| {
                if covered.contains(&index) {
                    None
                } else {
                    Some(format_type_with_literals(variant))
                }
            })
            .collect::<Vec<_>>();
        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "non-exhaustive match for union; missing variants: {}",
                missing.join(", ")
            ),
        ));
    }

    fn infer_do(&mut self, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        let mut ty = Type::Nil;
        for arg in args {
            ty = self.infer_expr(arg, env);
        }
        ty
    }

    fn infer_test_group(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects a name and at least one test", name),
            ));
        }
        if let Some(label) = args.first() {
            let label_ty = self.infer_expr(label, env);
            self.unify(Type::String, label_ty, label.span);
        }
        for entry in args.iter().skip(1) {
            self.infer_expr(entry, env);
        }
        Type::Js
    }

    fn infer_test_case(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects a name and at least one assertion", name),
            ));
        }
        if let Some(label) = args.first() {
            let label_ty = self.infer_expr(label, env);
            self.unify(Type::String, label_ty, label.span);
        }
        for assertion in args.iter().skip(1) {
            self.infer_expr(assertion, env);
        }
        Type::Js
    }

    fn infer_expect_equal(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        _negated: bool,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Js;
        }
        let actual = self.infer_expr(&args[0], env);
        let expected = self.infer_expr(&args[1], env);
        if !matches!(self.resolve(actual.clone()), Type::Js)
            && !matches!(self.resolve(expected.clone()), Type::Js)
        {
            self.unify(actual, expected, args[1].span);
        }
        Type::Js
    }

    fn infer_expect_ok(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Js;
        }
        let actual = self.infer_expr(&args[0], env);
        self.unify(Type::Bool, actual, args[0].span);
        Type::Js
    }

    fn infer_expect_err(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Js;
        }
        let actual = self.infer_expr(&args[0], env);
        let ok = self.fresh();
        let err = self.fresh();
        self.unify(
            Type::Result(Box::new(ok), Box::new(err)),
            actual,
            args[0].span,
        );
        Type::Js
    }

    fn infer_expect_some(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Js;
        }
        self.infer_expr(&args[0], env);
        Type::Js
    }

    fn infer_expect_nil(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Js;
        }
        self.infer_expr(&args[0], env);
        Type::Js
    }

    fn infer_expect_match(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Js;
        }
        self.infer_expr(&args[0], env);
        self.infer_expr(&args[1], env);
        Type::Js
    }

    fn infer_expect_throws(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 && args.len() != 2 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects a thunk and optional expected message", name),
            ));
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Js;
        }
        let thunk = self.infer_expr(&args[0], env);
        if !matches!(self.resolve(thunk.clone()), Type::Js) {
            let ret = self.fresh();
            self.unify(thunk, Type::Fn(Vec::new(), Box::new(ret)), args[0].span);
        }
        if let Some(expected) = args.get(1) {
            let expected_ty = self.infer_expr(expected, env);
            self.unify(Type::String, expected_ty, expected.span);
        }
        Type::Js
    }

    fn infer_render_harness(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Js;
        }
        let component = self.infer_expr(&args[0], env);
        self.unify(Type::Html, component, args[0].span);
        Type::Js
    }

    fn infer_render_to_string(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        match args.len() {
            1 => {
                let component = self.infer_expr(&args[0], env);
                self.unify(Type::Html, component, args[0].span);
            }
            2 => {
                let view = self.infer_expr(&args[0], env);
                let state = self.infer_expr(&args[1], env);
                self.unify(
                    view,
                    Type::Fn(vec![state], Box::new(Type::Html)),
                    args[0].span,
                );
            }
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    args.first().map_or(Span::default(), |arg| arg.span),
                    format!("{} expects a component or view and state", name),
                ));
                for arg in args {
                    self.infer_expr(arg, env);
                }
            }
        }
        Type::String
    }

    fn infer_rerender_harness(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Js;
        }
        self.infer_expr(&args[0], env);
        let component = self.infer_expr(&args[1], env);
        self.unify(Type::Html, component, args[1].span);
        Type::Js
    }

    fn infer_dispose_harness(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
        }
        for arg in args {
            self.infer_expr(arg, env);
        }
        Type::Nil
    }

    fn infer_query_helper(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        all: bool,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
        }
        self.infer_optional_harness_arg(args.first(), env);
        self.infer_optional_string_arg(args.get(1), env);
        if all {
            Type::Vector(Box::new(Type::Js))
        } else {
            Type::Js
        }
    }

    fn infer_harness_string_query(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 && args.len() != 2 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects a harness and optional selector", name),
            ));
        }
        self.infer_optional_harness_arg(args.first(), env);
        if let Some(selector) = args.get(1) {
            let selector_ty = self.infer_expr(selector, env);
            self.unify(Type::String, selector_ty, selector.span);
        }
        Type::String
    }

    fn infer_harness_attr_query(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
        }
        self.infer_optional_harness_arg(args.first(), env);
        self.infer_optional_string_arg(args.get(1), env);
        self.infer_optional_string_arg(args.get(2), env);
        Type::String
    }

    fn infer_harness_class_query(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
        }
        self.infer_optional_harness_arg(args.first(), env);
        self.infer_optional_string_arg(args.get(1), env);
        self.infer_optional_string_arg(args.get(2), env);
        Type::Bool
    }

    fn infer_harness_records(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
        }
        self.infer_optional_harness_arg(args.first(), env);
        Type::Vector(Box::new(self.fresh()))
    }

    fn infer_fire_event(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 && args.len() != 4 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!(
                    "{} expects harness, selector, event type, and optional event data",
                    name
                ),
            ));
        }
        self.infer_optional_harness_arg(args.first(), env);
        self.infer_optional_string_arg(args.get(1), env);
        self.infer_optional_string_arg(args.get(2), env);
        if let Some(init) = args.get(3) {
            self.infer_expr(init, env);
        }
        Type::Js
    }

    fn infer_fire_selector_event(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 && args.len() != 3 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!(
                    "{} expects harness, selector, and optional event data",
                    name
                ),
            ));
        }
        self.infer_optional_harness_arg(args.first(), env);
        self.infer_optional_string_arg(args.get(1), env);
        if let Some(init) = args.get(2) {
            self.infer_expr(init, env);
        }
        Type::Js
    }

    fn infer_fire_value_event(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 && args.len() != 3 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!(
                    "{} expects harness, selector, and optional value or event data",
                    name
                ),
            ));
        }
        self.infer_optional_harness_arg(args.first(), env);
        self.infer_optional_string_arg(args.get(1), env);
        if let Some(value) = args.get(2) {
            self.infer_expr(value, env);
        }
        Type::Js
    }

    fn infer_optional_harness_arg(&mut self, arg: Option<&Expr>, env: &mut HashMap<String, Type>) {
        if let Some(arg) = arg {
            self.infer_expr(arg, env);
        }
    }

    fn infer_optional_string_arg(&mut self, arg: Option<&Expr>, env: &mut HashMap<String, Type>) {
        if let Some(arg) = arg {
            let ty = self.infer_expr(arg, env);
            self.unify(Type::String, ty, arg.span);
        }
    }

    fn infer_unsafe_cast(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            if let Some(value) = args.get(1) {
                self.infer_expr(value, env);
            }
            return self.fresh();
        }

        self.infer_expr(&args[1], env);
        let Ok(syntax) = parse_type_syntax(&args[0]) else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "unsafe-cast expects a type expression",
            ));
            return self.fresh();
        };
        let aliases = self.type_aliases.clone();
        self.type_syntax_to_type(&syntax, &aliases, args[0].span)
            .unwrap_or_else(|| self.fresh())
    }

    fn infer_event_control(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Event(Box::new(self.fresh()));
        }

        let msg = self.infer_expr(&args[0], env);
        Type::Event(Box::new(msg))
    }

    fn infer_task_succeed(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Task(Box::new(self.fresh()), Box::new(self.fresh()));
        }

        let ok = self.infer_expr(&args[0], env);
        Type::Task(Box::new(self.fresh()), Box::new(ok))
    }

    fn infer_task_fail(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Task(Box::new(self.fresh()), Box::new(self.fresh()));
        }

        let err = self.infer_expr(&args[0], env);
        Type::Task(Box::new(err), Box::new(self.fresh()))
    }

    fn infer_task_map(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Task(Box::new(self.fresh()), Box::new(self.fresh()));
        }

        let err = self.fresh();
        let ok = self.fresh();
        let next_ok = self.fresh();
        let task = self.infer_expr(&args[0], env);
        self.unify(
            task,
            Type::Task(Box::new(err.clone()), Box::new(ok.clone())),
            args[0].span,
        );
        let mapper = self.infer_expr(&args[1], env);
        self.unify(
            mapper,
            Type::Fn(vec![ok], Box::new(next_ok.clone())),
            args[1].span,
        );
        Type::Task(Box::new(self.resolve(err)), Box::new(self.resolve(next_ok)))
    }

    fn infer_task_map_error(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Task(Box::new(self.fresh()), Box::new(self.fresh()));
        }

        let err = self.fresh();
        let ok = self.fresh();
        let next_err = self.fresh();
        let task = self.infer_expr(&args[0], env);
        self.unify(
            task,
            Type::Task(Box::new(err.clone()), Box::new(ok.clone())),
            args[0].span,
        );
        let mapper = self.infer_expr(&args[1], env);
        self.unify(
            mapper,
            Type::Fn(vec![err], Box::new(next_err.clone())),
            args[1].span,
        );
        Type::Task(Box::new(self.resolve(next_err)), Box::new(self.resolve(ok)))
    }

    fn infer_task_and_then(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Task(Box::new(self.fresh()), Box::new(self.fresh()));
        }

        let err = self.fresh();
        let ok = self.fresh();
        let next_ok = self.fresh();
        let task = self.infer_expr(&args[0], env);
        self.unify(
            task,
            Type::Task(Box::new(err.clone()), Box::new(ok.clone())),
            args[0].span,
        );
        let next = self.infer_expr(&args[1], env);
        self.unify(
            next,
            Type::Fn(
                vec![ok],
                Box::new(Type::Task(Box::new(err.clone()), Box::new(next_ok.clone()))),
            ),
            args[1].span,
        );
        Type::Task(Box::new(self.resolve(err)), Box::new(self.resolve(next_ok)))
    }

    fn infer_task_perform(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        match args.len() {
            3 => self.infer_task_perform_value(args, env),
            4 => self.infer_task_perform_call(args, env),
            found => {
                self.diagnostics.push(Diagnostic::error(
                    args.first().map_or(Span::default(), |arg| arg.span),
                    format!("{} expects 3 or 4 arguments, found {}", name, found),
                ));
                for arg in args {
                    self.infer_expr(arg, env);
                }
                Type::Cmd(Box::new(self.fresh()))
            }
        }
    }

    fn infer_task_perform_value(&mut self, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        let err = self.fresh();
        let ok = self.fresh();
        let task = self.infer_expr(&args[0], env);
        self.unify(
            task,
            Type::Task(Box::new(err.clone()), Box::new(ok.clone())),
            args[0].span,
        );
        self.infer_task_perform_mappers(err, ok, &args[1], &args[2], env)
    }

    fn infer_task_perform_call(&mut self, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        let input = self.infer_expr(&args[1], env);
        let err = self.fresh();
        let ok = self.fresh();
        let task_fn = self.infer_expr(&args[0], env);
        self.unify(
            task_fn,
            Type::Fn(
                vec![input],
                Box::new(Type::Task(Box::new(err.clone()), Box::new(ok.clone()))),
            ),
            args[0].span,
        );
        self.infer_task_perform_mappers(err, ok, &args[2], &args[3], env)
    }

    fn infer_task_perform_mappers(
        &mut self,
        err: Type,
        ok: Type,
        ok_mapper_expr: &Expr,
        err_mapper_expr: &Expr,
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let ok_msg = self.fresh();
        let err_msg = self.fresh();
        let ok_mapper = self.infer_expr(ok_mapper_expr, env);
        self.unify(
            ok_mapper,
            Type::Fn(vec![ok], Box::new(ok_msg.clone())),
            ok_mapper_expr.span,
        );
        let err_mapper = self.infer_expr(err_mapper_expr, env);
        self.unify(
            err_mapper,
            Type::Fn(vec![err], Box::new(err_msg.clone())),
            err_mapper_expr.span,
        );
        let msg = self.join_types(ok_msg, err_msg, err_mapper_expr.span);
        Type::Cmd(Box::new(self.resolve(msg)))
    }

    fn infer_http_get_text(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Task(Box::new(Type::String), Box::new(Type::String));
        }

        let url = self.infer_expr(&args[0], env);
        self.unify(Type::String, url, args[0].span);
        Type::Task(Box::new(Type::String), Box::new(Type::String))
    }

    fn infer_http_get_json(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Task(Box::new(Type::String), Box::new(Type::Js));
        }

        let url = self.infer_expr(&args[0], env);
        self.unify(Type::String, url, args[0].span);
        Type::Task(Box::new(Type::String), Box::new(Type::Js))
    }

    fn infer_scope_update(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 5 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                5,
                args.len(),
            );
            return Type::Tuple(vec![self.fresh(), Type::Cmd(Box::new(self.fresh()))]);
        }

        let (parent_state, child_state) =
            self.infer_scope_parent_child_state(&args[0], &args[1], env);
        let child_msg = self.infer_expr(&args[2], env);
        let update_ty = self.infer_expr(&args[3], env);
        self.unify(
            update_ty,
            Type::Fn(
                vec![child_state.clone(), child_msg.clone()],
                Box::new(Type::Tuple(vec![
                    child_state.clone(),
                    Type::Cmd(Box::new(child_msg.clone())),
                ])),
            ),
            args[3].span,
        );

        let parent_msg = self.scope_wrapper_message_type(&args[4], child_msg, env);
        Type::Tuple(vec![parent_state, Type::Cmd(Box::new(parent_msg))])
    }

    fn infer_scope_subscriptions(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::Sub(Box::new(self.fresh()));
        }

        let child_state = self.infer_expr(&args[0], env);
        let child_msg = self.fresh();
        let subscriptions_ty = self.infer_expr(&args[1], env);
        self.unify(
            subscriptions_ty,
            Type::Fn(
                vec![child_state],
                Box::new(Type::Sub(Box::new(child_msg.clone()))),
            ),
            args[1].span,
        );

        let parent_msg = self.scope_wrapper_message_type(&args[2], child_msg, env);
        Type::Sub(Box::new(parent_msg))
    }

    fn infer_scope_view(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::Html;
        }

        self.infer_scope_tag(&args[0], env);
        let child_state = self.infer_expr(&args[2], env);
        let view_ty = self.infer_expr(&args[1], env);
        self.unify(
            view_ty,
            Type::Fn(vec![child_state], Box::new(Type::Html)),
            args[1].span,
        );
        Type::Html
    }

    fn infer_scope_parent_child_state(
        &mut self,
        parent_expr: &Expr,
        field_expr: &Expr,
        env: &mut HashMap<String, Type>,
    ) -> (Type, Type) {
        let parent_ty = self.infer_expr(parent_expr, env);
        let Some(field) = self.scope_keyword_name(field_expr, env, "scope field") else {
            let child_state = self.fresh();
            return (parent_ty, child_state);
        };

        let (child_state, updated_parent) =
            self.infer_field_read(parent_ty, &field, field_expr.span);
        let resolved_parent = self.resolve(updated_parent);
        if let ExprKind::Symbol(name) = &parent_expr.kind {
            env.insert(name.clone(), resolved_parent.clone());
        }
        (resolved_parent, child_state)
    }

    fn scope_wrapper_message_type(
        &mut self,
        tag_expr: &Expr,
        child_msg: Type,
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let tag = self
            .scope_keyword_name(tag_expr, env, "scope message tag")
            .map(|tag| Type::Keyword(Some(tag)))
            .unwrap_or(Type::Keyword(None));
        Type::Record(BTreeMap::from([
            ("kind".to_string(), tag),
            ("msg".to_string(), child_msg),
        ]))
    }

    fn infer_scope_tag(&mut self, tag_expr: &Expr, env: &mut HashMap<String, Type>) {
        let _ = self.scope_keyword_name(tag_expr, env, "scope message tag");
    }

    fn scope_keyword_name(
        &mut self,
        expr: &Expr,
        env: &mut HashMap<String, Type>,
        _label: &str,
    ) -> Option<String> {
        let ty = self.infer_expr(expr, env);
        self.unify(ty, Type::Keyword(None), expr.span);
        match &expr.kind {
            ExprKind::Keyword(name) => Some(name.clone()),
            _ => None,
        }
    }

    fn infer_sub_batch(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Sub(Box::new(self.fresh()));
        }

        let msg = self.fresh();
        let batch_ty = self.infer_expr(&args[0], env);
        match self.resolve(batch_ty) {
            Type::Vector(item) => {
                self.unify(Type::Sub(Box::new(msg.clone())), *item, args[0].span);
            }
            Type::Tuple(items) => {
                for item in items {
                    self.unify(Type::Sub(Box::new(msg.clone())), item, args[0].span);
                }
            }
            Type::Var(_) => {}
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    args[0].span,
                    format!(
                        "Sub.batch expects a vector of subscriptions, found {}",
                        found
                    ),
                ));
            }
        }
        Type::Sub(Box::new(self.resolve(msg)))
    }

    fn infer_cmd_batch(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }

        let msg = self.fresh();
        let batch_ty = self.infer_expr(&args[0], env);
        match self.resolve(batch_ty) {
            Type::Vector(item) => {
                self.unify(Type::Cmd(Box::new(msg.clone())), *item, args[0].span);
            }
            Type::Tuple(items) => {
                for item in items {
                    self.unify(Type::Cmd(Box::new(msg.clone())), item, args[0].span);
                }
            }
            Type::Var(_) => {}
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    args[0].span,
                    format!("Cmd.batch expects a vector of commands, found {}", found),
                ));
            }
        }
        Type::Cmd(Box::new(self.resolve(msg)))
    }

    fn infer_msg_of(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return self.msg_record_type(None, BTreeMap::new());
        }

        let kind = self.infer_expr(&args[0], env);
        let tag = self.expect_keyword_literal(kind, args[0].span, "Msg.of kind");
        self.msg_record_type(tag, BTreeMap::new())
    }

    fn infer_msg_with(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return self.msg_record_type(None, BTreeMap::new());
        }

        let kind = self.infer_expr(&args[0], env);
        let tag = self.expect_keyword_literal(kind, args[0].span, "Msg.with kind");
        let field = self.infer_expr(&args[1], env);
        let field_name = self
            .expect_keyword_literal(field, args[1].span, "Msg.with field")
            .unwrap_or_else(|| "value".to_string());
        let value = self.infer_expr(&args[2], env);
        self.msg_record_type(tag, BTreeMap::from([(field_name, value)]))
    }

    fn infer_msg_mapper(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            let value = self.fresh();
            return Type::Fn(
                vec![value.clone()],
                Box::new(
                    self.msg_record_type(None, BTreeMap::from([("value".to_string(), value)])),
                ),
            );
        }

        let kind = self.infer_expr(&args[0], env);
        let tag = self.expect_keyword_literal(kind, args[0].span, "Msg.mapper kind");
        let field = self.infer_expr(&args[1], env);
        let field_name = self
            .expect_keyword_literal(field, args[1].span, "Msg.mapper field")
            .unwrap_or_else(|| "value".to_string());
        let value = self.fresh();
        Type::Fn(
            vec![value.clone()],
            Box::new(self.msg_record_type(tag, BTreeMap::from([(field_name, value)]))),
        )
    }

    fn infer_msg_with2(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 5 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                5,
                args.len(),
            );
            return self.msg_record_type(None, BTreeMap::new());
        }

        let kind = self.infer_expr(&args[0], env);
        let tag = self.expect_keyword_literal(kind, args[0].span, "Msg.with2 kind");
        let first_field = self.infer_expr(&args[1], env);
        let first_field_name = self
            .expect_keyword_literal(first_field, args[1].span, "Msg.with2 first field")
            .unwrap_or_else(|| "first".to_string());
        let first_value = self.infer_expr(&args[2], env);
        let second_field = self.infer_expr(&args[3], env);
        let second_field_name = self
            .expect_keyword_literal(second_field, args[3].span, "Msg.with2 second field")
            .unwrap_or_else(|| "second".to_string());
        let second_value = self.infer_expr(&args[4], env);
        self.msg_record_type(
            tag,
            BTreeMap::from([
                (first_field_name, first_value),
                (second_field_name, second_value),
            ]),
        )
    }

    fn msg_record_type(&self, tag: Option<String>, mut fields: BTreeMap<String, Type>) -> Type {
        fields.insert("kind".to_string(), Type::Keyword(tag));
        Type::Record(fields)
    }

    fn expect_keyword_literal(&mut self, ty: Type, span: Span, label: &str) -> Option<String> {
        match self.resolve(ty) {
            Type::Keyword(Some(name)) => Some(name),
            Type::Keyword(None) | Type::Var(_) => None,
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!("{} must be a keyword literal, found {}", label, found),
                ));
                None
            }
        }
    }

    fn command_payload_type_from_format_expr(
        &mut self,
        expr: &Expr,
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let ty = self.infer_expr(expr, env);
        match self.resolve(ty) {
            Type::Keyword(Some(name)) => command_payload_type_from_format_name(Some(&name)),
            Type::String => match expr {
                Expr {
                    kind: ExprKind::String(name),
                    ..
                } => command_payload_type_from_format_name(Some(name)),
                _ => Type::Js,
            },
            _ => Type::Js,
        }
    }

    fn infer_cmd_payload_mapper(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        command_kind: &str,
        payload_ty: Type,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        let mapper_ty = self.infer_expr(&args[0], env);
        let msg = self.infer_command_mapper_message(
            command_kind,
            mapper_ty,
            payload_ty,
            args[0].span,
            ":toMessage",
        );
        Type::Cmd(Box::new(msg))
    }

    fn infer_cmd_storage_get(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 4 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                4,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        let payload = self.command_payload_type_from_format_expr(&args[1], env);
        let ok = self.infer_expr(&args[2], env);
        let ok_msg = self.infer_command_mapper_message(
            "storage/get",
            ok,
            payload,
            args[2].span,
            ":toMessage",
        );
        let err_msg = self.infer_command_tag_message("storage/get", "onError", &args[3], env);
        Type::Cmd(Box::new(self.join_types(ok_msg, err_msg, args[0].span)))
    }

    fn infer_cmd_storage_set(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 && args.len() != 4 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        self.infer_expr(&args[1], env);
        let msg = self.infer_expr(&args[2], env);
        let msg = if let Some(error) = args.get(3) {
            let err_msg = self.infer_command_tag_message("storage/set", "onError", error, env);
            self.join_types(msg, err_msg, args[0].span)
        } else {
            msg
        };
        Type::Cmd(Box::new(msg))
    }

    fn infer_cmd_storage_set_silent(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        self.infer_expr(&args[1], env);
        let err_msg = self.infer_command_tag_message("storage/set", "onError", &args[2], env);
        Type::Cmd(Box::new(err_msg))
    }

    fn infer_cmd_random_number(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_number_arg(&args[0], env);
        self.require_number_arg(&args[1], env);
        let mapper_ty = self.infer_expr(&args[2], env);
        let msg = self.infer_command_mapper_message(
            "random/number",
            mapper_ty,
            Type::Number,
            args[2].span,
            ":toMessage",
        );
        Type::Cmd(Box::new(msg))
    }

    fn infer_cmd_timer(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        command_kind: &str,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        self.require_number_arg(&args[1], env);
        let _ = command_kind;
        Type::Cmd(Box::new(self.infer_expr(&args[2], env)))
    }

    fn infer_cmd_timer_cancel(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        Type::Cmd(Box::new(self.fresh()))
    }

    fn infer_cmd_animation_frame(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        let msg = self.infer_command_tag_message("animation/frame", "onFrame", &args[1], env);
        Type::Cmd(Box::new(msg))
    }

    fn infer_cmd_animation_cancel(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        Type::Cmd(Box::new(self.infer_expr(&args[1], env)))
    }

    fn infer_cmd_dom_ref_action(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        command_kind: &str,
    ) -> Type {
        if args.len() != 2 && args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        let msg = self.infer_expr(&args[1], env);
        let msg = if let Some(error) = args.get(2) {
            let err_msg = self.infer_command_tag_message(command_kind, "onError", error, env);
            self.join_types(msg, err_msg, args[0].span)
        } else {
            msg
        };
        Type::Cmd(Box::new(msg))
    }

    fn infer_cmd_file_read_selected(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 5 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                5,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        let payload = self.command_payload_type_from_format_expr(&args[1], env);
        let ok = self.infer_expr(&args[2], env);
        let ok_msg = self.infer_command_mapper_message(
            "file/read-selected",
            ok,
            payload,
            args[2].span,
            ":toMessage",
        );
        let err_msg =
            self.infer_command_tag_message("file/read-selected", "onError", &args[3], env);
        let cancel_msg =
            self.infer_command_tag_message("file/read-selected", "onCancel", &args[4], env);
        let msg = self.join_types(ok_msg, err_msg, args[0].span);
        Type::Cmd(Box::new(self.join_types(msg, cancel_msg, args[0].span)))
    }

    fn infer_cmd_file_download(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 5 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                5,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        self.require_string_arg(&args[1], env);
        self.require_string_arg(&args[2], env);
        let msg = self.infer_expr(&args[3], env);
        let err_msg = self.infer_command_tag_message("file/download", "onError", &args[4], env);
        Type::Cmd(Box::new(self.join_types(msg, err_msg, args[0].span)))
    }

    fn infer_cmd_canvas_draw(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 5 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                5,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        self.require_number_arg(&args[1], env);
        self.require_number_arg(&args[2], env);
        self.infer_expr(&args[3], env);
        let err_msg = self.infer_command_tag_message("canvas/draw", "onError", &args[4], env);
        Type::Cmd(Box::new(err_msg))
    }

    fn infer_cmd_resize_watch(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 4 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                4,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        self.require_string_arg(&args[1], env);
        let change_msg =
            self.infer_command_tag_message("dom-ref/resize-watch", "onChange", &args[2], env);
        let err_msg =
            self.infer_command_tag_message("dom-ref/resize-watch", "onError", &args[3], env);
        Type::Cmd(Box::new(self.join_types(change_msg, err_msg, args[0].span)))
    }

    fn infer_cmd_dom_ref_measure(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        let mapper_ty = self.infer_expr(&args[1], env);
        let payload_ty = self
            .command_success_value_type(Some("dom-ref/measure"), &BTreeMap::new())
            .unwrap_or_else(|| self.fresh());
        let ok_msg = self.infer_command_mapper_message(
            "dom-ref/measure",
            mapper_ty,
            payload_ty,
            args[1].span,
            ":toMessage",
        );
        let err_msg = self.infer_command_tag_message("dom-ref/measure", "onError", &args[2], env);
        Type::Cmd(Box::new(self.join_types(ok_msg, err_msg, args[0].span)))
    }

    fn infer_cmd_bluetooth_connect_heart_rate(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 6 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                6,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        let options_ty = self.infer_expr(&args[1], env);
        self.unify(Type::Record(BTreeMap::new()), options_ty, args[1].span);
        let mapper_ty = self.infer_expr(&args[2], env);
        let payload_ty = self
            .command_success_value_type(Some("bluetooth/connect-heart-rate"), &BTreeMap::new())
            .unwrap_or_else(|| self.fresh());
        let ok_msg = self.infer_command_mapper_message(
            "bluetooth/connect-heart-rate",
            mapper_ty,
            payload_ty,
            args[2].span,
            ":toMessage",
        );
        let reading_msg = self.infer_command_tag_message(
            "bluetooth/connect-heart-rate",
            "onReading",
            &args[3],
            env,
        );
        let disconnected_msg = self.infer_command_tag_message(
            "bluetooth/connect-heart-rate",
            "onDisconnected",
            &args[4],
            env,
        );
        let err_msg = self.infer_command_tag_message(
            "bluetooth/connect-heart-rate",
            "onError",
            &args[5],
            env,
        );
        let msg = self.join_types(ok_msg, reading_msg, args[0].span);
        let msg = self.join_types(msg, disconnected_msg, args[0].span);
        Type::Cmd(Box::new(self.join_types(msg, err_msg, args[0].span)))
    }

    fn infer_cmd_bluetooth_disconnect(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        Type::Cmd(Box::new(self.infer_expr(&args[1], env)))
    }

    fn infer_cmd_simulation_heart_rate(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 5 && args.len() != 6 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects 5 or 6 arguments, found {}", name, args.len()),
            ));
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        let options_ty = self.infer_expr(&args[1], env);
        self.unify(Type::Record(BTreeMap::new()), options_ty, args[1].span);
        let mapper_ty = self.infer_expr(&args[2], env);
        let payload_ty = self
            .command_success_value_type(Some("simulation/heart-rate"), &BTreeMap::new())
            .unwrap_or_else(|| self.fresh());
        let ok_msg = self.infer_command_mapper_message(
            "simulation/heart-rate",
            mapper_ty,
            payload_ty,
            args[2].span,
            ":toMessage",
        );
        let reading_msg =
            self.infer_command_tag_message("simulation/heart-rate", "onReading", &args[3], env);
        let msg = self.join_types(ok_msg, reading_msg, args[0].span);
        let (msg, error_index) = if args.len() == 6 {
            let disconnected_msg = self.infer_command_tag_message(
                "simulation/heart-rate",
                "onDisconnected",
                &args[4],
                env,
            );
            (self.join_types(msg, disconnected_msg, args[0].span), 5)
        } else {
            (msg, 4)
        };
        let err_msg = self.infer_command_tag_message(
            "simulation/heart-rate",
            "onError",
            &args[error_index],
            env,
        );
        Type::Cmd(Box::new(self.join_types(msg, err_msg, args[0].span)))
    }

    fn infer_cmd_simulation_stop(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Cmd(Box::new(self.fresh()));
        }
        self.require_string_arg(&args[0], env);
        Type::Cmd(Box::new(self.fresh()))
    }

    fn infer_command_mapper_message(
        &mut self,
        command_kind: &str,
        mapper_ty: Type,
        payload_ty: Type,
        span: Span,
        label: &str,
    ) -> Type {
        match self.resolve(mapper_ty) {
            Type::Var(_) => self.fresh(),
            Type::Fn(args, ret) => {
                if args.len() != 1 {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!(
                            "{} command {} must accept exactly one argument",
                            command_kind, label
                        ),
                    ));
                    return self.fresh();
                }
                self.unify(args[0].clone(), payload_ty, span);
                *ret
            }
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!(
                        "{} command {} must be a function, found {}",
                        command_kind, label, found
                    ),
                ));
                self.fresh()
            }
        }
    }

    fn infer_command_tag_message(
        &mut self,
        command_kind: &str,
        field: &str,
        arg: &Expr,
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let tag_ty = self.infer_expr(arg, env);
        match self.resolve(tag_ty) {
            Type::Keyword(Some(tag)) => {
                self.command_message_tag_type(Some(command_kind), field, &tag, &BTreeMap::new())
            }
            Type::Keyword(None) | Type::Var(_) => self.fresh(),
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    arg.span,
                    format!(
                        "{} command continuation :{} must be a keyword tag, found {}",
                        command_kind, field, found
                    ),
                ));
                self.fresh()
            }
        }
    }

    fn require_string_arg(&mut self, arg: &Expr, env: &mut HashMap<String, Type>) {
        let ty = self.infer_expr(arg, env);
        self.unify(Type::String, ty, arg.span);
    }

    fn require_number_arg(&mut self, arg: &Expr, env: &mut HashMap<String, Type>) {
        let ty = self.infer_expr(arg, env);
        self.unify(Type::Number, ty, arg.span);
    }

    fn infer_sub_timer_every(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::Sub(Box::new(self.fresh()));
        }

        let id = self.infer_expr(&args[0], env);
        self.unify(Type::String, id, args[0].span);
        let ms = self.infer_expr(&args[1], env);
        self.unify(Type::Number, ms, args[1].span);
        let msg = self.infer_expr(&args[2], env);
        Type::Sub(Box::new(msg))
    }

    fn infer_sub_change(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        subscription_kind: &str,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::Sub(Box::new(self.fresh()));
        }

        let id = self.infer_expr(&args[0], env);
        self.unify(Type::String, id, args[0].span);
        let target = self.infer_expr(&args[1], env);
        self.unify(Type::String, target, args[1].span);
        let tag = self.infer_expr(&args[2], env);
        let msg = self.subscription_tag_message_type(
            Some(subscription_kind),
            "onChange",
            tag,
            args[2].span,
        );
        Type::Sub(Box::new(msg))
    }

    fn infer_sub_window_event(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 && args.len() != 4 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::Sub(Box::new(self.fresh()));
        }

        let id = self.infer_expr(&args[0], env);
        self.unify(Type::String, id, args[0].span);
        let event_type = self.infer_expr(&args[1], env);
        self.unify(Type::String, event_type, args[1].span);
        let tag = self.infer_expr(&args[2], env);
        if let Some(options) = args.get(3) {
            self.infer_expr(options, env);
        }
        let msg = self.subscription_tag_message_type(
            Some("sub/window/event"),
            "onEvent",
            tag,
            args[2].span,
        );
        Type::Sub(Box::new(msg))
    }

    fn infer_sub_window_event_with(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 4 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                4,
                args.len(),
            );
            return Type::Sub(Box::new(self.fresh()));
        }

        let id = self.infer_expr(&args[0], env);
        self.unify(Type::String, id, args[0].span);
        let event_type = self.infer_expr(&args[1], env);
        self.unify(Type::String, event_type, args[1].span);
        let tag = self.infer_expr(&args[2], env);
        let config_ty = self.infer_expr(&args[3], env);
        self.unify(Type::Record(BTreeMap::new()), config_ty, args[3].span);
        let msg = self.subscription_tag_message_type(
            Some("sub/window/event"),
            "onEvent",
            tag,
            args[2].span,
        );
        Type::Sub(Box::new(msg))
    }

    fn infer_numeric_call(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.is_empty() {
            self.diagnostics.push(Diagnostic::error(
                Span::default(),
                format!("{} expects at least one argument", name),
            ));
        }
        for arg in args {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::Number, arg.span);
        }
        Type::Number
    }

    fn infer_numeric_vector_aggregate(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        allow_fallbacks: bool,
    ) -> Type {
        if args.is_empty() || (!allow_fallbacks && args.len() != 1) {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                if allow_fallbacks {
                    format!("{} expects a numeric vector", name)
                } else {
                    format!("{} expects exactly one numeric vector", name)
                },
            ));
            return Type::Number;
        }

        let values_ty = self.infer_expr(&args[0], env);
        self.unify(
            values_ty,
            Type::Vector(Box::new(Type::Number)),
            args[0].span,
        );
        for fallback in args.iter().skip(1) {
            let fallback_ty = self.infer_expr(fallback, env);
            self.unify(fallback_ty, Type::Number, fallback.span);
        }
        Type::Number
    }

    fn infer_unary_number(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
        }
        if let Some(arg) = args.first() {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::Number, arg.span);
        }
        Type::Number
    }

    fn infer_binary_number(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
        }
        for arg in args.iter().take(2) {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::Number, arg.span);
        }
        Type::Number
    }

    fn infer_zero_arg_bool(&mut self, name: &str, args: &[Expr]) -> Type {
        if !args.is_empty() {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                0,
                args.len(),
            );
        }
        Type::Bool
    }

    fn infer_to_fixed(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::String;
        }

        let value_ty = self.infer_expr(&args[0], env);
        self.unify(value_ty, Type::Number, args[0].span);
        let digits_ty = self.infer_expr(&args[1], env);
        self.unify(digits_ty, Type::Number, args[1].span);
        Type::String
    }

    fn infer_to_number(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
        }
        if let Some(arg) = args.first() {
            self.infer_expr(arg, env);
        }
        Type::Number
    }

    fn infer_date_format(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::String;
        }

        let timestamp_ty = self.infer_expr(&args[0], env);
        self.unify(timestamp_ty, Type::Number, args[0].span);
        let style_ty = self.infer_expr(&args[1], env);
        self.unify(style_ty, Type::Keyword(None), args[1].span);
        Type::String
    }

    fn infer_comparison_call(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects at least two arguments", name),
            ));
        }

        if name == "=" {
            let expected = args.first().map(|arg| self.infer_expr(arg, env));
            for arg in args.iter().skip(1) {
                let ty = self.infer_expr(arg, env);
                if let Some(expected) = &expected {
                    self.unify(ty, expected.clone(), arg.span);
                }
            }
        } else {
            for arg in args {
                let ty = self.infer_expr(arg, env);
                self.unify(ty, Type::Number, arg.span);
            }
        }
        Type::Bool
    }

    fn infer_identity_call(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects at least two arguments", name),
            ));
        }
        for arg in args {
            self.infer_expr(arg, env);
        }
        Type::Bool
    }

    fn infer_unary_bool(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
        }
        if let Some(arg) = args.first() {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::Bool, arg.span);
        }
        Type::Bool
    }

    fn infer_bool_call(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.is_empty() {
            self.diagnostics.push(Diagnostic::error(
                Span::default(),
                format!("{} expects at least one argument", name),
            ));
        }
        for arg in args {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::Bool, arg.span);
        }
        Type::Bool
    }

    fn infer_ok(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Result(Box::new(self.fresh()), Box::new(self.fresh()));
        }

        let value_ty = self.infer_expr(&args[0], env);
        Type::Result(Box::new(value_ty), Box::new(self.fresh()))
    }

    fn infer_err(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Result(Box::new(self.fresh()), Box::new(self.fresh()));
        }

        let error_ty = self.infer_expr(&args[0], env);
        Type::Result(Box::new(self.fresh()), Box::new(error_ty))
    }

    fn infer_result_predicate(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Bool;
        }

        let ok_ty = self.fresh();
        let err_ty = self.fresh();
        let result_ty = self.infer_expr(&args[0], env);
        self.unify(
            result_ty,
            Type::Result(Box::new(ok_ty), Box::new(err_ty)),
            args[0].span,
        );
        Type::Bool
    }

    fn infer_result_projection(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        error: bool,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Option(Box::new(self.fresh()));
        }

        let ok_ty = self.fresh();
        let err_ty = self.fresh();
        let result_ty = self.infer_expr(&args[0], env);
        self.unify(
            result_ty,
            Type::Result(Box::new(ok_ty.clone()), Box::new(err_ty.clone())),
            args[0].span,
        );

        if error {
            Type::Option(Box::new(self.resolve(err_ty)))
        } else {
            Type::Option(Box::new(self.resolve(ok_ty)))
        }
    }

    fn infer_unwrap_or(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return self.fresh();
        }

        let result_ty = self.infer_expr(&args[0], env);
        let fallback_ty = self.infer_expr(&args[1], env);
        let ok_ty = self.fresh();
        let err_ty = self.fresh();
        self.unify(fallback_ty, ok_ty.clone(), args[1].span);
        self.unify(
            result_ty,
            Type::Result(Box::new(ok_ty.clone()), Box::new(err_ty)),
            args[0].span,
        );
        self.resolve(ok_ty)
    }

    fn infer_fail(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return self.fresh();
        }

        let message_ty = self.infer_expr(&args[0], env);
        self.unify(message_ty, Type::String, args[0].span);
        self.fresh()
    }

    fn infer_hash_map(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() % 2 != 0 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects key/value pairs", name),
            ));
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Map(Box::new(self.fresh()), Box::new(self.fresh()));
        }

        let key_ty = self.fresh();
        let value_ty = self.fresh();
        for pair in args.chunks(2) {
            let [key, value] = pair else {
                continue;
            };
            let inferred_key = self.infer_expr(key, env);
            self.unify(inferred_key, key_ty.clone(), key.span);
            let inferred_value = self.infer_expr(value, env);
            self.unify(inferred_value, value_ty.clone(), value.span);
        }

        Type::Map(
            Box::new(self.resolve(key_ty)),
            Box::new(self.resolve(value_ty)),
        )
    }

    fn infer_map_get(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Option(Box::new(self.fresh()));
        }

        let key_ty = self.fresh();
        let value_ty = self.fresh();
        let map_ty = self.infer_expr(&args[0], env);
        self.unify(
            map_ty,
            Type::Map(Box::new(key_ty.clone()), Box::new(value_ty.clone())),
            args[0].span,
        );
        let actual_key_ty = self.infer_expr(&args[1], env);
        self.unify(actual_key_ty, key_ty, args[1].span);
        Type::Option(Box::new(self.resolve(value_ty)))
    }

    fn infer_map_assoc(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() < 3 || args[1..].len() % 2 != 0 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects a map followed by key/value pairs", name),
            ));
            for arg in args {
                self.infer_expr(arg, env);
            }
            return Type::Map(Box::new(self.fresh()), Box::new(self.fresh()));
        }

        let key_ty = self.fresh();
        let value_ty = self.fresh();
        let map_ty = self.infer_expr(&args[0], env);
        self.unify(
            map_ty,
            Type::Map(Box::new(key_ty.clone()), Box::new(value_ty.clone())),
            args[0].span,
        );
        for pair in args[1..].chunks(2) {
            let [key, value] = pair else {
                continue;
            };
            let inferred_key = self.infer_expr(key, env);
            self.unify(inferred_key, key_ty.clone(), key.span);
            let inferred_value = self.infer_expr(value, env);
            self.unify(inferred_value, value_ty.clone(), value.span);
        }

        Type::Map(
            Box::new(self.resolve(key_ty)),
            Box::new(self.resolve(value_ty)),
        )
    }

    fn infer_map_dissoc(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() < 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Map(Box::new(self.fresh()), Box::new(self.fresh()));
        }

        let key_ty = self.fresh();
        let value_ty = self.fresh();
        let map_ty = self.infer_expr(&args[0], env);
        self.unify(
            map_ty,
            Type::Map(Box::new(key_ty.clone()), Box::new(value_ty.clone())),
            args[0].span,
        );
        for key in &args[1..] {
            let inferred_key = self.infer_expr(key, env);
            self.unify(inferred_key, key_ty.clone(), key.span);
        }

        Type::Map(
            Box::new(self.resolve(key_ty)),
            Box::new(self.resolve(value_ty)),
        )
    }

    fn infer_map_entries(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let (key_ty, value_ty) = self.infer_map_projection_input(name, args, env);
        let mut fields = BTreeMap::new();
        fields.insert("key".to_string(), key_ty);
        fields.insert("value".to_string(), value_ty);
        Type::Vector(Box::new(Type::Record(fields)))
    }

    fn infer_map_keys(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let (key_ty, _) = self.infer_map_projection_input(name, args, env);
        Type::Vector(Box::new(key_ty))
    }

    fn infer_map_values(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let (_, value_ty) = self.infer_map_projection_input(name, args, env);
        Type::Vector(Box::new(value_ty))
    }

    fn infer_map_projection_input(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> (Type, Type) {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return (self.fresh(), self.fresh());
        }

        let key_ty = self.fresh();
        let value_ty = self.fresh();
        let map_ty = self.infer_expr(&args[0], env);
        self.unify(
            map_ty,
            Type::Map(Box::new(key_ty.clone()), Box::new(value_ty.clone())),
            args[0].span,
        );
        (self.resolve(key_ty), self.resolve(value_ty))
    }

    fn infer_object_entries(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            let fields = BTreeMap::from([
                ("key".to_string(), Type::String),
                ("value".to_string(), self.fresh()),
            ]);
            return Type::Vector(Box::new(Type::Record(fields)));
        }

        let value_ty = self.infer_object_projection_input(&args[0], env);
        let fields = BTreeMap::from([
            ("key".to_string(), Type::String),
            ("value".to_string(), value_ty),
        ]);
        Type::Vector(Box::new(Type::Record(fields)))
    }

    fn infer_object_keys(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Vector(Box::new(Type::String));
        }

        self.infer_object_projection_input(&args[0], env);
        Type::Vector(Box::new(Type::String))
    }

    fn infer_object_values(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Vector(Box::new(self.fresh()));
        }

        Type::Vector(Box::new(self.infer_object_projection_input(&args[0], env)))
    }

    fn infer_object_projection_input(
        &mut self,
        expr: &Expr,
        env: &mut HashMap<String, Type>,
    ) -> Type {
        let collection_ty = self.infer_expr(expr, env);
        let value_ty = self.fresh();
        match self.shallow_resolve(collection_ty) {
            Type::Record(fields) => {
                let mut merged: Option<Type> = None;
                for field_ty in fields.into_values() {
                    merged = Some(match merged {
                        Some(current) => {
                            self.unify(current.clone(), field_ty, expr.span);
                            current
                        }
                        None => field_ty,
                    });
                }
                self.resolve(merged.unwrap_or(value_ty))
            }
            Type::Map(key_ty, map_value_ty) => {
                self.unify(*key_ty, Type::String, expr.span);
                self.resolve(*map_value_ty)
            }
            Type::Var(_) => value_ty,
            _ => value_ty,
        }
    }

    fn infer_object_assoc(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() < 3 || args[1..].len() % 2 != 0 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects an object followed by key/value pairs", name),
            ));
            for arg in args {
                self.infer_expr(arg, env);
            }
            return self.fresh();
        }

        self.infer_expr(&args[0], env);
        for pair in args[1..].chunks(2) {
            let [key, value] = pair else {
                continue;
            };
            let key_ty = self.infer_expr(key, env);
            self.unify(key_ty, Type::String, key.span);
            self.infer_expr(value, env);
        }
        self.fresh()
    }

    fn infer_object_dissoc(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects an object followed by one or more keys", name),
            ));
            for arg in args {
                self.infer_expr(arg, env);
            }
            return self.fresh();
        }

        self.infer_expr(&args[0], env);
        for key in &args[1..] {
            let key_ty = self.infer_expr(key, env);
            self.unify(key_ty, Type::String, key.span);
        }
        self.fresh()
    }

    fn infer_get_in(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return self.fresh();
        }

        let base_ty = self.infer_expr(&args[0], env);
        let Some(path) = self.static_record_path(name, &args[1]) else {
            return self.fresh();
        };
        self.infer_static_path_read(base_ty, &path, args[1].span)
    }

    fn infer_update_in(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() < 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return self.fresh();
        }

        let base_ty = self.infer_expr(&args[0], env);
        let Some(path) = self.static_record_path(name, &args[1]) else {
            self.infer_expr(&args[2], env);
            for arg in args.iter().skip(3) {
                self.infer_expr(arg, env);
            }
            return base_ty;
        };
        let current_ty = self.infer_static_path_read(base_ty.clone(), &path, args[1].span);
        let extra_arg_tys = args
            .iter()
            .skip(3)
            .map(|arg| self.infer_expr(arg, env))
            .collect::<Vec<_>>();
        let next_ty = self.fresh();
        let mut updater_args = Vec::with_capacity(1 + extra_arg_tys.len());
        updater_args.push(current_ty);
        updater_args.extend(extra_arg_tys);
        let updater_ty = self.infer_expr(&args[2], env);
        self.unify(
            updater_ty,
            Type::Fn(updater_args, Box::new(next_ty.clone())),
            args[2].span,
        );
        let next_ty = self.resolve(next_ty);
        self.record_with_static_path_update(base_ty, &path, next_ty, args[1].span)
    }

    fn static_record_path(&mut self, name: &str, path: &Expr) -> Option<Vec<String>> {
        let ExprKind::Vector(items) = &path.kind else {
            self.diagnostics.push(Diagnostic::error(
                path.span,
                format!(
                    "{} path must be a static vector of keywords, strings, or symbols",
                    name
                ),
            ));
            return None;
        };

        let mut fields = Vec::new();
        for item in items {
            let Some(field) = record_key_name(item) else {
                self.diagnostics.push(Diagnostic::error(
                    item.span,
                    format!(
                        "{} path entries must be keywords, strings, or symbols",
                        name
                    ),
                ));
                return None;
            };
            fields.push(field);
        }
        Some(fields)
    }

    fn infer_static_path_read(&mut self, base_ty: Type, path: &[String], span: Span) -> Type {
        let mut current_ty = base_ty;
        for field in path {
            let (field_ty, _) = self.infer_field_read(current_ty, field, span);
            current_ty = field_ty;
        }
        current_ty
    }

    fn record_with_static_path_update(
        &mut self,
        base_ty: Type,
        path: &[String],
        next_ty: Type,
        span: Span,
    ) -> Type {
        let Some((field, rest)) = path.split_first() else {
            return next_ty;
        };

        match self.shallow_resolve(base_ty) {
            Type::Var(id) => {
                let nested_ty = self.fresh();
                let updated_nested =
                    self.record_with_static_path_update(nested_ty, rest, next_ty, span);
                self.bind(
                    id,
                    Type::Record(BTreeMap::from([(field.clone(), updated_nested)])),
                    span,
                )
            }
            Type::Record(mut fields) => {
                let current = fields.remove(field).unwrap_or_else(|| self.fresh());
                let updated_nested =
                    self.record_with_static_path_update(current, rest, next_ty, span);
                fields.insert(field.clone(), updated_nested);
                Type::Record(fields)
            }
            Type::Js => Type::Js,
            other => {
                let type_name = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!(
                        "update-in expects records along its path, found {}",
                        type_name
                    ),
                ));
                other
            }
        }
    }

    fn infer_assoc(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() < 3 || args[1..].len() % 2 != 0 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects a record followed by key/value pairs", name),
            ));
            return Type::Record(BTreeMap::new());
        }

        let base_ty = self.infer_expr(&args[0], env);
        let mut fields = match self.shallow_resolve(base_ty.clone()) {
            Type::Record(fields) => fields,
            Type::Var(_) => BTreeMap::new(),
            other => {
                let type_name = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    args[0].span,
                    format!("assoc expects a record, found {}", type_name),
                ));
                BTreeMap::new()
            }
        };

        for pair in args[1..].chunks(2) {
            let [key, value] = pair else {
                continue;
            };
            let Some(name) = record_key_name(key) else {
                self.diagnostics.push(Diagnostic::error(
                    key.span,
                    "assoc keys must be keywords, strings, or symbols",
                ));
                continue;
            };
            let value_ty = self.infer_expr(value, env);
            fields.insert(name, value_ty);
        }

        self.unify(base_ty, Type::Record(fields), args[0].span)
    }

    fn infer_merge(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.is_empty() {
            self.arity_error(Span::default(), name, 1, 0);
            return Type::Record(BTreeMap::new());
        }

        let mut merged = BTreeMap::new();
        for arg in args {
            let ty = self.infer_expr(arg, env);
            match self.shallow_resolve(ty.clone()) {
                Type::Record(fields) => {
                    merged.extend(fields);
                }
                Type::Var(_) => {
                    self.unify(ty, Type::Record(BTreeMap::new()), arg.span);
                }
                other => {
                    let type_name = self.format_type(&other);
                    self.diagnostics.push(Diagnostic::error(
                        arg.span,
                        format!("merge expects records, found {}", type_name),
                    ));
                }
            }
        }

        Type::Record(merged)
    }

    fn infer_dissoc(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() < 2 {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects a record and at least one key", name),
            ));
            return Type::Record(BTreeMap::new());
        }

        let base_ty = self.infer_expr(&args[0], env);
        let mut fields = match self.shallow_resolve(base_ty.clone()) {
            Type::Record(fields) => fields,
            Type::Var(_) => BTreeMap::new(),
            other => {
                let type_name = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    args[0].span,
                    format!("dissoc expects a record, found {}", type_name),
                ));
                BTreeMap::new()
            }
        };

        for key in &args[1..] {
            let Some(name) = record_key_name(key) else {
                self.diagnostics.push(Diagnostic::error(
                    key.span,
                    "dissoc keys must be keywords, strings, or symbols",
                ));
                continue;
            };
            fields.remove(&name);
        }

        match self.shallow_resolve(base_ty) {
            Type::Var(id) => self.bind(id, Type::Record(fields), args[0].span),
            Type::Record(_) => Type::Record(fields),
            _ => Type::Record(fields),
        }
    }

    fn infer_unary_string(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::String;
        }
        if let Some(arg) = args.first() {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::String, arg.span);
        }
        Type::String
    }

    fn infer_zero_arg_string(&mut self, name: &str, args: &[Expr]) -> Type {
        if !args.is_empty() {
            self.arity_error(Span::default(), name, 0, args.len());
        }
        Type::String
    }

    fn infer_zero_arg_nil(&mut self, name: &str, args: &[Expr]) -> Type {
        if !args.is_empty() {
            self.arity_error(Span::default(), name, 0, args.len());
        }
        Type::Nil
    }

    fn infer_fixed_string_args(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        arity: usize,
        return_type: Type,
    ) -> Type {
        if args.len() != arity {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                arity,
                args.len(),
            );
            return return_type;
        }
        for arg in args {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::String, arg.span);
        }
        return_type
    }

    fn infer_clipboard_text(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::String;
        }
        self.infer_expr(&args[0], env);
        Type::String
    }

    fn infer_regex_capture(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if !(2..=3).contains(&args.len()) {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects 2 or 3 arguments, found {}", name, args.len()),
            ));
            return Type::String;
        }
        for arg in args {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::String, arg.span);
        }
        Type::String
    }

    fn infer_regex_capture_all(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if !(2..=3).contains(&args.len()) {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects 2 or 3 arguments, found {}", name, args.len()),
            ));
            return Type::Vector(Box::new(Type::Vector(Box::new(Type::String))));
        }
        for arg in args {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::String, arg.span);
        }
        Type::Vector(Box::new(Type::Vector(Box::new(Type::String))))
    }

    fn infer_split(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Vector(Box::new(Type::String));
        }
        for arg in args {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::String, arg.span);
        }
        Type::Vector(Box::new(Type::String))
    }

    fn infer_join(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::String;
        }

        let items_ty = self.infer_expr(&args[0], env);
        self.unify(items_ty, Type::Vector(Box::new(Type::String)), args[0].span);
        let separator_ty = self.infer_expr(&args[1], env);
        self.unify(separator_ty, Type::String, args[1].span);
        Type::String
    }

    fn infer_binary_string_predicate(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Bool;
        }
        for arg in args {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::String, arg.span);
        }
        Type::Bool
    }

    fn infer_includes(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Bool;
        }
        let haystack_ty = self.infer_expr(&args[0], env);
        let needle_ty = self.infer_expr(&args[1], env);
        match self.shallow_resolve(haystack_ty) {
            Type::String => {
                self.unify(needle_ty, Type::String, args[1].span);
            }
            Type::Vector(element_ty) => {
                self.unify(needle_ty, *element_ty, args[1].span);
            }
            Type::Var(id) => {
                self.bind(id, Type::Vector(Box::new(needle_ty)), args[0].span);
            }
            other => {
                self.unify(other, Type::Vector(Box::new(needle_ty)), args[0].span);
            }
        }
        Type::Bool
    }

    fn infer_contains(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Bool;
        }

        let collection_ty = self.infer_expr(&args[0], env);
        let value_ty = self.infer_expr(&args[1], env);
        match self.shallow_resolve(collection_ty) {
            Type::Set(element_ty) | Type::Vector(element_ty) => {
                self.unify(value_ty, *element_ty, args[1].span);
            }
            Type::Map(key_ty, _) => {
                self.unify(value_ty, *key_ty, args[1].span);
            }
            Type::String => {
                self.unify(value_ty, Type::String, args[1].span);
            }
            Type::Var(id) => {
                self.bind(id, Type::Set(Box::new(value_ty)), args[0].span);
            }
            other => {
                self.unify(other, Type::Set(Box::new(value_ty)), args[0].span);
            }
        }
        Type::Bool
    }

    fn infer_pad_start(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return Type::String;
        }
        let value_ty = self.infer_expr(&args[0], env);
        self.unify(value_ty, Type::String, args[0].span);
        let width_ty = self.infer_expr(&args[1], env);
        self.unify(width_ty, Type::Number, args[1].span);
        let fill_ty = self.infer_expr(&args[2], env);
        self.unify(fill_ty, Type::String, args[2].span);
        Type::String
    }

    fn infer_to_radix(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::String;
        }
        let value_ty = self.infer_expr(&args[0], env);
        self.unify(value_ty, Type::Number, args[0].span);
        let radix_ty = self.infer_expr(&args[1], env);
        self.unify(radix_ty, Type::Number, args[1].span);
        Type::String
    }

    fn infer_string_slice(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if !(2..=3).contains(&args.len()) {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!(
                    "{} expects a string, start index, and optional end index",
                    name
                ),
            ));
            return Type::String;
        }
        let value_ty = self.infer_expr(&args[0], env);
        self.unify(value_ty, Type::String, args[0].span);
        for index in &args[1..] {
            let index_ty = self.infer_expr(index, env);
            self.unify(index_ty, Type::Number, index.span);
        }
        Type::String
    }

    fn infer_regex_test(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if !(2..=3).contains(&args.len()) {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects a string, pattern, and optional flags", name),
            ));
            return Type::Bool;
        }

        for arg in args {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::String, arg.span);
        }
        Type::Bool
    }

    fn infer_locale_compare(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Number;
        }

        let left_ty = self.infer_expr(&args[0], env);
        self.unify(left_ty, Type::String, args[0].span);
        let right_ty = self.infer_expr(&args[1], env);
        self.unify(right_ty, Type::String, args[1].span);
        Type::Number
    }

    fn infer_json_stringify(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if !(1..=2).contains(&args.len()) {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::String;
        }

        if let Some(value) = args.first() {
            self.infer_expr(value, env);
        }
        if let Some(space) = args.get(1) {
            let ty = self.infer_expr(space, env);
            self.unify(ty, Type::Number, space.span);
        }

        Type::String
    }

    fn infer_json_parse(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return self.fresh();
        }

        let ty = self.infer_expr(&args[0], env);
        self.unify(ty, Type::String, args[0].span);
        self.fresh()
    }

    fn infer_json_parse_result(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Result(Box::new(self.fresh()), Box::new(Type::String));
        }

        let ty = self.infer_expr(&args[0], env);
        self.unify(ty, Type::String, args[0].span);
        Type::Result(Box::new(self.fresh()), Box::new(Type::String))
    }

    fn infer_zero_arg_decoder(&mut self, name: &str, args: &[Expr], ty: Type) -> Type {
        if !args.is_empty() {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                0,
                args.len(),
            );
        }
        Type::Decoder(Box::new(ty))
    }

    fn infer_decoder_literal(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Decoder(Box::new(self.fresh()));
        }

        let ty = self.infer_expr(&args[0], env);
        Type::Decoder(Box::new(self.resolve(ty)))
    }

    fn infer_decoder_optional(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Decoder(Box::new(Type::Option(Box::new(self.fresh()))));
        }

        let inner = self.infer_decoder_inner_arg(&args[0], env);
        Type::Decoder(Box::new(Type::Option(Box::new(inner))))
    }

    fn infer_decoder_vector(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Decoder(Box::new(Type::Vector(Box::new(self.fresh()))));
        }

        let inner = self.infer_decoder_inner_arg(&args[0], env);
        Type::Decoder(Box::new(Type::Vector(Box::new(inner))))
    }

    fn infer_decoder_record(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Decoder(Box::new(Type::Record(BTreeMap::new())));
        }

        let ExprKind::Map(entries) = &args[0].kind else {
            self.diagnostics.push(Diagnostic::error(
                args[0].span,
                "decoder-record expects a record literal of decoder fields",
            ));
            self.infer_expr(&args[0], env);
            return Type::Decoder(Box::new(Type::Record(BTreeMap::new())));
        };

        let mut fields = BTreeMap::new();
        for (key, decoder) in entries {
            let Some(name) = record_key_name(key) else {
                self.diagnostics.push(Diagnostic::error(
                    key.span,
                    "decoder-record field names must be keywords, strings, or symbols",
                ));
                self.infer_expr(decoder, env);
                continue;
            };
            let field_ty = self.infer_decoder_inner_arg(decoder, env);
            fields.insert(name, field_ty);
        }
        Type::Decoder(Box::new(Type::Record(fields)))
    }

    fn infer_decode(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Result(Box::new(self.fresh()), Box::new(Type::String));
        }

        let inner = self.infer_decoder_inner_arg(&args[0], env);
        self.infer_expr(&args[1], env);
        Type::Result(Box::new(inner), Box::new(Type::String))
    }

    fn infer_decoder_inner_arg(&mut self, expr: &Expr, env: &mut HashMap<String, Type>) -> Type {
        let inner = self.fresh();
        let decoder = self.infer_expr(expr, env);
        self.unify(decoder, Type::Decoder(Box::new(inner.clone())), expr.span);
        self.resolve(inner)
    }

    fn infer_url_resolve(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Option(Box::new(Type::String));
        }

        for arg in args {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::String, arg.span);
        }
        Type::Option(Box::new(Type::String))
    }

    fn infer_url_part(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Option(Box::new(Type::String));
        }

        let ty = self.infer_expr(&args[0], env);
        self.unify(ty, Type::String, args[0].span);
        Type::Option(Box::new(Type::String))
    }

    fn infer_count(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Number;
        }
        let collection_ty = self.infer_expr(&args[0], env);
        let element_ty = self.fresh();
        self.unify_sized_collection(collection_ty, element_ty, args[0].span);
        Type::Number
    }

    fn infer_empty(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Bool;
        }
        let collection_ty = self.infer_expr(&args[0], env);
        let element_ty = self.fresh();
        self.unify_sized_collection(collection_ty, element_ty, args[0].span);
        Type::Bool
    }

    fn unify_sized_collection(&mut self, collection_ty: Type, element_ty: Type, span: Span) {
        match self.shallow_resolve(collection_ty) {
            Type::Var(id) => {
                self.bind(id, Type::Vector(Box::new(element_ty)), span);
            }
            Type::List(_) | Type::Vector(_) | Type::Set(_) | Type::Map(_, _) | Type::String => {}
            other => {
                self.unify(other, Type::Vector(Box::new(element_ty)), span);
            }
        }
    }

    fn infer_some(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
        }
        if let Some(arg) = args.first() {
            self.infer_expr(arg, env);
        }
        Type::Bool
    }

    fn infer_predicate(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
        }
        if let Some(arg) = args.first() {
            self.infer_expr(arg, env);
        }
        Type::Bool
    }

    fn infer_get(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return self.fresh();
        }
        let base_ty = self.infer_expr(&args[0], env);
        self.infer_expr(&args[1], env);
        if matches!(self.resolve(base_ty), Type::Js) {
            return Type::Js;
        }
        self.fresh()
    }

    fn infer_object_get(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return self.fresh();
        }

        let base_ty = self.infer_expr(&args[0], env);
        let key_ty = self.infer_expr(&args[1], env);
        self.unify(key_ty, Type::String, args[1].span);
        if matches!(self.resolve(base_ty), Type::Js) {
            return Type::Js;
        }
        self.fresh()
    }

    fn infer_nth(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return self.fresh();
        }
        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify_ordered_collection(
            collection_ty,
            element_ty.clone(),
            args[0].span,
            CollectionKind::Vector,
        );
        let index_ty = self.infer_expr(&args[1], env);
        self.unify(index_ty, Type::Number, args[1].span);
        self.resolve(element_ty)
    }

    fn infer_ordered_access(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return self.fresh();
        }
        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify_ordered_collection(
            collection_ty,
            element_ty.clone(),
            args[0].span,
            CollectionKind::Vector,
        );
        self.resolve(element_ty)
    }

    fn infer_last(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return self.fresh();
        }
        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify_ordered_collection(
            collection_ty,
            element_ty.clone(),
            args[0].span,
            CollectionKind::Vector,
        );
        self.resolve(element_ty)
    }

    fn infer_cons(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::List(Box::new(self.fresh()));
        }

        let item_ty = self.infer_expr(&args[0], env);
        let collection_ty = self.infer_expr(&args[1], env);
        let element_ty = self.unify_ordered_collection(
            collection_ty,
            item_ty.clone(),
            args[1].span,
            CollectionKind::List,
        );
        self.unify(item_ty, element_ty.clone(), args[0].span);
        Type::List(Box::new(self.resolve(element_ty)))
    }

    fn infer_rest(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::List(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        let element_ty = self.unify_ordered_collection(
            collection_ty,
            element_ty,
            args[0].span,
            CollectionKind::List,
        );
        Type::List(Box::new(self.resolve(element_ty)))
    }

    fn infer_find_overloaded(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if matches!(args.get(1).map(|arg| &arg.kind), Some(ExprKind::String(_))) {
            return self.infer_query_helper(name, args, env, false);
        }
        self.infer_find(name, args, env)
    }

    fn infer_find(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return self.fresh();
        }
        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );
        let predicate_ty = self.infer_expr(&args[1], env);
        self.unify(
            predicate_ty,
            Type::Fn(vec![element_ty.clone()], Box::new(Type::Bool)),
            args[1].span,
        );
        self.resolve(element_ty)
    }

    fn infer_map_transform(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
        indexed: bool,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Vector(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );

        let mapped_ty = self.fresh();
        let mapper_ty = self.infer_expr(&args[1], env);
        let mut fn_args = vec![element_ty];
        if indexed {
            fn_args.push(Type::Number);
        }
        self.unify(
            mapper_ty,
            Type::Fn(fn_args, Box::new(mapped_ty.clone())),
            args[1].span,
        );
        Type::Vector(Box::new(self.resolve(mapped_ty)))
    }

    fn infer_filter(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Vector(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );
        let predicate_ty = self.infer_expr(&args[1], env);
        self.unify(
            predicate_ty,
            Type::Fn(vec![element_ty.clone()], Box::new(Type::Bool)),
            args[1].span,
        );
        Type::Vector(Box::new(self.resolve(element_ty)))
    }

    fn infer_vector_predicate_call(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Bool;
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );
        let predicate_ty = self.infer_expr(&args[1], env);
        self.unify(
            predicate_ty,
            Type::Fn(vec![element_ty], Box::new(Type::Bool)),
            args[1].span,
        );
        Type::Bool
    }

    fn infer_range(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if !(1..=3).contains(&args.len()) {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!(
                    "{} expects between 1 and 3 arguments, found {}",
                    name,
                    args.len()
                ),
            ));
            return Type::Vector(Box::new(Type::Number));
        }

        for arg in args {
            let ty = self.infer_expr(arg, env);
            self.unify(ty, Type::Number, arg.span);
        }
        Type::Vector(Box::new(Type::Number))
    }

    fn infer_conj(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() < 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Vector(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        let (collection_kind, element_ty) =
            self.unify_conj_collection(collection_ty, element_ty, args[0].span);
        for item in &args[1..] {
            let item_ty = self.infer_expr(item, env);
            self.unify(item_ty, element_ty.clone(), item.span);
        }

        match collection_kind {
            CollectionKind::Set => Type::Set(Box::new(self.resolve(element_ty))),
            CollectionKind::List => Type::List(Box::new(self.resolve(element_ty))),
            CollectionKind::Vector => Type::Vector(Box::new(self.resolve(element_ty))),
        }
    }

    fn unify_conj_collection(
        &mut self,
        collection_ty: Type,
        fallback_element_ty: Type,
        span: Span,
    ) -> (CollectionKind, Type) {
        match self.shallow_resolve(collection_ty) {
            Type::Set(element_ty) => (CollectionKind::Set, *element_ty),
            Type::List(element_ty) => (CollectionKind::List, *element_ty),
            Type::Vector(element_ty) => (CollectionKind::Vector, *element_ty),
            Type::Var(id) => {
                self.bind(
                    id,
                    Type::Vector(Box::new(fallback_element_ty.clone())),
                    span,
                );
                (CollectionKind::Vector, fallback_element_ty)
            }
            other => {
                self.unify(
                    other,
                    Type::Vector(Box::new(fallback_element_ty.clone())),
                    span,
                );
                (CollectionKind::Vector, fallback_element_ty)
            }
        }
    }

    fn unify_ordered_collection(
        &mut self,
        collection_ty: Type,
        fallback_element_ty: Type,
        span: Span,
        fallback_kind: CollectionKind,
    ) -> Type {
        match self.shallow_resolve(collection_ty) {
            Type::List(element_ty) | Type::Vector(element_ty) => *element_ty,
            Type::Var(id) => {
                self.bind(
                    id,
                    ordered_collection_type(fallback_kind, fallback_element_ty.clone()),
                    span,
                );
                fallback_element_ty
            }
            other => {
                self.unify(
                    other,
                    ordered_collection_type(fallback_kind, fallback_element_ty.clone()),
                    span,
                );
                fallback_element_ty
            }
        }
    }

    fn infer_disj(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() < 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Set(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        let element_ty = self.unify_set_collection(collection_ty, element_ty, args[0].span);
        for item in &args[1..] {
            let item_ty = self.infer_expr(item, env);
            self.unify(item_ty, element_ty.clone(), item.span);
        }

        Type::Set(Box::new(self.resolve(element_ty)))
    }

    fn infer_set_values(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 1 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                1,
                args.len(),
            );
            return Type::Vector(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        let element_ty = self.unify_set_collection(collection_ty, element_ty, args[0].span);
        Type::Vector(Box::new(self.resolve(element_ty)))
    }

    fn unify_set_collection(
        &mut self,
        collection_ty: Type,
        fallback_element_ty: Type,
        span: Span,
    ) -> Type {
        match self.shallow_resolve(collection_ty) {
            Type::Set(element_ty) => *element_ty,
            Type::Var(id) => {
                self.bind(id, Type::Set(Box::new(fallback_element_ty.clone())), span);
                fallback_element_ty
            }
            other => {
                self.unify(
                    other,
                    Type::Set(Box::new(fallback_element_ty.clone())),
                    span,
                );
                fallback_element_ty
            }
        }
    }

    fn infer_sort_by(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Vector(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );
        let key_ty = self.fresh();
        let key_fn_ty = self.infer_expr(&args[1], env);
        self.unify(
            key_fn_ty,
            Type::Fn(vec![element_ty.clone()], Box::new(key_ty)),
            args[1].span,
        );
        Type::Vector(Box::new(self.resolve(element_ty)))
    }

    fn infer_sort_with(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Vector(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );
        let comparator_ty = self.infer_expr(&args[1], env);
        self.unify(
            comparator_ty,
            Type::Fn(
                vec![element_ty.clone(), element_ty.clone()],
                Box::new(Type::Number),
            ),
            args[1].span,
        );
        Type::Vector(Box::new(self.resolve(element_ty)))
    }

    fn infer_slice(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if !(2..=3).contains(&args.len()) {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Vector(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );
        for arg in &args[1..] {
            let index_ty = self.infer_expr(arg, env);
            self.unify(index_ty, Type::Number, arg.span);
        }
        Type::Vector(Box::new(self.resolve(element_ty)))
    }

    fn infer_drop_last(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if !(1..=2).contains(&args.len()) {
            self.diagnostics.push(Diagnostic::error(
                args.first().map_or(Span::default(), |arg| arg.span),
                format!("{} expects a vector and optional count", name),
            ));
            return Type::Vector(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );
        if let Some(count) = args.get(1) {
            let count_ty = self.infer_expr(count, env);
            self.unify(count_ty, Type::Number, count.span);
        }
        Type::Vector(Box::new(self.resolve(element_ty)))
    }

    fn infer_take_last(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 2 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                2,
                args.len(),
            );
            return Type::Vector(Box::new(self.fresh()));
        }

        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );
        let count_ty = self.infer_expr(&args[1], env);
        self.unify(count_ty, Type::Number, args[1].span);
        Type::Vector(Box::new(self.resolve(element_ty)))
    }

    fn infer_reduce(&mut self, name: &str, args: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return self.fresh();
        }
        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );
        let acc_ty = self.infer_expr(&args[1], env);
        self.infer_fn_expr_with_expected(
            &args[2],
            env,
            vec![acc_ty.clone(), element_ty],
            acc_ty.clone(),
        );
        self.resolve(acc_ty)
    }

    fn infer_reduce_indexed(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut HashMap<String, Type>,
    ) -> Type {
        if args.len() != 3 {
            self.arity_error(
                args.first().map_or(Span::default(), |arg| arg.span),
                name,
                3,
                args.len(),
            );
            return self.fresh();
        }
        let element_ty = self.fresh();
        let collection_ty = self.infer_expr(&args[0], env);
        self.unify(
            collection_ty,
            Type::Vector(Box::new(element_ty.clone())),
            args[0].span,
        );
        let acc_ty = self.infer_expr(&args[1], env);
        self.infer_fn_expr_with_expected(
            &args[2],
            env,
            vec![acc_ty.clone(), element_ty, Type::Number],
            acc_ty.clone(),
        );
        self.resolve(acc_ty)
    }

    fn infer_collection(
        &mut self,
        items: &[Expr],
        env: &mut HashMap<String, Type>,
        kind: CollectionKind,
    ) -> Type {
        let element = self.fresh();
        for item in items {
            let ty = self.infer_expr(item, env);
            self.unify(ty, element.clone(), item.span);
        }
        let element = Box::new(self.resolve(element));
        match kind {
            CollectionKind::List => Type::List(element),
            CollectionKind::Vector => Type::Vector(element),
            CollectionKind::Set => Type::Set(element),
        }
    }

    fn infer_vector(&mut self, items: &[Expr], env: &mut HashMap<String, Type>) -> Type {
        if items.is_empty() {
            return Type::Vector(Box::new(self.fresh()));
        }

        let item_types = items
            .iter()
            .map(|item| self.infer_expr(item, env))
            .collect::<Vec<_>>();

        if item_types
            .windows(2)
            .all(|pair| could_be_homogeneous(&pair[0], &pair[1]))
        {
            let element = item_types[0].clone();
            for (ty, item) in item_types.iter().skip(1).zip(items.iter().skip(1)) {
                self.unify(ty.clone(), element.clone(), item.span);
            }
            return Type::Vector(Box::new(self.resolve(element)));
        }

        Type::Tuple(
            item_types
                .into_iter()
                .map(|ty| self.resolve(ty))
                .collect::<Vec<_>>(),
        )
    }

    fn infer_map(&mut self, entries: &[(Expr, Expr)], env: &mut HashMap<String, Type>) -> Type {
        if let Some(record) = self.infer_record(entries, env) {
            return record;
        }

        let key_ty = self.fresh();
        let value_ty = self.fresh();
        for (key, value) in entries {
            let inferred_key = self.infer_expr(key, env);
            self.unify(inferred_key, key_ty.clone(), key.span);
            let inferred_value = self.infer_expr(value, env);
            self.unify(inferred_value, value_ty.clone(), value.span);
        }
        Type::Map(
            Box::new(self.resolve(key_ty)),
            Box::new(self.resolve(value_ty)),
        )
    }

    fn infer_record(
        &mut self,
        entries: &[(Expr, Expr)],
        env: &mut HashMap<String, Type>,
    ) -> Option<Type> {
        if entries.is_empty() {
            return Some(Type::Record(BTreeMap::new()));
        }

        let mut fields = BTreeMap::new();
        for (key, value) in entries {
            let Some(field) = record_key_name(key) else {
                return None;
            };
            let value_ty = self.infer_expr(value, env);
            fields.insert(field, value_ty);
        }
        Some(Type::Record(fields))
    }

    fn unify(&mut self, left: Type, right: Type, span: Span) -> Type {
        if let Type::Var(id) = &left {
            if let Some(bound) = self.subst.get(id).cloned() {
                let unified = self.unify(bound, right, span);
                self.subst.insert(*id, unified.clone());
                return unified;
            }
        }

        if let Type::Var(id) = &right {
            if let Some(bound) = self.subst.get(id).cloned() {
                let unified = self.unify(left, bound, span);
                self.subst.insert(*id, unified.clone());
                return unified;
            }
        }

        let left = self.shallow_resolve(left);
        let right = self.shallow_resolve(right);
        if left == right {
            return left;
        }

        match (left, right) {
            (Type::Var(id), ty) | (ty, Type::Var(id)) => self.bind(id, ty, span),
            (Type::Number, Type::Number) => Type::Number,
            (Type::String, Type::String) => Type::String,
            (Type::Bool, Type::Bool) => Type::Bool,
            (Type::Nil, Type::Nil) => Type::Nil,
            (Type::Keyword(left), Type::Keyword(right)) => {
                let literal = if left == right { left } else { None };
                Type::Keyword(literal)
            }
            (Type::Syntax, Type::Syntax) => Type::Syntax,
            (Type::Js, Type::Js) => Type::Js,
            (Type::Js, ty) | (ty, Type::Js) => ty,
            (Type::Html, Type::Html) => Type::Html,
            (Type::TrustedHtml, Type::TrustedHtml) => Type::TrustedHtml,
            (Type::Nil, Type::Option(inner)) | (Type::Option(inner), Type::Nil) => {
                Type::Option(inner)
            }
            (Type::Nil, ty) | (ty, Type::Nil) => Type::Option(Box::new(ty)),
            (Type::Option(a), Type::Option(b)) => {
                let inner = self.unify(*a, *b, span);
                Type::Option(Box::new(inner))
            }
            (Type::Decoder(a), Type::Decoder(b)) => {
                let inner = self.unify(*a, *b, span);
                Type::Decoder(Box::new(inner))
            }
            (Type::Option(inner), ty) | (ty, Type::Option(inner)) => {
                let inner = self.unify(*inner, ty, span);
                Type::Option(Box::new(inner))
            }
            (Type::List(a), Type::List(b)) => {
                let element = self.unify(*a, *b, span);
                Type::List(Box::new(element))
            }
            (Type::Vector(a), Type::Vector(b)) => {
                let element = self.unify(*a, *b, span);
                Type::Vector(Box::new(element))
            }
            (Type::Tuple(items), Type::Vector(element))
            | (Type::Vector(element), Type::Tuple(items)) => {
                let mut element = *element;
                for item in items {
                    element = self.unify(item, element, span);
                }
                Type::Vector(Box::new(self.resolve(element)))
            }
            (Type::Tuple(a), Type::Tuple(b)) => {
                if a.len() != b.len() {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!(
                            "tuple length mismatch: expected {} items, found {}",
                            a.len(),
                            b.len()
                        ),
                    ));
                    return Type::Tuple(a);
                }
                Type::Tuple(
                    a.into_iter()
                        .zip(b)
                        .map(|(a, b)| self.unify(a, b, span))
                        .collect(),
                )
            }
            (Type::Set(a), Type::Set(b)) => {
                let element = self.unify(*a, *b, span);
                Type::Set(Box::new(element))
            }
            (Type::Map(ak, av), Type::Map(bk, bv)) => {
                let key = self.unify(*ak, *bk, span);
                let value = self.unify(*av, *bv, span);
                Type::Map(Box::new(key), Box::new(value))
            }
            (Type::Result(a_ok, a_err), Type::Result(b_ok, b_err)) => {
                let ok = self.unify(*a_ok, *b_ok, span);
                let err = self.unify(*a_err, *b_err, span);
                Type::Result(Box::new(ok), Box::new(err))
            }
            (Type::Cmd(expected), Type::Cmd(actual)) => {
                self.require_command_message_matches(
                    (*expected).clone(),
                    (*actual).clone(),
                    span,
                    ":commands item",
                );
                Type::Cmd(Box::new(self.resolve(*expected)))
            }
            (Type::Cmd(msg), Type::Record(fields)) | (Type::Record(fields), Type::Cmd(msg)) => {
                self.unify_cmd_record(*msg, fields, span)
            }
            (Type::Cmd(msg), Type::Union(variants)) | (Type::Union(variants), Type::Cmd(msg)) => {
                self.unify_cmd_union(*msg, variants, span)
            }
            (Type::Task(a_err, a_ok), Type::Task(b_err, b_ok)) => {
                let err = self.unify(*a_err, *b_err, span);
                let ok = self.unify(*a_ok, *b_ok, span);
                Type::Task(Box::new(err), Box::new(ok))
            }
            (Type::Sub(a), Type::Sub(b)) => {
                self.require_subscription_message_matches(
                    (*a).clone(),
                    (*b).clone(),
                    span,
                    ":subscriptions item",
                );
                Type::Sub(Box::new(self.resolve(*a)))
            }
            (Type::Sub(msg), Type::Record(fields)) | (Type::Record(fields), Type::Sub(msg)) => {
                self.unify_sub_record(*msg, fields, span)
            }
            (Type::Sub(msg), Type::Union(variants)) | (Type::Union(variants), Type::Sub(msg)) => {
                self.unify_sub_union(*msg, variants, span)
            }
            (Type::Event(a), Type::Event(b)) => {
                let msg = self.unify(*a, *b, span);
                Type::Event(Box::new(msg))
            }
            (Type::Union(a), Type::Union(b)) => {
                if a.len() != b.len() {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!(
                            "union variant mismatch: expected {} variants, found {}",
                            a.len(),
                            b.len()
                        ),
                    ));
                    return Type::Union(a);
                }
                Type::Union(
                    a.into_iter()
                        .zip(b)
                        .map(|(a, b)| self.unify(a, b, span))
                        .collect(),
                )
            }
            (Type::Union(variants), Type::Record(fields))
            | (Type::Record(fields), Type::Union(variants)) => {
                self.unify_union_record(variants, fields, span)
            }
            (Type::Record(a), Type::Record(b)) => {
                let mut fields = a;
                for (name, b_ty) in b {
                    match fields.remove(&name) {
                        Some(a_ty) => {
                            fields.insert(name, self.unify(a_ty, b_ty, span));
                        }
                        None => {
                            fields.insert(name, b_ty);
                        }
                    }
                }
                Type::Record(fields)
            }
            (Type::Fn(a_args, a_ret), Type::Fn(b_args, b_ret)) => {
                if a_args.len() != b_args.len() {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!(
                            "function arity mismatch: expected {} args, found {}",
                            a_args.len(),
                            b_args.len()
                        ),
                    ));
                    return Type::Fn(a_args, a_ret);
                }
                let args = a_args
                    .into_iter()
                    .zip(b_args)
                    .map(|(a, b)| self.unify(a, b, span))
                    .collect();
                let ret = self.unify(*a_ret, *b_ret, span);
                Type::Fn(args, Box::new(ret))
            }
            (left, right) => {
                let left_name = self.format_type(&left);
                let right_name = self.format_type(&right);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!(
                        "type mismatch: expected {}, found {}",
                        left_name, right_name
                    ),
                ));
                left
            }
        }
    }

    fn join_types(&mut self, left: Type, right: Type, span: Span) -> Type {
        let left = self.resolve(left);
        let right = self.resolve(right);
        if left == right {
            return left;
        }

        match (left, right) {
            (Type::Tuple(left), Type::Tuple(right)) if left.len() == right.len() => Type::Tuple(
                left.into_iter()
                    .zip(right)
                    .map(|(left, right)| self.join_types(left, right, span))
                    .collect(),
            ),
            (Type::Decoder(left), Type::Decoder(right)) => {
                Type::Decoder(Box::new(self.join_types(*left, *right, span)))
            }
            (Type::Cmd(left), Type::Cmd(right)) => {
                Type::Cmd(Box::new(self.join_types(*left, *right, span)))
            }
            (Type::Cmd(left), Type::Record(right)) if is_batch_command_record_fields(&right) => {
                let right_msg = self.infer_command_record_message_type(right, span);
                Type::Cmd(Box::new(self.join_types(*left, right_msg, span)))
            }
            (Type::Cmd(left), Type::Record(right)) if is_command_record_fields(&right) => {
                let right_msg = self.infer_command_record_message_type(right, span);
                Type::Cmd(Box::new(self.join_types(*left, right_msg, span)))
            }
            (Type::Record(left), Type::Cmd(right)) if is_batch_command_record_fields(&left) => {
                let left_msg = self.infer_command_record_message_type(left, span);
                Type::Cmd(Box::new(self.join_types(left_msg, *right, span)))
            }
            (Type::Record(left), Type::Cmd(right)) if is_command_record_fields(&left) => {
                let left_msg = self.infer_command_record_message_type(left, span);
                Type::Cmd(Box::new(self.join_types(left_msg, *right, span)))
            }
            (Type::Task(left_err, left_ok), Type::Task(right_err, right_ok)) => Type::Task(
                Box::new(self.join_types(*left_err, *right_err, span)),
                Box::new(self.join_types(*left_ok, *right_ok, span)),
            ),
            (Type::Sub(left), Type::Sub(right)) => {
                Type::Sub(Box::new(self.join_types(*left, *right, span)))
            }
            (Type::Event(left), Type::Event(right)) => {
                Type::Event(Box::new(self.join_types(*left, *right, span)))
            }
            (Type::Event(left), right) | (right, Type::Event(left)) => {
                Type::Event(Box::new(self.join_types(*left, right, span)))
            }
            (Type::Union(left), Type::Union(right)) => self.join_union_variants(left, right, span),
            (Type::Union(variants), ty) | (ty, Type::Union(variants)) => {
                self.join_union_with_type(variants, ty, span)
            }
            (Type::Record(left), Type::Record(right))
                if is_batch_command_record_fields(&left)
                    && is_batch_command_record_fields(&right) =>
            {
                let left_msg = self.infer_command_record_message_type(left, span);
                let right_msg = self.infer_command_record_message_type(right, span);
                Type::Cmd(Box::new(self.join_types(left_msg, right_msg, span)))
            }
            (Type::Record(left), Type::Record(right))
                if is_batch_command_record_fields(&left) && is_command_record_fields(&right) =>
            {
                let left_msg = self.infer_command_record_message_type(left, span);
                let right_msg = self.infer_command_record_message_type(right, span);
                Type::Cmd(Box::new(self.join_types(left_msg, right_msg, span)))
            }
            (Type::Record(left), Type::Record(right))
                if is_command_record_fields(&left) && is_batch_command_record_fields(&right) =>
            {
                let left_msg = self.infer_command_record_message_type(left, span);
                let right_msg = self.infer_command_record_message_type(right, span);
                Type::Cmd(Box::new(self.join_types(left_msg, right_msg, span)))
            }
            (Type::Record(left), Type::Record(right))
                if records_have_distinct_tags(&left, &right) =>
            {
                Type::Union(vec![Type::Record(left), Type::Record(right)])
            }
            (left, right) => self.unify(left, right, span),
        }
    }

    fn join_union_variants(&mut self, left: Vec<Type>, right: Vec<Type>, span: Span) -> Type {
        let mut variants = left;
        for variant in right {
            variants = match self.join_union_with_type(variants, variant, span) {
                Type::Union(joined) => joined,
                joined => vec![joined],
            };
        }
        Type::Union(variants)
    }

    fn join_union_with_type(&mut self, mut variants: Vec<Type>, ty: Type, span: Span) -> Type {
        let ty = self.resolve(ty);
        if let Some(tag) = tagged_record_literal(&ty).map(str::to_string) {
            for variant in &mut variants {
                if tagged_record_literal(variant).is_some_and(|variant_tag| variant_tag == tag) {
                    *variant = self.join_types(variant.clone(), ty, span);
                    return Type::Union(variants);
                }
            }
            variants.push(ty);
            return Type::Union(variants);
        }

        if variants
            .iter()
            .any(|variant| self.union_field_matches(variant.clone(), ty.clone(), span))
        {
            return Type::Union(variants);
        }

        variants.push(ty);
        Type::Union(variants)
    }

    fn unify_union_record(
        &mut self,
        variants: Vec<Type>,
        fields: BTreeMap<String, Type>,
        span: Span,
    ) -> Type {
        if self.union_record_matches(&variants, &fields, span) {
            Type::Union(variants)
        } else {
            let union_name = self.format_type(&Type::Union(variants));
            let record_name = self.format_type(&Type::Record(fields));
            self.diagnostics.push(Diagnostic::error(
                span,
                format!(
                    "type mismatch: expected {}, found {}",
                    union_name, record_name
                ),
            ));
            Type::Union(Vec::new())
        }
    }

    fn union_record_matches(
        &mut self,
        variants: &[Type],
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) -> bool {
        let tag = record_fields_kind_literal(fields);
        variants
            .iter()
            .any(|variant| match self.resolve(variant.clone()) {
                Type::Record(variant_fields) => fields.iter().all(|(field, field_ty)| {
                    if tag
                        .is_some_and(|tag| record_fields_kind_literal(&variant_fields) != Some(tag))
                    {
                        return false;
                    }
                    let Some(variant_field_ty) = variant_fields.get(field).cloned() else {
                        return false;
                    };
                    self.union_field_matches(variant_field_ty, field_ty.clone(), span)
                }),
                _ => false,
            })
    }

    fn union_field_matches(&mut self, expected: Type, actual: Type, span: Span) -> bool {
        let expected = self.resolve(expected);
        let actual = self.resolve(actual);
        match (expected, actual) {
            (Type::Var(_), _) | (_, Type::Var(_)) => true,
            (Type::Number, Type::Number)
            | (Type::String, Type::String)
            | (Type::Bool, Type::Bool)
            | (Type::Nil, Type::Nil)
            | (Type::Syntax, Type::Syntax)
            | (Type::Js, Type::Js)
            | (Type::Html, Type::Html)
            | (Type::TrustedHtml, Type::TrustedHtml) => true,
            (Type::Keyword(expected), Type::Keyword(actual)) => {
                keyword_type_accepts(&expected, &actual)
            }
            (Type::Option(left), Type::Option(right))
            | (Type::Decoder(left), Type::Decoder(right))
            | (Type::List(left), Type::List(right))
            | (Type::Vector(left), Type::Vector(right))
            | (Type::Set(left), Type::Set(right))
            | (Type::Cmd(left), Type::Cmd(right))
            | (Type::Sub(left), Type::Sub(right))
            | (Type::Event(left), Type::Event(right)) => {
                self.union_field_matches(*left, *right, span)
            }
            (Type::Map(left_key, left_value), Type::Map(right_key, right_value))
            | (Type::Result(left_key, left_value), Type::Result(right_key, right_value)) => {
                self.union_field_matches(*left_key, *right_key, span)
                    && self.union_field_matches(*left_value, *right_value, span)
            }
            (Type::Task(left_err, left_ok), Type::Task(right_err, right_ok)) => {
                self.union_field_matches(*left_err, *right_err, span)
                    && self.union_field_matches(*left_ok, *right_ok, span)
            }
            (Type::Tuple(left), Type::Tuple(right)) => {
                left.len() == right.len()
                    && left
                        .into_iter()
                        .zip(right)
                        .all(|(left, right)| self.union_field_matches(left, right, span))
            }
            (Type::Record(left), Type::Record(right)) => right.into_iter().all(|(field, right)| {
                left.get(&field)
                    .cloned()
                    .is_some_and(|left| self.union_field_matches(left, right, span))
            }),
            (Type::Union(variants), record @ Type::Record(_))
            | (record @ Type::Record(_), Type::Union(variants)) => {
                if let Type::Record(fields) = record {
                    self.union_record_matches(&variants, &fields, span)
                } else {
                    false
                }
            }
            (Type::Union(left), Type::Union(right)) => {
                left.len() == right.len()
                    && left
                        .into_iter()
                        .zip(right)
                        .all(|(left, right)| self.union_field_matches(left, right, span))
            }
            (Type::Fn(left_args, left_ret), Type::Fn(right_args, right_ret)) => {
                left_args.len() == right_args.len()
                    && left_args
                        .into_iter()
                        .zip(right_args)
                        .all(|(left, right)| self.union_field_matches(left, right, span))
                    && self.union_field_matches(*left_ret, *right_ret, span)
            }
            _ => false,
        }
    }

    fn unify_cmd_record(
        &mut self,
        msg: Type,
        mut fields: BTreeMap<String, Type>,
        span: Span,
    ) -> Type {
        let msg = self.resolve(msg);
        let mut command_kind = None;
        match fields.remove("kind") {
            Some(kind_ty) => {
                let kind_ty = self.resolve(kind_ty);
                self.unify(kind_ty.clone(), Type::Keyword(None), span);
                command_kind = keyword_literal_name(&kind_ty).map(str::to_string);
                if let Some(kind) = command_kind.as_deref() {
                    if !is_known_command_kind(kind) {
                        self.diagnostics.push(Diagnostic::error(
                            span,
                            format!("unknown command kind :{}", kind),
                        ));
                    } else {
                        self.validate_command_schema(kind, &fields, span);
                    }
                }
                if command_kind.as_deref() == Some("batch") {
                    self.validate_batch_commands(&msg, &fields, span);
                }
            }
            None => self.diagnostics.push(Diagnostic::error(
                span,
                "command value must include :kind to satisfy Cmd annotation",
            )),
        }
        self.validate_command_messages(command_kind.as_deref(), &msg, &fields, span);
        Type::Cmd(Box::new(msg))
    }

    fn unify_cmd_union(&mut self, msg: Type, variants: Vec<Type>, span: Span) -> Type {
        let msg = self.resolve(msg);
        for variant in variants {
            match self.resolve(variant) {
                Type::Record(fields) => {
                    self.unify_cmd_record(msg.clone(), fields, span);
                }
                Type::Union(variants) => {
                    self.unify_cmd_union(msg.clone(), variants, span);
                }
                Type::Var(_) => {}
                other => {
                    let found = self.format_type(&other);
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!(
                            "command union variant must be a command record, found {}",
                            found
                        ),
                    ));
                }
            }
        }
        Type::Cmd(Box::new(msg))
    }

    fn unify_sub_record(
        &mut self,
        msg: Type,
        mut fields: BTreeMap<String, Type>,
        span: Span,
    ) -> Type {
        let msg = self.resolve(msg);
        let mut subscription_kind = None;
        let mut known_subscription = false;
        match fields.remove("kind") {
            Some(kind_ty) => {
                let kind_ty = self.resolve(kind_ty);
                self.unify(kind_ty.clone(), Type::Keyword(None), span);
                subscription_kind = keyword_literal_name(&kind_ty).map(str::to_string);
                if let Some(kind) = subscription_kind.as_deref() {
                    if !is_known_subscription_kind(kind) {
                        self.diagnostics.push(Diagnostic::error(
                            span,
                            format!("unknown subscription kind :{}", kind),
                        ));
                    } else {
                        known_subscription = true;
                        self.validate_subscription_schema(kind, &fields, span);
                    }
                }
                if subscription_kind.as_deref() == Some("batch") {
                    self.validate_batch_subscriptions(&msg, &fields, span);
                }
            }
            None => self.diagnostics.push(Diagnostic::error(
                span,
                "subscription value must include :kind to satisfy Sub annotation",
            )),
        }
        if known_subscription {
            self.validate_subscription_messages(subscription_kind.as_deref(), &msg, &fields, span);
        }
        Type::Sub(Box::new(msg))
    }

    fn unify_sub_union(&mut self, msg: Type, variants: Vec<Type>, span: Span) -> Type {
        let msg = self.resolve(msg);
        for variant in variants {
            match self.resolve(variant) {
                Type::Record(fields) => {
                    self.unify_sub_record(msg.clone(), fields, span);
                }
                Type::Union(variants) => {
                    self.unify_sub_union(msg.clone(), variants, span);
                }
                Type::Var(_) => {}
                other => {
                    let found = self.format_type(&other);
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!(
                            "subscription union variant must be a subscription record, found {}",
                            found
                        ),
                    ));
                }
            }
        }
        Type::Sub(Box::new(msg))
    }

    fn validate_command_schema(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        if command_kind == "task/perform" {
            self.require_command_fields(
                command_kind,
                fields,
                &["task", "onSuccess", "onError"],
                span,
            );
        } else if matches!(command_kind, "none" | "batch") {
            self.reject_structural_command_continuations(command_kind, fields, span);
        } else if matches!(command_kind, "dom-ref/resize-watch" | "media-query/watch") {
            self.reject_change_command_success_continuations(command_kind, fields, span);
            self.reject_unsupported_continuation_fields(command_kind, fields, span);
        } else {
            self.reject_conflicting_success_command_fields(command_kind, fields, span);
            self.reject_payloadless_success_continuations(command_kind, fields, span);
            self.reject_unsupported_continuation_fields(command_kind, fields, span);
        }

        match command_kind {
            "batch" => {
                if !fields.contains_key("commands") {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        "batch command is missing a :commands vector",
                    ));
                }
            }
            "bluetooth/request-device" => {
                self.require_success_command_field(command_kind, fields, span);
                self.require_one_command_field(
                    command_kind,
                    fields,
                    &["options", "filters", "acceptAllDevices"],
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "options",
                    Type::Record(BTreeMap::new()),
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "filters",
                    Type::Vector(Box::new(bluetooth_filter_type())),
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "optionalServices",
                    Type::Vector(Box::new(Type::String)),
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "acceptAllDevices",
                    Type::Bool,
                    span,
                );
            }
            "bluetooth/connect-heart-rate" => {
                self.require_command_fields(command_kind, fields, &["id", "onReading"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_success_command_field(command_kind, fields, span);
                self.require_one_command_field(
                    command_kind,
                    fields,
                    &["options", "filters", "acceptAllDevices"],
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "options",
                    Type::Record(BTreeMap::new()),
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "filters",
                    Type::Vector(Box::new(bluetooth_filter_type())),
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "optionalServices",
                    Type::Vector(Box::new(Type::String)),
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "acceptAllDevices",
                    Type::Bool,
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "service",
                    Type::String,
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "characteristic",
                    Type::String,
                    span,
                );
            }
            "bluetooth/disconnect" => {
                self.require_command_fields(command_kind, fields, &["id"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
            }
            "timer/after" => {
                self.require_command_fields(command_kind, fields, &["ms", "msg"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_command_field_type(command_kind, fields, "ms", Type::Number, span);
            }
            "timer/every" => {
                self.require_command_fields(command_kind, fields, &["ms", "msg", "id"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_command_field_type(command_kind, fields, "ms", Type::Number, span);
            }
            "timer/cancel" => {
                self.require_command_fields(command_kind, fields, &["id"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
            }
            "animation/frame" => {
                self.require_command_fields(command_kind, fields, &["onFrame"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
            }
            "animation/cancel" => {
                self.require_command_fields(command_kind, fields, &["id"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
            }
            "time/now" => {
                self.require_success_command_field(command_kind, fields, span);
            }
            "storage/get" => {
                self.require_command_fields(command_kind, fields, &["key"], span);
                self.require_command_field_type(command_kind, fields, "key", Type::String, span);
                self.require_command_field_keyword_or_string(command_kind, fields, "format", span);
                self.require_command_field_keyword_or_string(command_kind, fields, "parse", span);
                self.require_success_command_field(command_kind, fields, span);
            }
            "storage/set" => {
                self.require_command_fields(command_kind, fields, &["key", "value"], span);
                self.require_command_field_type(command_kind, fields, "key", Type::String, span);
            }
            "storage/remove" => {
                self.require_command_fields(command_kind, fields, &["key"], span);
                self.require_command_field_type(command_kind, fields, "key", Type::String, span);
            }
            "browser/history-replace-search-param" => {
                self.require_command_fields(command_kind, fields, &["name", "value"], span);
                self.require_command_field_type(command_kind, fields, "name", Type::String, span);
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "value",
                    Type::Option(Box::new(Type::String)),
                    span,
                );
            }
            "browser/history-write-route" => {
                self.require_command_fields(
                    command_kind,
                    fields,
                    &["url", "op", "definition"],
                    span,
                );
                self.require_command_field_type(command_kind, fields, "url", Type::String, span);
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "op",
                    Type::Option(Box::new(Type::String)),
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "definition",
                    Type::Option(Box::new(Type::String)),
                    span,
                );
            }
            "browser/theme-load" => {
                self.require_command_fields(command_kind, fields, &["key"], span);
                self.require_command_field_type(command_kind, fields, "key", Type::String, span);
                self.require_success_command_field(command_kind, fields, span);
            }
            "browser/theme-apply" => {
                self.require_command_fields(command_kind, fields, &["theme", "key"], span);
                self.require_command_field_type(command_kind, fields, "theme", Type::String, span);
                self.require_command_field_type(command_kind, fields, "key", Type::String, span);
            }
            "browser/clipboard-write" => {
                self.require_command_fields(command_kind, fields, &["text"], span);
                self.require_command_field_type(command_kind, fields, "text", Type::String, span);
            }
            "browser/set-cookie" => {
                self.require_command_fields(command_kind, fields, &["name", "value"], span);
                self.require_command_field_type(command_kind, fields, "name", Type::String, span);
                self.require_command_field_type(command_kind, fields, "value", Type::String, span);
            }
            "auth-storage/load" => {
                self.require_command_fields(command_kind, fields, &["sourceUrl"], span);
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "sourceUrl",
                    Type::String,
                    span,
                );
                self.require_success_command_field(command_kind, fields, span);
            }
            "auth-storage/persist" => {
                self.require_command_fields(command_kind, fields, &["sourceUrl", "entries"], span);
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "sourceUrl",
                    Type::String,
                    span,
                );
            }
            "random/number" => {
                self.require_success_command_field(command_kind, fields, span);
                self.require_command_field_type(command_kind, fields, "min", Type::Number, span);
                self.require_command_field_type(command_kind, fields, "max", Type::Number, span);
            }
            "simulation/heart-rate" => {
                self.require_command_fields(command_kind, fields, &["id", "onReading"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_command_field_type(command_kind, fields, "ms", Type::Number, span);
                self.require_command_field_type(command_kind, fields, "min", Type::Number, span);
                self.require_command_field_type(command_kind, fields, "max", Type::Number, span);
                self.require_command_field_type(command_kind, fields, "jitter", Type::Number, span);
                self.require_command_field_type(command_kind, fields, "start", Type::Number, span);
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "deviceName",
                    Type::String,
                    span,
                );
                self.require_success_command_field(command_kind, fields, span);
            }
            "simulation/stop" => {
                self.require_command_fields(command_kind, fields, &["id"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
            }
            "file/download" => {
                self.require_command_fields(command_kind, fields, &["name", "content"], span);
                self.require_command_field_type(command_kind, fields, "name", Type::String, span);
                self.require_command_field_type(command_kind, fields, "mime", Type::String, span);
            }
            "file/import" => {
                self.require_success_command_field(command_kind, fields, span);
                self.require_command_field_type(command_kind, fields, "accept", Type::String, span);
                self.require_command_field_keyword_or_string(command_kind, fields, "format", span);
                self.require_command_field_keyword_or_string(command_kind, fields, "parse", span);
                self.require_command_field_type(command_kind, fields, "multiple", Type::Bool, span);
            }
            "file/read-selected" => {
                self.require_command_fields(command_kind, fields, &["ref"], span);
                self.require_command_field_type(command_kind, fields, "ref", Type::String, span);
                self.require_command_field_keyword_or_string(command_kind, fields, "format", span);
                self.require_command_field_keyword_or_string(command_kind, fields, "parse", span);
                self.require_command_field_type(command_kind, fields, "multiple", Type::Bool, span);
                self.require_command_field_type(command_kind, fields, "clear", Type::Bool, span);
                self.require_success_command_field(command_kind, fields, span);
            }
            "canvas/draw" => {
                self.require_command_fields(command_kind, fields, &["ref", "ops"], span);
                self.require_command_field_type(command_kind, fields, "ref", Type::String, span);
                self.require_command_field_type(command_kind, fields, "width", Type::Number, span);
                self.require_command_field_type(command_kind, fields, "height", Type::Number, span);
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "cssWidth",
                    Type::Number,
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "cssHeight",
                    Type::Number,
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "setCssSize",
                    Type::Bool,
                    span,
                );
            }
            "canvas/measure-text" => {
                self.require_command_fields(command_kind, fields, &["ref"], span);
                self.require_command_field_type(command_kind, fields, "ref", Type::String, span);
                self.require_success_command_field(command_kind, fields, span);
                self.require_one_command_field(command_kind, fields, &["text", "texts"], span);
                self.require_command_field_type(command_kind, fields, "text", Type::String, span);
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "texts",
                    Type::Vector(Box::new(Type::String)),
                    span,
                );
                self.require_command_field_type(command_kind, fields, "font", Type::String, span);
            }
            "dom-ref/focus" | "dom-ref/click" => {
                self.require_command_fields(command_kind, fields, &["ref"], span);
                self.require_command_field_type(command_kind, fields, "ref", Type::String, span);
            }
            "dom-ref/measure" => {
                self.require_command_fields(command_kind, fields, &["ref"], span);
                self.require_command_field_type(command_kind, fields, "ref", Type::String, span);
                self.require_success_command_field(command_kind, fields, span);
            }
            "dom/scroll-into-view" => {
                self.require_one_command_field(
                    command_kind,
                    fields,
                    &["selector", "testId", "id"],
                    span,
                );
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "selector",
                    Type::String,
                    span,
                );
                self.require_command_field_type(command_kind, fields, "testId", Type::String, span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "behavior",
                    Type::String,
                    span,
                );
                self.require_command_field_type(command_kind, fields, "block", Type::String, span);
                self.require_command_field_type(command_kind, fields, "inline", Type::String, span);
                self.require_command_field_type(
                    command_kind,
                    fields,
                    "skipIfVisible",
                    Type::Bool,
                    span,
                );
                self.require_command_field_type(command_kind, fields, "smooth", Type::Bool, span);
            }
            "dom-ref/resize-watch" => {
                self.require_command_fields(command_kind, fields, &["ref", "onChange"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_command_field_type(command_kind, fields, "ref", Type::String, span);
            }
            "dom-ref/resize-unwatch" => {
                self.require_one_command_field(command_kind, fields, &["id", "ref"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_command_field_type(command_kind, fields, "ref", Type::String, span);
            }
            "window/event-watch" => {
                self.require_command_fields(command_kind, fields, &["type", "onEvent"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_command_field_type(command_kind, fields, "type", Type::String, span);
            }
            "window/event-unwatch" => {
                self.require_one_command_field(command_kind, fields, &["id", "type"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_command_field_type(command_kind, fields, "type", Type::String, span);
            }
            "media-query/watch" => {
                self.require_command_fields(command_kind, fields, &["query", "onChange"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_command_field_type(command_kind, fields, "query", Type::String, span);
            }
            "media-query/unwatch" => {
                self.require_one_command_field(command_kind, fields, &["id", "query"], span);
                self.require_command_field_type(command_kind, fields, "id", Type::String, span);
                self.require_command_field_type(command_kind, fields, "query", Type::String, span);
            }
            "http/request" => {
                self.require_one_command_field(command_kind, fields, &["request", "url"], span);
                self.require_command_field_type(command_kind, fields, "url", Type::String, span);
                self.require_command_field_type(command_kind, fields, "method", Type::String, span);
                self.require_command_field_keyword_or_string(
                    command_kind,
                    fields,
                    "response",
                    span,
                );
                self.require_command_field_keyword_or_string(command_kind, fields, "format", span);
                self.require_command_record_field_type(
                    command_kind,
                    fields,
                    "request",
                    "url",
                    Type::String,
                    span,
                );
                self.require_command_record_field_type(
                    command_kind,
                    fields,
                    "request",
                    "method",
                    Type::String,
                    span,
                );
                self.require_success_command_field(command_kind, fields, span);
            }
            "none" => {}
            _ => {}
        }
    }

    fn validate_subscription_schema(
        &mut self,
        subscription_kind: &str,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        match subscription_kind {
            "none" => {}
            "batch" => {
                if !fields.contains_key("subscriptions") && !fields.contains_key("subs") {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        "batch subscription is missing a :subscriptions vector",
                    ));
                }
            }
            "sub/timer/every" => {
                self.require_command_fields(subscription_kind, fields, &["id", "ms", "msg"], span);
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "id",
                    Type::String,
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "ms",
                    Type::Number,
                    span,
                );
            }
            "sub/dom-ref/resize" => {
                self.require_command_fields(subscription_kind, fields, &["ref", "onChange"], span);
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "id",
                    Type::String,
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "ref",
                    Type::String,
                    span,
                );
            }
            "sub/window/event" => {
                self.require_command_fields(subscription_kind, fields, &["type", "onEvent"], span);
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "id",
                    Type::String,
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "type",
                    Type::String,
                    span,
                );
            }
            "sub/media-query" => {
                self.require_command_fields(
                    subscription_kind,
                    fields,
                    &["query", "onChange"],
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "id",
                    Type::String,
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "query",
                    Type::String,
                    span,
                );
            }
            "sub/simulation/heart-rate" => {
                self.require_command_fields(subscription_kind, fields, &["id", "onReading"], span);
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "id",
                    Type::String,
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "ms",
                    Type::Number,
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "min",
                    Type::Number,
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "max",
                    Type::Number,
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "jitter",
                    Type::Number,
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "start",
                    Type::Number,
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "deviceName",
                    Type::String,
                    span,
                );
            }
            "sub/bluetooth/connect-heart-rate" => {
                self.require_command_fields(subscription_kind, fields, &["id", "onReading"], span);
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "id",
                    Type::String,
                    span,
                );
                self.require_one_command_field(
                    subscription_kind,
                    fields,
                    &["options", "filters", "acceptAllDevices"],
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "options",
                    Type::Record(BTreeMap::new()),
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "filters",
                    Type::Vector(Box::new(bluetooth_filter_type())),
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "optionalServices",
                    Type::Vector(Box::new(Type::String)),
                    span,
                );
                self.require_command_field_type(
                    subscription_kind,
                    fields,
                    "acceptAllDevices",
                    Type::Bool,
                    span,
                );
            }
            _ => {}
        }
    }

    fn reject_conflicting_success_command_fields(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        let present = ["msg", "onSuccess", "toMessage"]
            .into_iter()
            .filter(|field| fields.contains_key(*field))
            .collect::<Vec<_>>();

        if present.len() <= 1 {
            return;
        }

        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "{} command has conflicting success continuations {}; use only one of :msg, :onSuccess, :toMessage",
                command_kind,
                present
                    .iter()
                    .map(|field| format!(":{}", field))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    fn reject_structural_command_continuations(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        let present = COMMAND_CONTINUATION_FIELDS
            .iter()
            .filter(|field| fields.contains_key(**field))
            .copied()
            .collect::<Vec<_>>();

        if present.is_empty() {
            return;
        }

        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "{} command does not support continuations {}",
                command_kind,
                present
                    .iter()
                    .map(|field| format!(":{}", field))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    fn reject_change_command_success_continuations(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        let present = ["msg", "onSuccess", "toMessage"]
            .into_iter()
            .filter(|field| fields.contains_key(*field))
            .collect::<Vec<_>>();

        if present.is_empty() {
            return;
        }

        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "{} command dispatches changes through :onChange and does not support success continuations {}",
                command_kind,
                present
                    .iter()
                    .map(|field| format!(":{}", field))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    fn reject_payloadless_success_continuations(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        if !matches!(
            command_kind,
            "bluetooth/disconnect" | "timer/after" | "timer/every" | "timer/cancel"
        ) {
            return;
        }

        let present = ["onSuccess", "toMessage"]
            .into_iter()
            .filter(|field| fields.contains_key(*field))
            .collect::<Vec<_>>();

        if present.is_empty() {
            return;
        }

        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "{} command has no success payload; use :msg for completion messages instead of {}",
                command_kind,
                present
                    .iter()
                    .map(|field| format!(":{}", field))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    fn reject_unsupported_continuation_fields(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        for (field, supported) in [
            ("onCancel", &["file/import", "file/read-selected"][..]),
            (
                "onDisconnected",
                &["bluetooth/connect-heart-rate", "simulation/heart-rate"][..],
            ),
            (
                "onReading",
                &["bluetooth/connect-heart-rate", "simulation/heart-rate"][..],
            ),
            ("onFrame", &["animation/frame"][..]),
            (
                "onChange",
                &["dom-ref/resize-watch", "media-query/watch"][..],
            ),
            ("onEvent", &["window/event-watch"][..]),
        ] {
            if fields.contains_key(field) && !supported.contains(&command_kind) {
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!(
                        "{} command does not support :{}; supported on {}",
                        command_kind,
                        field,
                        supported
                            .iter()
                            .map(|kind| format!(":{}", kind))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
        }
    }

    fn require_command_fields(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        required: &[&str],
        span: Span,
    ) {
        for field in required {
            if !fields.contains_key(*field) {
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!("{} command is missing a :{} field", command_kind, field),
                ));
            }
        }
    }

    fn require_command_field_type(
        &mut self,
        _command_kind: &str,
        fields: &BTreeMap<String, Type>,
        field: &str,
        expected: Type,
        span: Span,
    ) {
        if let Some(actual) = fields.get(field).cloned() {
            self.unify(expected, actual, span);
        }
    }

    fn require_command_field_keyword_or_string(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        field: &str,
        span: Span,
    ) {
        let Some(actual) = fields.get(field).cloned() else {
            return;
        };

        match self.resolve(actual) {
            Type::Keyword(_) | Type::String | Type::Var(_) => {}
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!(
                        "{} command :{} must be a keyword or string, found {}",
                        command_kind, field, found
                    ),
                ));
            }
        }
    }

    fn require_command_record_field_type(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        record_field: &str,
        nested_field: &str,
        expected: Type,
        span: Span,
    ) {
        let Some(actual) = fields.get(record_field).cloned() else {
            return;
        };

        match self.resolve(actual) {
            Type::Record(record_fields) => {
                if let Some(nested_ty) = record_fields.get(nested_field).cloned() {
                    self.unify(expected, nested_ty, span);
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!(
                            "{} command :{} is missing a :{} field",
                            command_kind, record_field, nested_field
                        ),
                    ));
                }
            }
            Type::Var(_) => {}
            other => {
                let mut expected_fields = BTreeMap::new();
                expected_fields.insert(nested_field.to_string(), expected);
                self.unify(Type::Record(expected_fields), other, span);
            }
        }
    }

    fn require_one_command_field(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        choices: &[&str],
        span: Span,
    ) {
        if choices.iter().any(|field| fields.contains_key(*field)) {
            return;
        }

        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "{} command is missing one of {}",
                command_kind,
                choices
                    .iter()
                    .map(|field| format!(":{}", field))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    fn require_success_command_field(
        &mut self,
        command_kind: &str,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        if fields.contains_key("onSuccess") || fields.contains_key("toMessage") {
            return;
        }

        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "{} command is missing one of :onSuccess, :toMessage",
                command_kind
            ),
        ));
    }

    fn validate_batch_commands(&mut self, msg: &Type, fields: &BTreeMap<String, Type>, span: Span) {
        let Some(commands_ty) = fields.get("commands").cloned() else {
            return;
        };
        match self.resolve(commands_ty) {
            Type::Vector(command_ty) => {
                self.unify(Type::Cmd(Box::new(msg.clone())), *command_ty, span);
            }
            Type::Tuple(commands) => {
                for command_ty in commands {
                    self.unify(Type::Cmd(Box::new(msg.clone())), command_ty, span);
                }
            }
            Type::Var(_) => {}
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!(
                        "batch :commands must be a vector of command records, found {}",
                        found
                    ),
                ));
            }
        }
    }

    fn validate_batch_subscriptions(
        &mut self,
        msg: &Type,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        let Some(subscriptions_ty) = fields
            .get("subscriptions")
            .or_else(|| fields.get("subs"))
            .cloned()
        else {
            return;
        };
        match self.resolve(subscriptions_ty) {
            Type::Vector(subscription_ty) => {
                self.unify(Type::Sub(Box::new(msg.clone())), *subscription_ty, span);
            }
            Type::Tuple(subscriptions) => {
                for subscription_ty in subscriptions {
                    self.unify(Type::Sub(Box::new(msg.clone())), subscription_ty, span);
                }
            }
            Type::Var(_) => {}
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!(
                        "batch :subscriptions must be a vector of subscription records, found {}",
                        found
                    ),
                ));
            }
        }
    }

    fn validate_command_messages(
        &mut self,
        command_kind: Option<&str>,
        msg: &Type,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        if command_kind == Some("task/perform") {
            self.validate_task_perform_messages(msg, fields, span);
            return;
        }

        if let Some(message_ty) = fields.get("msg").cloned() {
            self.require_command_message_matches(msg.clone(), message_ty, span, ":msg");
        }

        if let Some(to_message_ty) = fields.get("toMessage").cloned() {
            self.require_command_to_message_matches(
                command_kind,
                msg.clone(),
                to_message_ty,
                fields,
                span,
            );
        }

        for field in COMMAND_MESSAGE_TAG_FIELDS {
            if let Some(tag_ty) = fields.get(*field).cloned() {
                self.require_command_tag_matches(
                    command_kind,
                    msg.clone(),
                    tag_ty,
                    span,
                    field,
                    fields,
                );
            }
        }
    }

    fn infer_command_message_type(&mut self, command_ty: Type, span: Span) -> Type {
        match self.resolve(command_ty) {
            Type::Cmd(msg) => self.resolve(*msg),
            Type::Record(fields) if is_command_record_fields(&fields) => {
                self.infer_command_record_message_type(fields, span)
            }
            Type::Union(variants) => {
                let mut joined = None;
                for variant in variants {
                    let msg = self.infer_command_message_type(variant, span);
                    joined = Some(match joined {
                        Some(existing) => self.join_types(existing, msg, span),
                        None => msg,
                    });
                }
                joined.unwrap_or_else(|| self.fresh())
            }
            Type::Vector(command) => self.infer_command_message_type(*command, span),
            Type::Tuple(commands) => {
                let mut joined = None;
                for command in commands {
                    let msg = self.infer_command_message_type(command, span);
                    joined = Some(match joined {
                        Some(existing) => self.join_types(existing, msg, span),
                        None => msg,
                    });
                }
                joined.unwrap_or_else(|| self.fresh())
            }
            Type::Var(_) => self.fresh(),
            _ => self.fresh(),
        }
    }

    fn infer_command_record_message_type(
        &mut self,
        fields: BTreeMap<String, Type>,
        span: Span,
    ) -> Type {
        let command_kind = fields
            .get("kind")
            .cloned()
            .and_then(|kind| keyword_literal_name(&self.resolve(kind)).map(str::to_string));

        if let Some(kind) = command_kind.as_deref() {
            self.validate_command_schema(kind, &fields, span);
        }

        if command_kind.as_deref() == Some("batch") {
            if let Some(commands_ty) = fields.get("commands").cloned() {
                return self.infer_command_message_type(commands_ty, span);
            }
            return self.fresh();
        }

        let mut joined = None;
        if let Some(message_ty) = fields.get("msg").cloned() {
            joined = Some(message_ty);
        }

        if let Some(to_message_ty) = fields.get("toMessage").cloned() {
            if let Type::Fn(args, ret) = self.resolve(to_message_ty) {
                if let Some(value_ty) =
                    self.command_success_value_type(command_kind.as_deref(), &fields)
                {
                    if let Some(arg) = args.first().cloned() {
                        self.unify(arg, value_ty, span);
                    }
                }
                joined = Some(match joined {
                    Some(existing) => self.join_types(existing, *ret, span),
                    None => *ret,
                });
            }
        }

        for field in COMMAND_MESSAGE_TAG_FIELDS {
            let Some(tag_ty) = fields.get(*field).cloned() else {
                continue;
            };
            if let Type::Keyword(Some(tag)) = self.resolve(tag_ty) {
                let message_ty =
                    self.command_message_tag_type(command_kind.as_deref(), field, &tag, &fields);
                joined = Some(match joined {
                    Some(existing) => self.join_types(existing, message_ty, span),
                    None => message_ty,
                });
            }
        }

        joined.unwrap_or_else(|| self.fresh())
    }

    fn validate_task_perform_messages(
        &mut self,
        msg: &Type,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        let err = self.fresh();
        let ok = self.fresh();
        if let Some(task_ty) = fields.get("task").cloned() {
            self.unify(
                task_ty,
                Type::Task(Box::new(err.clone()), Box::new(ok.clone())),
                span,
            );
        }

        if let Some(on_success_ty) = fields.get("onSuccess").cloned() {
            self.require_task_perform_mapper_matches(
                msg.clone(),
                ok,
                on_success_ty,
                span,
                ":onSuccess",
            );
        }

        if let Some(on_error_ty) = fields.get("onError").cloned() {
            self.require_task_perform_mapper_matches(
                msg.clone(),
                err,
                on_error_ty,
                span,
                ":onError",
            );
        }
    }

    fn require_task_perform_mapper_matches(
        &mut self,
        expected_msg: Type,
        payload_ty: Type,
        mapper_ty: Type,
        span: Span,
        label: &str,
    ) {
        match self.resolve(mapper_ty) {
            Type::Var(_) => {}
            Type::Fn(args, ret) => {
                if args.len() != 1 {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!("task/perform {} must accept exactly one argument", label),
                    ));
                    return;
                }
                self.unify(args[0].clone(), payload_ty, span);
                self.require_command_message_matches(
                    expected_msg,
                    *ret,
                    span,
                    &format!("{} return", label),
                );
            }
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!("task/perform {} must be a function, found {}", label, found),
                ));
            }
        }
    }

    fn validate_subscription_messages(
        &mut self,
        subscription_kind: Option<&str>,
        msg: &Type,
        fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        if let Some(message_ty) = fields.get("msg").cloned() {
            self.require_subscription_message_matches(msg.clone(), message_ty, span, ":msg");
        }

        for field in [
            "onError",
            "onChange",
            "onEvent",
            "onReading",
            "onDisconnected",
            "onSuccess",
        ] {
            if let Some(tag_ty) = fields.get(field).cloned() {
                self.require_subscription_tag_matches(
                    subscription_kind,
                    msg.clone(),
                    tag_ty,
                    span,
                    field,
                    fields,
                );
            }
        }
    }

    fn require_subscription_tag_matches(
        &mut self,
        subscription_kind: Option<&str>,
        expected_msg: Type,
        tag_ty: Type,
        span: Span,
        field: &str,
        subscription_fields: &BTreeMap<String, Type>,
    ) {
        let label = match self.resolve(tag_ty.clone()) {
            Type::Keyword(Some(tag)) => format!(":{} {}", field, format_keyword_literal(&tag)),
            _ => format!(":{}", field),
        };
        let message_ty = self.subscription_tag_message_type(subscription_kind, field, tag_ty, span);
        if let Type::Var(_) = self.resolve(message_ty.clone()) {
            return;
        }
        let _ = subscription_fields;
        self.require_subscription_message_matches(expected_msg, message_ty, span, &label);
    }

    fn subscription_tag_message_type(
        &mut self,
        subscription_kind: Option<&str>,
        field: &str,
        tag_ty: Type,
        span: Span,
    ) -> Type {
        match self.resolve(tag_ty) {
            Type::Var(_) | Type::Keyword(None) => self.fresh(),
            Type::Keyword(Some(tag)) => self.command_message_tag_type(
                subscription_command_kind(subscription_kind),
                field,
                &tag,
                &BTreeMap::new(),
            ),
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!(
                        "subscription continuation :{} must be a keyword tag, found {}",
                        field, found
                    ),
                ));
                self.fresh()
            }
        }
    }

    fn require_command_to_message_matches(
        &mut self,
        command_kind: Option<&str>,
        expected_msg: Type,
        to_message_ty: Type,
        command_fields: &BTreeMap<String, Type>,
        span: Span,
    ) {
        match self.resolve(to_message_ty) {
            Type::Var(_) => {}
            Type::Fn(args, ret) => {
                if args.len() != 1 {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        "command :toMessage must accept exactly one payload argument",
                    ));
                    return;
                }
                if let Some(value_ty) =
                    self.command_success_value_type(command_kind, command_fields)
                {
                    self.unify(args[0].clone(), value_ty, span);
                }
                self.require_command_message_matches(expected_msg, *ret, span, ":toMessage return");
            }
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!("command :toMessage must be a function, found {}", found),
                ));
            }
        }
    }

    fn require_command_tag_matches(
        &mut self,
        command_kind: Option<&str>,
        expected_msg: Type,
        tag_ty: Type,
        span: Span,
        field: &str,
        command_fields: &BTreeMap<String, Type>,
    ) {
        match self.resolve(tag_ty) {
            Type::Var(_) | Type::Keyword(None) => {}
            Type::Keyword(Some(tag)) => {
                let message_ty =
                    self.command_message_tag_type(command_kind, field, &tag, command_fields);
                self.require_command_message_matches(
                    expected_msg,
                    message_ty,
                    span,
                    &format!(":{} {}", field, format_keyword_literal(&tag)),
                );
            }
            other => {
                let found = self.format_type(&other);
                self.diagnostics.push(Diagnostic::error(
                    span,
                    format!(
                        "command continuation :{} must be a keyword tag, found {}",
                        field, found
                    ),
                ));
            }
        }
    }

    fn require_command_message_matches(
        &mut self,
        expected_msg: Type,
        actual_msg: Type,
        span: Span,
        label: &str,
    ) {
        if self.command_message_matches(expected_msg.clone(), actual_msg.clone(), span) {
            return;
        }

        let expected = format_type_with_literals(&self.resolve(expected_msg));
        let actual = format_type_with_literals(&self.resolve(actual_msg));
        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "command message {} has type {}, which is not part of Cmd message type {}",
                label, actual, expected
            ),
        ));
    }

    fn require_subscription_message_matches(
        &mut self,
        expected_msg: Type,
        actual_msg: Type,
        span: Span,
        label: &str,
    ) {
        if self.command_message_matches(expected_msg.clone(), actual_msg.clone(), span) {
            return;
        }

        let expected = format_type_with_literals(&self.resolve(expected_msg));
        let actual = format_type_with_literals(&self.resolve(actual_msg));
        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "subscription message {} has type {}, which is not part of Sub message type {}",
                label, actual, expected
            ),
        ));
    }

    fn require_html_event_message_matches(
        &mut self,
        expected_msg: Type,
        actual_msg: Type,
        span: Span,
        event_name: &str,
    ) {
        if self.html_event_message_matches(expected_msg.clone(), actual_msg.clone(), span) {
            return;
        }

        let expected = format_type_with_literals(&self.resolve(expected_msg));
        let actual = format_type_with_literals(&self.resolve(actual_msg));
        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "template event {} message has type {}, which is not part of update message type {}",
                event_name, actual, expected
            ),
        ));
    }

    fn html_event_message_matches(&mut self, expected: Type, actual: Type, span: Span) -> bool {
        match self.resolve(actual) {
            Type::Nil => true,
            Type::Option(inner) => self.html_event_message_matches(expected, *inner, span),
            Type::Event(inner) => self.html_event_message_matches(expected, *inner, span),
            actual => self.command_message_matches(expected, actual, span),
        }
    }

    fn command_message_matches(&mut self, expected: Type, actual: Type, span: Span) -> bool {
        let expected = self.resolve(expected);
        let actual = self.resolve(actual);
        if expected == actual {
            return true;
        }
        match (expected, actual) {
            (Type::Var(_), _) | (_, Type::Var(_)) => true,
            (expected, Type::Union(variants)) => variants
                .into_iter()
                .all(|variant| self.command_message_matches(expected.clone(), variant, span)),
            (Type::Union(variants), actual) => {
                if let Some(tag) = type_kind_literal(&actual) {
                    return variants.into_iter().any(|variant| {
                        tagged_record_literal(&variant) == Some(tag)
                            && self.command_message_matches(variant, actual.clone(), span)
                    });
                }
                variants
                    .into_iter()
                    .any(|variant| self.command_message_matches(variant, actual.clone(), span))
            }
            (Type::Record(expected), Type::Record(actual)) => {
                expected.into_iter().all(|(field, expected_ty)| {
                    actual.get(&field).cloned().is_some_and(|actual_ty| {
                        self.command_message_matches(expected_ty, actual_ty, span)
                    })
                })
            }
            (Type::Keyword(expected), Type::Keyword(actual)) => {
                keyword_type_accepts(&expected, &actual)
            }
            (Type::Js, Type::Js)
            | (Type::TrustedHtml, Type::TrustedHtml)
            | (Type::Html, Type::Html) => true,
            (Type::Option(expected), Type::Option(actual))
            | (Type::Decoder(expected), Type::Decoder(actual))
            | (Type::List(expected), Type::List(actual))
            | (Type::Vector(expected), Type::Vector(actual))
            | (Type::Set(expected), Type::Set(actual))
            | (Type::Cmd(expected), Type::Cmd(actual))
            | (Type::Sub(expected), Type::Sub(actual))
            | (Type::Event(expected), Type::Event(actual)) => {
                self.command_message_matches(*expected, *actual, span)
            }
            (Type::Map(expected_key, expected_value), Type::Map(actual_key, actual_value))
            | (
                Type::Result(expected_key, expected_value),
                Type::Result(actual_key, actual_value),
            ) => {
                self.command_message_matches(*expected_key, *actual_key, span)
                    && self.command_message_matches(*expected_value, *actual_value, span)
            }
            (Type::Task(expected_err, expected_ok), Type::Task(actual_err, actual_ok)) => {
                self.command_message_matches(*expected_err, *actual_err, span)
                    && self.command_message_matches(*expected_ok, *actual_ok, span)
            }
            (Type::Tuple(expected), Type::Tuple(actual)) => {
                expected.len() == actual.len()
                    && expected.into_iter().zip(actual).all(|(expected, actual)| {
                        self.command_message_matches(expected, actual, span)
                    })
            }
            (Type::Fn(expected_args, expected_ret), Type::Fn(actual_args, actual_ret)) => {
                expected_args.len() == actual_args.len()
                    && expected_args
                        .into_iter()
                        .zip(actual_args)
                        .all(|(expected, actual)| {
                            self.command_message_matches(expected, actual, span)
                        })
                    && self.command_message_matches(*expected_ret, *actual_ret, span)
            }
            (expected, actual) => self.union_field_matches(expected, actual, span),
        }
    }

    fn command_message_tag_type(
        &mut self,
        command_kind: Option<&str>,
        field: &str,
        tag: &str,
        command_fields: &BTreeMap<String, Type>,
    ) -> Type {
        let mut fields =
            BTreeMap::from([("kind".to_string(), Type::Keyword(Some(tag.to_string())))]);

        match field {
            "onError" => {
                fields.insert("error".to_string(), Type::String);
            }
            "onReading" => {
                fields.insert("bpm".to_string(), Type::Number);
            }
            "onFrame" => {
                fields.insert("id".to_string(), Type::String);
                fields.insert("timestamp".to_string(), Type::Number);
                fields.insert("value".to_string(), Type::Number);
            }
            "onEvent" => {
                let event = window_event_payload_type();
                fields.extend(record_fields(&event));
                fields.insert("id".to_string(), Type::String);
                fields.insert("value".to_string(), event);
            }
            "onChange" => match command_kind {
                Some("dom-ref/resize-watch") => {
                    let rect = rect_type();
                    fields.extend(record_fields(&rect));
                    fields.insert("id".to_string(), Type::String);
                    fields.insert("ref".to_string(), Type::String);
                    fields.insert("value".to_string(), rect);
                }
                Some("media-query/watch") => {
                    fields.insert("id".to_string(), Type::String);
                    fields.insert("media".to_string(), Type::String);
                    fields.insert("matches".to_string(), Type::Bool);
                }
                _ => {
                    fields.insert("value".to_string(), self.fresh());
                }
            },
            "onSuccess" => {
                if let Some(value_ty) =
                    self.command_success_value_type(command_kind, command_fields)
                {
                    fields.insert("value".to_string(), value_ty);
                }
            }
            "onCancel" | "onDisconnected" => {}
            _ => {}
        }

        Type::Record(fields)
    }

    fn command_success_value_type(
        &mut self,
        command_kind: Option<&str>,
        command_fields: &BTreeMap<String, Type>,
    ) -> Option<Type> {
        match command_kind {
            Some("time/now") | Some("random/number") => Some(Type::Number),
            Some("bluetooth/disconnect")
            | Some("timer/after")
            | Some("timer/every")
            | Some("timer/cancel") => None,
            Some("storage/set") => command_fields
                .get("value")
                .cloned()
                .or_else(|| Some(self.fresh())),
            Some("storage/get") | Some("file/import") | Some("file/read-selected") => {
                Some(command_payload_type_from_command_fields(command_fields))
            }
            Some("bluetooth/request-device") => Some(self.fresh()),
            Some("browser/theme-load") | Some("browser/theme-apply") => Some(Type::String),
            Some("auth-storage/load") => Some(Type::Js),
            Some("storage/remove") => Some(Type::Record(BTreeMap::from([(
                "key".to_string(),
                Type::String,
            )]))),
            Some("http/request") => Some(Type::Record(BTreeMap::from([
                ("status".to_string(), Type::Number),
                ("ok".to_string(), Type::Bool),
                ("body".to_string(), self.fresh()),
            ]))),
            Some("bluetooth/connect-heart-rate") | Some("simulation/heart-rate") => {
                Some(Type::Record(BTreeMap::from([
                    ("id".to_string(), Type::String),
                    ("deviceName".to_string(), Type::String),
                    ("connected".to_string(), Type::Bool),
                ])))
            }
            Some("simulation/stop") => Some(Type::Record(BTreeMap::from([(
                "id".to_string(),
                Type::String,
            )]))),
            Some("file/download") => Some(Type::Record(BTreeMap::from([
                ("name".to_string(), Type::String),
                ("content".to_string(), Type::String),
                ("mime".to_string(), Type::String),
            ]))),
            Some("canvas/draw") => Some(Type::Record(BTreeMap::from([
                ("ref".to_string(), Type::String),
                ("width".to_string(), Type::Number),
                ("height".to_string(), Type::Number),
                ("cssWidth".to_string(), Type::Number),
                ("cssHeight".to_string(), Type::Number),
                ("pixelRatio".to_string(), Type::Number),
            ]))),
            Some("canvas/measure-text") => Some(Type::Record(BTreeMap::from([
                ("ref".to_string(), Type::String),
                ("font".to_string(), Type::String),
                ("texts".to_string(), Type::Vector(Box::new(Type::String))),
                ("widths".to_string(), Type::Vector(Box::new(Type::Number))),
                (
                    "measurements".to_string(),
                    Type::Vector(Box::new(text_measurement_type())),
                ),
            ]))),
            Some("dom-ref/focus") | Some("dom-ref/click") => {
                Some(Type::Record(BTreeMap::from([(
                    "ref".to_string(),
                    Type::String,
                )])))
            }
            Some("dom-ref/measure") => Some({
                let mut fields = record_fields(&rect_type());
                fields.insert("ref".to_string(), Type::String);
                Type::Record(fields)
            }),
            Some("animation/frame")
            | Some("animation/cancel")
            | Some("dom-ref/resize-unwatch")
            | Some("window/event-unwatch")
            | Some("media-query/unwatch") => Some(id_payload_type()),
            Some("window/event-watch") => Some(Type::Record(BTreeMap::from([
                ("id".to_string(), Type::String),
                ("type".to_string(), Type::String),
            ]))),
            _ => Some(self.fresh()),
        }
    }

    fn bind(&mut self, id: u32, ty: Type, span: Span) -> Type {
        if ty == Type::Var(id) {
            return ty;
        }
        if self.occurs(id, &ty) {
            let ty_name = self.format_type(&ty);
            self.diagnostics.push(Diagnostic::error(
                span,
                format!("recursive type t{} occurs in {}", id, ty_name),
            ));
            return Type::Var(id);
        }
        self.subst.insert(id, ty.clone());
        ty
    }

    fn occurs(&mut self, id: u32, ty: &Type) -> bool {
        match self.resolve(ty.clone()) {
            Type::Var(other) => id == other,
            Type::Option(inner)
            | Type::Decoder(inner)
            | Type::List(inner)
            | Type::Vector(inner)
            | Type::Set(inner)
            | Type::Cmd(inner)
            | Type::Sub(inner)
            | Type::Event(inner) => self.occurs(id, &inner),
            Type::Tuple(items) => items.iter().any(|item| self.occurs(id, item)),
            Type::Map(key, value) | Type::Result(key, value) => {
                self.occurs(id, &key) || self.occurs(id, &value)
            }
            Type::Task(err, ok) => self.occurs(id, &err) || self.occurs(id, &ok),
            Type::Union(variants) => variants.iter().any(|variant| self.occurs(id, variant)),
            Type::Record(fields) => fields.values().any(|field| self.occurs(id, field)),
            Type::Fn(args, ret) => {
                args.iter().any(|arg| self.occurs(id, arg)) || self.occurs(id, &ret)
            }
            _ => false,
        }
    }

    fn resolve(&mut self, ty: Type) -> Type {
        match ty {
            Type::Var(id) => match self.subst.get(&id).cloned() {
                Some(bound) => {
                    let resolved = self.resolve(bound);
                    self.subst.insert(id, resolved.clone());
                    resolved
                }
                None => Type::Var(id),
            },
            Type::Option(inner) => Type::Option(Box::new(self.resolve(*inner))),
            Type::Decoder(inner) => Type::Decoder(Box::new(self.resolve(*inner))),
            Type::List(inner) => Type::List(Box::new(self.resolve(*inner))),
            Type::Vector(inner) => Type::Vector(Box::new(self.resolve(*inner))),
            Type::Tuple(items) => Type::Tuple(
                items
                    .into_iter()
                    .map(|item| self.resolve(item))
                    .collect::<Vec<_>>(),
            ),
            Type::Set(inner) => Type::Set(Box::new(self.resolve(*inner))),
            Type::Map(key, value) => {
                Type::Map(Box::new(self.resolve(*key)), Box::new(self.resolve(*value)))
            }
            Type::Result(ok, err) => {
                Type::Result(Box::new(self.resolve(*ok)), Box::new(self.resolve(*err)))
            }
            Type::Cmd(msg) => Type::Cmd(Box::new(self.resolve(*msg))),
            Type::Task(err, ok) => {
                Type::Task(Box::new(self.resolve(*err)), Box::new(self.resolve(*ok)))
            }
            Type::Sub(msg) => Type::Sub(Box::new(self.resolve(*msg))),
            Type::Event(msg) => Type::Event(Box::new(self.resolve(*msg))),
            Type::Union(variants) => Type::Union(
                variants
                    .into_iter()
                    .map(|variant| self.resolve(variant))
                    .collect(),
            ),
            Type::Record(fields) => Type::Record(
                fields
                    .into_iter()
                    .map(|(name, ty)| (name, self.resolve(ty)))
                    .collect(),
            ),
            Type::Fn(args, ret) => Type::Fn(
                args.into_iter().map(|arg| self.resolve(arg)).collect(),
                Box::new(self.resolve(*ret)),
            ),
            other => other,
        }
    }

    fn shallow_resolve(&mut self, ty: Type) -> Type {
        match ty {
            Type::Var(id) => match self.subst.get(&id).cloned() {
                Some(Type::Var(other)) if other != id => {
                    let resolved = self.shallow_resolve(Type::Var(other));
                    self.subst.insert(id, resolved.clone());
                    resolved
                }
                Some(bound) => bound,
                None => Type::Var(id),
            },
            other => other,
        }
    }

    fn fresh(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        Type::Var(id)
    }

    fn format_type(&mut self, ty: &Type) -> String {
        let resolved = self.resolve(ty.clone());
        format_type_inner(&resolved)
    }

    fn type_syntax_to_type(
        &mut self,
        syntax: &TypeSyntax,
        aliases: &HashMap<String, TypeAlias>,
        span: Span,
    ) -> Option<Type> {
        let mut resolving = BTreeSet::new();
        let mut type_vars = HashMap::new();
        self.type_syntax_to_type_inner(syntax, aliases, span, &mut resolving, &mut type_vars)
    }

    fn type_syntax_to_type_inner(
        &mut self,
        syntax: &TypeSyntax,
        aliases: &HashMap<String, TypeAlias>,
        span: Span,
        resolving: &mut BTreeSet<String>,
        type_vars: &mut HashMap<String, Type>,
    ) -> Option<Type> {
        match syntax {
            TypeSyntax::Named(name) => match name.as_str() {
                "Number" => Some(Type::Number),
                "String" => Some(Type::String),
                "Bool" => Some(Type::Bool),
                "Nil" => Some(Type::Nil),
                "Keyword" => Some(Type::Keyword(None)),
                "Syntax" => Some(Type::Syntax),
                "Js" => Some(Type::Js),
                "Html" => Some(Type::Html),
                "TrustedHtml" => Some(Type::TrustedHtml),
                _ => {
                    if is_annotation_type_var(name) {
                        return Some(
                            type_vars
                                .entry(name.clone())
                                .or_insert_with(|| self.fresh())
                                .clone(),
                        );
                    }
                    let Some(alias) = aliases.get(name) else {
                        self.diagnostics.push(Diagnostic::error(
                            span,
                            format!("unknown type name `{}` in annotation", name),
                        ));
                        return None;
                    };
                    if !alias.params.is_empty() {
                        self.diagnostics.push(Diagnostic::error(
                            span,
                            format!(
                                "{} expects {} type arguments, found 0",
                                name,
                                alias.params.len()
                            ),
                        ));
                        return None;
                    }
                    if !resolving.insert(name.clone()) {
                        self.diagnostics.push(Diagnostic::error(
                            span,
                            format!("recursive type alias `{}` cannot be checked", name),
                        ));
                        return None;
                    }
                    let ty = self.type_syntax_to_type_inner(
                        &alias.syntax,
                        aliases,
                        span,
                        resolving,
                        type_vars,
                    );
                    resolving.remove(name);
                    ty
                }
            },
            TypeSyntax::Keyword(name) => Some(Type::Keyword(Some(name.clone()))),
            TypeSyntax::Record(fields) => {
                let mut checked = BTreeMap::new();
                for (name, field) in fields {
                    let Some(ty) =
                        self.type_syntax_to_type_inner(field, aliases, span, resolving, type_vars)
                    else {
                        return None;
                    };
                    checked.insert(name.clone(), ty);
                }
                Some(Type::Record(checked))
            }
            TypeSyntax::Tuple(items) => {
                let mut checked = Vec::new();
                for item in items {
                    let Some(ty) =
                        self.type_syntax_to_type_inner(item, aliases, span, resolving, type_vars)
                    else {
                        return None;
                    };
                    checked.push(ty);
                }
                Some(Type::Tuple(checked))
            }
            TypeSyntax::Apply { name, args } => match name.as_str() {
                "Decoder" => self
                    .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)
                    .map(|inner| Type::Decoder(Box::new(inner))),
                "Option" => self
                    .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)
                    .map(|inner| Type::Option(Box::new(inner))),
                "List" => self
                    .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)
                    .map(|inner| Type::List(Box::new(inner))),
                "Vector" => self
                    .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)
                    .map(|inner| Type::Vector(Box::new(inner))),
                "Set" => self
                    .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)
                    .map(|inner| Type::Set(Box::new(inner))),
                "Map" => {
                    let key = self
                        .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)?;
                    let value = self
                        .type_syntax_to_type_inner(&args[1], aliases, span, resolving, type_vars)?;
                    Some(Type::Map(Box::new(key), Box::new(value)))
                }
                "Result" => {
                    let ok = self
                        .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)?;
                    let err = self
                        .type_syntax_to_type_inner(&args[1], aliases, span, resolving, type_vars)?;
                    Some(Type::Result(Box::new(ok), Box::new(err)))
                }
                "Cmd" => {
                    let msg = self
                        .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)?;
                    Some(Type::Cmd(Box::new(msg)))
                }
                "Sub" => {
                    let msg = self
                        .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)?;
                    Some(Type::Sub(Box::new(msg)))
                }
                "Event" => {
                    let msg = self
                        .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)?;
                    Some(Type::Event(Box::new(msg)))
                }
                "Task" => {
                    let err = self
                        .type_syntax_to_type_inner(&args[0], aliases, span, resolving, type_vars)?;
                    let ok = self
                        .type_syntax_to_type_inner(&args[1], aliases, span, resolving, type_vars)?;
                    Some(Type::Task(Box::new(err), Box::new(ok)))
                }
                _ => {
                    if let Some(alias) = aliases.get(name) {
                        if alias.params.len() != args.len() {
                            self.diagnostics.push(Diagnostic::error(
                                span,
                                format!(
                                    "{} expects {} type arguments, found {}",
                                    name,
                                    alias.params.len(),
                                    args.len()
                                ),
                            ));
                            return None;
                        }
                        if !resolving.insert(name.clone()) {
                            self.diagnostics.push(Diagnostic::error(
                                span,
                                format!("recursive type alias `{}` cannot be checked", name),
                            ));
                            return None;
                        }
                        let bindings = alias
                            .params
                            .iter()
                            .cloned()
                            .zip(args.iter().cloned())
                            .collect::<HashMap<_, _>>();
                        let substituted = substitute_type_syntax(&alias.syntax, &bindings);
                        let ty = self.type_syntax_to_type_inner(
                            &substituted,
                            aliases,
                            span,
                            resolving,
                            type_vars,
                        );
                        resolving.remove(name);
                        return ty;
                    }
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!("unknown type constructor `{}` in annotation", name),
                    ));
                    None
                }
            },
            TypeSyntax::Fn { args, ret } => {
                let mut checked_args = Vec::new();
                for arg in args {
                    let Some(ty) =
                        self.type_syntax_to_type_inner(arg, aliases, span, resolving, type_vars)
                    else {
                        return None;
                    };
                    checked_args.push(ty);
                }
                let ret =
                    self.type_syntax_to_type_inner(ret, aliases, span, resolving, type_vars)?;
                Some(Type::Fn(checked_args, Box::new(ret)))
            }
            TypeSyntax::Union(variants) => {
                let mut checked = Vec::new();
                for variant in variants {
                    let Some(ty) = self
                        .type_syntax_to_type_inner(variant, aliases, span, resolving, type_vars)
                    else {
                        return None;
                    };
                    checked.push(ty);
                }
                Some(Type::Union(checked))
            }
        }
    }

    fn arity_error(&mut self, span: Span, form: &str, expected: usize, found: usize) {
        self.diagnostics.push(Diagnostic::error(
            span,
            format!("{} expects {} arguments, found {}", form, expected, found),
        ));
    }
}

#[derive(Clone, Copy)]
enum CollectionKind {
    List,
    Vector,
    Set,
}

fn ordered_collection_type(kind: CollectionKind, element: Type) -> Type {
    match kind {
        CollectionKind::List => Type::List(Box::new(element)),
        CollectionKind::Vector | CollectionKind::Set => Type::Vector(Box::new(element)),
    }
}

struct HtmlForSpec<'a> {
    item: &'a str,
    index: Option<&'a str>,
    collection: &'a Expr,
    key: &'a Expr,
    template: &'a HtmlNode,
}

impl<'a> HtmlForSpec<'a> {
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

struct HtmlIfSpec<'a> {
    condition: &'a Expr,
    then_template: &'a HtmlNode,
    else_template: &'a HtmlNode,
}

impl<'a> HtmlIfSpec<'a> {
    fn parse(expr: &'a Expr) -> Option<Self> {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        if items.len() != 4 || !matches_symbol(&items[0], "if") {
            return None;
        }

        let ExprKind::HtmlTemplate(then_template) = &items[2].kind else {
            return None;
        };
        let ExprKind::HtmlTemplate(else_template) = &items[3].kind else {
            return None;
        };

        Some(Self {
            condition: &items[1],
            then_template,
            else_template,
        })
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

fn update_message_type_syntax(syntax: &TypeSyntax) -> Option<&TypeSyntax> {
    let TypeSyntax::Fn { args, .. } = syntax else {
        return None;
    };
    args.get(1)
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

fn is_annotation_type_var(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
}

fn definition_name(expr: &Expr) -> Option<&str> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    if let [head, name, _] = items.as_slice() {
        if matches_symbol(head, "def") {
            if let ExprKind::Symbol(name) = &name.kind {
                return Some(name);
            }
        }
    }
    if items.len() >= 4 && matches_symbol(&items[0], "defn") {
        if let ExprKind::Symbol(name) = &items[1].kind {
            return Some(name);
        }
    }
    None
}

fn parse_type_declaration_form(expr: &Expr) -> Option<Result<TypeDeclaration, Diagnostic>> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    if !items
        .first()
        .is_some_and(|head| matches_symbol(head, "type"))
    {
        return None;
    }
    if items.len() < 3 {
        return Some(Err(Diagnostic::error(
            expr.span,
            "type expects a name, optional type parameters, and a type expression",
        )));
    }

    let ExprKind::Symbol(name) = &items[1].kind else {
        return Some(Err(Diagnostic::error(
            items[1].span,
            "type name must be a symbol",
        )));
    };
    let mut params = Vec::new();
    let mut seen = BTreeSet::new();
    for param in &items[2..items.len() - 1] {
        let ExprKind::Symbol(param_name) = &param.kind else {
            return Some(Err(Diagnostic::error(
                param.span,
                "type parameters must be symbols",
            )));
        };
        if !seen.insert(param_name.clone()) {
            return Some(Err(Diagnostic::error(
                param.span,
                format!("duplicate type parameter `{}`", param_name),
            )));
        }
        params.push(param_name.clone());
    }
    Some(
        parse_type_syntax(items.last().expect("type expression should exist")).map(|syntax| {
            TypeDeclaration {
                name: name.clone(),
                params,
                schema: syntax.render(),
                syntax,
                span: expr.span,
            }
        }),
    )
}

fn parse_type_annotation_form(expr: &Expr) -> Option<Result<TypeAnnotation, Diagnostic>> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    if !items
        .first()
        .is_some_and(|head| matches_symbol(head, "ann"))
    {
        return None;
    }
    if items.len() != 3 {
        return Some(Err(Diagnostic::error(
            expr.span,
            "ann expects a name and a type expression",
        )));
    }

    let ExprKind::Symbol(name) = &items[1].kind else {
        return Some(Err(Diagnostic::error(
            items[1].span,
            "ann name must be a symbol",
        )));
    };
    Some(parse_type_syntax(&items[2]).map(|syntax| TypeAnnotation {
        name: name.clone(),
        schema: syntax.render(),
        syntax,
        span: expr.span,
    }))
}

fn parse_foreign_declaration_form(expr: &Expr) -> Option<Result<ForeignDeclaration, Diagnostic>> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    if !items
        .first()
        .is_some_and(|head| matches_symbol(head, "foreign"))
    {
        return None;
    }
    if items.len() != 4 {
        return Some(Err(Diagnostic::error(
            expr.span,
            "foreign expects a mode, name, and type expression",
        )));
    }

    let ExprKind::Symbol(mode) = &items[1].kind else {
        return Some(Err(Diagnostic::error(
            items[1].span,
            "foreign mode must be a symbol",
        )));
    };
    if !matches!(mode.as_str(), "pure" | "task" | "command") {
        return Some(Err(Diagnostic::error(
            items[1].span,
            "foreign mode must be pure, task, or command",
        )));
    }

    let ExprKind::Symbol(name) = &items[2].kind else {
        return Some(Err(Diagnostic::error(
            items[2].span,
            "foreign name must be a symbol",
        )));
    };

    Some(
        parse_type_syntax(&items[3]).map(|syntax| ForeignDeclaration {
            mode: mode.clone(),
            name: name.clone(),
            schema: syntax.render(),
            syntax,
            span: expr.span,
        }),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TypeSyntax {
    Named(String),
    Keyword(String),
    Record(BTreeMap<String, TypeSyntax>),
    Tuple(Vec<TypeSyntax>),
    Apply {
        name: String,
        args: Vec<TypeSyntax>,
    },
    Fn {
        args: Vec<TypeSyntax>,
        ret: Box<TypeSyntax>,
    },
    Union(Vec<TypeSyntax>),
}

impl TypeSyntax {
    fn render(&self) -> String {
        match self {
            TypeSyntax::Named(name) => name.clone(),
            TypeSyntax::Keyword(name) => format!(":{}", name),
            TypeSyntax::Record(fields) => {
                let fields = fields
                    .iter()
                    .map(|(name, ty)| format!(":{} {}", name, ty.render()))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{{{}}}", fields)
            }
            TypeSyntax::Tuple(items) => {
                let items = items
                    .iter()
                    .map(TypeSyntax::render)
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("[{}]", items)
            }
            TypeSyntax::Apply { name, args } => {
                let args = args
                    .iter()
                    .map(TypeSyntax::render)
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("({} {})", name, args)
            }
            TypeSyntax::Fn { args, ret } => {
                let args = args
                    .iter()
                    .map(TypeSyntax::render)
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(Fn [{}] {})", args, ret.render())
            }
            TypeSyntax::Union(variants) => {
                let variants = variants
                    .iter()
                    .map(TypeSyntax::render)
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(Union {})", variants)
            }
        }
    }
}

fn substitute_type_syntax(
    syntax: &TypeSyntax,
    bindings: &HashMap<String, TypeSyntax>,
) -> TypeSyntax {
    match syntax {
        TypeSyntax::Named(name) => bindings
            .get(name)
            .cloned()
            .unwrap_or_else(|| TypeSyntax::Named(name.clone())),
        TypeSyntax::Keyword(name) => TypeSyntax::Keyword(name.clone()),
        TypeSyntax::Record(fields) => TypeSyntax::Record(
            fields
                .iter()
                .map(|(name, field)| (name.clone(), substitute_type_syntax(field, bindings)))
                .collect(),
        ),
        TypeSyntax::Tuple(items) => TypeSyntax::Tuple(
            items
                .iter()
                .map(|item| substitute_type_syntax(item, bindings))
                .collect(),
        ),
        TypeSyntax::Apply { name, args } => TypeSyntax::Apply {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_type_syntax(arg, bindings))
                .collect(),
        },
        TypeSyntax::Fn { args, ret } => TypeSyntax::Fn {
            args: args
                .iter()
                .map(|arg| substitute_type_syntax(arg, bindings))
                .collect(),
            ret: Box::new(substitute_type_syntax(ret, bindings)),
        },
        TypeSyntax::Union(variants) => TypeSyntax::Union(
            variants
                .iter()
                .map(|variant| substitute_type_syntax(variant, bindings))
                .collect(),
        ),
    }
}

fn type_parameter_validation_bindings(params: &[String]) -> HashMap<String, TypeSyntax> {
    params
        .iter()
        .map(|param| (param.clone(), TypeSyntax::Named("Js".to_string())))
        .collect()
}

fn parse_type_syntax(expr: &Expr) -> Result<TypeSyntax, Diagnostic> {
    match &expr.kind {
        ExprKind::Symbol(name) => Ok(TypeSyntax::Named(name.clone())),
        ExprKind::Keyword(name) => Ok(TypeSyntax::Keyword(name.clone())),
        ExprKind::Vector(items) => items
            .iter()
            .map(parse_type_syntax)
            .collect::<Result<Vec<_>, _>>()
            .map(TypeSyntax::Tuple),
        ExprKind::Map(entries) => parse_record_type_syntax(entries),
        ExprKind::List(items) => parse_type_application(expr.span, items),
        _ => Err(Diagnostic::error(expr.span, "expected a type expression")),
    }
}

fn parse_record_type_syntax(entries: &[(Expr, Expr)]) -> Result<TypeSyntax, Diagnostic> {
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let Some(name) = record_key_name(key) else {
            return Err(Diagnostic::error(
                key.span,
                "record type field names must be keywords, strings, or symbols",
            ));
        };
        fields.insert(name, parse_type_syntax(value)?);
    }
    Ok(TypeSyntax::Record(fields))
}

fn parse_type_application(span: Span, items: &[Expr]) -> Result<TypeSyntax, Diagnostic> {
    let Some((head, args)) = items.split_first() else {
        return Err(Diagnostic::error(span, "expected a type constructor"));
    };
    let ExprKind::Symbol(name) = &head.kind else {
        return Err(Diagnostic::error(
            head.span,
            "type constructor must be a symbol",
        ));
    };

    match name.as_str() {
        "Decoder" | "Option" | "List" | "Vector" | "Set" | "Cmd" | "Sub" | "Event" => {
            require_type_arity(span, name, args, 1)?;
            Ok(TypeSyntax::Apply {
                name: name.clone(),
                args: vec![parse_type_syntax(&args[0])?],
            })
        }
        "Map" | "Result" | "Task" => {
            require_type_arity(span, name, args, 2)?;
            Ok(TypeSyntax::Apply {
                name: name.clone(),
                args: args
                    .iter()
                    .map(parse_type_syntax)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        }
        "Tuple" => {
            if args.is_empty() {
                return Err(Diagnostic::error(span, "Tuple expects at least one type"));
            }
            args.iter()
                .map(parse_type_syntax)
                .collect::<Result<Vec<_>, _>>()
                .map(TypeSyntax::Tuple)
        }
        "Fn" => parse_fn_type_syntax(span, args),
        "Union" => {
            if args.is_empty() {
                return Err(Diagnostic::error(
                    span,
                    "Union expects at least one variant",
                ));
            }
            args.iter()
                .map(parse_type_syntax)
                .collect::<Result<Vec<_>, _>>()
                .map(TypeSyntax::Union)
        }
        _ => Ok(TypeSyntax::Apply {
            name: name.clone(),
            args: args
                .iter()
                .map(parse_type_syntax)
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

fn parse_fn_type_syntax(span: Span, args: &[Expr]) -> Result<TypeSyntax, Diagnostic> {
    require_type_arity(span, "Fn", args, 2)?;
    let ExprKind::Vector(params) = &args[0].kind else {
        return Err(Diagnostic::error(
            args[0].span,
            "Fn expects a vector of parameter types",
        ));
    };
    let params = params
        .iter()
        .map(parse_type_syntax)
        .collect::<Result<Vec<_>, _>>()?;
    let ret = parse_type_syntax(&args[1])?;
    Ok(TypeSyntax::Fn {
        args: params,
        ret: Box::new(ret),
    })
}

fn require_type_arity(
    span: Span,
    constructor: &str,
    args: &[Expr],
    expected: usize,
) -> Result<(), Diagnostic> {
    if args.len() == expected {
        return Ok(());
    }
    Err(Diagnostic::error(
        span,
        format!(
            "{} expects {} type arguments, found {}",
            constructor,
            expected,
            args.len()
        ),
    ))
}

struct ImportSpec {
    path: String,
    names: Vec<String>,
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
            Ok(symbol) => imported.push(symbol),
            Err(diagnostic) => return Some(Err(diagnostic)),
        }
    }

    Some(Ok(ImportSpec {
        path: path.clone(),
        names: imported,
    }))
}

fn parse_import_name(expr: &Expr) -> Result<String, Diagnostic> {
    match &expr.kind {
        ExprKind::Symbol(symbol) => Ok(symbol.clone()),
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
            Ok(local.clone())
        }
        ExprKind::List(items)
            if items.len() == 3
                && matches!(&items[1].kind, ExprKind::Symbol(name) if name == "as") =>
        {
            let ExprKind::Symbol(local) = &items[2].kind else {
                return Err(Diagnostic::error(
                    items[2].span,
                    "aliased import local name must be a symbol",
                ));
            };
            Ok(local.clone())
        }
        _ => Err(Diagnostic::error(
            expr.span,
            "imported name must be a symbol, (default local), or (name as local)",
        )),
    }
}

fn is_closkell_import_path(path: &str) -> bool {
    path.ends_with(".clsk")
}

fn format_type_inner(ty: &Type) -> String {
    format_type_inner_with_keywords(ty, false)
}

fn format_type_with_literals(ty: &Type) -> String {
    format_type_inner_with_keywords(ty, true)
}

fn type_returns_cmd(ty: &Type) -> bool {
    match ty {
        Type::Cmd(_) => true,
        Type::Fn(_, ret) => type_returns_cmd(ret),
        _ => false,
    }
}

fn type_returns_sub(ty: &Type) -> bool {
    match ty {
        Type::Sub(_) => true,
        Type::Fn(_, ret) => type_returns_sub(ret),
        _ => false,
    }
}

fn is_known_subscription_kind(kind: &str) -> bool {
    matches!(
        kind,
        "none"
            | "batch"
            | "sub/timer/every"
            | "sub/dom-ref/resize"
            | "sub/window/event"
            | "sub/media-query"
            | "sub/simulation/heart-rate"
            | "sub/bluetooth/connect-heart-rate"
    )
}

fn subscription_command_kind(kind: Option<&str>) -> Option<&'static str> {
    match kind {
        Some("sub/timer/every") => Some("timer/every"),
        Some("sub/dom-ref/resize") => Some("dom-ref/resize-watch"),
        Some("sub/window/event") => Some("window/event-watch"),
        Some("sub/media-query") => Some("media-query/watch"),
        Some("sub/simulation/heart-rate") => Some("simulation/heart-rate"),
        Some("sub/bluetooth/connect-heart-rate") => Some("bluetooth/connect-heart-rate"),
        _ => kind.and_then(|kind| match kind {
            "none" => Some("none"),
            "batch" => Some("batch"),
            _ => None,
        }),
    }
}

fn format_type_inner_with_keywords(ty: &Type, show_keyword_literals: bool) -> String {
    match ty {
        Type::Var(id) => format!("t{}", id),
        Type::Number => "Number".to_string(),
        Type::String => "String".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::Nil => "Nil".to_string(),
        Type::Keyword(Some(name)) if show_keyword_literals => format!(":{}", name),
        Type::Keyword(_) => "Keyword".to_string(),
        Type::Syntax => "Syntax".to_string(),
        Type::Js => "Js".to_string(),
        Type::Html => "Html".to_string(),
        Type::TrustedHtml => "TrustedHtml".to_string(),
        Type::Decoder(inner) => {
            format!(
                "(Decoder {})",
                format_type_inner_with_keywords(inner, show_keyword_literals)
            )
        }
        Type::Option(inner) => {
            format!(
                "(Option {})",
                format_type_inner_with_keywords(inner, show_keyword_literals)
            )
        }
        Type::List(inner) => {
            format!(
                "(List {})",
                format_type_inner_with_keywords(inner, show_keyword_literals)
            )
        }
        Type::Vector(inner) => {
            format!(
                "(Vector {})",
                format_type_inner_with_keywords(inner, show_keyword_literals)
            )
        }
        Type::Tuple(items) => {
            let items = items
                .iter()
                .map(|item| format_type_inner_with_keywords(item, show_keyword_literals))
                .collect::<Vec<_>>();
            format!("[{}]", items.join(" "))
        }
        Type::Set(inner) => {
            format!(
                "(Set {})",
                format_type_inner_with_keywords(inner, show_keyword_literals)
            )
        }
        Type::Map(key, value) => {
            format!(
                "(Map {} {})",
                format_type_inner_with_keywords(key, show_keyword_literals),
                format_type_inner_with_keywords(value, show_keyword_literals)
            )
        }
        Type::Result(ok, err) => {
            format!(
                "(Result {} {})",
                format_type_inner_with_keywords(ok, show_keyword_literals),
                format_type_inner_with_keywords(err, show_keyword_literals)
            )
        }
        Type::Cmd(msg) => {
            format!(
                "(Cmd {})",
                format_type_inner_with_keywords(msg, show_keyword_literals)
            )
        }
        Type::Task(err, ok) => {
            format!(
                "(Task {} {})",
                format_type_inner_with_keywords(err, show_keyword_literals),
                format_type_inner_with_keywords(ok, show_keyword_literals)
            )
        }
        Type::Sub(msg) => {
            format!(
                "(Sub {})",
                format_type_inner_with_keywords(msg, show_keyword_literals)
            )
        }
        Type::Event(msg) => {
            format!(
                "(Event {})",
                format_type_inner_with_keywords(msg, show_keyword_literals)
            )
        }
        Type::Union(variants) => {
            let variants = variants
                .iter()
                .map(|variant| format_type_inner_with_keywords(variant, show_keyword_literals))
                .collect::<Vec<_>>();
            format!("(Union {})", variants.join(" "))
        }
        Type::Record(fields) => {
            let fields = fields
                .iter()
                .map(|(name, ty)| {
                    format!(
                        ":{} {}",
                        name,
                        format_type_inner_with_keywords(ty, show_keyword_literals)
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("{{{}}}", fields)
        }
        Type::Fn(args, ret) => {
            let args = args
                .iter()
                .map(|arg| format_type_inner_with_keywords(arg, show_keyword_literals))
                .collect::<Vec<_>>();
            format!(
                "(Fn [{}] {})",
                args.join(" "),
                format_type_inner_with_keywords(ret, show_keyword_literals)
            )
        }
    }
}

fn keyword_type_accepts(expected: &Option<String>, actual: &Option<String>) -> bool {
    match (expected, actual) {
        (None, _) => true,
        (Some(expected), Some(actual)) => expected == actual,
        (Some(_), None) => false,
    }
}

fn keyword_literal_matches(expected: &Option<String>, actual: &str) -> bool {
    match expected {
        Some(expected) => expected == actual,
        None => true,
    }
}

fn primitive_decoder_type(name: &str) -> Option<Type> {
    match name {
        "decoder-string" => Some(Type::Decoder(Box::new(Type::String))),
        "decoder-number" => Some(Type::Decoder(Box::new(Type::Number))),
        "decoder-bool" => Some(Type::Decoder(Box::new(Type::Bool))),
        "decoder-keyword" => Some(Type::Decoder(Box::new(Type::Keyword(None)))),
        _ => None,
    }
}

fn could_be_homogeneous(left: &Type, right: &Type) -> bool {
    match (left, right) {
        (Type::Record(left), Type::Record(right)) => left.keys().eq(right.keys()),
        (Type::Tuple(left), Type::Tuple(right)) => left.len() == right.len(),
        (Type::Var(_), Type::Record(_)) | (Type::Record(_), Type::Var(_)) => false,
        (Type::Var(_), Type::Cmd(_)) | (Type::Cmd(_), Type::Var(_)) => false,
        (Type::Var(_), Type::Sub(_)) | (Type::Sub(_), Type::Var(_)) => false,
        (Type::Var(_), Type::Task(_, _)) | (Type::Task(_, _), Type::Var(_)) => false,
        (Type::Var(_), Type::Event(_)) | (Type::Event(_), Type::Var(_)) => false,
        (Type::Var(_), _) | (_, Type::Var(_)) => true,
        (Type::Option(left), Type::Option(right))
        | (Type::Decoder(left), Type::Decoder(right))
        | (Type::List(left), Type::List(right))
        | (Type::Vector(left), Type::Vector(right))
        | (Type::Set(left), Type::Set(right)) => could_be_homogeneous(left, right),
        (Type::Map(left_key, left_value), Type::Map(right_key, right_value)) => {
            could_be_homogeneous(left_key, right_key)
                && could_be_homogeneous(left_value, right_value)
        }
        (Type::Result(left_ok, left_err), Type::Result(right_ok, right_err)) => {
            could_be_homogeneous(left_ok, right_ok) && could_be_homogeneous(left_err, right_err)
        }
        (Type::Cmd(_), Type::Cmd(_)) => false,
        (Type::Task(left_err, left_ok), Type::Task(right_err, right_ok)) => {
            could_be_homogeneous(left_err, right_err) && could_be_homogeneous(left_ok, right_ok)
        }
        (Type::Sub(left), Type::Sub(right)) | (Type::Event(left), Type::Event(right)) => {
            could_be_homogeneous(left, right)
        }
        (Type::Union(left), Type::Union(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| could_be_homogeneous(left, right))
        }
        (Type::Fn(left_args, left_ret), Type::Fn(right_args, right_ret)) => {
            left_args.len() == right_args.len()
                && left_args
                    .iter()
                    .zip(right_args)
                    .all(|(left, right)| could_be_homogeneous(left, right))
                && could_be_homogeneous(left_ret, right_ret)
        }
        _ => std::mem::discriminant(left) == std::mem::discriminant(right),
    }
}

fn is_boolean_html_attr(name: &str) -> bool {
    matches!(
        name,
        "allowfullscreen"
            | "async"
            | "autofocus"
            | "autoplay"
            | "checked"
            | "controls"
            | "default"
            | "defer"
            | "disabled"
            | "formnovalidate"
            | "hidden"
            | "inert"
            | "ismap"
            | "loop"
            | "multiple"
            | "muted"
            | "nomodule"
            | "novalidate"
            | "open"
            | "playsinline"
            | "readonly"
            | "required"
            | "reversed"
            | "selected"
    )
}

pub fn free_symbols(expr: &Expr) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    collect_symbols(expr, &mut symbols);
    symbols.into_iter().collect()
}

fn collect_symbols(expr: &Expr, symbols: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Symbol(name) => {
            symbols.insert(name.clone());
        }
        ExprKind::List(items) => {
            if let Some((head, args)) = items.split_first() {
                if matches!(&head.kind, ExprKind::Symbol(_)) {
                    for item in args {
                        collect_symbols(item, symbols);
                    }
                    return;
                }
            }
            for item in items {
                collect_symbols(item, symbols);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_symbols(item, symbols);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_symbols(key, symbols);
                collect_symbols(value, symbols);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_symbols(inner, symbols),
        ExprKind::HtmlTemplate(_) => {}
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn split_path(name: &str) -> Option<(&str, Vec<&str>)> {
    let parts = name.split('.').collect::<Vec<_>>();
    if parts.len() < 2 || parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    Some((parts[0], parts[1..].to_vec()))
}

fn record_key_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Keyword(name) | ExprKind::Symbol(name) | ExprKind::String(name) => {
            Some(name.clone())
        }
        _ => None,
    }
}

const COMMAND_MESSAGE_TAG_FIELDS: &[&str] = &[
    "onSuccess",
    "onError",
    "onCancel",
    "onFrame",
    "onChange",
    "onReading",
    "onDisconnected",
    "onEvent",
];

const COMMAND_CONTINUATION_FIELDS: &[&str] = &[
    "msg",
    "onSuccess",
    "toMessage",
    "onError",
    "onCancel",
    "onFrame",
    "onChange",
    "onReading",
    "onDisconnected",
    "onEvent",
];

fn is_known_command_kind(kind: &str) -> bool {
    matches!(
        kind,
        "none"
            | "batch"
            | "bluetooth/request-device"
            | "bluetooth/connect-heart-rate"
            | "bluetooth/disconnect"
            | "timer/after"
            | "timer/every"
            | "timer/cancel"
            | "animation/frame"
            | "animation/cancel"
            | "time/now"
            | "storage/get"
            | "storage/set"
            | "storage/remove"
            | "browser/history-replace-search-param"
            | "browser/history-write-route"
            | "browser/theme-load"
            | "browser/theme-apply"
            | "browser/clipboard-write"
            | "browser/set-cookie"
            | "auth-storage/load"
            | "auth-storage/persist"
            | "random/number"
            | "simulation/heart-rate"
            | "simulation/stop"
            | "task/perform"
            | "file/download"
            | "file/import"
            | "file/read-selected"
            | "canvas/draw"
            | "canvas/measure-text"
            | "dom-ref/focus"
            | "dom-ref/click"
            | "dom-ref/measure"
            | "dom/scroll-into-view"
            | "dom-ref/resize-watch"
            | "dom-ref/resize-unwatch"
            | "window/event-watch"
            | "window/event-unwatch"
            | "media-query/watch"
            | "media-query/unwatch"
            | "http/request"
    )
}

fn keyword_literal_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Keyword(Some(name)) => Some(name),
        _ => None,
    }
}

fn command_payload_type_from_format_name(name: Option<&str>) -> Type {
    match name {
        Some("text" | "string" | "raw") => Type::String,
        Some("json") | Some("auto") | None => Type::Js,
        Some(_) => Type::Js,
    }
}

fn command_payload_type_from_command_fields(fields: &BTreeMap<String, Type>) -> Type {
    let format = fields
        .get("format")
        .or_else(|| fields.get("parse"))
        .and_then(keyword_literal_name);
    command_payload_type_from_format_name(format)
}

fn tagged_record_literal(ty: &Type) -> Option<&str> {
    match ty {
        Type::Record(fields) => fields.get("kind").and_then(keyword_literal_name),
        _ => None,
    }
}

fn type_kind_literal(ty: &Type) -> Option<&str> {
    match ty {
        Type::Keyword(Some(name)) => Some(name),
        Type::Record(fields) => record_fields_kind_literal(fields),
        _ => None,
    }
}

fn record_fields_kind_literal(fields: &BTreeMap<String, Type>) -> Option<&str> {
    fields.get("kind").and_then(keyword_literal_name)
}

fn pattern_kind_literal(pattern: &Expr) -> Option<&str> {
    match &pattern.kind {
        ExprKind::Keyword(name) => Some(name),
        ExprKind::Map(entries) => entries.iter().find_map(|(key, value)| {
            (record_key_name(key).as_deref() == Some("kind"))
                .then(|| keyword_expr_literal(value))?
        }),
        ExprKind::List(items) if items.first().is_some_and(|head| matches_symbol(head, "as")) => {
            (items.len() == 3).then(|| pattern_kind_literal(&items[1]))?
        }
        _ => None,
    }
}

fn pattern_record_kind_literal<'a>(pattern_fields: &[(String, &'a Expr)]) -> Option<&'a str> {
    pattern_fields
        .iter()
        .find_map(|(field, value)| (field == "kind").then(|| keyword_expr_literal(value))?)
}

fn keyword_expr_literal(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Keyword(name) => Some(name),
        _ => None,
    }
}

fn records_have_distinct_tags(
    left: &BTreeMap<String, Type>,
    right: &BTreeMap<String, Type>,
) -> bool {
    match (
        left.get("kind").and_then(keyword_literal_name),
        right.get("kind").and_then(keyword_literal_name),
    ) {
        (Some(left), Some(right)) => left != right,
        _ => false,
    }
}

fn is_command_record_fields(fields: &BTreeMap<String, Type>) -> bool {
    fields
        .get("kind")
        .and_then(keyword_literal_name)
        .is_some_and(is_known_command_kind)
}

fn is_batch_command_record_fields(fields: &BTreeMap<String, Type>) -> bool {
    fields
        .get("kind")
        .and_then(keyword_literal_name)
        .is_some_and(|kind| kind == "batch")
}

fn format_keyword_literal(name: &str) -> String {
    format!(":{}", name)
}

fn rect_type() -> Type {
    Type::Record(BTreeMap::from([
        ("x".to_string(), Type::Number),
        ("y".to_string(), Type::Number),
        ("width".to_string(), Type::Number),
        ("height".to_string(), Type::Number),
        ("top".to_string(), Type::Number),
        ("right".to_string(), Type::Number),
        ("bottom".to_string(), Type::Number),
        ("left".to_string(), Type::Number),
    ]))
}

fn window_event_payload_type() -> Type {
    Type::Record(BTreeMap::from([
        ("type".to_string(), Type::String),
        ("clientX".to_string(), Type::Number),
        ("clientY".to_string(), Type::Number),
        ("pageX".to_string(), Type::Number),
        ("pageY".to_string(), Type::Number),
        ("screenX".to_string(), Type::Number),
        ("screenY".to_string(), Type::Number),
        ("movementX".to_string(), Type::Number),
        ("movementY".to_string(), Type::Number),
        ("button".to_string(), Type::Number),
        ("buttons".to_string(), Type::Number),
        ("pointerId".to_string(), Type::Number),
        ("pointerType".to_string(), Type::String),
        ("isPrimary".to_string(), Type::Bool),
        ("key".to_string(), Type::String),
        ("code".to_string(), Type::String),
        ("altKey".to_string(), Type::Bool),
        ("ctrlKey".to_string(), Type::Bool),
        ("metaKey".to_string(), Type::Bool),
        ("shiftKey".to_string(), Type::Bool),
    ]))
}

fn text_measurement_type() -> Type {
    Type::Record(BTreeMap::from([
        ("text".to_string(), Type::String),
        ("width".to_string(), Type::Number),
        ("actualBoundingBoxLeft".to_string(), Type::Number),
        ("actualBoundingBoxRight".to_string(), Type::Number),
        ("actualBoundingBoxAscent".to_string(), Type::Number),
        ("actualBoundingBoxDescent".to_string(), Type::Number),
    ]))
}

fn id_payload_type() -> Type {
    Type::Record(BTreeMap::from([("id".to_string(), Type::String)]))
}

fn bluetooth_filter_type() -> Type {
    Type::Record(BTreeMap::from([(
        "services".to_string(),
        Type::Vector(Box::new(Type::String)),
    )]))
}

fn record_fields(ty: &Type) -> BTreeMap<String, Type> {
    match ty {
        Type::Record(fields) => fields.clone(),
        _ => BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(input: &str) -> CheckResult {
        let source = syntax::parse_source(input);
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);
        check_source(&source)
    }

    fn check_with_imports(input: &str, imports: &[ImportedBinding]) -> CheckResult {
        let source = syntax::parse_source(input);
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);
        check_source_with_imports(&source, imports)
    }

    fn check_with_module_imports(
        input: &str,
        imports: &[ImportedBinding],
        imported_types: &[ImportedTypeDeclaration],
    ) -> CheckResult {
        let source = syntax::parse_source(input);
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);
        check_source_with_module_imports(&source, imports, imported_types)
    }

    fn imported_binding(result: &CheckResult, name: &str) -> ImportedBinding {
        result
            .bindings
            .iter()
            .find(|binding| binding.name == name)
            .unwrap_or_else(|| panic!("missing binding `{}`", name))
            .import_as(name)
    }

    fn imported_type(result: &CheckResult, name: &str) -> ImportedTypeDeclaration {
        result
            .type_declarations
            .iter()
            .find(|declaration| declaration.name == name)
            .unwrap_or_else(|| panic!("missing type declaration `{}`", name))
            .import_as(name)
    }

    #[test]
    fn infers_arithmetic_let() {
        let result = check("(let [x 41] (+ x 1))");

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "Number");
    }

    #[test]
    fn infers_identity_function() {
        let result = check("(fn [x] x)");

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [t0] t0)");
    }

    #[test]
    fn infers_self_recursive_functions() {
        let result = check(
            "(ann sum-down (Fn [Number Number] Number))\n\
             (defn sum-down [n total]\n  (if (<= n 0)\n      total\n      (sum-down (- n 1) (+ total n))))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [Number Number] Number)");
    }

    #[test]
    fn imported_names_are_available_to_typecheck() {
        let result = check(
            "(import \"./hrweb_metrics.clsk\" [calculate-trimp])\n\
             (defn summarize [entry] (calculate-trimp entry))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms.len(), 1);
        assert!(result.forms[0].ty.starts_with("(Fn ["));
    }

    #[test]
    fn imported_type_declarations_are_available_to_annotations() {
        let api = check(
            "(type Entry {:id String :durationMs Number})\n\
             (ann entry-label (Fn [Entry] String))\n\
             (defn entry-label [entry]\n  entry.id)",
        );
        assert!(api.diagnostics.is_empty(), "{:?}", api.diagnostics);
        let entry_ty = imported_type(&api, "Entry");
        let entry_label = imported_binding(&api, "entry-label");

        let result = check_with_module_imports(
            "(import \"./api.clsk\" [Entry entry-label])\n\
             (ann summarize (Fn [Entry] String))\n\
             (defn summarize [entry]\n  (entry-label entry))",
            &[entry_label],
            &[entry_ty],
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:durationMs Number :id String}] String)"
        );
    }

    #[test]
    fn infers_parametric_type_aliases() {
        let result = check(
            "(type RemoteData a\n\
               (Union\n\
                 {:kind :idle}\n\
                 {:kind :ready :value a}\n\
                 {:kind :failed :error String}))\n\
             (type ApiResult ok err (Result ok err))\n\
             (ann loaded (RemoteData {:id String}))\n\
             (def loaded {:kind :ready :value {:id \"spec\"}})\n\
             (ann parsed (ApiResult Number String))\n\
             (def parsed (ok 42))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.type_declarations[0].params, vec!["a"]);
        assert_eq!(result.type_declarations[1].params, vec!["ok", "err"]);
        assert_eq!(
            result.forms[0].ty,
            "(Union {:kind Keyword} {:kind Keyword :value {:id String}} {:error String :kind Keyword})"
        );
        assert_eq!(result.forms[1].ty, "(Result Number String)");
    }

    #[test]
    fn imported_parametric_type_declarations_are_available_to_annotations() {
        let api = check(
            "(type Box a {:value a})\n\
             (ann string-box (Box String))\n\
             (def string-box {:value \"ready\"})",
        );
        assert!(api.diagnostics.is_empty(), "{:?}", api.diagnostics);
        let box_ty = imported_type(&api, "Box");

        let result = check_with_module_imports(
            "(import \"./api.clsk\" [Box])\n\
             (ann value (Box Number))\n\
             (def value {:value 7})",
            &[],
            &[box_ty],
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "{:value Number}");
    }

    #[test]
    fn reports_parametric_type_alias_arity_errors() {
        let result = check(
            "(type Box a {:value a})\n\
             (ann bare Box)\n\
             (def bare {:value 1})\n\
             (ann too-many (Box String Number))\n\
             (def too-many {:value \"x\"})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("Box expects 1 type arguments, found 0")),
            "{:?}",
            result.diagnostics
        );
        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("Box expects 1 type arguments, found 2")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn collects_exported_api_type_declarations() {
        let result = check(
            "(type HeartReading {:bpm Number :time Number})\n\
             (type WorkoutMsg (Union {:kind :start} {:kind :heart-rate :reading HeartReading}))\n\
             (type UpdateResult [WorkoutState (Cmd WorkoutMsg)])\n\
             (def answer 42)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms.len(), 1);
        assert_eq!(result.type_declarations.len(), 3);
        assert_eq!(result.type_declarations[0].name, "HeartReading");
        assert_eq!(
            result.type_declarations[0].schema,
            "{:bpm Number :time Number}"
        );
        assert_eq!(
            result.type_declarations[1].schema,
            "(Union {:kind :start} {:kind :heart-rate :reading HeartReading})"
        );
        assert_eq!(
            result.type_declarations[2].schema,
            "[WorkoutState (Cmd WorkoutMsg)]"
        );
    }

    #[test]
    fn reports_invalid_api_type_declarations() {
        let result = check(
            "(type BadOption (Option String Number))\n\
             (type BadFn (Fn String Number))\n\
             (type BadUnion (Union))",
        );

        assert_eq!(result.forms.len(), 0);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Option expects 1"))
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Fn expects a vector"))
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Union expects at least one"))
        );
    }

    #[test]
    fn checks_exported_api_type_annotations() {
        let result = check(
            "(type HeartReading {:bpm Number :time Number})\n\
             (type Msg (Union {:kind :start} {:kind :stop} {:kind :heart-rate :reading HeartReading}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann reading-label (Fn [HeartReading] String))\n\
             (defn reading-label [reading] (str reading.bpm \" bpm\"))\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  (match msg\n    {:kind :heart-rate :reading reading} [{:label (str reading.bpm)} {:kind :none}]\n    _ [state {:kind :none}]))\n\
             (ann sample-count Number)\n\
             (def sample-count 2)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.type_annotations.len(), 3);
        assert_eq!(result.type_annotations[0].name, "reading-label");
        assert_eq!(
            result.type_annotations[0].schema,
            "(Fn [HeartReading] String)"
        );
        assert_eq!(
            result.type_annotations[1].schema,
            "(Fn [{:label String} Msg] UpdateResult)"
        );
        assert_eq!(result.type_annotations[2].schema, "Number");
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:bpm Number :time Number}] String)"
        );
        assert_eq!(
            result.forms[1].ty,
            "(Fn [{:label String} (Union {:kind Keyword} {:kind Keyword} {:kind Keyword :reading {:bpm Number :time Number}})] [{:label String} (Cmd (Union {:kind Keyword} {:kind Keyword} {:kind Keyword :reading {:bpm Number :time Number}}))])"
        );
        assert_eq!(result.forms[2].ty, "Number");
    }

    #[test]
    fn reports_non_exhaustive_union_match() {
        let result = check(
            "(type Msg (Union {:kind :start} {:kind :stop} {:kind :heart-rate :bpm Number}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  (match msg\n    {:kind :start} [state {:kind :none}]\n    {:kind :heart-rate :bpm bpm} [{:label (str bpm)} {:kind :none}]))",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .message
                    .contains("non-exhaustive match for union")
                    && diagnostic.message.contains("{:kind :stop}")
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn checks_command_continuation_messages_against_cmd_type() {
        let result = check(
            "(type Msg (Union {:kind :load} {:kind :loaded} {:kind :failed} {:kind :tick}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  (match msg\n    {:kind :load}\n      [state {:kind :batch\n              :commands [{:kind :storage/get :key \"hrweb\" :onSuccess :loaded :onError :failed}\n                         {:kind :timer/after :id \"refresh\" :ms 1000 :msg {:kind :tick}}]}]\n    _ [state {:kind :none}]))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn infers_mixed_browser_command_batches() {
        let result = check(
            "(type Msg (Union {:kind :loaded :value String} {:kind :failed :error String} {:kind :theme-loaded :value String} {:kind :auth-loaded :value Js}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  [state {:kind :batch\n          :commands [{:kind :browser/history-write-route :url \"/docs\" :op nil :definition nil}\n                     {:kind :http/request\n                      :url \"/openapi.json\"\n                      :response :text\n                      :toMessage (fn [response] {:kind :loaded :value (get response :body)})\n                      :onError :failed}\n                     {:kind :browser/theme-load :key \"theme\" :onSuccess :theme-loaded}\n                     {:kind :auth-storage/load :sourceUrl \"/docs\" :onSuccess :auth-loaded}]}])",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn infers_cmd_none_and_cmd_batch_helpers() {
        let result = check(
            "(type State {:count Number})\n\
             (type Msg (Union {:kind :load} {:kind :tick}))\n\
             (type UpdateResult [State (Cmd Msg)])\n\
             (ann update (Fn [State Msg] UpdateResult))\n\
             (defn update [state msg]\n  (match msg\n    {:kind :load}\n      [state (Cmd.batch [Cmd.none {:kind :timer/after :id \"tick\" :ms 1000 :msg {:kind :tick}}])]\n    _ [state Cmd.none]))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn infers_browser_command_constructor_helpers() {
        let result = check(
            "(type State {:ready Bool})\n\
             (type Rect {:x Number :y Number :width Number :height Number :top Number :right Number :bottom Number :left Number})\n\
             (type Info {:id String :deviceName String :connected Bool})\n\
             (type Msg (Union {:kind :start} {:kind :measured :left Number :width Number :value Rect} {:kind :measure-failed :error String} {:kind :connected :info Info} {:kind :heart-rate :bpm Number} {:kind :disconnected} {:kind :failed :error String}))\n\
             (type UpdateResult [State (Cmd Msg)])\n\
             (ann update (Fn [State Msg] UpdateResult))\n\
             (defn update [state msg]\n  [state (Cmd.batch [(Cmd.dom-ref/measure \"track\"\n                                                   (fn [rect] {:kind :measured\n                                                               :left rect.left\n                                                               :width rect.width\n                                                               :value {:x rect.x :y rect.y :width rect.width :height rect.height :top rect.top :right rect.right :bottom rect.bottom :left rect.left}})\n                                                   :measure-failed)\n                         (Cmd.bluetooth/connect-heart-rate \"hr\"\n                                                           {:filters [{:services [\"heart_rate\"]}]\n                                                            :optionalServices [\"heart_rate\"]}\n                                                           (Msg.mapper :connected :info)\n                                                           :heart-rate\n                                                           :disconnected\n                                                           :failed)\n                         (Cmd.simulation/heart-rate \"sim\"\n                                                    {:ms 1000 :min 90 :max 160 :jitter 3 :start 120 :deviceName \"Sim\"}\n                                                    (Msg.mapper :connected :info)\n                                                    :heart-rate\n                                                    :failed)])])",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn keeps_unannotated_cmd_none_update_results_as_tuples() {
        let result = check(
            "(defn update [state msg]\n  (match msg\n    {:kind :loaded :value value}\n      [(merge state {:loading? false :value value}) Cmd.none]\n    _ [state Cmd.none]))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(
            result.forms[0].ty.contains(" (Cmd "),
            "update result should keep [state cmd] tuple shape: {}",
            result.forms[0].ty
        );
    }

    #[test]
    fn rejects_unknown_command_kind_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :done}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :browser/do-it})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("unknown command kind :browser/do-it")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn rejects_abstract_command_category_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :done}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :http})",
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("unknown command kind :http")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn rejects_missing_required_command_fields_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :tick}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :timer/every :id \"clock\" :msg {:kind :tick}})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("timer/every command is missing a :ms field")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn rejects_value_command_without_success_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :loaded}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :storage/get :key \"heartRateExercise.log.v1\"})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("storage/get command is missing one of :onSuccess, :toMessage")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn rejects_malformed_batch_commands_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :tick}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :batch :commands \"not commands\"})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("batch :commands must be a vector of command records")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn rejects_wrong_command_field_types_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :tick} {:kind :loaded :value String} {:kind :changed :id String :media String :matches Bool}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper []\n  {:kind :batch\n   :commands [{:kind :timer/every :id \"clock\" :ms \"fast\" :msg {:kind :tick}}\n              {:kind :storage/get :key 42 :onSuccess :loaded}\n              {:kind :media-query/watch :id 7 :query \"(max-width: 700px)\" :onChange :changed}]})",
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected Number, found String")),
            "{:?}",
            result.diagnostics
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected String, found Number")),
            "{:?}",
            result.diagnostics
        );

        let result = check(
            "(type Msg (Union {:kind :loaded :value {:status Number :ok Bool :body {:entries (Vector {:id String})}}}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :http/request :url \"/api/log\" :method 42 :onSuccess :loaded})",
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected String, found Number")),
            "{:?}",
            result.diagnostics
        );

        let result = check(
            "(type Msg (Union {:kind :loaded :value {:status Number :ok Bool :body {:entries (Vector {:id String})}}}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :http/request :request {:url \"/api/log\" :method 42} :onSuccess :loaded})",
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected String, found Number")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn checks_http_request_record_url_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :loaded :value {:status Number :ok Bool :body {:entries (Vector {:id String})}}}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :http/request :request {:url \"/api/log\" :method \"GET\"} :onSuccess :loaded})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);

        let result = check(
            "(type Msg (Union {:kind :loaded :value {:status Number :ok Bool :body {:entries (Vector {:id String})}}}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :http/request :request {:method \"GET\"} :onSuccess :loaded})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("http/request command :request is missing a :url field")),
            "{:?}",
            result.diagnostics
        );

        let result = check(
            "(type Msg (Union {:kind :loaded :value {:status Number :ok Bool :body {:entries (Vector {:id String})}}}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :http/request :request {:url 42 :method \"GET\"} :onSuccess :loaded})",
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected String, found Number")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn checks_bluetooth_command_option_field_types_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :device :value {:name String}} {:kind :connected :value {:id String :deviceName String :connected Bool}} {:kind :rate :bpm Number}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper []\n  {:kind :batch\n   :commands [{:kind :bluetooth/request-device\n               :filters [{:services [\"heart_rate\"]}]\n               :optionalServices [\"heart_rate\"]\n               :onSuccess :device}\n              {:kind :bluetooth/connect-heart-rate\n               :id \"hr\"\n               :filters [{:services [\"heart_rate\"]}]\n               :optionalServices [\"heart_rate\"]\n               :service \"heart_rate\"\n               :characteristic \"heart_rate_measurement\"\n               :onSuccess :connected\n               :onReading :rate}]})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);

        let result = check(
            "(type Msg (Union {:kind :device :value {:name String}} {:kind :connected :value {:id String :deviceName String :connected Bool}} {:kind :rate :bpm Number}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper []\n  {:kind :batch\n   :commands [{:kind :bluetooth/request-device\n               :options \"not options\"\n               :onSuccess :device}\n              {:kind :bluetooth/connect-heart-rate\n               :id \"hr\"\n               :filters [{:services [42]}]\n               :optionalServices [42]\n               :service 42\n               :characteristic 42\n               :onSuccess :connected\n               :onReading :rate}]})",
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected {}, found String")),
            "{:?}",
            result.diagnostics
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected String, found Number")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn checks_simulation_command_field_types_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :connected :value {:id String :deviceName String :connected Bool}} {:kind :rate :bpm Number} {:kind :disconnected} {:kind :stopped :value {:id String}}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper []\n  {:kind :batch\n   :commands [{:kind :simulation/heart-rate\n               :id \"sim\"\n               :ms 1000\n               :min 120\n               :max 150\n               :jitter 3\n               :start 135\n               :deviceName \"Test simulator\"\n               :onSuccess :connected\n               :onReading :rate\n               :onDisconnected :disconnected}\n              {:kind :simulation/stop\n               :id \"sim\"\n               :onSuccess :stopped}]})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);

        let result = check(
            "(type Msg (Union {:kind :connected :value {:id String :deviceName String :connected Bool}} {:kind :rate :bpm Number}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :simulation/heart-rate :id 7 :ms \"fast\" :min \"low\" :max \"high\" :jitter \"small\" :start \"now\" :deviceName 42 :onSuccess :connected :onReading :rate})",
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected String, found Number")),
            "{:?}",
            result.diagnostics
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected Number, found String")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn rejects_conflicting_success_continuations_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :tick} {:kind :timestamp :value Number}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :time/now :msg {:kind :tick} :onSuccess :timestamp})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("time/now command has conflicting success continuations")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn rejects_unsupported_continuation_fields_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :tick} {:kind :cancelled}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :timer/after :id \"later\" :ms 100 :msg {:kind :tick} :onCancel :cancelled})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("timer/after command does not support :onCancel")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn rejects_structural_command_continuations_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :done} {:kind :failed :error String}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :none :msg {:kind :done}})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("none command does not support continuations :msg")),
            "{:?}",
            result.diagnostics
        );

        let result = check(
            "(type Msg (Union {:kind :done} {:kind :failed :error String}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :batch :commands [] :onError :failed})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("batch command does not support continuations :onError")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn rejects_change_watch_success_continuations_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :ready} {:kind :changed :id String :ref String :value {:x Number :y Number :width Number :height Number :top Number :right Number :bottom Number :left Number}}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :dom-ref/resize-watch :id \"chart\" :ref \"heart-chart\" :onChange :changed :onSuccess :ready})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic.message.contains(
                "dom-ref/resize-watch command dispatches changes through :onChange and does not support success continuations :onSuccess"
            )),
            "{:?}",
            result.diagnostics
        );

        let result = check(
            "(type Msg (Union {:kind :ready} {:kind :changed :id String :media String :matches Bool}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :media-query/watch :id \"mobile\" :query \"(max-width: 820px)\" :onChange :changed :msg {:kind :ready}})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic.message.contains(
                "media-query/watch command dispatches changes through :onChange and does not support success continuations :msg"
            )),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn rejects_payloadless_success_continuations_in_cmd_annotations() {
        let result = check(
            "(type Msg (Union {:kind :cancelled} {:kind :disconnected}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :timer/cancel :id \"clock\" :msg {:kind :cancelled}})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);

        let result = check(
            "(type Msg (Union {:kind :cancelled} {:kind :disconnected}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :timer/cancel :id \"clock\" :onSuccess :cancelled})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic.message.contains(
                "timer/cancel command has no success payload; use :msg for completion messages instead of :onSuccess"
            )),
            "{:?}",
            result.diagnostics
        );

        let result = check(
            "(type Msg (Union {:kind :cancelled} {:kind :disconnected}))\n\
             (ann helper (Fn [] (Cmd Msg)))\n\
             (defn helper [] {:kind :bluetooth/disconnect :id \"hr\" :toMessage (fn [value] {:kind :disconnected})})",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| diagnostic.message.contains(
                "bluetooth/disconnect command has no success payload; use :msg for completion messages instead of :toMessage"
            )),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn checks_command_to_message_against_cmd_type() {
        let result = check(
            "(type Entry {:id String})\n\
             (type Msg (Union {:kind :load} {:kind :started :timestamp Number} {:kind :log-loaded :payload {:entries (Vector Entry)}}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  (match msg\n    {:kind :load}\n      [state {:kind :batch\n              :commands [{:kind :time/now\n                          :toMessage (fn [timestamp] {:kind :started :timestamp timestamp})}\n                         {:kind :file/read-selected\n                          :ref \"import-file\"\n                          :format :json\n                          :toMessage (fn [payload] {:kind :log-loaded :payload payload})}]}]\n    _ [state {:kind :none}]))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn reports_command_to_message_return_mismatches() {
        let result = check(
            "(type Msg (Union {:kind :start} {:kind :started :timestamp String}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  [state {:kind :time/now\n          :toMessage (fn [timestamp] {:kind :started :timestamp timestamp})}])",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .message
                    .contains("command message :toMessage return")
                    && diagnostic.message.contains(":timestamp Number")
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn reports_command_continuation_messages_outside_cmd_type() {
        let result = check(
            "(type Msg (Union {:kind :load} {:kind :loaded}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  [state {:kind :storage/get :key \"hrweb\" :onSuccess :missing}])",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .message
                    .contains("command message :onSuccess :missing")
                    && diagnostic.message.contains(":kind :missing")
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn checks_command_continuation_payload_shapes() {
        let result = check(
            "(type Msg (Union {:kind :start} {:kind :started :value Number} {:kind :connected :value {:id String :deviceName String :connected Bool}} {:kind :rate :bpm Number} {:kind :frame :timestamp Number} {:kind :reset :value {:key String}} {:kind :import-opened :value {:ref String}} {:kind :log-imported :value {:entries (Vector {:id String})}}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  (match msg\n    {:kind :start}\n      [state {:kind :batch\n              :commands [{:kind :time/now :onSuccess :started}\n                         {:kind :bluetooth/connect-heart-rate\n                          :id \"hr\"\n                          :filters [{:services [\"heart_rate\"]}]\n                          :onSuccess :connected\n                          :onReading :rate}\n                         {:kind :animation/frame :id \"hold\" :onFrame :frame}\n                         {:kind :storage/remove :key \"heartRateExercise.log.v1\" :onSuccess :reset}\n                         {:kind :dom-ref/click :ref \"import-file\" :onSuccess :import-opened}\n                         {:kind :file/read-selected :ref \"import-file\" :format :json :onSuccess :log-imported}]}]\n    _ [state {:kind :none}]))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn checks_command_success_payload_shapes_for_lifecycle_commands() {
        let result = check(
            "(type Msg (Union {:kind :start} {:kind :frame-ready :value {:id String}} {:kind :frame :id String :timestamp Number :value Number} {:kind :cancelled :value {:id String}} {:kind :resize-stopped :value {:id String}} {:kind :window-ready :value {:id String :type String}} {:kind :event :id String} {:kind :window-stopped :value {:id String}} {:kind :media-stopped :value {:id String}}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  [state {:kind :batch\n          :commands [{:kind :animation/frame :id \"hold\" :onFrame :frame :onSuccess :frame-ready}\n                     {:kind :animation/cancel :id \"hold\" :onSuccess :cancelled}\n                     {:kind :dom-ref/resize-unwatch :id \"chart\" :onSuccess :resize-stopped}\n                     {:kind :window/event-watch :id \"dev\" :type \"keydown\" :onEvent :event :onSuccess :window-ready}\n                     {:kind :window/event-unwatch :id \"dev\" :onSuccess :window-stopped}\n                     {:kind :media-query/unwatch :id \"mobile\" :onSuccess :media-stopped}]}])",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn reports_command_continuation_payload_mismatches() {
        let result = check(
            "(type Msg (Union {:kind :start} {:kind :started :value String} {:kind :rate :value Number} {:kind :tick :value Number}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  [state {:kind :batch\n          :commands [{:kind :time/now :onSuccess :started}\n                     {:kind :bluetooth/connect-heart-rate\n                      :id \"hr\"\n                      :filters [{:services [\"heart_rate\"]}]\n                      :onSuccess :started\n                      :onReading :rate}\n                     {:kind :timer/after :id \"tick\" :ms 1000 :msg {:kind :tick}}]}])",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .message
                    .contains("command message :onSuccess :started")
                    && diagnostic.message.contains(":value Number")
            }),
            "{:?}",
            result.diagnostics
        );
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .message
                    .contains("command message :onReading :rate")
                    && diagnostic.message.contains(":bpm Number")
            }),
            "{:?}",
            result.diagnostics
        );
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.message.contains("command message :msg")
                    && diagnostic.message.contains(":tick")
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn reports_command_success_payload_mismatches_for_lifecycle_commands() {
        let result = check(
            "(type Msg (Union {:kind :start} {:kind :frame-ready :value {:id Number}} {:kind :frame :id String :timestamp Number :value Number} {:kind :cancelled :value {:id Number}} {:kind :resize-stopped :value {:id Number}} {:kind :window-ready :value {:id Number :type Number}} {:kind :event :id String} {:kind :window-stopped :value {:id Number}} {:kind :media-stopped :value {:id Number}}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  [state {:kind :batch\n          :commands [{:kind :animation/frame :id \"hold\" :onFrame :frame :onSuccess :frame-ready}\n                     {:kind :animation/cancel :id \"hold\" :onSuccess :cancelled}\n                     {:kind :dom-ref/resize-unwatch :id \"chart\" :onSuccess :resize-stopped}\n                     {:kind :window/event-watch :id \"dev\" :type \"keydown\" :onEvent :event :onSuccess :window-ready}\n                     {:kind :window/event-unwatch :id \"dev\" :onSuccess :window-stopped}\n                     {:kind :media-query/unwatch :id \"mobile\" :onSuccess :media-stopped}]}])",
        );

        for tag in [
            ":frame-ready",
            ":cancelled",
            ":resize-stopped",
            ":window-ready",
            ":window-stopped",
            ":media-stopped",
        ] {
            assert!(
                result.diagnostics.iter().any(|diagnostic| {
                    diagnostic.message.contains("command message :onSuccess")
                        && diagnostic.message.contains(tag)
                        && diagnostic.message.contains(":id String")
                }),
                "missing payload mismatch for {}: {:?}",
                tag,
                result.diagnostics
            );
        }
    }

    #[test]
    fn reports_branch_command_payload_mismatches_after_join() {
        let result = check(
            "(type Msg (Union {:kind :start} {:kind :started :value String}))\n\
             (type UpdateResult [{:label String} (Cmd Msg)])\n\
             (ann update (Fn [{:label String} Msg] UpdateResult))\n\
             (defn update [state msg]\n  (match msg\n    {:kind :start} [state {:kind :time/now :onSuccess :started}]\n    _ [state {:kind :none}]))",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .message
                    .contains("command message :onSuccess :started")
                    && diagnostic.message.contains(":value Number")
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn infers_task_perform_command_messages() {
        let result = check(
            "(type Spec {:title String})\n\
             (type Msg (Union {:kind :load} {:kind :loaded :value Spec} {:kind :failed :error String}))\n\
             (ann decode-spec (Fn [String] (Task String Spec)))\n\
             (defn decode-spec [text]\n  (Task.succeed {:title text}))\n\
             (ann load-spec-task (Fn [String] (Task String Spec)))\n\
             (defn load-spec-task [url]\n  (Task.and-then (Http.get-text url) decode-spec))\n\
             (ann load-spec-command (Fn [String] (Cmd Msg)))\n\
             (defn load-spec-command [url]\n  (Task.perform load-spec-task\n                url\n                (fn [spec] {:kind :loaded :value spec})\n                (fn [error] {:kind :failed :error error})))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn rejects_task_perform_messages_outside_cmd_message_type() {
        let result = check(
            "(type Msg (Union {:kind :loaded :value {:title String}}))\n\
             (ann load-spec-command (Fn [String] (Cmd Msg)))\n\
             (defn load-spec-command [url]\n  (Task.perform (Http.get-text url)\n                (fn [text] {:kind :loaded :value {:title text}})\n                (fn [error] {:kind :failed :error error})))",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.message.contains("command message")
                    && diagnostic.message.contains(":failed")
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn imported_command_helpers_keep_cmd_message_types() {
        let helper = check(
            "(type Rect {:x Number :y Number :width Number :height Number :top Number :right Number :bottom Number :left Number})\n\
             (type ChartMsg (Union {:kind :chart-resized :id String :ref String :x Number :y Number :width Number :height Number :top Number :right Number :bottom Number :left Number :value Rect} {:kind :chart-error :error String}))\n\
             (type ChartState {:live Bool})\n\
             (ann chart-command (Fn [ChartState] (Cmd ChartMsg)))\n\
             (defn chart-command [state]\n  (if state.live\n      {:kind :dom-ref/resize-watch :id \"chart\" :ref \"chart\" :onChange :chart-resized :onError :chart-error}\n      {:kind :none}))",
        );
        assert!(helper.diagnostics.is_empty(), "{:?}", helper.diagnostics);
        let binding = imported_binding(&helper, "chart-command");

        let result = check_with_imports(
            "(import \"./chart.clsk\" [chart-command])\n\
             (type Rect {:x Number :y Number :width Number :height Number :top Number :right Number :bottom Number :left Number})\n\
             (type AppMsg (Union {:kind :start} {:kind :chart-resized :id String :ref String :x Number :y Number :width Number :height Number :top Number :right Number :bottom Number :left Number :value Rect} {:kind :chart-error :error String}))\n\
             (type UpdateResult [{:live Bool :label String} (Cmd AppMsg)])\n\
             (ann update (Fn [{:live Bool :label String} AppMsg] UpdateResult))\n\
             (defn update [state msg]\n  [state {:kind :batch :commands [(chart-command state)]}])",
            &[binding],
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn annotation_type_variables_keep_command_helpers_polymorphic() {
        let helper = check(
            "(ann silent-command (Fn [String payload] (Cmd msg)))\n\
             (defn silent-command [key payload]\n  Cmd.none)",
        );
        assert!(helper.diagnostics.is_empty(), "{:?}", helper.diagnostics);
        let binding = imported_binding(&helper, "silent-command");

        let result = check_with_imports(
            "(import \"./helpers.clsk\" [silent-command])\n\
             (def one (silent-command \"one\" {:id \"alpha\"}))\n\
             (def two (silent-command \"two\" {:id \"beta\"}))",
            &[binding],
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn unannotated_update_joins_imported_cmd_with_plain_command_records() {
        let helper = check(
            "(type HelperMsg (Union {:kind :helper-done}))\n\
             (ann helper-command (Fn [] (Cmd HelperMsg)))\n\
             (defn helper-command []\n  {:kind :timer/cancel :id \"helper\" :msg {:kind :helper-done}})",
        );
        assert!(helper.diagnostics.is_empty(), "{:?}", helper.diagnostics);
        let binding = imported_binding(&helper, "helper-command");

        let result = check_with_imports(
            "(import \"./helpers.clsk\" [helper-command])\n\
             (defn update [state msg]\n\
               (match msg\n\
                 {:kind :start} [state (helper-command)]\n\
                 _ [state {:kind :file/download :name \"x.txt\" :content \"\" :mime \"text/plain\" :msg {:kind :noop}}]))",
            &[binding],
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.forms[0].ty.contains("(Cmd (Union"));
    }

    #[test]
    fn imported_command_helpers_report_missing_app_messages() {
        let helper = check(
            "(type Rect {:x Number :y Number :width Number :height Number :top Number :right Number :bottom Number :left Number})\n\
             (type ChartMsg (Union {:kind :chart-resized :id String :ref String :x Number :y Number :width Number :height Number :top Number :right Number :bottom Number :left Number :value Rect} {:kind :chart-error :error String}))\n\
             (type ChartState {:live Bool})\n\
             (ann chart-command (Fn [ChartState] (Cmd ChartMsg)))\n\
             (defn chart-command [state]\n  (if state.live\n      {:kind :dom-ref/resize-watch :id \"chart\" :ref \"chart\" :onChange :chart-resized :onError :chart-error}\n      {:kind :none}))",
        );
        assert!(helper.diagnostics.is_empty(), "{:?}", helper.diagnostics);
        let binding = imported_binding(&helper, "chart-command");

        let result = check_with_imports(
            "(import \"./chart.clsk\" [chart-command])\n\
             (type AppMsg (Union {:kind :start} {:kind :chart-error :error String}))\n\
             (type UpdateResult [{:live Bool} (Cmd AppMsg)])\n\
             (ann update (Fn [{:live Bool} AppMsg] UpdateResult))\n\
             (defn update [state msg]\n  [state {:kind :batch :commands [(chart-command state)]}])",
            &[binding],
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .message
                    .contains("command message :commands item")
                    && diagnostic.message.contains(":chart-resized")
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn reports_api_type_annotation_mismatches() {
        let result = check(
            "(ann duration-label (Fn [Number] Number))\n\
             (defn duration-label [ms] (str ms))\n\
             (ann sample-count String)\n\
             (def sample-count 2)",
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected Number, found String"))
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected String, found Number"))
        );
    }

    #[test]
    fn reports_orphan_and_invalid_cmd_api_type_annotations() {
        let result = check(
            "(type Msg (Union {:kind :start} {:kind :stop}))\n\
             (ann missing Number)\n\
             (ann update-command (Cmd Msg))\n\
             (def update-command {:key \"missing-kind\"})",
        );

        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("does not match any def or defn")
        }));
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.message.contains("must include :kind") })
        );
    }

    #[test]
    fn reports_branch_mismatch() {
        let result = check("(if true 1 \"no\")");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("type mismatch"))
        );
    }

    #[test]
    fn templates_have_html_type() {
        let result = check("(defn view [label] #html <button>{label}</button>)");

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [t0] Html)");
    }

    #[test]
    fn infers_render_to_string_for_view_functions() {
        let result = check(
            "(defn view [state]\n  #html <article>{state.title}</article>)\n\
             (render-to-string view {:title \"Pulse\"})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[1].ty, "String");
        assert!(result.forms[0].ty.ends_with("] Html)"));
    }

    #[test]
    fn infers_template_attrs_and_text_reads() {
        let result = check(
            "(defn view [state]\n  #html <button class={state.buttonClass} disabled={not state.connected?}>{state.label}</button>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:buttonClass t1 :connected? Bool :label t3}] Html)"
        );
    }

    #[test]
    fn accepts_and_types_template_dynamic_refs() {
        let result = check(
            "(defn view [state]\n  #html <section ref={state.panelRef}>\n          <canvas ref={(if state.live? \"heart-chart\" nil)}></canvas>\n          <input ref={:import-file}></input>\n          {state.label}\n        </section>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.forms[0].ty.contains(":panelRef String"));
        assert!(result.forms[0].ty.contains(":live? Bool"));
        assert!(result.forms[0].ty.contains(":label"));
        assert!(result.forms[0].ty.ends_with("] Html)"));
    }

    #[test]
    fn rejects_template_refs_with_invalid_shapes() {
        let result = check(
            "(defn bad-number [] #html <div ref={42}></div>)\n\
             (defn bad-bool [] #html <div ref={true}></div>)\n\
             (defn bad-record [] #html <div ref={{:id \"panel\"}}></div>)",
        );

        let ref_errors = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic
                    .message
                    .contains("ref attribute expects a string")
            })
            .count();
        assert_eq!(ref_errors, 3, "{:?}", result.diagnostics);
    }

    #[test]
    fn accepts_static_template_special_attrs() {
        let result = check(
            "(defn view []\n  #html <section ref=\"panel\" class=\"\" style=\"color: red\">\n          <button disabled></button>\n          <input checked=\"checked\"></input>\n          <option selected=\"\"></option>\n        </section>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn rejects_bare_special_template_attrs() {
        let result = check(
            "(defn view []\n  #html <section>\n          <div ref></div>\n          <div class></div>\n          <div style></div>\n        </section>)",
        );

        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("ref attribute requires a value")
        }));
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("class attribute requires a value")
        }));
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("style attribute requires a value")
        }));
    }

    #[test]
    fn rejects_misleading_static_boolean_attrs() {
        let result = check(
            "(defn view []\n  #html <section>\n          <button disabled=\"false\"></button>\n          <input checked=\"true\"></input>\n          <select multiple=\"yes\"></select>\n        </section>)",
        );

        let boolean_errors = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.message.contains("boolean attribute"))
            .count();
        assert_eq!(boolean_errors, 3, "{:?}", result.diagnostics);
    }

    #[test]
    fn accepts_template_structured_class_values() {
        let result = check(
            "(defn view [state]\n  #html <section class={[\"panel\"\n                                  :interactive\n                                  {:active state.active? :empty nil}\n                                  (hash-map :selected state.selected? :muted false)\n                                  #{:ready :wide}]}>\n          {state.label}\n        </section>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.forms[0].ty.contains(":active? Bool"));
        assert!(result.forms[0].ty.contains(":selected? Bool"));
        assert!(result.forms[0].ty.contains(":label"));
        assert!(result.forms[0].ty.ends_with("] Html)"));
    }

    #[test]
    fn rejects_template_class_values_with_invalid_shapes() {
        let result = check(
            "(defn bad-number [] #html <div class={42}></div>)\n\
             (defn bad-record [] #html <div class={{:active \"yes\"}}></div>)\n\
             (defn bad-vector [] #html <div class={[1]}></div>)\n\
             (defn bad-map [] #html <div class={(hash-map 1 true)}></div>)",
        );

        let class_errors = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic
                    .message
                    .contains("class attribute expects a CSS class")
            })
            .count();
        assert_eq!(class_errors, 4, "{:?}", result.diagnostics);
    }

    #[test]
    fn accepts_template_style_records_and_persistent_maps() {
        let result = check(
            "(defn view [state]\n  #html <section style={{:width (str state.percent \"%\")\n                                  :opacity state.opacity\n                                  :boxShadow nil}}\n                  data-count={state.count}>\n          <span style={(hash-map :background state.color :--accent state.color)}>{state.label}</span>\n        </section>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.forms[0].ty.contains(":color"));
        assert!(result.forms[0].ty.contains(":count"));
        assert!(result.forms[0].ty.contains(":label"));
        assert!(result.forms[0].ty.contains(":opacity"));
        assert!(result.forms[0].ty.contains(":percent"));
        assert!(result.forms[0].ty.ends_with("] Html)"));
    }

    #[test]
    fn rejects_template_style_values_with_invalid_shapes() {
        let result = check(
            "(defn bad-number [] #html <div style={42}></div>)\n\
             (defn bad-record [] #html <div style={{:width [1 2]}}></div>)\n\
             (defn bad-map-key [] #html <div style={(hash-map 1 \"red\")}></div>)",
        );

        let style_errors = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic
                    .message
                    .contains("style attribute expects a CSS string")
            })
            .count();
        assert_eq!(style_errors, 3, "{:?}", result.diagnostics);
    }

    #[test]
    fn infers_template_loop_item_reads() {
        let result = check(
            "(defn view [state]\n  #html <section>{(for [entry state.entries index :key entry.id] #html <article data-index={index}>{(str index entry.label)}</article>)}</section>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:entries (Vector {:id t3 :label t4})}] Html)"
        );
    }

    #[test]
    fn infers_template_component_call_props() {
        let result = check(
            "(defn summary-card [summary]\n  #html <article>{summary.label}<span>{summary.value}</span></article>)\n\
             (defn view [state]\n  #html <section>{(summary-card state.summary)}</section>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[1].ty,
            "(Fn [{:summary {:label t1 :value t2}}] Html)"
        );
    }

    #[test]
    fn infers_template_event_payload_reads() {
        let result = check(
            "(defn view [state]\n  #html <input value={state.draft} on:input={{:kind :draft :value event.currentTarget.value}} on:keydown={(if (= event.key \"Enter\") {:kind :confirm} {:kind :none})}></input>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [{:draft t1}] Html)");
    }

    #[test]
    fn infers_template_checkbox_event_payload_reads() {
        let result = check(
            "(defn view [state]\n  #html <input type=\"checkbox\" checked={state.enabled?} on:change={{:kind :toggle :checked event.currentTarget.checked}}></input>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [{:enabled? Bool}] Html)");
    }

    #[test]
    fn infers_template_event_prevent_default_values() {
        let result = check(
            "(defn view [state]\n  #html <button on:keydown={(if (= event.key \"ArrowLeft\") (Event.prevent {:kind :step :value -1}) {:kind :none})}>{state.label}</button>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [{:label t1}] Html)");
    }

    #[test]
    fn infers_state_derived_subscriptions() {
        let result = check(
            "(type State {:running? Bool})\n\
             (type Msg (Union {:kind :tick} {:kind :media-changed :id String :media String :matches Bool}))\n\
             (ann subscriptions (Fn [State] (Sub Msg)))\n\
             (defn subscriptions [state]\n  (Sub.batch [(if state.running?\n                   (Sub.timer/every \"clock\" 250 {:kind :tick})\n                   Sub.none)\n              (Sub.media-query \"mobile\" \"(max-width: 700px)\" :media-changed)]))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms.len(), 1);
        assert!(result.forms[0].ty.contains("(Sub (Union"));
    }

    #[test]
    fn rejects_subscription_messages_outside_sub_message_type() {
        let result = check(
            "(type State {:running? Bool})\n\
             (type Msg (Union {:kind :tick}))\n\
             (ann subscriptions (Fn [State] (Sub Msg)))\n\
             (defn subscriptions [state]\n  (Sub.media-query \"mobile\" \"(max-width: 700px)\" :media-changed))",
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("subscription message")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn infers_scoped_update_subscriptions_and_view() {
        let result = check(
            "(type ChildState {:count Number})\n\
             (type ChildMsg (Union {:kind :inc} {:kind :tick}))\n\
             (type AppState {:log ChildState :route String})\n\
             (type AppMsg (Union {:kind :log :msg ChildMsg} {:kind :route-changed :route String}))\n\
             (type ChildResult [ChildState (Cmd ChildMsg)])\n\
             (type AppResult [AppState (Cmd AppMsg)])\n\
             (ann child-update (Fn [ChildState ChildMsg] ChildResult))\n\
             (defn child-update [state msg]\n  [(assoc state :count (+ state.count 1)) {:kind :none}])\n\
             (ann child-subscriptions (Fn [ChildState] (Sub ChildMsg)))\n\
             (defn child-subscriptions [state]\n  (if (> state.count 0) (Sub.timer/every \"child\" 100 {:kind :tick}) Sub.none))\n\
             (defn child-view [state]\n  #html <button>{state.count}</button>)\n\
             (ann update (Fn [AppState AppMsg] AppResult))\n\
             (defn update [state msg]\n  (match msg\n    {:kind :log :msg child-msg}\n      (scope-update state :log child-msg child-update :log)\n    _ [state {:kind :none}]))\n\
             (ann subscriptions (Fn [AppState] (Sub AppMsg)))\n\
             (defn subscriptions [state]\n  (scope-subscriptions state.log child-subscriptions :log))\n\
             (defn view [state]\n  #html <main>{(scope-view :log child-view state.log)}</main>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn infers_closkell_test_api_forms() {
        let result = check(
            "(import \"closkell/test\" [describe test expect= expect-not= expect-ok expect-err expect-some expect-match expect-throws])\n\
             \n\
             (describe \"math\"\n\
               (test \"assertions\"\n\
                 (expect= (+ 1 1) 2)\n\
                 (expect-not= (+ 1 1) 3)\n\
                 (expect-ok true)\n\
                 (expect-err (err \"bad\"))\n\
                 (expect-some (find [1 2 3] (fn [value] (= value 2))))\n\
                 (expect-match {:kind :loaded :value 42 :meta {:source \"cache\"}}\n\
                               {:kind :loaded :meta {:source \"cache\"}})\n\
                 (expect-throws (fn [] (fail \"boom\")) \"boom\")))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn checks_template_event_messages_against_update_msg_type() {
        let result = check(
            "(type State {:label String})\n\
             (type Msg (Union {:kind :select-tab :view String} {:kind :draft :text String}))\n\
             (type UpdateResult [State (Cmd Msg)])\n\
             (ann update (Fn [State Msg] UpdateResult))\n\
             (defn update [state msg] [state {:kind :none}])\n\
             (defn tab-button [view]\n  #html <button on:click={{:kind :select-tab :view view}}>Tab</button>)\n\
             (defn view [state]\n  #html <main>{(tab-button \"log\")}<input value={state.label} on:input={{:kind :draft :text event.currentTarget.value}}></input></main>)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn reports_template_event_messages_outside_update_msg_type() {
        let result = check(
            "(type State {})\n\
             (type Msg (Union {:kind :start} {:kind :stop}))\n\
             (type UpdateResult [State (Cmd Msg)])\n\
             (ann update (Fn [State Msg] UpdateResult))\n\
             (defn update [state msg] [state {:kind :none}])\n\
             (defn view [state]\n  #html <button on:click={{:kind :strat}}>Start</button>)",
        );

        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .message
                    .contains("template event on:click message")
                    && diagnostic.message.contains(":strat")
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn reports_event_payload_outside_event_attr() {
        let result = check("(defn view [state]\n  #html <span>{event.currentTarget.value}</span>)");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("unknown symbol `event`"))
        );
    }

    #[test]
    fn reports_template_conditional_non_bool() {
        let result = check(
            "(defn view [state]\n  #html <section>{(if \"yes\" #html <strong>Yes</strong> #html <em>No</em>)}</section>)",
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("type mismatch"))
        );
    }

    #[test]
    fn infers_defn_with_structural_record_reads() {
        let result = check("(defn in-zone? [zone bpm] (and (>= bpm zone.min) (<= bpm zone.max)))");

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:max Number :min Number} Number] Bool)"
        );
    }

    #[test]
    fn infers_option_from_nil_branch() {
        let result = check(
            "(defn zone2-adherence [entry zone2-ms]\n  (if (or (empty? entry.readings) (<= entry.durationMs 0))\n      nil\n      (round (* (/ zone2-ms entry.durationMs) 100))))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:durationMs Number :readings (Vector t3)} Number] (Option Number))"
        );
    }

    #[test]
    fn infers_record_literals() {
        let result = check("(def sample {:durationMs 60000 :readings [{:bpm 120 :time 0}]})");

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "{:durationMs Number :readings (Vector {:bpm Number :time Number})}"
        );
    }

    #[test]
    fn infers_let_destructuring_patterns() {
        let result = check(
            "(defn summarize [payload]\n\
                 (let [{:reading {:bpm bpm}\n\
                      :samples (cons first rest)} payload\n\
                     (cons second tail) rest]\n\
                 {:bpm (+ bpm 0)\n\
                  :delta (- second first)\n\
                  :tailCount (count tail)}))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:reading {:bpm Number} :samples (List Number)}] {:bpm Number :delta Number :tailCount Number})"
        );
    }

    #[test]
    fn infers_fn_parameter_destructuring_patterns() {
        let result = check(
            "(def summaries\n\
               (map [{:reading {:bpm 142} :samples (list 100 136 150)}]\n\
                    (fn [{:reading {:bpm bpm}\n\
                          :samples (cons head rest)}]\n\
                      {:bpm (+ bpm 0)\n\
                       :delta (- (first rest) head)\n\
                       :tailCount (count rest)})))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Vector {:bpm Number :delta Number :tailCount Number})"
        );
    }

    #[test]
    fn infers_defn_parameter_destructuring_patterns() {
        let result = check(
            "(defn summarize [{:reading {:bpm bpm}\n\
                               :samples (cons head rest)}]\n\
               {:bpm (+ bpm 0)\n\
                :delta (- (first rest) head)\n\
                :tailCount (count rest)})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:reading {:bpm Number} :samples (List Number)}] {:bpm Number :delta Number :tailCount Number})"
        );
    }

    #[test]
    fn rejects_template_defn_parameter_destructuring() {
        let result = check("(defn view [{:label label}] #html <p>{label}</p>)");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("fn parameter must be a symbol")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn infers_update_tuple_return() {
        let result = check(
            "(defn update [state msg]\n  (if (= msg :start)\n      [{:connected? true :label \"Pause\"} {:kind :none}]\n      [state {:kind :none}]))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:connected? Bool :label String} Keyword] [{:connected? Bool :label String} {:kind Keyword}])"
        );
    }

    #[test]
    fn infers_keyword_match_update() {
        let result = check(
            "(defn update [state msg]\n  (match msg\n    :start [{:label \"Pause\"} {:kind :none}]\n    _ [state {:kind :none}]))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:label String} Keyword] [{:label String} {:kind Keyword}])"
        );
    }

    #[test]
    fn infers_record_pattern_bindings() {
        let result = check(
            "(defn update [state msg]\n  (match msg\n    {:kind :rate :bpm bpm} [{:latest bpm} {:kind :none}]\n    _ [state {:kind :none}]))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [{:latest t3} {:bpm t3 :kind Keyword}] [{:latest t3} {:kind Keyword}])"
        );
    }

    #[test]
    fn infers_as_pattern_bindings() {
        let result = check(
            "(defn normalize [msg]\n  (match msg\n    (as {:kind :rate :bpm bpm} whole) (assoc whole :bpm (+ bpm 1))\n    _ msg))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(
            result.forms[0].ty.contains(":bpm Number")
                && result.forms[0].ty.contains(":kind Keyword"),
            "{}",
            result.forms[0].ty
        );
    }

    #[test]
    fn infers_collection_primitives_for_metrics() {
        let result = check(
            "(defn total [readings]\n  (reduce-indexed readings 0 (fn [sum reading index] (+ sum reading.bpm))))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [(Vector {:bpm Number})] Number)");
    }

    #[test]
    fn infers_value_equality_and_identity_predicates() {
        let result = check(
            "(def equal-records (= {:items [1 2] :tag :ok} {:tag :ok :items [1 2]}))\n\
             (def equal-symbols (= :ready :ready))\n\
             (def shared-identity (let [value {:items [1 2]}] (identical? value value)))\n\
             (def distinct-identity (identical? {:items [1 2]} {:items [1 2]}))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "Bool");
        assert_eq!(result.forms[1].ty, "Bool");
        assert_eq!(result.forms[2].ty, "Bool");
        assert_eq!(result.forms[3].ty, "Bool");
    }

    #[test]
    fn infers_collection_transforms_for_log_views() {
        let result = check(
            "(defn visible [entries]\n  (filter entries (fn [entry] (not (some? entry.hiddenAt)))))\n\
             (defn bars [entries]\n  (map (take-last (sort-by entries (fn [entry] entry.stoppedAt)) 2)\n       (fn [entry] {:label entry.id :value entry.durationMs})))\n\
             (defn ranked [entries]\n  (map-indexed (sort-by-desc entries (fn [entry] entry.stoppedAt))\n    (fn [entry index] {:id entry.id :rank (+ index 1)})))\n\
             (defn all-typed? [entries]\n  (every? entries (fn [entry] (some? entry.exerciseType))))\n\
             (defn has-selected? [entries selected-id]\n  (any? entries (fn [entry] (= entry.id selected-id))))\n\
             (defn by-newest [entries]\n  (sort-with entries (fn [first second] (- second.stoppedAt first.stoppedAt))))\n\
             (defn by-label [labels]\n  (sort-with labels (fn [first second] (locale-compare first second))))\n\
             (defn page [entries] (slice (sort-by-desc entries (fn [entry] entry.stoppedAt)) 0 2))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms.len(), 8);
        assert!(result.forms[0].ty.contains("(Vector {:hiddenAt"));
        assert!(result.forms[1].ty.contains("(Vector {:label"));
        assert!(result.forms[1].ty.contains(":value"));
        assert!(result.forms[2].ty.contains(":rank Number"));
        assert!(
            result.forms[3]
                .ty
                .starts_with("(Fn [(Vector {:exerciseType")
        );
        assert!(result.forms[3].ty.ends_with("})] Bool)"));
        assert!(result.forms[4].ty.starts_with("(Fn [(Vector {:id "));
        assert!(result.forms[4].ty.ends_with("] Bool)"));
        assert!(result.forms[5].ty.contains("(Vector {:stoppedAt Number})"));
        assert_eq!(result.forms[6].ty, "(Fn [(Vector String)] (Vector String))");
        assert!(result.forms[7].ty.contains("(Vector"));
    }

    #[test]
    fn infers_vector_edges_for_zone_boundaries() {
        let result = check(
            "(defn zone-edges [zones]\n\
               (let [first-zone (first zones)\n\
                     second-zone (second zones)\n\
                     last-zone (last zones)\n\
                     draggable (drop-last zones)]\n\
                 {:firstMin (+ first-zone.min 0)\n\
                  :secondMin (+ second-zone.min 0)\n\
                  :lastMax (+ last-zone.max 0)\n\
                  :dragCount (count draggable)}))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.forms[0].ty.contains("(Vector"));
        assert!(result.forms[0].ty.contains(":firstMin Number"));
        assert!(result.forms[0].ty.contains(":secondMin Number"));
        assert!(result.forms[0].ty.contains(":lastMax Number"));
        assert!(result.forms[0].ty.contains(":dragCount Number"));
    }

    #[test]
    fn infers_vector_conj_for_log_updates() {
        let result = check(
            "(defn append-reading [readings reading]\n  (conj readings reading))\n\
             (def sample (append-reading [{:bpm 120 :time 0}] {:bpm 130 :time 1000}))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.forms[0].ty.contains("(Vector"));
        assert!(
            result.forms[1]
                .ty
                .contains("(Vector {:bpm Number :time Number})")
        );
    }

    #[test]
    fn infers_list_operations_for_persistent_sequences() {
        let result = check(
            "(def base (list 2 3))\n\
             (def prefixed (cons 1 base))\n\
             (def tail (rest prefixed))\n\
             (def appended (conj prefixed 4))\n\
             (def summary {:list (list? tail)\n\
                           :count (count tail)\n\
                           :first (+ (first tail) 0)\n\
                           :last (+ (last tail) 0)})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(List Number)");
        assert_eq!(result.forms[1].ty, "(List Number)");
        assert_eq!(result.forms[2].ty, "(List Number)");
        assert_eq!(result.forms[3].ty, "(List Number)");
        assert!(result.forms[4].ty.contains(":list Bool"));
        assert!(result.forms[4].ty.contains(":count Number"));
        assert!(result.forms[4].ty.contains(":first Number"));
        assert!(result.forms[4].ty.contains(":last Number"));
    }

    #[test]
    fn infers_list_match_pattern_bindings() {
        let result = check(
            "(type Samples (List Number))\n\
             (ann delta (Fn [Samples] Number))\n\
             (defn delta [samples]\n  (match samples\n    (list first second _) (- second first)\n    _ 0))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [(List Number)] Number)");
    }

    #[test]
    fn infers_cons_match_pattern_bindings() {
        let result = check(
            "(type Samples (List Number))\n\
             (ann head-plus-tail-count (Fn [Samples] Number))\n\
             (defn head-plus-tail-count [samples]\n\
               (match samples\n\
                 (cons head tail) (+ head (count tail))\n\
                 (list) 0))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [(List Number)] Number)");
    }

    #[test]
    fn infers_set_operations_for_workout_tags() {
        let result = check(
            "(def tags (set \"steady\" \"zone2\" \"steady\"))\n\
             (def literal #{\"warmup\" \"recovery\"})\n\
             (def expanded (conj tags \"tempo\"))\n\
             (def trimmed (disj expanded \"steady\"))\n\
             (def ordered (set-values trimmed))\n\
             (def collected (reduce [{:tag \"LISS\"} {:tag \"Strength\"}]\n                          (set)\n                          (fn [items entry] (conj items entry.tag))))\n\
             (def summary {:hasZone2 (contains? trimmed \"zone2\")\n\
                           :count (count trimmed)\n\
                           :empty (empty? trimmed)\n\
                           :set (set? trimmed)})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Set String)");
        assert_eq!(result.forms[1].ty, "(Set String)");
        assert_eq!(result.forms[2].ty, "(Set String)");
        assert_eq!(result.forms[3].ty, "(Set String)");
        assert_eq!(result.forms[4].ty, "(Vector String)");
        assert_eq!(result.forms[5].ty, "(Set String)");
        assert!(result.forms[6].ty.contains(":hasZone2 Bool"));
        assert!(result.forms[6].ty.contains(":count Number"));
        assert!(result.forms[6].ty.contains(":empty Bool"));
        assert!(result.forms[6].ty.contains(":set Bool"));
    }

    #[test]
    fn infers_map_operations_for_metric_registry() {
        let result = check(
            "(type Metric {:id String :label String :value Number})\n\
             (type MetricRegistry (Map String Metric))\n\
             (ann base-registry MetricRegistry)\n\
             (def base-registry\n  (hash-map \"zone2\" {:id \"zone2\" :label \"Zone 2\" :value 50}\n            \"trimp\" {:id \"trimp\" :label \"TRIMP\" :value 12.5}))\n\
             (ann selected-metric (Fn [MetricRegistry String] (Option Metric)))\n\
             (defn selected-metric [registry id] (map-get registry id))\n\
             (ann upsert-metric (Fn [MetricRegistry Metric] MetricRegistry))\n\
             (defn upsert-metric [registry metric] (map-assoc registry metric.id metric))\n\
             (ann remove-metric (Fn [MetricRegistry String] MetricRegistry))\n\
             (defn remove-metric [registry id] (map-dissoc registry id))\n\
             (def summary {:selected (selected-metric base-registry \"zone2\")\n\
                           :contains (contains? base-registry \"trimp\")\n\
                           :count (count base-registry)\n\
                           :map (map? base-registry)})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Map String {:id String :label String :value Number})"
        );
        assert!(
            result.forms[1]
                .ty
                .contains("(Option {:id String :label String :value Number})")
        );
        assert_eq!(
            result.forms[2].ty,
            "(Fn [(Map String {:id String :label String :value Number}) {:id String :label String :value Number}] (Map String {:id String :label String :value Number}))"
        );
        assert_eq!(
            result.forms[3].ty,
            "(Fn [(Map String {:id String :label String :value Number}) String] (Map String {:id String :label String :value Number}))"
        );
        assert!(
            result.forms[4]
                .ty
                .contains(":selected (Option {:id String :label String :value Number})")
        );
        assert!(result.forms[4].ty.contains(":contains Bool"));
        assert!(result.forms[4].ty.contains(":count Number"));
        assert!(result.forms[4].ty.contains(":map Bool"));
    }

    #[test]
    fn infers_map_enumeration_for_metric_buckets() {
        let result = check(
            "(type ZoneDurations (Map Number Number))\n\
             (ann durations ZoneDurations)\n\
             (def durations (hash-map 1 0 2 45000 3 15000))\n\
             (def entries (map-entries durations))\n\
             (def keys (map-keys durations))\n\
             (def values (map-values durations))\n\
             (ann trimp-from-durations (Fn [ZoneDurations] Number))\n\
             (defn trimp-from-durations [items]\n\
               (reduce (map-entries items)\n\
                       0\n\
                       (fn [sum entry]\n\
                         (+ sum (* (/ entry.value 60000) entry.key)))))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Map Number Number)");
        assert_eq!(result.forms[1].ty, "(Vector {:key Number :value Number})");
        assert_eq!(result.forms[2].ty, "(Vector Number)");
        assert_eq!(result.forms[3].ty, "(Vector Number)");
        assert_eq!(result.forms[4].ty, "(Fn [(Map Number Number)] Number)");
    }

    #[test]
    fn infers_dynamic_object_get() {
        let result = check(
            "(def method \"get\")\n\
             (def operation (object-get {:get {:id \"listPets\"}} method))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "String");
        assert!(result.forms[1].ty.starts_with("t"));
    }

    #[test]
    fn infers_result_helpers_for_imports() {
        let result = check(
            "(type ImportResult (Result (Vector {:id String}) String))\n\
             (ann parse-import (Fn [String] ImportResult))\n\
             (defn parse-import [text]\n  (let [payload (json-parse text)\n        entries (get payload :entries)]\n    (if (vector? entries)\n        (ok entries)\n        (err \"missing entries\"))))\n\
             (def parsed (parse-import \"{}\"))\n\
             (def fallback (unwrap-or parsed []))\n\
             (def message (result-error parsed))\n\
             (def flags {:ok (ok? parsed) :err (err? parsed)})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [String] (Result (Vector {:id String}) String))"
        );
        assert_eq!(result.forms[1].ty, "(Result (Vector {:id String}) String)");
        assert_eq!(result.forms[2].ty, "(Vector {:id String})");
        assert_eq!(result.forms[3].ty, "(Option String)");
        assert_eq!(result.forms[4].ty, "{:err Bool :ok Bool}");
    }

    #[test]
    fn infers_schema_decoders_for_records() {
        let result = check(
            "(type Spec {:draft (Option Number) :tags (Vector String) :title String})\n\
             (ann spec-decoder (Decoder Spec))\n\
             (def spec-decoder\n\
               (decoder-record {:title decoder-string\n\
                                :tags (decoder-vector decoder-string)\n\
                                :draft (decoder-optional decoder-number)}))\n\
             (def decoded\n\
               (decode spec-decoder (json-parse \"{\\\"title\\\":\\\"Pulse\\\",\\\"tags\\\":[\\\"zone\\\"]}\")))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Decoder {:draft (Option Number) :tags (Vector String) :title String})"
        );
        assert_eq!(
            result.forms[1].ty,
            "(Result {:draft (Option Number) :tags (Vector String) :title String} String)"
        );
    }

    #[test]
    fn infers_option_and_result_match_patterns() {
        let result = check(
            "(type ImportResult (Result (Vector {:id String}) String))\n\
             (type Selected (Option {:id String}))\n\
             (ann summarize (Fn [ImportResult] String))\n\
             (defn summarize [result]\n  (match result\n    (ok entries) (str \"Imported \" (count entries))\n    (err message) message))\n\
             (ann selected-id (Fn [Selected] String))\n\
             (defn selected-id [entry]\n  (match entry\n    (some selected) selected.id\n    nil \"none\"))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [(Result (Vector {:id String}) String)] String)"
        );
        assert_eq!(result.forms[1].ty, "(Fn [(Option {:id String})] String)");
    }

    #[test]
    fn infers_numeric_ranges_for_chart_ticks() {
        let result = check(
            "(defn tick-indices [count]\n  (range 0 (+ count 1)))\n\
             (def descending (range 5 0 -2))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [Number] (Vector Number))");
        assert_eq!(result.forms[1].ty, "(Vector Number)");
    }

    #[test]
    fn infers_numeric_vector_aggregates_for_chart_bounds() {
        let result = check(
            "(defn chart-bounds [points]\n\
                (let [values (map points (fn [point] point.value))]\n\
                  {:min (min-of values 0)\n\
                   :max (max-of values 100)\n\
                   :sum (sum values)}))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.forms[0].ty.contains("(Vector {:value Number})"));
        assert!(result.forms[0].ty.contains(":min Number"));
        assert!(result.forms[0].ty.contains(":max Number"));
        assert!(result.forms[0].ty.contains(":sum Number"));
    }

    #[test]
    fn infers_date_primitives_for_metric_trends() {
        let result = check(
            "(defn week-label [timestamp]\n  (let [start (date-start-of-week timestamp)\n        end (date-add-days start 6)]\n    (str (date-format start :month) \" \" (date-day start) \"-\" (date-day end))))\n\
             (defn month-key [timestamp] (date-start-of-month timestamp))\n\
             (defn log-timestamp [entry] (date-format entry.stoppedAt :month-day-time))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [Number] String)");
        assert_eq!(result.forms[1].ty, "(Fn [Number] Number)");
        assert_eq!(result.forms[2].ty, "(Fn [{:stoppedAt Number}] String)");
    }

    #[test]
    fn infers_record_update_helpers_for_state_changes() {
        let result = check(
            "(def updated-entry (assoc {:id \"warmup\" :hiddenAt nil} :hiddenAt 42 :exerciseType \"Strength\"))\n\
             (def imported (merge {:message \"Ready\" :selectedLogId nil} {:message \"Imported\" :selectedLogId \"warmup\"}))\n\
             (def cleared (dissoc {:message \"Ready\" :visibleCount 2} :message))\n\
             (def nested {:summary {:value 1 :label \"Warmup\"} :status \"Ready\"})\n\
             (def bumped (update-in nested [:summary :value] (fn [value] (+ value 1))))\n\
             (def relabeled (update-in bumped [:summary :label] (fn [label suffix] (str label suffix)) \"!\"))\n\
             (def nested-value (get-in relabeled [:summary :value]))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.forms[0].ty.contains(":exerciseType String"));
        assert!(result.forms[0].ty.contains(":hiddenAt (Option Number)"));
        assert!(result.forms[1].ty.contains(":message String"));
        assert!(result.forms[1].ty.contains(":selectedLogId String"));
        assert_eq!(result.forms[2].ty, "{:visibleCount Number}");
        assert!(
            result.forms[4]
                .ty
                .contains(":summary {:label String :value Number}")
        );
        assert!(
            result.forms[5]
                .ty
                .contains(":summary {:label String :value Number}")
        );
        assert_eq!(result.forms[6].ty, "Number");
    }

    #[test]
    fn infers_string_primitives_for_type_matching() {
        let result = check(
            "(defn matches-liss-type? [exercise-type]\n  (if (some? exercise-type)\n      (regex-test? (trim exercise-type) \"liss|steady|aerob\" \"i\")\n      false))\n\
             (defn matches-plain? [label]\n  (regex-test? label \"recovery\"))\n\
             (defn id-suffix [roll]\n  (string-slice (to-radix roll 36) 2 9))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [String] Bool)");
        assert_eq!(result.forms[1].ty, "(Fn [String] Bool)");
        assert_eq!(result.forms[2].ty, "(Fn [Number] String)");
    }

    #[test]
    fn infers_vector_includes_for_metric_visibility() {
        let result = check(
            "(ann metric-enabled? (Fn [(Vector String) String] Bool))\n\
             (defn metric-enabled? [enabled metric] (includes? enabled metric))\n\
             (def default-metrics [\"zone2\" \"trimp\"])\n\
             (def zone2-visible? (includes? default-metrics \"zone2\"))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [(Vector String) String] Bool)");
        assert_eq!(result.forms[1].ty, "(Vector String)");
        assert_eq!(result.forms[2].ty, "Bool");
    }

    #[test]
    fn infers_duration_formatting_primitives() {
        let result = check(
            "(defn pad2 [value] (pad-start (str value) 2 \"0\"))\n\
             (defn seconds-part [ms]\n  (mod (floor (/ ms 1000)) 60))\n\
             (defn percent-alias [value]\n  (% (+ value 1) 60))\n\
             (defn trend-delta [current previous]\n  (abs (- current previous)))\n\
             (defn short-minute-label [minutes]\n  (to-fixed minutes 1))\n\
             (defn stored-number [value]\n  (to-number value))\n\
             (def recovery-ms 60_000)",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [t0] String)");
        assert_eq!(result.forms[1].ty, "(Fn [Number] Number)");
        assert_eq!(result.forms[2].ty, "(Fn [Number] Number)");
        assert_eq!(result.forms[3].ty, "(Fn [Number Number] Number)");
        assert_eq!(result.forms[4].ty, "(Fn [Number] String)");
        assert!(result.forms[5].ty.starts_with("(Fn [t"));
        assert!(result.forms[5].ty.ends_with("] Number)"));
        assert_eq!(result.forms[6].ty, "Number");
    }

    #[test]
    fn infers_json_helpers_for_log_import_export() {
        let result = check(
            "(defn export-log [entries]\n  (json-stringify {:version 2 :entries entries} 2))\n\
             (defn imported-count [text]\n  (count (let [parsed (json-parse text)] parsed.entries)))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [t0] String)");
        assert_eq!(result.forms[1].ty, "(Fn [String] Number)");
    }

    #[test]
    fn infers_dev_environment_flag_as_bool() {
        let result = check(
            "(defn init []\n\
               [{:dev? (env-dev?) :status (if (env-dev?) \"Dev\" \"Prod\")}\n\
                (if (env-dev?)\n\
                    {:kind :window/event-watch :id \"dev-hotkey\" :type \"keydown\" :onEvent :dev-key}\n\
                    {:kind :none})])",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.forms[0].ty.contains(":dev? Bool"));
        assert!(result.forms[0].ty.contains(":status String"));
        assert!(result.forms[0].ty.contains("(Union"));
        assert!(result.forms[0].ty.contains(":id String"));
        assert!(result.forms[0].ty.contains(":onEvent Keyword"));
    }

    #[test]
    fn infers_env_mode_and_regex_capture_all() {
        let result = check(
            "(defn parse [text]\n\
               {:mode (env-mode)\n\
                :pairs (regex-capture-all text \"name=([^;]+);url=([^;]+)\" \"g\")})",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.forms[0].ty.contains(":mode String"));
        assert!(
            result.forms[0]
                .ty
                .contains(":pairs (Vector (Vector String))")
        );
    }

    #[test]
    fn infers_safe_get_and_runtime_type_predicates_for_imports() {
        let result = check(
            "(defn valid-entry? [entry]\n  (and (string? (get entry :id))\n       (number? (get entry :startedAt))\n       (number? (get entry :durationMs))\n       (vector? (get entry :readings))))\n\
             (defn payload-entries [parsed]\n  (if (vector? parsed) parsed (get parsed :entries)))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms.len(), 2);
        assert!(result.forms[0].ty.starts_with("(Fn ["));
        assert!(result.forms[0].ty.ends_with(" Bool)"));
        assert!(result.forms[1].ty.starts_with("(Fn ["));
    }

    #[test]
    fn nth_field_reads_constrain_vector_elements() {
        let result = check(
            "(defn peaked? [readings]\n  (let [previous (nth readings 0)]\n    (> previous.bpm 120)))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [(Vector {:bpm Number})] Bool)");
    }

    #[test]
    fn last_field_reads_constrain_vector_elements() {
        let result = check(
            "(defn latest-bpm [readings]\n  (let [reading (last readings)]\n    reading.bpm))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.forms[0].ty, "(Fn [(Vector {:bpm t2})] t2)");
    }

    #[test]
    fn nth_field_reads_in_bool_chains_constrain_vector_elements() {
        let result = check(
            "(defn valid-peak? [readings index current]\n  (let [previous (nth readings (- index 1))\n        next (nth readings (+ index 1))]\n    (and (> index 0)\n         (>= current.bpm previous.bpm)\n         (> current.bpm next.bpm))))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [(Vector {:bpm Number}) Number {:bpm Number}] Bool)"
        );
    }

    #[test]
    fn count_and_nth_field_reads_preserve_vector_element_constraints() {
        let result = check(
            "(defn valid-peak? [readings index current]\n  (let [previous (nth readings (- index 1))\n        next (nth readings (+ index 1))]\n    (and (> index 0)\n         (< index (- (count readings) 1))\n         (>= current.bpm previous.bpm)\n         (> current.bpm next.bpm))))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[0].ty,
            "(Fn [(Vector {:bpm Number}) Number {:bpm Number}] Bool)"
        );
    }

    #[test]
    fn widens_bound_record_vectors_across_helper_calls() {
        let result = check(
            "(defn has-time [readings]\n  (find readings (fn [reading] (> reading.time 0))))\n\
             (defn bpm-total [readings]\n  (reduce readings 0 (fn [sum reading] (+ sum reading.bpm))))\n\
             (defn both [readings]\n  (+ (bpm-total readings) (let [reading (has-time readings)] reading.time)))",
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[2].ty,
            "(Fn [(Vector {:bpm Number :time Number})] Number)"
        );
    }

    #[test]
    fn collects_state_read_paths_without_call_heads() {
        let source = syntax::parse_source("(not entry.connected?)");
        let expr = &source.forms[0];

        assert_eq!(free_symbols(expr), vec!["entry.connected?"]);
    }
}
