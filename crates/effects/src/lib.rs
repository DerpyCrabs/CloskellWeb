use std::collections::{HashMap, HashSet};

use syntax::{Diagnostic, Expr, ExprKind, SourceFile, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandType {
    pub name: &'static str,
    pub payload_type: &'static str,
    pub message_type: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectReport {
    pub commands: Vec<CommandType>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn core_command_types() -> Vec<CommandType> {
    vec![
        CommandType {
            name: "Bluetooth",
            payload_type: "BluetoothRequest",
            message_type: "msg",
        },
        CommandType {
            name: "Timer",
            payload_type: "TimerRequest",
            message_type: "msg",
        },
        CommandType {
            name: "Animation",
            payload_type: "AnimationRequest",
            message_type: "msg",
        },
        CommandType {
            name: "Time",
            payload_type: "TimeRequest",
            message_type: "msg",
        },
        CommandType {
            name: "Storage",
            payload_type: "StorageRequest",
            message_type: "msg",
        },
        CommandType {
            name: "Http",
            payload_type: "HttpRequest",
            message_type: "msg",
        },
        CommandType {
            name: "Random",
            payload_type: "RandomRequest",
            message_type: "msg",
        },
        CommandType {
            name: "Canvas",
            payload_type: "CanvasRequest",
            message_type: "msg",
        },
        CommandType {
            name: "DomRef",
            payload_type: "DomRefRequest",
            message_type: "msg",
        },
        CommandType {
            name: "MediaQuery",
            payload_type: "MediaQueryRequest",
            message_type: "msg",
        },
        CommandType {
            name: "File",
            payload_type: "FileRequest",
            message_type: "msg",
        },
        CommandType {
            name: "Window",
            payload_type: "WindowRequest",
            message_type: "msg",
        },
    ]
}

pub fn validate_purity(source: &SourceFile) -> EffectReport {
    validate_purity_with_imported_command_helpers(source, &HashSet::new())
}

pub fn validate_purity_with_imported_command_helpers(
    source: &SourceFile,
    imported_command_helpers: &HashSet<String>,
) -> EffectReport {
    let command_helpers = collect_defn_infos(source);
    let command_params_by_helper = collect_command_params_by_helper(source, &command_helpers);
    let mut validator = EffectValidator {
        command_helpers,
        command_values: collect_def_values(source),
        command_params_by_helper,
        imported_command_helpers,
        validated_helpers: HashSet::new(),
        validating_helpers: HashSet::new(),
        local_command_scopes: Vec::new(),
        validating_local_commands: Vec::new(),
        diagnostics: Vec::new(),
    };
    for form in &source.forms {
        collect_forbidden_browser_access(form, &mut validator.diagnostics);
        validator.validate_init_form(form);
        validator.validate_update_form(form);
    }

    EffectReport {
        commands: core_command_types(),
        diagnostics: validator.diagnostics,
    }
}

#[derive(Clone)]
struct DefnInfo {
    params: Vec<Vec<String>>,
    body: Expr,
}

#[derive(Clone)]
enum CommandSymbol {
    Expr(Expr),
    Opaque,
}

struct EffectValidator<'a> {
    command_helpers: HashMap<String, DefnInfo>,
    command_values: HashMap<String, Expr>,
    command_params_by_helper: HashMap<String, HashSet<String>>,
    imported_command_helpers: &'a HashSet<String>,
    validated_helpers: HashSet<String>,
    validating_helpers: HashSet<String>,
    local_command_scopes: Vec<HashMap<String, CommandSymbol>>,
    validating_local_commands: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

fn collect_defn_infos(source: &SourceFile) -> HashMap<String, DefnInfo> {
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
            let params = params.iter().map(pattern_symbols).collect();
            Some((
                name.clone(),
                DefnInfo {
                    params,
                    body: items.last()?.clone(),
                },
            ))
        })
        .collect()
}

fn collect_def_values(source: &SourceFile) -> HashMap<String, Expr> {
    source
        .forms
        .iter()
        .filter_map(|form| {
            let ExprKind::List(items) = &form.kind else {
                return None;
            };
            if items.len() != 3 || !matches_symbol(&items[0], "def") {
                return None;
            }
            let ExprKind::Symbol(name) = &items[1].kind else {
                return None;
            };
            Some((name.clone(), items[2].clone()))
        })
        .collect()
}

fn collect_command_params_by_helper(
    source: &SourceFile,
    helpers: &HashMap<String, DefnInfo>,
) -> HashMap<String, HashSet<String>> {
    let mut command_params_by_helper = HashMap::new();
    for form in &source.forms {
        let Some((name, command_param_indices)) = command_param_indices_from_annotation(form)
        else {
            continue;
        };
        let Some(helper) = helpers.get(&name) else {
            continue;
        };
        let command_params = command_param_indices
            .into_iter()
            .filter_map(|index| helper.params.get(index))
            .flat_map(|params| params.iter().cloned())
            .collect::<HashSet<_>>();
        if !command_params.is_empty() {
            command_params_by_helper.insert(name, command_params);
        }
    }
    command_params_by_helper
}

fn require_command_fields(
    span: Span,
    entries: &[(Expr, Expr)],
    fields: &[&str],
    command: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for field in fields {
        if map_get(entries, field).is_none() {
            diagnostics.push(Diagnostic::error(
                span,
                format!("{} command is missing a :{} field", command, field),
            ));
        }
    }
}

fn require_success_command_field(
    span: Span,
    entries: &[(Expr, Expr)],
    command: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if map_get(entries, "onSuccess").is_some() || map_get(entries, "toMessage").is_some() {
        return;
    }

    diagnostics.push(Diagnostic::error(
        span,
        format!(
            "{} command is missing one of :onSuccess, :toMessage",
            command
        ),
    ));
}

fn reject_conflicting_success_command_fields(
    span: Span,
    entries: &[(Expr, Expr)],
    command: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let present = ["msg", "onSuccess", "toMessage"]
        .into_iter()
        .filter(|field| map_get(entries, field).is_some())
        .collect::<Vec<_>>();

    if present.len() <= 1 {
        return;
    }

    diagnostics.push(Diagnostic::error(
        span,
        format!(
            "{} command has conflicting success continuations {}; use only one of :msg, :onSuccess, :toMessage",
            command,
            present
                .iter()
                .map(|field| format!(":{}", field))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ));
}

fn reject_structural_command_continuations(
    span: Span,
    entries: &[(Expr, Expr)],
    command: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !matches!(command, "none" | "batch") {
        return;
    }

    let present = COMMAND_CONTINUATION_FIELDS
        .iter()
        .filter(|field| map_get(entries, field).is_some())
        .copied()
        .collect::<Vec<_>>();

    if present.is_empty() {
        return;
    }

    diagnostics.push(Diagnostic::error(
        span,
        format!(
            "{} command does not support continuations {}",
            command,
            present
                .iter()
                .map(|field| format!(":{}", field))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ));
}

fn reject_change_command_success_continuations(
    span: Span,
    entries: &[(Expr, Expr)],
    command: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !matches!(command, "dom-ref/resize-watch" | "media-query/watch") {
        return;
    }

    let present = ["msg", "onSuccess", "toMessage"]
        .into_iter()
        .filter(|field| map_get(entries, field).is_some())
        .collect::<Vec<_>>();

    if present.is_empty() {
        return;
    }

    diagnostics.push(Diagnostic::error(
        span,
        format!(
            "{} command dispatches changes through :onChange and does not support success continuations {}",
            command,
            present
                .iter()
                .map(|field| format!(":{}", field))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ));
}

fn reject_payloadless_success_continuations(
    span: Span,
    entries: &[(Expr, Expr)],
    command: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !matches!(
        command,
        "bluetooth/disconnect" | "timer/after" | "timer/every" | "timer/cancel"
    ) {
        return;
    }

    let present = ["onSuccess", "toMessage"]
        .into_iter()
        .filter(|field| map_get(entries, field).is_some())
        .collect::<Vec<_>>();

    if present.is_empty() {
        return;
    }

    diagnostics.push(Diagnostic::error(
        span,
        format!(
            "{} command has no success payload; use :msg for completion messages instead of {}",
            command,
            present
                .iter()
                .map(|field| format!(":{}", field))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ));
}

fn require_http_request_url(
    span: Span,
    entries: &[(Expr, Expr)],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if map_get(entries, "url").is_some() {
        return;
    }

    let Some(request) = map_get(entries, "request") else {
        return;
    };

    if let ExprKind::Map(request_entries) = &request.kind {
        if map_get(request_entries, "url").is_none() {
            diagnostics.push(Diagnostic::error(
                span,
                "http/request command :request is missing a :url field",
            ));
        }
    }
}

fn reject_unsupported_continuation_fields(
    span: Span,
    entries: &[(Expr, Expr)],
    command: &str,
    diagnostics: &mut Vec<Diagnostic>,
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
        if map_get(entries, field).is_some() && !supported.contains(&command) {
            diagnostics.push(Diagnostic::error(
                span,
                format!(
                    "{} command does not support :{}; supported on {}",
                    command,
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

impl EffectValidator<'_> {
    fn validate_update_form(&mut self, expr: &Expr) {
        let ExprKind::List(items) = &expr.kind else {
            return;
        };
        if items.len() < 4
            || !matches_symbol(&items[0], "defn")
            || !matches_symbol(&items[1], "update")
        {
            return;
        }

        if let Some(body) = items.last() {
            self.validate_update_result(body);
        }
    }

    fn validate_init_form(&mut self, expr: &Expr) {
        let ExprKind::List(items) = &expr.kind else {
            return;
        };

        if items.len() >= 4
            && matches_symbol(&items[0], "defn")
            && matches_symbol(&items[1], "init")
        {
            if let Some(body) = items.last() {
                self.validate_init_result(body);
            }
            return;
        }

        if items.len() == 3 && matches_symbol(&items[0], "def") && matches_symbol(&items[1], "init")
        {
            self.validate_init_result(&items[2]);
        }
    }

    fn validate_init_result(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Vector(items) if items.len() == 2 => self.validate_command_expr(&items[1]),
            ExprKind::List(items) if matches_head(items, "if") && items.len() == 4 => {
                self.validate_init_result(&items[2]);
                self.validate_init_result(&items[3]);
            }
            ExprKind::List(items) if matches_head(items, "match") && items.len() >= 4 => {
                self.validate_match_bodies(expr, items, "init", Self::validate_init_result);
            }
            ExprKind::List(items) if matches_head(items, "let") => {
                self.with_let_command_scope(items, |validator, body| {
                    validator.validate_init_result(body);
                });
            }
            ExprKind::List(items) if matches_head(items, "do") => {
                if let Some(last) = items.last() {
                    self.validate_init_result(last);
                }
            }
            _ => {}
        }
    }

    fn validate_update_result(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Vector(items) if items.len() == 2 => self.validate_command_expr(&items[1]),
            ExprKind::Vector(items) => self.diagnostics.push(Diagnostic::error(
                expr.span,
                format!(
                    "update must return [state cmd], found a vector with {} items",
                    items.len()
                ),
            )),
            ExprKind::List(items) if matches_head(items, "if") && items.len() == 4 => {
                self.validate_update_result(&items[2]);
                self.validate_update_result(&items[3]);
            }
            ExprKind::List(items) if matches_head(items, "match") && items.len() >= 4 => {
                self.validate_match_bodies(expr, items, "update", Self::validate_update_result);
            }
            ExprKind::List(items) if matches_head(items, "do") => {
                if let Some(last) = items.last() {
                    self.validate_update_result(last);
                }
            }
            ExprKind::List(items) if matches_head(items, "let") => {
                self.with_let_command_scope(items, |validator, body| {
                    validator.validate_update_result(body);
                });
            }
            _ => self.diagnostics.push(Diagnostic::error(
                expr.span,
                "update must return [state cmd] so effects stay explicit command data",
            )),
        }
    }

    fn validate_command_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Map(entries) => validate_command_map(self, expr.span, entries),
            ExprKind::Vector(items) => {
                for item in items {
                    self.validate_command_expr(item);
                }
            }
            ExprKind::List(items) if matches_head(items, "if") && items.len() == 4 => {
                self.validate_command_expr(&items[2]);
                self.validate_command_expr(&items[3]);
            }
            ExprKind::List(items) if matches_head(items, "match") && items.len() >= 4 => {
                self.validate_match_bodies(expr, items, "command", Self::validate_command_expr);
            }
            ExprKind::List(items) if matches_head(items, "let") => {
                self.with_let_command_scope(items, |validator, body| {
                    validator.validate_command_expr(body);
                });
            }
            ExprKind::List(items) if matches_head(items, "do") => {
                if let Some(last) = items.last() {
                    self.validate_command_expr(last);
                }
            }
            ExprKind::List(items) => {
                if let Some(name) = function_call_name(items) {
                    if self.command_helpers.contains_key(name) {
                        self.validate_command_helper(name, expr.span);
                        return;
                    }
                    if self.imported_command_helpers.contains(name) {
                        return;
                    }
                }
                self.diagnostics.push(Diagnostic::error(
                    expr.span,
                    "command position must be command data such as {:kind :none}",
                ));
            }
            ExprKind::Symbol(name) => self.validate_command_symbol(name, expr.span),
            _ => self.diagnostics.push(Diagnostic::error(
                expr.span,
                "command position must be command data such as {:kind :none}",
            )),
        }
    }

    fn validate_match_bodies(
        &mut self,
        expr: &Expr,
        items: &[Expr],
        context: &str,
        mut validate_body: impl FnMut(&mut Self, &Expr),
    ) {
        if items.len() % 2 != 0 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                format!("match in {} must contain pattern/body pairs", context),
            ));
            return;
        }
        for arm in items[2..].chunks(2) {
            if let [_, body] = arm {
                validate_body(self, body);
            }
        }
    }

    fn with_let_command_scope(
        &mut self,
        items: &[Expr],
        validate_body: impl FnOnce(&mut Self, &Expr),
    ) {
        let Some(last) = items.last() else {
            return;
        };
        let Some(scope) = command_scope_from_let(items) else {
            validate_body(self, last);
            return;
        };

        self.local_command_scopes.push(scope);
        validate_body(self, last);
        self.local_command_scopes.pop();
    }

    fn validate_command_symbol(&mut self, name: &str, span: Span) {
        if let Some(symbol) = self.lookup_local_command_symbol(name) {
            self.validate_command_symbol_binding(name, symbol, span);
            return;
        }
        if let Some(expr) = self.command_values.get(name).cloned() {
            self.validate_command_symbol_binding(name, CommandSymbol::Expr(expr), span);
            return;
        }

        self.diagnostics.push(Diagnostic::error(
            span,
            format!(
                "command position symbol `{}` is not known command data; return a command record or call an annotated command helper",
                name
            ),
        ));
    }

    fn validate_command_symbol_binding(&mut self, name: &str, symbol: CommandSymbol, span: Span) {
        match symbol {
            CommandSymbol::Opaque => {}
            CommandSymbol::Expr(expr) => {
                if self
                    .validating_local_commands
                    .iter()
                    .any(|validating| validating == name)
                {
                    self.diagnostics.push(Diagnostic::error(
                        span,
                        format!("recursive command alias `{}` cannot be validated", name),
                    ));
                    return;
                }
                self.validating_local_commands.push(name.to_string());
                self.validate_command_expr(&expr);
                self.validating_local_commands.pop();
            }
        }
    }

    fn lookup_local_command_symbol(&self, name: &str) -> Option<CommandSymbol> {
        self.local_command_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn validate_command_helper(&mut self, name: &str, span: Span) {
        if self.validated_helpers.contains(name) {
            return;
        }
        if !self.validating_helpers.insert(name.to_string()) {
            self.diagnostics.push(Diagnostic::error(
                span,
                format!("recursive command helper `{}` cannot be validated", name),
            ));
            return;
        }

        let helper = self.command_helpers.get(name).cloned();
        if let Some(helper) = helper {
            let command_params = self
                .command_params_by_helper
                .get(name)
                .cloned()
                .unwrap_or_default();
            if command_params.is_empty() {
                self.validate_command_expr(&helper.body);
            } else {
                let scope = helper
                    .params
                    .into_iter()
                    .flatten()
                    .filter(|param| command_params.contains(param))
                    .map(|param| (param, CommandSymbol::Opaque))
                    .collect::<HashMap<_, _>>();
                self.local_command_scopes.push(scope);
                self.validate_command_expr(&helper.body);
                self.local_command_scopes.pop();
            }
        }

        self.validating_helpers.remove(name);
        self.validated_helpers.insert(name.to_string());
    }
}

fn validate_command_map(validator: &mut EffectValidator<'_>, span: Span, entries: &[(Expr, Expr)]) {
    let Some(kind_expr) = map_get(entries, "kind") else {
        validator.diagnostics.push(Diagnostic::error(
            span,
            "command record is missing a :kind field",
        ));
        return;
    };

    let Some(kind) = command_kind_literal(kind_expr) else {
        return;
    };

    if !is_known_command_kind(&kind) {
        validator.diagnostics.push(Diagnostic::error(
            kind_expr.span,
            format!("unknown command kind :{}", kind),
        ));
        return;
    }

    if matches!(kind.as_str(), "none" | "batch") {
        reject_structural_command_continuations(span, entries, &kind, &mut validator.diagnostics);
    } else if matches!(kind.as_str(), "dom-ref/resize-watch" | "media-query/watch") {
        reject_change_command_success_continuations(
            span,
            entries,
            &kind,
            &mut validator.diagnostics,
        );
        reject_unsupported_continuation_fields(span, entries, &kind, &mut validator.diagnostics);
    } else {
        reject_conflicting_success_command_fields(span, entries, &kind, &mut validator.diagnostics);
        reject_payloadless_success_continuations(span, entries, &kind, &mut validator.diagnostics);
        reject_unsupported_continuation_fields(span, entries, &kind, &mut validator.diagnostics);
    }

    if kind == "batch" {
        let Some(commands) = map_get(entries, "commands") else {
            validator.diagnostics.push(Diagnostic::error(
                span,
                "batch command is missing a :commands vector",
            ));
            return;
        };
        match &commands.kind {
            ExprKind::Vector(items) | ExprKind::List(items) => {
                for item in items {
                    validator.validate_command_expr(item);
                }
            }
            _ => validator.diagnostics.push(Diagnostic::error(
                commands.span,
                "batch :commands must be a vector of command records",
            )),
        }
    }

    if kind == "file/download" {
        require_command_fields(
            span,
            entries,
            &["name", "content"],
            "file/download",
            &mut validator.diagnostics,
        );
    }

    if kind == "file/import" {
        require_success_command_field(span, entries, "file/import", &mut validator.diagnostics);
    }

    if kind == "file/read-selected" {
        require_command_fields(
            span,
            entries,
            &["ref"],
            "file/read-selected",
            &mut validator.diagnostics,
        );
        require_success_command_field(
            span,
            entries,
            "file/read-selected",
            &mut validator.diagnostics,
        );
    }

    if kind == "bluetooth/request-device" {
        require_success_command_field(
            span,
            entries,
            "bluetooth/request-device",
            &mut validator.diagnostics,
        );
        require_one_command_field(
            span,
            entries,
            &["options", "filters", "acceptAllDevices"],
            "bluetooth/request-device",
            &mut validator.diagnostics,
        );
    }

    if kind == "bluetooth/connect-heart-rate" {
        require_command_fields(
            span,
            entries,
            &["id", "onReading"],
            "bluetooth/connect-heart-rate",
            &mut validator.diagnostics,
        );
        require_success_command_field(
            span,
            entries,
            "bluetooth/connect-heart-rate",
            &mut validator.diagnostics,
        );
        require_one_command_field(
            span,
            entries,
            &["options", "filters", "acceptAllDevices"],
            "bluetooth/connect-heart-rate",
            &mut validator.diagnostics,
        );
    }

    if kind == "bluetooth/disconnect" {
        require_command_fields(
            span,
            entries,
            &["id"],
            "bluetooth/disconnect",
            &mut validator.diagnostics,
        );
    }

    match kind.as_str() {
        "timer/after" => {
            require_command_fields(
                span,
                entries,
                &["ms", "msg"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "timer/every" => {
            require_command_fields(
                span,
                entries,
                &["ms", "msg", "id"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "timer/cancel" => {
            require_command_fields(span, entries, &["id"], &kind, &mut validator.diagnostics);
        }
        "animation/frame" => {
            require_command_fields(
                span,
                entries,
                &["onFrame"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "animation/cancel" => {
            require_command_fields(span, entries, &["id"], &kind, &mut validator.diagnostics);
        }
        "time/now" => {
            require_success_command_field(span, entries, &kind, &mut validator.diagnostics);
        }
        "storage/get" => {
            require_command_fields(span, entries, &["key"], &kind, &mut validator.diagnostics);
            require_success_command_field(span, entries, &kind, &mut validator.diagnostics);
        }
        "storage/remove" => {
            require_command_fields(span, entries, &["key"], &kind, &mut validator.diagnostics);
        }
        "storage/set" => {
            require_command_fields(
                span,
                entries,
                &["key", "value"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "random/number" => {
            require_success_command_field(span, entries, &kind, &mut validator.diagnostics);
        }
        "simulation/heart-rate" => {
            require_command_fields(
                span,
                entries,
                &["id", "onReading"],
                &kind,
                &mut validator.diagnostics,
            );
            require_success_command_field(span, entries, &kind, &mut validator.diagnostics);
        }
        "simulation/stop" => {
            require_command_fields(span, entries, &["id"], &kind, &mut validator.diagnostics);
        }
        "http/request" => {
            require_one_command_field(
                span,
                entries,
                &["request", "url"],
                &kind,
                &mut validator.diagnostics,
            );
            require_http_request_url(span, entries, &mut validator.diagnostics);
            require_success_command_field(span, entries, &kind, &mut validator.diagnostics);
        }
        "canvas/draw" => {
            require_command_fields(
                span,
                entries,
                &["ref", "ops"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "canvas/measure-text" => {
            require_command_fields(span, entries, &["ref"], &kind, &mut validator.diagnostics);
            require_success_command_field(span, entries, &kind, &mut validator.diagnostics);
            require_one_command_field(
                span,
                entries,
                &["text", "texts"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "dom-ref/focus" | "dom-ref/click" => {
            require_command_fields(span, entries, &["ref"], &kind, &mut validator.diagnostics);
        }
        "dom-ref/measure" => {
            require_command_fields(span, entries, &["ref"], &kind, &mut validator.diagnostics);
            require_success_command_field(span, entries, &kind, &mut validator.diagnostics);
        }
        "dom-ref/resize-watch" => {
            require_command_fields(
                span,
                entries,
                &["ref", "onChange"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "dom-ref/resize-unwatch" => {
            require_one_command_field(
                span,
                entries,
                &["id", "ref"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "window/event-watch" => {
            require_command_fields(
                span,
                entries,
                &["type", "onEvent"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "window/event-unwatch" => {
            require_one_command_field(
                span,
                entries,
                &["id", "type"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "media-query/watch" => {
            require_command_fields(
                span,
                entries,
                &["query", "onChange"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        "media-query/unwatch" => {
            require_one_command_field(
                span,
                entries,
                &["id", "query"],
                &kind,
                &mut validator.diagnostics,
            );
        }
        _ => {}
    }
}

fn require_one_command_field(
    span: Span,
    entries: &[(Expr, Expr)],
    fields: &[&str],
    command: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if fields.iter().any(|field| map_get(entries, field).is_some()) {
        return;
    }

    diagnostics.push(Diagnostic::error(
        span,
        format!(
            "{} command is missing one of {}",
            command,
            fields
                .iter()
                .map(|field| format!(":{}", field))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ));
}

fn command_scope_from_let(items: &[Expr]) -> Option<HashMap<String, CommandSymbol>> {
    if items.len() < 3 || !matches_head(items, "let") {
        return None;
    }
    let ExprKind::Vector(bindings) = &items[1].kind else {
        return None;
    };

    let mut scope = HashMap::new();
    for pair in bindings.chunks(2) {
        let [pattern, value_expr] = pair else {
            continue;
        };
        collect_command_pattern_bindings(pattern, value_expr, &mut scope);
    }
    Some(scope)
}

fn collect_command_pattern_bindings(
    pattern: &Expr,
    value: &Expr,
    scope: &mut HashMap<String, CommandSymbol>,
) {
    match &pattern.kind {
        ExprKind::Symbol(name) if name == "_" => {}
        ExprKind::Symbol(name) => {
            scope.insert(name.clone(), CommandSymbol::Expr(value.clone()));
        }
        ExprKind::List(items) if matches_head(items, "as") => {
            if items.len() == 3 {
                collect_command_pattern_bindings(&items[1], value, scope);
                if let ExprKind::Symbol(name) = &items[2].kind {
                    if name != "_" {
                        scope.insert(name.clone(), CommandSymbol::Expr(value.clone()));
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
                    let Some(value_items) = list_literal_items(value) else {
                        return;
                    };
                    for (pattern, value) in items[1..].iter().zip(value_items) {
                        collect_command_pattern_bindings(pattern, value, scope);
                    }
                }
                "some" | "ok" | "err" if items.len() == 2 => {
                    let Some(value_items) = constructor_literal_items(value, name) else {
                        return;
                    };
                    if let Some(inner) = value_items.first() {
                        collect_command_pattern_bindings(&items[1], inner, scope);
                    }
                }
                "cons" if items.len() == 3 => {
                    if let Some(value_items) = list_literal_items(value) {
                        if let Some(head) = value_items.first() {
                            collect_command_pattern_bindings(&items[1], head, scope);
                        }
                    }
                }
                _ => {}
            }
        }
        ExprKind::Map(entries) => {
            let ExprKind::Map(value_entries) = &value.kind else {
                return;
            };
            for (key, pattern) in entries {
                let Some(name) = record_key_name(key) else {
                    continue;
                };
                let Some(value) = map_get(value_entries, &name) else {
                    continue;
                };
                collect_command_pattern_bindings(pattern, value, scope);
            }
        }
        ExprKind::Vector(pattern_items) => {
            let ExprKind::Vector(value_items) = &value.kind else {
                return;
            };
            for (pattern, value) in pattern_items.iter().zip(value_items) {
                collect_command_pattern_bindings(pattern, value, scope);
            }
        }
        ExprKind::Set(_)
        | ExprKind::Quote(_)
        | ExprKind::QuasiQuote(_)
        | ExprKind::Unquote(_)
        | ExprKind::UnquoteSplicing(_)
        | ExprKind::HtmlTemplate(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn pattern_symbols(pattern: &Expr) -> Vec<String> {
    let mut symbols = Vec::new();
    collect_pattern_symbols(pattern, &mut symbols);
    symbols
}

fn collect_pattern_symbols(pattern: &Expr, symbols: &mut Vec<String>) {
    match &pattern.kind {
        ExprKind::Symbol(name) if name == "_" => {}
        ExprKind::Symbol(name) => symbols.push(name.clone()),
        ExprKind::List(items) if matches_head(items, "as") => {
            if items.len() == 3 {
                collect_pattern_symbols(&items[1], symbols);
                if let ExprKind::Symbol(name) = &items[2].kind {
                    if name != "_" {
                        symbols.push(name.clone());
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
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_pattern_symbols(item, symbols);
            }
        }
        ExprKind::Quote(_)
        | ExprKind::QuasiQuote(_)
        | ExprKind::Unquote(_)
        | ExprKind::UnquoteSplicing(_)
        | ExprKind::HtmlTemplate(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn list_literal_items(expr: &Expr) -> Option<&[Expr]> {
    match &expr.kind {
        ExprKind::List(items) if matches_head(items, "list") => Some(&items[1..]),
        ExprKind::Vector(items) => Some(items.as_slice()),
        _ => None,
    }
}

fn constructor_literal_items<'a>(expr: &'a Expr, name: &str) -> Option<&'a [Expr]> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    matches_head(items, name).then_some(&items[1..])
}

fn command_param_indices_from_annotation(expr: &Expr) -> Option<(String, Vec<usize>)> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    if items.len() != 3 || !matches_symbol(&items[0], "ann") {
        return None;
    }
    let ExprKind::Symbol(name) = &items[1].kind else {
        return None;
    };
    let ExprKind::List(type_items) = &items[2].kind else {
        return None;
    };
    if type_items.len() != 3 || !matches_symbol(&type_items[0], "Fn") {
        return None;
    }
    let ExprKind::Vector(param_types) = &type_items[1].kind else {
        return None;
    };

    let command_param_indices = param_types
        .iter()
        .enumerate()
        .filter_map(|(index, param_type)| type_expr_is_cmd(param_type).then_some(index))
        .collect::<Vec<_>>();
    Some((name.clone(), command_param_indices))
}

fn type_expr_is_cmd(expr: &Expr) -> bool {
    let ExprKind::List(items) = &expr.kind else {
        return false;
    };
    items.len() == 2 && matches_symbol(&items[0], "Cmd")
}

fn collect_forbidden_browser_access(expr: &Expr, diagnostics: &mut Vec<Diagnostic>) {
    match &expr.kind {
        ExprKind::Symbol(name) if browser_api_symbol(name) => {
            diagnostics.push(Diagnostic::error(
                expr.span,
                format!(
                    "`{}` is a browser API; pure code must return typed command data instead",
                    name
                ),
            ));
        }
        ExprKind::List(items) | ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_forbidden_browser_access(item, diagnostics);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_forbidden_browser_access(key, diagnostics);
                collect_forbidden_browser_access(value, diagnostics);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_forbidden_browser_access(inner, diagnostics),
        ExprKind::HtmlTemplate(node) => {
            collect_forbidden_browser_access_html_node(node, diagnostics)
        }
        ExprKind::Symbol(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn collect_forbidden_browser_access_html_node(
    node: &syntax::HtmlNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match node {
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_forbidden_browser_access(expr, diagnostics);
                }
            }
            for child in &element.children {
                collect_forbidden_browser_access_html_node(child, diagnostics);
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => collect_forbidden_browser_access(expr, diagnostics),
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn browser_api_symbol(name: &str) -> bool {
    matches!(
        name,
        "window" | "document" | "navigator" | "localStorage" | "fetch"
    ) || [
        "window.",
        "document.",
        "navigator.",
        "localStorage.",
        "fetch.",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
}

fn matches_symbol(expr: &Expr, expected: &str) -> bool {
    matches!(&expr.kind, ExprKind::Symbol(name) if name == expected)
}

fn matches_head(items: &[Expr], expected: &str) -> bool {
    items
        .first()
        .is_some_and(|head| matches_symbol(head, expected))
}

fn function_call_name(items: &[Expr]) -> Option<&str> {
    let ExprKind::Symbol(name) = &items.first()?.kind else {
        return None;
    };
    Some(name)
}

fn symbol_name(expr: &Expr) -> Option<&str> {
    let ExprKind::Symbol(name) = &expr.kind else {
        return None;
    };
    Some(name)
}

fn map_get<'a>(entries: &'a [(Expr, Expr)], name: &str) -> Option<&'a Expr> {
    entries
        .iter()
        .find_map(|(key, value)| (record_key_name(key).as_deref() == Some(name)).then_some(value))
}

fn record_key_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Keyword(name) | ExprKind::String(name) | ExprKind::Symbol(name) => {
            Some(name.clone())
        }
        _ => None,
    }
}

fn command_kind_literal(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Keyword(name) | ExprKind::String(name) | ExprKind::Symbol(name) => {
            Some(name.clone())
        }
        _ => None,
    }
}

pub fn is_known_command_kind(kind: &str) -> bool {
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
            | "random/number"
            | "simulation/heart-rate"
            | "simulation/stop"
            | "file/download"
            | "file/import"
            | "file/read-selected"
            | "canvas/draw"
            | "canvas/measure-text"
            | "dom-ref/focus"
            | "dom-ref/click"
            | "dom-ref/measure"
            | "dom-ref/resize-watch"
            | "dom-ref/resize-unwatch"
            | "window/event-watch"
            | "window/event-unwatch"
            | "media-query/watch"
            | "media-query/unwatch"
            | "http/request"
    )
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_direct_browser_api_access() {
        let source = syntax::parse_source("(def bad fetch)");
        let report = validate_purity(&source);

        assert_eq!(report.commands.len(), 12);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("typed command"))
        );
    }

    #[test]
    fn flags_dotted_browser_api_access() {
        let source = syntax::parse_source("(def width window.innerWidth)");
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("window.innerWidth")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn flags_browser_api_access_inside_html_expression() {
        let source = syntax::parse_source("(defn view [state] #html <p>{document.title}</p>)");
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("document.title")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn flags_browser_api_access_inside_html_event_handler() {
        let source = syntax::parse_source(
            "(defn view [state] #html <button on:click={(fetch \"/api/workouts\")}>Load</button>)",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("fetch")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn validates_update_command_records() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  (if (= msg :start)\n      [state {:kind :timer/after :ms 1000 :msg :tick}]\n      [state {:kind :none}]))",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_bluetooth_request_device_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :bluetooth/request-device :filters [{:services [\"heart_rate\"]}] :optionalServices [\"heart_rate\"] :onSuccess :connected :onError :bluetooth-error}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_bluetooth_request_device_without_selection_options() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :bluetooth/request-device :onSuccess :connected}])",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":filters"))
        );
    }

    #[test]
    fn validates_bluetooth_heart_rate_connection_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :bluetooth/connect-heart-rate :id \"hr\" :filters [{:services [\"heart_rate\"]}] :optionalServices [\"heart_rate\"] :onSuccess :connected :onReading :heart-rate :onDisconnected :disconnected :onError :bluetooth-error}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_bluetooth_heart_rate_connection_without_reading_message() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :bluetooth/connect-heart-rate :id \"hr\" :filters [{:services [\"heart_rate\"]}] :onSuccess :connected}])",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":onReading"))
        );
    }

    #[test]
    fn validates_bluetooth_disconnect_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :bluetooth/disconnect :id \"hr\" :msg :disconnected}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_recurring_timer_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  (if (= msg :start)\n      [state {:kind :timer/every :id \"clock\" :ms 250 :msg :tick}]\n      [state {:kind :timer/cancel :id \"clock\"}]))",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_cancelable_timer_after_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :timer/after :id \"hold-stop\" :ms 800 :msg {:kind :hold-complete}}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_time_now_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :time/now :onSuccess :timestamp}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_success_to_message_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :time/now :toMessage (fn [timestamp] {:kind :timestamp :timestamp timestamp})}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_time_now_without_success_message() {
        let source = syntax::parse_source("(defn update [state msg]\n  [state {:kind :time/now}])");
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("time/now command is missing one of :onSuccess, :toMessage")
        }));
    }

    #[test]
    fn rejects_conflicting_success_continuations() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :time/now :msg {:kind :tick} :onSuccess :timestamp}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("time/now command has conflicting success continuations")
        }));
    }

    #[test]
    fn rejects_unsupported_continuation_fields() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :time/now :onSuccess :timestamp :onCancel :cancelled}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("time/now command does not support :onCancel")
        }));
    }

    #[test]
    fn rejects_structural_command_continuations() {
        let source =
            syntax::parse_source("(defn update [state msg]\n  [state {:kind :none :msg :done}])");
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("none command does not support continuations :msg")
        }));

        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :batch :commands [] :onError :failed}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("batch command does not support continuations :onError")
        }));
    }

    #[test]
    fn rejects_change_watch_success_continuations() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:width 0} {:kind :dom-ref/resize-watch :ref \"heart-chart\" :onChange :changed :onSuccess :ready}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.message.contains(
            "dom-ref/resize-watch command dispatches changes through :onChange and does not support success continuations :onSuccess"
        )));

        let source = syntax::parse_source(
            "(defn init []\n  [{:mobile? false} {:kind :media-query/watch :query \"(max-width: 820px)\" :onChange :media-changed :msg :ready}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.message.contains(
            "media-query/watch command dispatches changes through :onChange and does not support success continuations :msg"
        )));
    }

    #[test]
    fn rejects_payloadless_success_continuations() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :timer/cancel :id \"clock\" :onSuccess :cancelled}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.message.contains(
            "timer/cancel command has no success payload; use :msg for completion messages instead of :onSuccess"
        )));

        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :bluetooth/disconnect :id \"hr\" :toMessage (fn [value] {:kind :disconnected})}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.message.contains(
            "bluetooth/disconnect command has no success payload; use :msg for completion messages instead of :toMessage"
        )));
    }

    #[test]
    fn rejects_timer_cancel_without_id() {
        let source =
            syntax::parse_source("(defn update [state msg]\n  [state {:kind :timer/cancel}])");
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":id"))
        );
    }

    #[test]
    fn validates_animation_frame_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :animation/frame :id \"hold-progress\" :onFrame :hold-frame}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_animation_frame_without_frame_message() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :animation/frame :id \"hold-progress\"}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("animation/frame command is missing a :onFrame field")
        }));
    }

    #[test]
    fn validates_animation_cancel_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :animation/cancel :id \"hold-progress\" :msg :cancelled}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_effectful_init_command_records() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:label \"Loading\"} {:kind :storage/get :key \"heartRateExercise.log.v1\" :format :json :onSuccess :log-loaded :onError :log-load-failed}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_storage_get_without_key() {
        let source =
            syntax::parse_source("(defn init []\n  [{:label \"Loading\"} {:kind :storage/get}])");
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":key"))
        );
    }

    #[test]
    fn rejects_storage_get_without_success_message() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:label \"Loading\"} {:kind :storage/get :key \"heartRateExercise.log.v1\"}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("storage/get command is missing one of :onSuccess, :toMessage")
        }));
    }

    #[test]
    fn rejects_unknown_update_command_kind() {
        let source =
            syntax::parse_source("(defn update [state msg] [state {:kind :browser/do-it}])");
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("unknown command kind"))
        );
    }

    #[test]
    fn rejects_abstract_command_category_kinds() {
        for kind in [
            "bluetooth",
            "timer",
            "animation",
            "time",
            "storage",
            "random",
            "simulation",
            "simulation/tick",
            "file",
            "canvas",
            "dom-ref",
            "window",
            "media-query",
            "http",
        ] {
            let source = syntax::parse_source(&format!(
                "(defn update [state msg]\n  [state {{:kind :{}}}])",
                kind
            ));
            let report = validate_purity(&source);

            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains("unknown command kind")),
                "abstract command kind :{} was accepted: {:?}",
                kind,
                report.diagnostics
            );
        }
    }

    #[test]
    fn rejects_update_without_command_pair() {
        let source = syntax::parse_source("(defn update [state msg] state)");
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("[state cmd]"))
        );
    }

    #[test]
    fn validates_match_update_command_records() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  (match msg\n    {:kind :start} [state {:kind :storage/set :key \"x\" :value \"y\" :msg :stored}]\n    _ [state {:kind :none}]))",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_match_init_command_records() {
        let source = syntax::parse_source(
            "(defn init []\n  (match (env-dev?)\n    true [{:label \"Dev\"} {:kind :timer/every :id \"clock\" :ms 250 :msg :tick}]\n    _ {:label \"Ready\"}))",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_invalid_match_init_command_records() {
        let source = syntax::parse_source(
            "(defn init []\n  (match (env-dev?)\n    true [{:label \"Dev\"} {:kind :timer/every :id \"clock\" :ms 250}]\n    _ {:label \"Ready\"}))",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":msg")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn validates_update_command_helper_calls() {
        let source = syntax::parse_source(
            "(defn start-hold-command []\n  {:kind :batch\n   :commands [{:kind :timer/after :id \"delete\" :ms 1000 :msg {:kind :delete}}\n              {:kind :animation/frame :id \"progress\" :onFrame :frame}]})\n\
             (defn persist-log-command [entries]\n  {:kind :storage/set :key \"heartRateExercise.log.v1\" :value {:version 2 :entries entries} :msg {:kind :saved}})\n\
             (defn update [state msg]\n  (match msg\n    {:kind :start} [state (start-hold-command)]\n    {:kind :persist} [state (persist-log-command state.entries)]\n    _ [state {:kind :none}]))",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_match_command_helper_records() {
        let source = syntax::parse_source(
            "(defn choose-command [msg]\n  (match msg\n    {:kind :stop} {:kind :timer/cancel :id \"clock\"}\n    _ {:kind :none}))\n\
             (defn update [state msg]\n  [state (choose-command msg)])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_invalid_match_command_helper_records() {
        let source = syntax::parse_source(
            "(defn choose-command [msg]\n  (match msg\n    {:kind :stop} {:kind :timer/cancel}\n    _ {:kind :none}))\n\
             (defn update [state msg]\n  [state (choose-command msg)])",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":id")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn validates_imported_command_helper_calls_when_allowed() {
        let source = syntax::parse_source(
            "(import \"./chart.clsk\" [heart-chart-command])\n\
             (defn update [state msg]\n  [state (heart-chart-command state)])",
        );
        let imported = HashSet::from(["heart-chart-command".to_string()]);
        let report = validate_purity_with_imported_command_helpers(&source, &imported);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_invalid_update_command_helper_calls() {
        let source = syntax::parse_source(
            "(defn bad-cancel-command []\n  {:kind :timer/cancel})\n\
             (defn update [state msg]\n  [state (bad-cancel-command)])",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":id")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn validates_let_bound_command_aliases() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  (let [command {:kind :timer/cancel :id \"clock\"}]\n    [state command]))",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_destructured_let_bound_command_aliases() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  (let [{:command command} {:command {:kind :timer/cancel :id \"clock\"}}]\n    [state command]))",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_invalid_destructured_let_bound_command_aliases() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  (let [{:command command} {:command \"not a command\"}]\n    [state command]))",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("command position must be command data")
        }));
    }

    #[test]
    fn validates_top_level_command_aliases() {
        let source = syntax::parse_source(
            "(def stop-clock {:kind :timer/cancel :id \"clock\"})\n\
             (defn update [state msg]\n  [state stop-clock])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_unbound_command_symbols() {
        let source = syntax::parse_source("(defn update [state msg]\n  [state command])");
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("command position symbol `command` is not known command data")
        }));
    }

    #[test]
    fn rejects_invalid_top_level_command_aliases() {
        let source = syntax::parse_source(
            "(def stop-clock \"not a command\")\n\
             (defn update [state msg]\n  [state stop-clock])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("command position must be command data")
        }));
    }

    #[test]
    fn validates_annotated_command_parameters_in_helpers() {
        let source = syntax::parse_source(
            "(ann with-chart-command (Fn [(Cmd Msg)] (Cmd Msg)))\n\
             (defn with-chart-command [command]\n\
               {:kind :batch :commands [command {:kind :none}]})\n\
             (defn update [state msg]\n  [state (with-chart-command {:kind :none})])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_annotated_destructured_command_parameters_in_helpers() {
        let source = syntax::parse_source(
            "(ann with-chart-command (Fn [(Cmd Msg)] (Cmd Msg)))\n\
             (defn with-chart-command [(as _ command)]\n\
               {:kind :batch :commands [command {:kind :none}]})\n\
             (defn update [state msg]\n  [state (with-chart-command {:kind :none})])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_unannotated_command_parameters_in_helpers() {
        let source = syntax::parse_source(
            "(defn with-chart-command [command]\n\
               {:kind :batch :commands [command {:kind :none}]})\n\
             (defn update [state msg]\n  [state (with-chart-command {:kind :none})])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("command position symbol `command` is not known command data")
        }));
    }

    #[test]
    fn validates_file_download_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :file/download :name \"exercise-log.json\" :content \"[]\" :mime \"application/json\" :msg :exported}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_file_download_without_content() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :file/download :name \"exercise-log.json\"}])",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":content"))
        );
    }

    #[test]
    fn validates_file_import_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :file/import :accept \"application/json,.json\" :format :json :onSuccess :imported :onError :import-failed}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_file_read_selected_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :file/read-selected :ref \"import-file\" :format :json :onSuccess :imported :onError :import-failed}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_file_read_selected_without_ref() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :file/read-selected :onSuccess :imported}])",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":ref"))
        );
    }

    #[test]
    fn rejects_file_import_without_success_message() {
        let source =
            syntax::parse_source("(defn update [state msg]\n  [state {:kind :file/import}])");
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":onSuccess"))
        );
    }

    #[test]
    fn validates_canvas_draw_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :canvas/draw :ref \"heart-chart\" :ops [{:op :clear} {:op :fill-rect :x 0 :y 0 :width 10 :height 10}] :msg :drawn}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_canvas_draw_without_ref() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :canvas/draw :ops [{:op :clear}]}])",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":ref"))
        );
    }

    #[test]
    fn validates_canvas_measure_text_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :canvas/measure-text :ref \"metrics-chart\" :texts [\"Zone 2\" \"TRIMP\"] :font \"700 12px system-ui\" :onSuccess :labels-measured}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_canvas_measure_text_without_texts() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :canvas/measure-text :ref \"metrics-chart\" :onSuccess :labels-measured}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("canvas/measure-text command is missing one of :text, :texts")
        }));
    }

    #[test]
    fn validates_dom_ref_focus_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :dom-ref/focus :ref \"exercise-type\" :msg :focused}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_dom_ref_click_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :dom-ref/click :ref \"import-file\" :msg :clicked}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_dom_ref_measure_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :dom-ref/measure :ref \"heart-chart\" :onSuccess :chart-measured :onError :measure-failed}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_dom_ref_measure_without_success_message() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :dom-ref/measure :ref \"heart-chart\"}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("dom-ref/measure command is missing one of :onSuccess, :toMessage")
        }));
    }

    #[test]
    fn validates_dom_ref_resize_watch_schema() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:width 0} {:kind :dom-ref/resize-watch :id \"chart\" :ref \"heart-chart\" :onChange :chart-resized :onError :resize-failed}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_dom_ref_resize_watch_without_change_message() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:width 0} {:kind :dom-ref/resize-watch :ref \"heart-chart\"}])",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":onChange"))
        );
    }

    #[test]
    fn validates_dom_ref_resize_unwatch_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :dom-ref/resize-unwatch :id \"chart\" :msg :stopped}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_window_event_watch_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :window/event-watch :id \"drag\" :type \"pointermove\" :onEvent :pointer-moved :options {:passive true}}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_window_event_watch_without_message() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :window/event-watch :type \"pointermove\"}])",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":onEvent"))
        );
    }

    #[test]
    fn validates_window_event_unwatch_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :window/event-unwatch :id \"drag\" :msg :stopped}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_media_query_watch_schema() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:mobile? false} {:kind :media-query/watch :id \"mobile\" :query \"(max-width: 700px)\" :onChange :media-changed}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_media_query_watch_without_change_message() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:mobile? false} {:kind :media-query/watch :query \"(max-width: 700px)\"}])",
        );
        let report = validate_purity(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(":onChange"))
        );
    }

    #[test]
    fn validates_media_query_unwatch_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :media-query/unwatch :id \"mobile\" :msg :stopped}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_random_number_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :random/number :min 111 :max 150 :onSuccess :simulated-bpm}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_random_number_without_success_message() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :random/number :min 111 :max 150}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("random/number command is missing one of :onSuccess, :toMessage")
        }));
    }

    #[test]
    fn validates_simulation_heart_rate_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :simulation/heart-rate :id \"sim\" :ms 1000 :min 120 :max 150 :jitter 3 :onSuccess :connected :onReading :heart-rate :onDisconnected :disconnected :onError :failed}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_simulation_heart_rate_without_reading_message() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :simulation/heart-rate :id \"sim\" :onSuccess :connected}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("simulation/heart-rate command is missing a :onReading field")
        }));
    }

    #[test]
    fn validates_simulation_stop_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :simulation/stop :id \"sim\" :msg :stopped}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_http_request_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :http/request :request {:url \"/api/log\" :method \"GET\"} :onSuccess :loaded :onError :failed}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_http_request_without_url_or_request() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :http/request :onSuccess :loaded}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("http/request command is missing one of :request, :url")
        }));
    }

    #[test]
    fn rejects_http_request_record_without_url() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :http/request :request {:method \"GET\"} :onSuccess :loaded}])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("http/request command :request is missing a :url field")
        }));
    }
}
