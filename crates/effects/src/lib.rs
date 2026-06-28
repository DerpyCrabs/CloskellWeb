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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectOptions {
    command_types: Vec<CommandType>,
    command_schemas: Vec<EffectCommandSchemaRule>,
    command_helpers: Vec<String>,
    subscription_helpers: Vec<String>,
    subscription_symbols: Vec<String>,
    subscription_schemas: Vec<EffectSubscriptionSchemaRule>,
    forbidden_forms: Vec<ForbiddenFormRule>,
    forbidden_symbols: Vec<ForbiddenSymbolRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectCommandSchemaRule {
    kind: String,
    required_fields: Vec<String>,
    one_of_field_groups: Vec<Vec<String>>,
    require_success: bool,
    reject_success_continuations: bool,
    payloadless_success: bool,
    supported_continuations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectSubscriptionSchemaRule {
    kind: String,
    required_fields: Vec<String>,
    collection_fields: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForbiddenFormKind {
    HtmlTemplate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForbiddenFormRule {
    kind: ForbiddenFormKind,
    message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForbiddenSymbolRule {
    pattern: SymbolPattern,
    message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolPattern {
    Exact(String),
    Prefix(String),
}

impl EffectOptions {
    pub fn command_schema(mut self, rule: EffectCommandSchemaRule) -> Self {
        self.command_schemas.push(rule);
        self
    }

    pub fn command_type(mut self, command_type: CommandType) -> Self {
        self.command_types.push(command_type);
        self
    }

    pub fn command_helper(mut self, name: impl Into<String>) -> Self {
        self.command_helpers.push(name.into());
        self
    }

    pub fn subscription_helper(mut self, name: impl Into<String>) -> Self {
        self.subscription_helpers.push(name.into());
        self
    }

    pub fn subscription_symbol(mut self, name: impl Into<String>) -> Self {
        self.subscription_symbols.push(name.into());
        self
    }

    pub fn subscription_schema(mut self, rule: EffectSubscriptionSchemaRule) -> Self {
        self.subscription_schemas.push(rule);
        self
    }

    pub fn forbid_form(mut self, kind: ForbiddenFormKind, message: impl Into<String>) -> Self {
        self.forbidden_forms.push(ForbiddenFormRule {
            kind,
            message: message.into(),
        });
        self
    }

    pub fn forbid_symbol(mut self, pattern: SymbolPattern, message: impl Into<String>) -> Self {
        self.forbidden_symbols.push(ForbiddenSymbolRule {
            pattern,
            message: message.into(),
        });
        self
    }

    fn forbidden_form_message(&self, kind: ForbiddenFormKind) -> Option<&str> {
        self.forbidden_forms
            .iter()
            .find(|rule| rule.kind == kind)
            .map(|rule| rule.message.as_str())
    }

    fn forbidden_symbol_message(&self, name: &str) -> Option<String> {
        self.forbidden_symbols
            .iter()
            .find(|rule| rule.pattern.matches(name))
            .map(|rule| rule.message.replace("{symbol}", name))
    }

    fn command_schema_rule(&self, kind: &str) -> Option<&EffectCommandSchemaRule> {
        self.command_schemas.iter().find(|rule| rule.kind == kind)
    }

    fn is_command_helper(&self, name: &str) -> bool {
        self.command_helpers.iter().any(|helper| helper == name)
    }

    fn is_subscription_helper(&self, name: &str) -> bool {
        self.subscription_helpers
            .iter()
            .any(|helper| helper == name)
    }

    fn is_subscription_symbol(&self, name: &str) -> bool {
        self.subscription_symbols
            .iter()
            .any(|symbol| symbol == name)
    }

    fn subscription_schema_rule(&self, kind: &str) -> Option<&EffectSubscriptionSchemaRule> {
        self.subscription_schemas
            .iter()
            .find(|rule| rule.kind == kind)
    }
}

impl EffectCommandSchemaRule {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            required_fields: Vec::new(),
            one_of_field_groups: Vec::new(),
            require_success: false,
            reject_success_continuations: false,
            payloadless_success: false,
            supported_continuations: Vec::new(),
        }
    }

    pub fn required_fields(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.required_fields = fields.into_iter().map(Into::into).collect();
        self
    }

    pub fn one_of_fields(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.one_of_field_groups
            .push(fields.into_iter().map(Into::into).collect());
        self
    }

    pub fn require_success(mut self) -> Self {
        self.require_success = true;
        self
    }

    pub fn reject_success_continuations(mut self) -> Self {
        self.reject_success_continuations = true;
        self
    }

    pub fn payloadless_success(mut self) -> Self {
        self.payloadless_success = true;
        self
    }

    pub fn supported_continuations(
        mut self,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.supported_continuations = fields.into_iter().map(Into::into).collect();
        self
    }

    fn supports_continuation(&self, field: &str) -> bool {
        self.supported_continuations
            .iter()
            .any(|supported| supported == field)
    }
}

impl EffectSubscriptionSchemaRule {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            required_fields: Vec::new(),
            collection_fields: Vec::new(),
        }
    }

    pub fn required_fields(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.required_fields = fields.into_iter().map(Into::into).collect();
        self
    }

    pub fn collection_fields(
        mut self,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.collection_fields = fields.into_iter().map(Into::into).collect();
        self
    }
}

impl SymbolPattern {
    pub fn exact(name: impl Into<String>) -> Self {
        Self::Exact(name.into())
    }

    pub fn prefix(prefix: impl Into<String>) -> Self {
        Self::Prefix(prefix.into())
    }

    fn matches(&self, name: &str) -> bool {
        match self {
            SymbolPattern::Exact(expected) => name == expected,
            SymbolPattern::Prefix(prefix) => name.starts_with(prefix),
        }
    }
}

pub fn core_command_types() -> Vec<CommandType> {
    vec![
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
            name: "Http",
            payload_type: "HttpRequest",
            message_type: "msg",
        },
        CommandType {
            name: "Task",
            payload_type: "TaskRequest",
            message_type: "msg",
        },
        CommandType {
            name: "Random",
            payload_type: "RandomRequest",
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
    validate_purity_with_options(source, imported_command_helpers, EffectOptions::default())
}

pub fn validate_purity_with_options(
    source: &SourceFile,
    imported_command_helpers: &HashSet<String>,
    options: EffectOptions,
) -> EffectReport {
    validate_purity_with_effect_helpers(
        source,
        imported_command_helpers,
        &HashSet::new(),
        &HashSet::new(),
        options,
    )
}

pub fn validate_purity_with_effect_helpers(
    source: &SourceFile,
    imported_command_helpers: &HashSet<String>,
    imported_update_result_helpers: &HashSet<String>,
    imported_subscription_helpers: &HashSet<String>,
    options: EffectOptions,
) -> EffectReport {
    let command_helpers = collect_defn_infos(source);
    let command_params_by_helper = collect_command_params_by_helper(source, &command_helpers);
    let mut validator = EffectValidator {
        command_helpers,
        command_values: collect_def_values(source),
        command_params_by_helper,
        imported_command_helpers,
        imported_update_result_helpers,
        imported_subscription_helpers,
        validated_helpers: HashSet::new(),
        validating_helpers: HashSet::new(),
        local_command_scopes: Vec::new(),
        validating_local_commands: Vec::new(),
        options: options.clone(),
        diagnostics: Vec::new(),
    };
    for form in &source.forms {
        collect_forbidden_access(form, &options, &mut validator.diagnostics);
        validator.validate_init_form(form);
        validator.validate_update_form(form);
        validator.validate_subscriptions_form(form);
    }

    let mut commands = core_command_types();
    commands.extend(options.command_types);
    EffectReport {
        commands,
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
    imported_update_result_helpers: &'a HashSet<String>,
    imported_subscription_helpers: &'a HashSet<String>,
    validated_helpers: HashSet<String>,
    validating_helpers: HashSet<String>,
    local_command_scopes: Vec<HashMap<String, CommandSymbol>>,
    validating_local_commands: Vec<String>,
    options: EffectOptions,
    diagnostics: Vec<Diagnostic>,
}

impl EffectValidator<'_> {
    fn is_known_command_kind(&self, kind: &str) -> bool {
        is_known_command_kind(kind) || self.options.command_schema_rule(kind).is_some()
    }

    fn command_schema_rule(&self, kind: &str) -> Option<EffectCommandSchemaRule> {
        self.options.command_schema_rule(kind).cloned()
    }

    fn is_known_subscription_kind(&self, kind: &str) -> bool {
        is_known_subscription_kind(kind) || self.options.subscription_schema_rule(kind).is_some()
    }

    fn subscription_schema_rule(&self, kind: &str) -> Option<EffectSubscriptionSchemaRule> {
        self.options.subscription_schema_rule(kind).cloned()
    }
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

fn validate_registered_command_schema(
    validator: &mut EffectValidator<'_>,
    span: Span,
    entries: &[(Expr, Expr)],
    schema: &EffectCommandSchemaRule,
) {
    for field in &schema.required_fields {
        if map_get(entries, field).is_none() {
            validator.diagnostics.push(Diagnostic::error(
                span,
                format!("{} command is missing a :{} field", schema.kind, field),
            ));
        }
    }
    for fields in &schema.one_of_field_groups {
        let fields = fields.iter().map(String::as_str).collect::<Vec<_>>();
        require_one_command_field(
            span,
            entries,
            &fields,
            &schema.kind,
            &mut validator.diagnostics,
        );
    }
    if schema.require_success {
        require_success_command_field(span, entries, &schema.kind, &mut validator.diagnostics);
    }
    if schema.reject_success_continuations {
        reject_change_command_success_continuations(
            span,
            entries,
            &schema.kind,
            &mut validator.diagnostics,
        );
    }
    if schema.payloadless_success {
        reject_payloadless_success_continuations(
            span,
            entries,
            &schema.kind,
            &mut validator.diagnostics,
        );
    }
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

fn is_builtin_payloadless_command(command: &str) -> bool {
    matches!(command, "timer/after" | "timer/every" | "timer/cancel")
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
    options: &EffectOptions,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (field, builtin_supported) in [
        ("onCancel", &[][..]),
        ("onDisconnected", &[][..]),
        ("onReading", &[][..]),
        ("onFrame", &["animation/frame"][..]),
        ("onChange", &[][..]),
        ("onEvent", &[][..]),
    ] {
        if map_get(entries, field).is_some() {
            let supported =
                command_kinds_supporting_continuation(field, builtin_supported, options);
            if supported.iter().any(|kind| kind == command) {
                continue;
            }
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

fn command_kinds_supporting_continuation(
    field: &str,
    builtin_supported: &[&str],
    options: &EffectOptions,
) -> Vec<String> {
    let mut supported = builtin_supported
        .iter()
        .map(|kind| (*kind).to_string())
        .collect::<Vec<_>>();
    for schema in &options.command_schemas {
        if schema.supports_continuation(field) && !supported.iter().any(|kind| kind == &schema.kind)
        {
            supported.push(schema.kind.clone());
        }
    }
    supported
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

    fn validate_subscriptions_form(&mut self, expr: &Expr) {
        let ExprKind::List(items) = &expr.kind else {
            return;
        };
        if items.len() < 4
            || !matches_symbol(&items[0], "defn")
            || !matches_symbol(&items[1], "subscriptions")
        {
            return;
        }

        if let Some(body) = items.last() {
            self.validate_subscription_expr(body);
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
                self.validate_update_match_result(expr, items);
            }
            ExprKind::List(items) if matches_head(items, "do") => {
                if let Some(last) = items.last() {
                    self.validate_update_result(last);
                }
            }
            ExprKind::List(items) if matches_head(items, "scope-update") => {}
            ExprKind::List(items) if self.is_update_result_helper_call(items) => {}
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

    fn validate_update_match_result(&mut self, expr: &Expr, items: &[Expr]) {
        if items.len() % 2 != 0 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                "match in update must contain pattern/body pairs",
            ));
            return;
        }

        let helper_match = match &items[1].kind {
            ExprKind::List(call_items) => self.is_update_result_helper_call(call_items),
            _ => false,
        };

        for arm in items[2..].chunks(2) {
            let [pattern, body] = arm else {
                continue;
            };
            if helper_match && update_result_binding_returned(pattern, body) {
                continue;
            }
            self.validate_update_result(body);
        }
    }

    fn is_update_result_helper_call(&self, items: &[Expr]) -> bool {
        function_call_name(items)
            .is_some_and(|name| self.imported_update_result_helpers.contains(name))
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
            ExprKind::List(items) if matches_head(items, "Cmd.map") => {
                if let Some(command) = items.get(1) {
                    self.validate_command_expr(command);
                }
            }
            ExprKind::List(items) => {
                if let Some(name) = function_call_name(items) {
                    if is_builtin_command_helper(name) || self.options.is_command_helper(name) {
                        return;
                    }
                    if name == "Task.perform" {
                        self.validate_task_perform_expr(expr, items);
                        return;
                    }
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
            ExprKind::Symbol(name) if name == "Cmd.none" => {}
            ExprKind::Symbol(name) => self.validate_command_symbol(name, expr.span),
            _ => self.diagnostics.push(Diagnostic::error(
                expr.span,
                "command position must be command data such as {:kind :none}",
            )),
        }
    }

    fn validate_subscription_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Map(entries) => validate_subscription_map(self, expr.span, entries),
            ExprKind::Vector(items) => {
                for item in items {
                    self.validate_subscription_expr(item);
                }
            }
            ExprKind::List(items) if matches_head(items, "if") && items.len() == 4 => {
                self.validate_subscription_expr(&items[2]);
                self.validate_subscription_expr(&items[3]);
            }
            ExprKind::List(items) if matches_head(items, "match") && items.len() >= 4 => {
                self.validate_match_bodies(
                    expr,
                    items,
                    "subscriptions",
                    Self::validate_subscription_expr,
                );
            }
            ExprKind::List(items) if matches_head(items, "let") => {
                self.with_let_command_scope(items, |validator, body| {
                    validator.validate_subscription_expr(body);
                });
            }
            ExprKind::List(items) if matches_head(items, "do") => {
                if let Some(last) = items.last() {
                    self.validate_subscription_expr(last);
                }
            }
            ExprKind::List(items) => {
                if let Some(name) = function_call_name(items) {
                    if self.options.is_subscription_helper(name) {
                        return;
                    }
                    if self.imported_subscription_helpers.contains(name) {
                        return;
                    }
                }
                self.diagnostics.push(Diagnostic::error(
                    expr.span,
                    "subscriptions must return configured subscription data",
                ));
            }
            ExprKind::Symbol(name) if self.options.is_subscription_symbol(name) => {}
            ExprKind::Symbol(_) => {}
            _ => self.diagnostics.push(Diagnostic::error(
                expr.span,
                "subscriptions must return configured subscription data",
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

    fn validate_task_perform_expr(&mut self, expr: &Expr, items: &[Expr]) {
        if items.len() != 4 && items.len() != 5 {
            self.diagnostics.push(Diagnostic::error(
                expr.span,
                format!(
                    "Task.perform expects 3 or 4 arguments, found {}",
                    items.len().saturating_sub(1)
                ),
            ));
        }
    }
}

fn is_builtin_command_helper(name: &str) -> bool {
    matches!(
        name,
        "Cmd.map"
            | "Cmd.batch"
            | "Cmd.time/now"
            | "Cmd.random/number"
            | "Cmd.timer/every"
            | "Cmd.timer/after"
            | "Cmd.timer/cancel"
            | "Cmd.animation/frame"
            | "Cmd.animation/cancel"
    )
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

    if !validator.is_known_command_kind(&kind) {
        validator.diagnostics.push(Diagnostic::error(
            kind_expr.span,
            format!("unknown command kind :{}", kind),
        ));
        return;
    }

    let registered_schema = validator.command_schema_rule(&kind);

    if matches!(kind.as_str(), "none" | "batch") {
        reject_structural_command_continuations(span, entries, &kind, &mut validator.diagnostics);
    } else if registered_schema
        .as_ref()
        .is_some_and(|schema| schema.reject_success_continuations)
    {
        reject_change_command_success_continuations(
            span,
            entries,
            &kind,
            &mut validator.diagnostics,
        );
        reject_unsupported_continuation_fields(
            span,
            entries,
            &kind,
            &validator.options,
            &mut validator.diagnostics,
        );
    } else {
        reject_conflicting_success_command_fields(span, entries, &kind, &mut validator.diagnostics);
        if is_builtin_payloadless_command(&kind) {
            reject_payloadless_success_continuations(
                span,
                entries,
                &kind,
                &mut validator.diagnostics,
            );
        }
        reject_unsupported_continuation_fields(
            span,
            entries,
            &kind,
            &validator.options,
            &mut validator.diagnostics,
        );
    }

    if let Some(schema) = registered_schema {
        validate_registered_command_schema(validator, span, entries, &schema);
        return;
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

    if kind == "task/perform" {
        require_command_fields(
            span,
            entries,
            &["task", "onSuccess", "onError"],
            "task/perform",
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
        "random/number" => {
            require_success_command_field(span, entries, &kind, &mut validator.diagnostics);
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
        _ => {}
    }
}

fn validate_subscription_map(
    validator: &mut EffectValidator<'_>,
    span: Span,
    entries: &[(Expr, Expr)],
) {
    let Some(kind_expr) = map_get(entries, "kind") else {
        validator.diagnostics.push(Diagnostic::error(
            span,
            "subscription record is missing a :kind field",
        ));
        return;
    };

    let Some(kind) = command_kind_literal(kind_expr) else {
        return;
    };

    if !validator.is_known_subscription_kind(&kind) {
        validator.diagnostics.push(Diagnostic::error(
            kind_expr.span,
            format!("unknown subscription kind :{}", kind),
        ));
        return;
    }

    if let Some(schema) = validator.subscription_schema_rule(&kind) {
        let required = schema
            .required_fields
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        require_command_fields(span, entries, &required, &kind, &mut validator.diagnostics);

        if !schema.collection_fields.is_empty() {
            let subscriptions = schema
                .collection_fields
                .iter()
                .find_map(|field| map_get(entries, field));
            let Some(subscriptions) = subscriptions else {
                let label = schema
                    .collection_fields
                    .first()
                    .map(String::as_str)
                    .unwrap_or("subscriptions");
                validator.diagnostics.push(Diagnostic::error(
                    span,
                    format!("{} subscription is missing a :{} vector", kind, label),
                ));
                return;
            };
            match &subscriptions.kind {
                ExprKind::Vector(items) | ExprKind::List(items) => {
                    for item in items {
                        validator.validate_subscription_expr(item);
                    }
                }
                _ => validator.diagnostics.push(Diagnostic::error(
                    subscriptions.span,
                    format!(
                        "{} subscription collection must be a vector of subscription records",
                        kind
                    ),
                )),
            }
        }
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

fn collect_forbidden_access(
    expr: &Expr,
    options: &EffectOptions,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &expr.kind {
        ExprKind::HtmlTemplate(node) => {
            if let Some(message) = options.forbidden_form_message(ForbiddenFormKind::HtmlTemplate) {
                diagnostics.push(Diagnostic::error(expr.span, message.to_string()));
            }
            collect_forbidden_access_html_node(node, options, diagnostics)
        }
        ExprKind::Symbol(name) => {
            if let Some(message) = options.forbidden_symbol_message(name) {
                diagnostics.push(Diagnostic::error(expr.span, message));
            }
        }
        ExprKind::List(items) | ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_forbidden_access(item, options, diagnostics);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_forbidden_access(key, options, diagnostics);
                collect_forbidden_access(value, options, diagnostics);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_forbidden_access(inner, options, diagnostics),
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn collect_forbidden_access_html_node(
    node: &syntax::HtmlNode,
    options: &EffectOptions,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match node {
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_forbidden_access(expr, options, diagnostics);
                }
            }
            for child in &element.children {
                collect_forbidden_access_html_node(child, options, diagnostics);
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => collect_forbidden_access(expr, options, diagnostics),
        syntax::HtmlNode::Text { .. } => {}
    }
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

fn update_result_binding_returned(pattern: &Expr, body: &Expr) -> bool {
    let ExprKind::List(items) = &pattern.kind else {
        return false;
    };
    if items.len() != 2 || !matches_head(items, "some") {
        return false;
    }
    let Some(binding) = symbol_name(&items[1]) else {
        return false;
    };
    symbol_name(body).is_some_and(|body| body == binding)
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
            | "timer/after"
            | "timer/every"
            | "timer/cancel"
            | "animation/frame"
            | "animation/cancel"
            | "time/now"
            | "random/number"
            | "task/perform"
            | "http/request"
    )
}

pub fn is_known_subscription_kind(kind: &str) -> bool {
    matches!(kind, "none")
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

    fn validate_with_forbidden_host_access(source: &SourceFile) -> EffectReport {
        validate_purity_with_options(
            source,
            &std::collections::HashSet::new(),
            forbidden_host_access_options(),
        )
    }

    fn forbidden_host_access_options() -> EffectOptions {
        let mut options = EffectOptions::default();
        for symbol in [
            "fetch",
            "sessionStorage",
            "setInterval",
            "requestAnimationFrame",
            "browser-current-url",
            "browser-theme-initial",
            "auth-storage-load",
            "selected-file-by-test-id",
            "has-selected-file",
            "multipart-form-body",
        ] {
            options = options.forbid_symbol(
                SymbolPattern::exact(symbol),
                "`{symbol}` is a host API; pure code must return typed command data instead",
            );
        }
        for prefix in ["window.", "document.", "location."] {
            options = options.forbid_symbol(
                SymbolPattern::prefix(prefix),
                "`{symbol}` is a host API; pure code must return typed command data instead",
            );
        }
        options = options.forbid_symbol(
            SymbolPattern::exact("event.preventDefault"),
            "`{symbol}` mutates host event data; return typed data instead",
        );
        options
    }

    fn browser_command_schema_options() -> EffectOptions {
        browser_command_type_options()
            .command_schema(
                EffectCommandSchemaRule::new("browser/history-replace-search-param")
                    .required_fields(["name", "value"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("browser/history-write-route").required_fields([
                    "url",
                    "op",
                    "definition",
                ]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("browser/theme-load")
                    .required_fields(["key"])
                    .require_success(),
            )
            .command_schema(
                EffectCommandSchemaRule::new("browser/theme-apply")
                    .required_fields(["theme", "key"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("browser/clipboard-write").required_fields(["text"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("browser/set-cookie")
                    .required_fields(["name", "value"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("storage/get")
                    .required_fields(["key"])
                    .require_success(),
            )
            .command_schema(
                EffectCommandSchemaRule::new("storage/set").required_fields(["key", "value"]),
            )
            .command_schema(EffectCommandSchemaRule::new("storage/remove").required_fields(["key"]))
            .command_schema(
                EffectCommandSchemaRule::new("auth-storage/load")
                    .required_fields(["sourceUrl"])
                    .require_success(),
            )
            .command_schema(
                EffectCommandSchemaRule::new("auth-storage/persist")
                    .required_fields(["sourceUrl", "entries"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("file/download").required_fields(["name", "content"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("file/import")
                    .require_success()
                    .supported_continuations(["onCancel"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("file/read-selected")
                    .required_fields(["ref"])
                    .require_success()
                    .supported_continuations(["onCancel"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("bluetooth/request-device")
                    .require_success()
                    .one_of_fields(["options", "filters", "acceptAllDevices"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("bluetooth/connect-heart-rate")
                    .required_fields(["id", "onReading"])
                    .require_success()
                    .one_of_fields(["options", "filters", "acceptAllDevices"])
                    .supported_continuations(["onReading", "onDisconnected"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("bluetooth/disconnect")
                    .required_fields(["id"])
                    .payloadless_success(),
            )
            .command_schema(
                EffectCommandSchemaRule::new("simulation/heart-rate")
                    .required_fields(["id", "onReading"])
                    .require_success()
                    .supported_continuations(["onReading", "onDisconnected"]),
            )
            .command_schema(EffectCommandSchemaRule::new("simulation/stop").required_fields(["id"]))
            .command_schema(
                EffectCommandSchemaRule::new("canvas/draw").required_fields(["ref", "ops"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("canvas/measure-text")
                    .required_fields(["ref"])
                    .require_success()
                    .one_of_fields(["text", "texts"]),
            )
            .command_schema(EffectCommandSchemaRule::new("dom-ref/focus").required_fields(["ref"]))
            .command_schema(EffectCommandSchemaRule::new("dom-ref/click").required_fields(["ref"]))
            .command_schema(
                EffectCommandSchemaRule::new("dom-ref/measure")
                    .required_fields(["ref"])
                    .require_success(),
            )
            .command_schema(
                EffectCommandSchemaRule::new("dom/scroll-into-view")
                    .one_of_fields(["selector", "testId", "id"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("dom-ref/resize-watch")
                    .required_fields(["ref", "onChange"])
                    .reject_success_continuations()
                    .supported_continuations(["onChange"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("dom-ref/resize-unwatch").one_of_fields(["id", "ref"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("window/event-watch")
                    .required_fields(["type", "onEvent"])
                    .supported_continuations(["onEvent"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("window/event-unwatch").one_of_fields(["id", "type"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("media-query/watch")
                    .required_fields(["query", "onChange"])
                    .reject_success_continuations()
                    .supported_continuations(["onChange"]),
            )
            .command_schema(
                EffectCommandSchemaRule::new("media-query/unwatch").one_of_fields(["id", "query"]),
            )
            .command_helper("Cmd.storage/get")
            .command_helper("Cmd.storage/set")
            .command_helper("Cmd.storage/set-silent")
            .command_helper("Cmd.dom-ref/click")
            .command_helper("Cmd.dom-ref/focus")
            .command_helper("Cmd.file/read-selected")
            .command_helper("Cmd.file/download")
            .command_helper("Cmd.canvas/draw")
            .command_helper("Cmd.dom-ref/measure")
            .command_helper("Cmd.dom-ref/resize-watch")
            .command_helper("Cmd.bluetooth/connect-heart-rate")
            .command_helper("Cmd.bluetooth/disconnect")
            .command_helper("Cmd.simulation/heart-rate")
            .command_helper("Cmd.simulation/stop")
            .subscription_helper("Sub.batch")
            .subscription_helper("Sub.timer/every")
            .subscription_helper("Sub.media-query")
            .subscription_helper("Sub.window/event")
            .subscription_helper("Sub.window/event-with")
            .subscription_helper("Sub.dom-ref/resize")
            .subscription_symbol("Sub.none")
            .subscription_schema(EffectSubscriptionSchemaRule::new("none"))
            .subscription_schema(
                EffectSubscriptionSchemaRule::new("batch")
                    .collection_fields(["subscriptions", "subs"]),
            )
            .subscription_schema(
                EffectSubscriptionSchemaRule::new("sub/timer/every")
                    .required_fields(["id", "ms", "msg"]),
            )
            .subscription_schema(
                EffectSubscriptionSchemaRule::new("sub/dom-ref/resize")
                    .required_fields(["ref", "onChange"]),
            )
            .subscription_schema(
                EffectSubscriptionSchemaRule::new("sub/window/event")
                    .required_fields(["type", "onEvent"]),
            )
            .subscription_schema(
                EffectSubscriptionSchemaRule::new("sub/media-query")
                    .required_fields(["query", "onChange"]),
            )
    }

    fn browser_command_type_options() -> EffectOptions {
        EffectOptions::default()
            .command_type(CommandType {
                name: "Bluetooth",
                payload_type: "BluetoothRequest",
                message_type: "msg",
            })
            .command_type(CommandType {
                name: "Storage",
                payload_type: "StorageRequest",
                message_type: "msg",
            })
            .command_type(CommandType {
                name: "Canvas",
                payload_type: "CanvasRequest",
                message_type: "msg",
            })
            .command_type(CommandType {
                name: "DomRef",
                payload_type: "DomRefRequest",
                message_type: "msg",
            })
            .command_type(CommandType {
                name: "MediaQuery",
                payload_type: "MediaQueryRequest",
                message_type: "msg",
            })
            .command_type(CommandType {
                name: "File",
                payload_type: "FileRequest",
                message_type: "msg",
            })
            .command_type(CommandType {
                name: "Window",
                payload_type: "WindowRequest",
                message_type: "msg",
            })
    }

    fn validate_browser_purity(source: &SourceFile) -> EffectReport {
        validate_purity_with_options(source, &HashSet::new(), browser_command_schema_options())
    }

    #[test]
    fn flags_configured_direct_host_api_access() {
        let source = syntax::parse_source("(def bad fetch)");
        let report = validate_with_forbidden_host_access(&source);

        assert_eq!(report.commands.len(), 6);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("typed command"))
        );
    }

    #[test]
    fn flags_configured_dotted_host_api_access() {
        let source = syntax::parse_source("(def width window.innerWidth)");
        let report = validate_with_forbidden_host_access(&source);

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
    fn flags_additional_configured_host_globals() {
        let source = syntax::parse_source(
            "(def session sessionStorage)\n\
             (def path location.pathname)\n\
             (def timer setInterval)\n\
             (def frame requestAnimationFrame)",
        );
        let report = validate_with_forbidden_host_access(&source);

        for name in [
            "sessionStorage",
            "location.pathname",
            "setInterval",
            "requestAnimationFrame",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(name)),
                "missing diagnostic for {name}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn flags_configured_legacy_host_intrinsics() {
        let source = syntax::parse_source(
            "(def current (browser-current-url))\n\
             (def theme (browser-theme-initial \"theme\"))\n\
             (def auth (auth-storage-load \"/docs\"))\n\
             (def file (selected-file-by-test-id \"upload\"))\n\
             (def has-file (has-selected-file \"upload\"))\n\
             (def form (multipart-form-body [] {}))",
        );
        let report = validate_with_forbidden_host_access(&source);

        for name in [
            "browser-current-url",
            "browser-theme-initial",
            "auth-storage-load",
            "selected-file-by-test-id",
            "has-selected-file",
            "multipart-form-body",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(name)),
                "missing diagnostic for {name}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn flags_configured_host_api_access_inside_html_expression() {
        let source = syntax::parse_source("(defn view [state] #html <p>{document.title}</p>)");
        let report = validate_with_forbidden_host_access(&source);

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
    fn flags_configured_host_api_access_inside_html_event_handler() {
        let source = syntax::parse_source(
            "(defn view [state] #html <button on:click={(fetch \"/api/workouts\")}>Load</button>)",
        );
        let report = validate_with_forbidden_host_access(&source);

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
    fn flags_configured_event_mutation_inside_html_event_handler() {
        let source = syntax::parse_source(
            "(defn view [state] #html <button on:click={(do (event.preventDefault) {:kind :clicked})}>Load</button>)",
        );
        let report = validate_with_forbidden_host_access(&source);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("event.preventDefault")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn validates_update_command_records() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  (if (= msg :start)\n      [state {:kind :timer/after :ms 1000 :msg :tick}]\n      [state {:kind :none}]))",
        );
        let report = validate_purity_with_options(
            &source,
            &std::collections::HashSet::new(),
            browser_command_schema_options(),
        );

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_command_helpers_as_command_data() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state (Cmd.batch [Cmd.none\n                         (Cmd.dom-ref/measure \"track\" (fn [rect] {:kind :measured :left rect.left}) :measure-failed)\n                         (Cmd.bluetooth/connect-heart-rate \"hr\" {:filters [{:services [\"heart_rate\"]}]} (fn [info] {:kind :connected :info info}) :heart-rate :disconnected :failed)\n                         (Cmd.simulation/heart-rate \"sim\" {:ms 1000 :min 90 :max 160 :jitter 3 :start 120 :deviceName \"Sim\"} (fn [info] {:kind :connected :info info}) :heart-rate :failed)\n                         {:kind :timer/after :id \"tick\" :ms 1000 :msg :tick}])])",
        );
        let report = validate_purity_with_options(
            &source,
            &std::collections::HashSet::new(),
            browser_command_schema_options(),
        );

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_subscription_helpers_as_subscription_data() {
        let source = syntax::parse_source(
            "(defn subscriptions [state]\n  (Sub.batch [(Sub.window/event-with \"drag\" \"pointermove\" :move {:preventDefault true :options {:passive false}})\n              (Sub.window/event \"dev\" \"keydown\" :key {:passive true})]))",
        );
        let report = validate_purity_with_options(
            &source,
            &std::collections::HashSet::new(),
            browser_command_schema_options(),
        );

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_bluetooth_request_device_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :bluetooth/request-device :filters [{:services [\"heart_rate\"]}] :optionalServices [\"heart_rate\"] :onSuccess :connected :onError :bluetooth-error}])",
        );
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_bluetooth_request_device_without_selection_options() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :bluetooth/request-device :onSuccess :connected}])",
        );
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_bluetooth_heart_rate_connection_without_reading_message() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :bluetooth/connect-heart-rate :id \"hr\" :filters [{:services [\"heart_rate\"]}] :onSuccess :connected}])",
        );
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

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
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.message.contains(
            "dom-ref/resize-watch command dispatches changes through :onChange and does not support success continuations :onSuccess"
        )));

        let source = syntax::parse_source(
            "(defn init []\n  [{:mobile? false} {:kind :media-query/watch :query \"(max-width: 820px)\" :onChange :media-changed :msg :ready}])",
        );
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

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
        let report = validate_browser_purity(&source);

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
    fn validates_task_perform_as_command_boundary() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state (Task.perform (Http.get-text state.url)\n                              (fn [text] {:kind :loaded :value text})\n                              (fn [error] {:kind :failed :error error}))])",
        );
        let report = validate_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_effectful_init_command_records() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:label \"Loading\"} {:kind :storage/get :key \"heartRateExercise.log.v1\" :format :json :onSuccess :log-loaded :onError :log-load-failed}])",
        );
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_storage_get_without_key() {
        let source =
            syntax::parse_source("(defn init []\n  [{:label \"Loading\"} {:kind :storage/get}])");
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_browser_side_effect_command_records() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :batch\n          :commands [{:kind :browser/history-replace-search-param :name \"op\" :value nil}\n                     {:kind :browser/history-write-route :url \"/docs\" :op nil :definition nil}\n                     {:kind :browser/theme-load :key \"theme\" :onSuccess :theme-loaded}\n                     {:kind :browser/theme-apply :theme \"dark\" :key \"theme\"}\n                     {:kind :browser/clipboard-write :text \"copied\"}\n                     {:kind :browser/set-cookie :name \"token\" :value \"secret\"}\n                     {:kind :auth-storage/load :sourceUrl \"/docs\" :onSuccess :auth-loaded}\n                     {:kind :auth-storage/persist :sourceUrl \"/docs\" :entries {}}]}])",
        );
        let report = validate_purity_with_options(
            &source,
            &std::collections::HashSet::new(),
            browser_command_schema_options(),
        );

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
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_file_download_without_content() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :file/download :name \"exercise-log.json\"}])",
        );
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_file_read_selected_command_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :file/read-selected :ref \"import-file\" :format :json :onSuccess :imported :onError :import-failed}])",
        );
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_file_read_selected_without_ref() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :file/read-selected :onSuccess :imported}])",
        );
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_canvas_draw_without_ref() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :canvas/draw :ops [{:op :clear}]}])",
        );
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_canvas_measure_text_without_texts() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :canvas/measure-text :ref \"metrics-chart\" :onSuccess :labels-measured}])",
        );
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_dom_ref_click_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :dom-ref/click :ref \"import-file\" :msg :clicked}])",
        );
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_dom_ref_measure_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :dom-ref/measure :ref \"heart-chart\" :onSuccess :chart-measured :onError :measure-failed}])",
        );
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_dom_scroll_into_view_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :dom/scroll-into-view :testId \"operation-get:/pets\" :block \"start\" :behavior \"auto\" :msg :scrolled}])",
        );
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_dom_ref_measure_without_success_message() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :dom-ref/measure :ref \"heart-chart\"}])",
        );
        let report = validate_browser_purity(&source);

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
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_dom_ref_resize_watch_without_change_message() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:width 0} {:kind :dom-ref/resize-watch :ref \"heart-chart\"}])",
        );
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

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
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_window_event_watch_schema() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :window/event-watch :id \"drag\" :type \"pointermove\" :onEvent :pointer-moved :options {:passive true}}])",
        );
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_window_event_watch_without_message() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :window/event-watch :type \"pointermove\"}])",
        );
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

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
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn validates_media_query_watch_schema() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:mobile? false} {:kind :media-query/watch :id \"mobile\" :query \"(max-width: 700px)\" :onChange :media-changed}])",
        );
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_media_query_watch_without_change_message() {
        let source = syntax::parse_source(
            "(defn init []\n  [{:mobile? false} {:kind :media-query/watch :query \"(max-width: 700px)\"}])",
        );
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

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
        let report = validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_command_schema_options(),
        );

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
        let report = validate_browser_purity(&source);

        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[test]
    fn rejects_simulation_heart_rate_without_reading_message() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  [state {:kind :simulation/heart-rate :id \"sim\" :onSuccess :connected}])",
        );
        let report = validate_browser_purity(&source);

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
        let report = validate_browser_purity(&source);

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
