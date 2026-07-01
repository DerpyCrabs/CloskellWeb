use std::collections::{BTreeMap, BTreeSet, HashSet};

use syntax::{Expr, ExprKind, HtmlAttrValue, HtmlNode, Span};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrowserAppOptions {
    pub root_id: String,
    pub css: Option<String>,
}

pub fn configure_browser_app_emit_options(options: &mut js_emit::EmitOptions) {
    options.html_templates = browser_app_html_template_emit_options();
    options.omit_replaced_defn_exports = true;
    add_browser_emit_intrinsics(options);
}

pub fn browser_html_template_emit_options() -> js_emit::HtmlTemplateEmitOptions {
    js_emit::HtmlTemplateEmitOptions::enabled("createCompiledHtmlTemplateComponent")
}

fn browser_app_html_template_emit_options() -> js_emit::HtmlTemplateEmitOptions {
    js_emit::HtmlTemplateEmitOptions::enabled("createBrowserCompiledHtmlTemplateComponent")
}

pub fn browser_effect_options() -> effects::EffectOptions {
    reject_browser_direct_access(add_browser_effect_command_schemas(
        effects::EffectOptions::default(),
    ))
}

pub fn reject_browser_framework_access(options: effects::EffectOptions) -> effects::EffectOptions {
    let mut options = options.forbid_form(
        effects::ForbiddenFormKind::HtmlTemplate,
        "#html is a browser framework form; check or build with --target browser",
    );
    for prefix in ["Sub.", "Event."] {
        options = options.forbid_symbol(
            effects::SymbolPattern::prefix(prefix),
            "`{symbol}` is a browser framework helper; check or build with --target browser",
        );
    }
    for symbol in BROWSER_FRAMEWORK_SYMBOLS {
        options = options.forbid_symbol(
            effects::SymbolPattern::exact(*symbol),
            "`{symbol}` is a browser framework helper; check or build with --target browser",
        );
    }
    options
}

pub fn reject_browser_direct_access(options: effects::EffectOptions) -> effects::EffectOptions {
    let mut options = options;
    for symbol in EVENT_MUTATION_SYMBOLS {
        options = options.forbid_symbol(
            effects::SymbolPattern::exact(*symbol),
            "`{symbol}` mutates a browser event; return Event.prevent/Event.stop data instead",
        );
    }
    for prefix in EVENT_MUTATION_PREFIXES {
        options = options.forbid_symbol(
            effects::SymbolPattern::prefix(*prefix),
            "`{symbol}` mutates a browser event; return Event.prevent/Event.stop data instead",
        );
    }
    for symbol in BROWSER_API_SYMBOLS {
        options = options.forbid_symbol(
            effects::SymbolPattern::exact(*symbol),
            "`{symbol}` is a browser API; pure code must return typed command data instead",
        );
    }
    for prefix in BROWSER_API_PREFIXES {
        options = options.forbid_symbol(
            effects::SymbolPattern::prefix(*prefix),
            "`{symbol}` is a browser API; pure code must return typed command data instead",
        );
    }
    options
}

pub fn add_browser_effect_command_schemas(
    options: effects::EffectOptions,
) -> effects::EffectOptions {
    let options = BROWSER_EFFECT_COMMAND_TYPES
        .iter()
        .cloned()
        .fold(options, |options, command_type| {
            options.command_type(command_type)
        });
    let options = BROWSER_FRAMEWORK_SYMBOLS
        .iter()
        .filter(|symbol| symbol.starts_with("Cmd."))
        .fold(options, |options, helper| options.command_helper(*helper));
    let options = BROWSER_EFFECT_COMMAND_SCHEMAS
        .iter()
        .fold(options, |options, builder| {
            options.command_schema(builder())
        });
    add_browser_effect_subscription_schemas(options)
}

pub fn is_browser_command_kind(kind: &str) -> bool {
    BROWSER_COMMAND_KINDS.contains(&kind)
}

pub fn is_browser_subscription_kind(kind: &str) -> bool {
    BROWSER_SUBSCRIPTION_KINDS.contains(&kind)
}

pub fn add_browser_effect_subscription_schemas(
    options: effects::EffectOptions,
) -> effects::EffectOptions {
    let options = BROWSER_EFFECT_SUBSCRIPTION_HELPERS
        .iter()
        .fold(options, |options, helper| {
            options.subscription_helper(*helper)
        });
    let options = BROWSER_EFFECT_SUBSCRIPTION_SYMBOLS
        .iter()
        .fold(options, |options, symbol| {
            options.subscription_symbol(*symbol)
        });
    BROWSER_EFFECT_SUBSCRIPTION_SCHEMAS
        .iter()
        .fold(options, |options, builder| {
            options.subscription_schema(builder())
        })
}

pub fn reject_browser_typecheck_access(
    options: typecheck::CheckOptions,
) -> typecheck::CheckOptions {
    let mut options = options.forbid_html_templates(
        "#html is a browser framework form; check or build with --target browser",
    );
    for name in BROWSER_FRAMEWORK_TYPES {
        options = options.forbid_type(
            *name,
            "{type} is a browser framework type; check or build with --target browser",
        );
    }
    options
}

pub fn browser_typecheck_options() -> typecheck::CheckOptions {
    add_browser_typecheck_custom_calls(add_browser_typecheck_intrinsics(
        add_browser_typecheck_types(add_browser_subscription_schemas(
            add_browser_command_schemas(add_browser_typecheck_symbols(
                typecheck::CheckOptions::default(),
            )),
        )),
    ))
}

pub fn add_browser_typecheck_types(options: typecheck::CheckOptions) -> typecheck::CheckOptions {
    options
        .named_type("Html", 0)
        .named_type("TrustedHtml", 0)
        .named_type_alias("BrowserBoot", browser_boot_type())
        .html_templates_with_checker(
            html_type(),
            trusted_html_type(),
            check_browser_html_template,
        )
}

pub fn add_browser_typecheck_intrinsics(
    options: typecheck::CheckOptions,
) -> typecheck::CheckOptions {
    BROWSER_TYPECHECK_INTRINSICS
        .iter()
        .fold(options, |options, builder| {
            options.intrinsic_call(builder())
        })
}

pub fn add_browser_typecheck_symbols(options: typecheck::CheckOptions) -> typecheck::CheckOptions {
    options.symbol_type(
        "Sub.none",
        typecheck::CheckType::Sub(Box::new(typecheck::CheckType::fresh("msg"))),
    )
}

pub fn add_browser_typecheck_custom_calls(
    options: typecheck::CheckOptions,
) -> typecheck::CheckOptions {
    BROWSER_TYPECHECK_CUSTOM_CALLS
        .iter()
        .fold(options, |options, builder| options.custom_call(builder()))
}

pub fn add_browser_command_schemas(options: typecheck::CheckOptions) -> typecheck::CheckOptions {
    BROWSER_COMMAND_SCHEMAS
        .iter()
        .fold(options, |options, builder| {
            options.command_schema(builder())
        })
}

pub fn add_browser_subscription_schemas(
    options: typecheck::CheckOptions,
) -> typecheck::CheckOptions {
    BROWSER_SUBSCRIPTION_SCHEMAS
        .iter()
        .fold(options, |options, builder| {
            options.subscription_schema(builder())
        })
}

pub fn add_browser_emit_intrinsics(options: &mut js_emit::EmitOptions) {
    if !options.html_templates.enabled {
        options.html_templates = browser_html_template_emit_options();
    }
    options
        .symbol_reads
        .extend(BROWSER_EMIT_SYMBOL_READS.iter().map(|builder| builder()));
    options
        .intrinsic_calls
        .extend(BROWSER_EMIT_INTRINSICS.iter().map(|builder| builder()));
    options
        .custom_calls
        .extend(BROWSER_EMIT_CUSTOM_CALLS.iter().map(|builder| builder()));
}

const BROWSER_FRAMEWORK_TYPES: &[&str] = &["Html", "TrustedHtml", "BrowserBoot"];
type BrowserIntrinsicBuilder = fn() -> typecheck::IntrinsicCallRule;
type BrowserCustomCallBuilder = fn() -> typecheck::CustomCallRule;
type BrowserEmitSymbolReadBuilder = fn() -> js_emit::SymbolReadEmitRule;
type BrowserEmitBuilder = fn() -> js_emit::IntrinsicCallEmitRule;
type BrowserCustomEmitBuilder = fn() -> js_emit::CustomCallEmitRule;
type BrowserCommandSchemaBuilder = fn() -> typecheck::CommandSchemaRule;
type BrowserSubscriptionSchemaBuilder = fn() -> typecheck::SubscriptionSchemaRule;
type BrowserEffectCommandSchemaBuilder = fn() -> effects::EffectCommandSchemaRule;
type BrowserEffectSubscriptionSchemaBuilder = fn() -> effects::EffectSubscriptionSchemaRule;

const BROWSER_TYPECHECK_INTRINSICS: &[BrowserIntrinsicBuilder] = &[
    event_prevent_rule,
    event_stop_rule,
    event_prevent_stop_rule,
    scope_view_rule,
    render_to_string_rule,
    browser_current_url_rule,
    history_replace_search_param_rule,
    history_write_route_rule,
    browser_theme_initial_rule,
    browser_theme_toggle_rule,
    clipboard_text_rule,
    clipboard_write_rule,
    browser_set_cookie_rule,
    auth_storage_load_rule,
    auth_storage_persist_rule,
    selected_file_or_blob_rule,
    selected_file_by_test_id_rule,
    has_selected_file_rule,
    multipart_form_body_rule,
    urlencoded_form_body_rule,
    install_virtual_json_viewer_rule,
];

const BROWSER_TYPECHECK_CUSTOM_CALLS: &[BrowserCustomCallBuilder] = &[
    cmd_storage_get_rule,
    cmd_storage_set_rule,
    cmd_storage_set_silent_rule,
    cmd_dom_ref_click_rule,
    cmd_dom_ref_focus_rule,
    cmd_file_read_selected_rule,
    cmd_file_download_rule,
    cmd_canvas_draw_rule,
    cmd_dom_ref_measure_rule,
    cmd_bluetooth_connect_heart_rate_rule,
    cmd_bluetooth_disconnect_rule,
    cmd_simulation_heart_rate_rule,
    cmd_simulation_stop_rule,
    cmd_dom_ref_resize_watch_rule,
    sub_batch_rule,
    sub_timer_every_rule,
    sub_media_query_rule,
    sub_window_event_rule,
    sub_window_event_with_rule,
    sub_dom_ref_resize_rule,
];

const BROWSER_EMIT_INTRINSICS: &[BrowserEmitBuilder] = &[
    scope_view_emit_rule,
    render_to_string_emit_rule,
    browser_current_url_emit_rule,
    history_replace_search_param_emit_rule,
    history_write_route_emit_rule,
    browser_theme_initial_emit_rule,
    browser_theme_toggle_emit_rule,
    clipboard_text_emit_rule,
    clipboard_write_emit_rule,
    browser_set_cookie_emit_rule,
    auth_storage_load_emit_rule,
    auth_storage_persist_emit_rule,
    selected_file_or_blob_emit_rule,
    selected_file_by_test_id_emit_rule,
    has_selected_file_emit_rule,
    multipart_form_body_emit_rule,
    urlencoded_form_body_emit_rule,
    install_virtual_json_viewer_emit_rule,
];

const BROWSER_EMIT_SYMBOL_READS: &[BrowserEmitSymbolReadBuilder] = &[sub_none_emit_rule];

const BROWSER_EMIT_CUSTOM_CALLS: &[BrowserCustomEmitBuilder] = &[
    cmd_storage_get_emit_rule,
    cmd_storage_set_emit_rule,
    cmd_storage_set_silent_emit_rule,
    cmd_dom_ref_click_emit_rule,
    cmd_dom_ref_focus_emit_rule,
    cmd_file_read_selected_emit_rule,
    cmd_file_download_emit_rule,
    cmd_canvas_draw_emit_rule,
    cmd_dom_ref_measure_emit_rule,
    cmd_bluetooth_connect_heart_rate_emit_rule,
    cmd_bluetooth_disconnect_emit_rule,
    cmd_simulation_heart_rate_emit_rule,
    cmd_simulation_stop_emit_rule,
    cmd_dom_ref_resize_watch_emit_rule,
    sub_batch_emit_rule,
    sub_timer_every_emit_rule,
    sub_media_query_emit_rule,
    sub_window_event_emit_rule,
    sub_window_event_with_emit_rule,
    sub_dom_ref_resize_emit_rule,
];

const BROWSER_COMMAND_SCHEMAS: &[BrowserCommandSchemaBuilder] = &[
    history_replace_search_param_command_schema,
    history_write_route_command_schema,
    browser_theme_load_command_schema,
    browser_theme_apply_command_schema,
    browser_clipboard_write_command_schema,
    browser_set_cookie_command_schema,
    browser_history_push_command_schema,
    browser_history_replace_command_schema,
    browser_location_assign_command_schema,
    browser_open_url_command_schema,
    browser_download_url_command_schema,
    browser_document_title_command_schema,
    browser_scroll_to_command_schema,
    event_source_open_command_schema,
    event_source_close_command_schema,
    media_play_selector_command_schema,
    media_restore_current_time_command_schema,
    media_sync_audio_element_command_schema,
    dom_breadcrumb_adaptive_command_schema,
    dom_focus_selector_command_schema,
    dom_document_set_attribute_command_schema,
    storage_get_command_schema,
    storage_set_command_schema,
    storage_remove_command_schema,
    auth_storage_load_command_schema,
    auth_storage_persist_command_schema,
    file_download_command_schema,
    file_import_command_schema,
    file_read_selected_command_schema,
    file_read_blob_command_schema,
    bluetooth_request_device_command_schema,
    bluetooth_connect_heart_rate_command_schema,
    bluetooth_disconnect_command_schema,
    simulation_heart_rate_command_schema,
    simulation_stop_command_schema,
    canvas_draw_command_schema,
    canvas_measure_text_command_schema,
    dom_ref_focus_command_schema,
    dom_ref_click_command_schema,
    dom_ref_measure_command_schema,
    dom_input_set_selection_command_schema,
    dom_scroll_into_view_command_schema,
    dom_ref_resize_watch_command_schema,
    dom_ref_resize_unwatch_command_schema,
    window_event_watch_command_schema,
    window_event_unwatch_command_schema,
    media_query_watch_command_schema,
    media_query_unwatch_command_schema,
];

const BROWSER_SUBSCRIPTION_SCHEMAS: &[BrowserSubscriptionSchemaBuilder] = &[
    sub_simulation_heart_rate_subscription_schema,
    sub_bluetooth_connect_heart_rate_subscription_schema,
    sub_timer_every_subscription_schema,
    sub_dom_ref_resize_subscription_schema,
    sub_window_event_subscription_schema,
    sub_media_query_subscription_schema,
];

const BROWSER_EFFECT_COMMAND_SCHEMAS: &[BrowserEffectCommandSchemaBuilder] = &[
    history_replace_search_param_effect_schema,
    history_write_route_effect_schema,
    browser_theme_load_effect_schema,
    browser_theme_apply_effect_schema,
    browser_clipboard_write_effect_schema,
    browser_set_cookie_effect_schema,
    browser_history_push_effect_schema,
    browser_history_replace_effect_schema,
    browser_location_assign_effect_schema,
    browser_open_url_effect_schema,
    browser_download_url_effect_schema,
    browser_document_title_effect_schema,
    browser_scroll_to_effect_schema,
    event_source_open_effect_schema,
    event_source_close_effect_schema,
    media_play_selector_effect_schema,
    media_restore_current_time_effect_schema,
    media_sync_audio_element_effect_schema,
    dom_breadcrumb_adaptive_effect_schema,
    dom_focus_selector_effect_schema,
    dom_document_set_attribute_effect_schema,
    storage_get_effect_schema,
    storage_set_effect_schema,
    storage_remove_effect_schema,
    auth_storage_load_effect_schema,
    auth_storage_persist_effect_schema,
    file_download_effect_schema,
    file_import_effect_schema,
    file_read_selected_effect_schema,
    file_read_blob_effect_schema,
    bluetooth_request_device_effect_schema,
    bluetooth_connect_heart_rate_effect_schema,
    bluetooth_disconnect_effect_schema,
    simulation_heart_rate_effect_schema,
    simulation_stop_effect_schema,
    canvas_draw_effect_schema,
    canvas_measure_text_effect_schema,
    dom_ref_focus_effect_schema,
    dom_ref_click_effect_schema,
    dom_ref_measure_effect_schema,
    dom_input_set_selection_effect_schema,
    dom_scroll_into_view_effect_schema,
    dom_ref_resize_watch_effect_schema,
    dom_ref_resize_unwatch_effect_schema,
    window_event_watch_effect_schema,
    window_event_unwatch_effect_schema,
    media_query_watch_effect_schema,
    media_query_unwatch_effect_schema,
];

const BROWSER_EFFECT_COMMAND_TYPES: &[effects::CommandType] = &[
    effects::CommandType {
        name: "Bluetooth",
        payload_type: "BluetoothRequest",
        message_type: "msg",
    },
    effects::CommandType {
        name: "Storage",
        payload_type: "StorageRequest",
        message_type: "msg",
    },
    effects::CommandType {
        name: "Canvas",
        payload_type: "CanvasRequest",
        message_type: "msg",
    },
    effects::CommandType {
        name: "DomRef",
        payload_type: "DomRefRequest",
        message_type: "msg",
    },
    effects::CommandType {
        name: "MediaQuery",
        payload_type: "MediaQueryRequest",
        message_type: "msg",
    },
    effects::CommandType {
        name: "File",
        payload_type: "FileRequest",
        message_type: "msg",
    },
    effects::CommandType {
        name: "Window",
        payload_type: "WindowRequest",
        message_type: "msg",
    },
];

const BROWSER_EFFECT_SUBSCRIPTION_SCHEMAS: &[BrowserEffectSubscriptionSchemaBuilder] = &[
    sub_none_effect_schema,
    sub_batch_effect_schema,
    sub_simulation_heart_rate_effect_schema,
    sub_bluetooth_connect_heart_rate_effect_schema,
    sub_timer_every_effect_schema,
    sub_dom_ref_resize_effect_schema,
    sub_window_event_effect_schema,
    sub_media_query_effect_schema,
];

const BROWSER_COMMAND_KINDS: &[&str] = &[
    "browser/history-replace-search-param",
    "browser/history-write-route",
    "browser/theme-load",
    "browser/theme-apply",
    "browser/clipboard-write",
    "browser/set-cookie",
    "browser/history-push",
    "browser/history-replace",
    "browser/location-assign",
    "browser/open-url",
    "browser/download-url",
    "browser/document-title",
    "browser/scroll-to",
    "event-source/open",
    "event-source/close",
    "media/play-selector",
    "media/restore-current-time",
    "media/sync-audio-element",
    "dom/breadcrumb-adaptive",
    "dom/focus-selector",
    "dom/document-set-attribute",
    "storage/get",
    "storage/set",
    "storage/remove",
    "auth-storage/load",
    "auth-storage/persist",
    "file/download",
    "file/import",
    "file/read-selected",
    "file/read-blob",
    "bluetooth/request-device",
    "bluetooth/connect-heart-rate",
    "bluetooth/disconnect",
    "simulation/heart-rate",
    "simulation/stop",
    "canvas/draw",
    "canvas/measure-text",
    "dom-ref/focus",
    "dom-ref/click",
    "dom-ref/measure",
    "dom/input-set-selection",
    "dom/scroll-into-view",
    "dom-ref/resize-watch",
    "dom-ref/resize-unwatch",
    "window/event-watch",
    "window/event-unwatch",
    "media-query/watch",
    "media-query/unwatch",
];

const BROWSER_SUBSCRIPTION_KINDS: &[&str] = &[
    "batch",
    "sub/simulation/heart-rate",
    "sub/bluetooth/connect-heart-rate",
    "sub/timer/every",
    "sub/dom-ref/resize",
    "sub/window/event",
    "sub/media-query",
];

const BROWSER_EFFECT_SUBSCRIPTION_HELPERS: &[&str] = &[
    "Sub.batch",
    "Sub.timer/every",
    "Sub.media-query",
    "Sub.window/event",
    "Sub.window/event-with",
    "Sub.dom-ref/resize",
    "scope-subscriptions",
];

const BROWSER_EFFECT_SUBSCRIPTION_SYMBOLS: &[&str] = &["Sub.none"];

fn cmd_storage_get_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Cmd.storage/get", check_cmd_storage_get_call)
}

fn cmd_storage_set_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Cmd.storage/set", check_cmd_storage_set_call)
}

fn cmd_storage_set_silent_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Cmd.storage/set-silent", check_cmd_storage_set_silent_call)
}

fn cmd_dom_ref_click_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Cmd.dom-ref/click", check_cmd_dom_ref_click_call)
}

fn cmd_dom_ref_focus_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Cmd.dom-ref/focus", check_cmd_dom_ref_focus_call)
}

fn cmd_file_read_selected_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Cmd.file/read-selected", check_cmd_file_read_selected_call)
}

fn cmd_file_download_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Cmd.file/download", check_cmd_file_download_call)
}

fn cmd_canvas_draw_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Cmd.canvas/draw", check_cmd_canvas_draw_call)
}

fn cmd_dom_ref_measure_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Cmd.dom-ref/measure", check_cmd_dom_ref_measure_call)
}

fn cmd_bluetooth_connect_heart_rate_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new(
        "Cmd.bluetooth/connect-heart-rate",
        check_cmd_bluetooth_connect_heart_rate_call,
    )
}

fn cmd_bluetooth_disconnect_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new(
        "Cmd.bluetooth/disconnect",
        check_cmd_bluetooth_disconnect_call,
    )
}

fn cmd_simulation_heart_rate_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new(
        "Cmd.simulation/heart-rate",
        check_cmd_simulation_heart_rate_call,
    )
}

fn cmd_simulation_stop_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Cmd.simulation/stop", check_cmd_simulation_stop_call)
}

fn cmd_dom_ref_resize_watch_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new(
        "Cmd.dom-ref/resize-watch",
        check_cmd_dom_ref_resize_watch_call,
    )
}

fn check_cmd_storage_get_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 4 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            4,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    let payload = context.command_payload_type_from_format_expr(&args[1]);
    let ok = context.infer_expr(&args[2]);
    let ok_msg = context.infer_command_mapper_message(
        "storage/get",
        ok,
        payload,
        args[2].span,
        ":toMessage",
    );
    let err_msg = context.infer_command_tag_message("storage/get", "onError", &args[3]);
    typecheck::Type::Cmd(Box::new(context.join_types(ok_msg, err_msg, args[0].span)))
}

fn check_cmd_storage_set_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 3 && args.len() != 4 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            3,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    context.infer_expr(&args[1]);
    let msg = context.infer_expr(&args[2]);
    let msg = if let Some(error) = args.get(3) {
        let err_msg = context.infer_command_tag_message("storage/set", "onError", error);
        context.join_types(msg, err_msg, args[0].span)
    } else {
        msg
    };
    typecheck::Type::Cmd(Box::new(msg))
}

fn check_cmd_storage_set_silent_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 3 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            3,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    context.infer_expr(&args[1]);
    let err_msg = context.infer_command_tag_message("storage/set", "onError", &args[2]);
    typecheck::Type::Cmd(Box::new(err_msg))
}

fn check_cmd_dom_ref_click_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    check_cmd_dom_ref_action_call(context, name, args, "dom-ref/click")
}

fn check_cmd_dom_ref_focus_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    check_cmd_dom_ref_action_call(context, name, args, "dom-ref/focus")
}

fn check_cmd_dom_ref_action_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
    command_kind: &str,
) -> typecheck::Type {
    if args.len() != 2 && args.len() != 3 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            2,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    let msg = context.infer_expr(&args[1]);
    let msg = if let Some(error) = args.get(2) {
        let err_msg = context.infer_command_tag_message(command_kind, "onError", error);
        context.join_types(msg, err_msg, args[0].span)
    } else {
        msg
    };
    typecheck::Type::Cmd(Box::new(msg))
}

fn check_cmd_file_read_selected_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 5 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            5,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    let payload = context.command_payload_type_from_format_expr(&args[1]);
    let ok = context.infer_expr(&args[2]);
    let ok_msg = context.infer_command_mapper_message(
        "file/read-selected",
        ok,
        payload,
        args[2].span,
        ":toMessage",
    );
    let err_msg = context.infer_command_tag_message("file/read-selected", "onError", &args[3]);
    let cancel_msg = context.infer_command_tag_message("file/read-selected", "onCancel", &args[4]);
    let msg = context.join_types(ok_msg, err_msg, args[0].span);
    typecheck::Type::Cmd(Box::new(context.join_types(msg, cancel_msg, args[0].span)))
}

fn check_cmd_file_download_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 5 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            5,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    require_call_arg(context, &args[1], typecheck::Type::String);
    require_call_arg(context, &args[2], typecheck::Type::String);
    let msg = context.infer_expr(&args[3]);
    let err_msg = context.infer_command_tag_message("file/download", "onError", &args[4]);
    typecheck::Type::Cmd(Box::new(context.join_types(msg, err_msg, args[0].span)))
}

fn check_cmd_canvas_draw_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 5 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            5,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    require_call_arg(context, &args[1], typecheck::Type::Number);
    require_call_arg(context, &args[2], typecheck::Type::Number);
    context.infer_expr(&args[3]);
    let err_msg = context.infer_command_tag_message("canvas/draw", "onError", &args[4]);
    typecheck::Type::Cmd(Box::new(err_msg))
}

fn check_cmd_dom_ref_measure_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 3 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            3,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    let mapper_ty = context.infer_expr(&args[1]);
    let payload_ty = context.check_type(&browser_dom_ref_measure_payload_check_type());
    let ok_msg = context.infer_command_mapper_message(
        "dom-ref/measure",
        mapper_ty,
        payload_ty,
        args[1].span,
        ":toMessage",
    );
    let err_msg = context.infer_command_tag_message("dom-ref/measure", "onError", &args[2]);
    typecheck::Type::Cmd(Box::new(context.join_types(ok_msg, err_msg, args[0].span)))
}

fn check_cmd_bluetooth_connect_heart_rate_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 6 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            6,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    require_call_arg(context, &args[1], typecheck::Type::Record(BTreeMap::new()));
    let mapper_ty = context.infer_expr(&args[2]);
    let payload_ty = context.check_type(&browser_connected_payload_check_type());
    let ok_msg = context.infer_command_mapper_message(
        "bluetooth/connect-heart-rate",
        mapper_ty,
        payload_ty,
        args[2].span,
        ":toMessage",
    );
    let reading_msg =
        context.infer_command_tag_message("bluetooth/connect-heart-rate", "onReading", &args[3]);
    let disconnected_msg = context.infer_command_tag_message(
        "bluetooth/connect-heart-rate",
        "onDisconnected",
        &args[4],
    );
    let err_msg =
        context.infer_command_tag_message("bluetooth/connect-heart-rate", "onError", &args[5]);
    let msg = context.join_types(ok_msg, reading_msg, args[0].span);
    let msg = context.join_types(msg, disconnected_msg, args[0].span);
    typecheck::Type::Cmd(Box::new(context.join_types(msg, err_msg, args[0].span)))
}

fn check_cmd_bluetooth_disconnect_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 2 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            2,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    typecheck::Type::Cmd(Box::new(context.infer_expr(&args[1])))
}

fn check_cmd_simulation_heart_rate_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 5 && args.len() != 6 {
        context.diagnostic(
            args.first().map_or(Span::default(), |arg| arg.span),
            format!("{} expects 5 or 6 arguments, found {}", name, args.len()),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    require_call_arg(context, &args[1], typecheck::Type::Record(BTreeMap::new()));
    let mapper_ty = context.infer_expr(&args[2]);
    let payload_ty = context.check_type(&browser_connected_payload_check_type());
    let ok_msg = context.infer_command_mapper_message(
        "simulation/heart-rate",
        mapper_ty,
        payload_ty,
        args[2].span,
        ":toMessage",
    );
    let reading_msg =
        context.infer_command_tag_message("simulation/heart-rate", "onReading", &args[3]);
    let msg = context.join_types(ok_msg, reading_msg, args[0].span);
    let (msg, error_index) = if args.len() == 6 {
        let disconnected_msg =
            context.infer_command_tag_message("simulation/heart-rate", "onDisconnected", &args[4]);
        (context.join_types(msg, disconnected_msg, args[0].span), 5)
    } else {
        (msg, 4)
    };
    let err_msg =
        context.infer_command_tag_message("simulation/heart-rate", "onError", &args[error_index]);
    typecheck::Type::Cmd(Box::new(context.join_types(msg, err_msg, args[0].span)))
}

fn check_cmd_simulation_stop_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 1 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            1,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    require_call_arg(context, &args[0], typecheck::Type::String);
    typecheck::Type::Cmd(Box::new(context.fresh()))
}

fn sub_batch_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Sub.batch", check_sub_batch_call)
}

fn check_cmd_dom_ref_resize_watch_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 4 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            4,
            args.len(),
        );
        return typecheck::Type::Cmd(Box::new(context.fresh()));
    }
    let ref_ty = context.infer_expr(&args[0]);
    context.unify(typecheck::Type::String, ref_ty, args[0].span);
    let id_ty = context.infer_expr(&args[1]);
    context.unify(typecheck::Type::String, id_ty, args[1].span);
    let change_msg =
        context.infer_command_tag_message("dom-ref/resize-watch", "onChange", &args[2]);
    let err_msg = context.infer_command_tag_message("dom-ref/resize-watch", "onError", &args[3]);
    typecheck::Type::Cmd(Box::new(context.join_types(
        change_msg,
        err_msg,
        args[0].span,
    )))
}

fn sub_timer_every_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Sub.timer/every", check_sub_timer_every_call)
}

fn sub_media_query_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Sub.media-query", check_sub_media_query_call)
}

fn sub_window_event_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Sub.window/event", check_sub_window_event_call)
}

fn sub_window_event_with_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Sub.window/event-with", check_sub_window_event_with_call)
}

fn sub_dom_ref_resize_rule() -> typecheck::CustomCallRule {
    typecheck::CustomCallRule::new("Sub.dom-ref/resize", check_sub_dom_ref_resize_call)
}

fn check_sub_batch_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 1 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            1,
            args.len(),
        );
        return typecheck::Type::Sub(Box::new(context.fresh()));
    }

    let msg = context.fresh();
    let batch_ty = context.infer_expr(&args[0]);
    match context.resolve(batch_ty) {
        typecheck::Type::Vector(item) => {
            context.unify(
                typecheck::Type::Sub(Box::new(msg.clone())),
                *item,
                args[0].span,
            );
        }
        typecheck::Type::Tuple(items) => {
            for item in items {
                context.unify(
                    typecheck::Type::Sub(Box::new(msg.clone())),
                    item,
                    args[0].span,
                );
            }
        }
        typecheck::Type::Var(_) => {}
        other => {
            let found = context.format_type(&other);
            context.diagnostic(
                args[0].span,
                format!(
                    "Sub.batch expects a vector of subscriptions, found {}",
                    found
                ),
            );
        }
    }
    typecheck::Type::Sub(Box::new(context.resolve(msg)))
}

fn check_sub_timer_every_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 3 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            3,
            args.len(),
        );
        return typecheck::Type::Sub(Box::new(context.fresh()));
    }

    require_call_arg(context, &args[0], typecheck::Type::String);
    require_call_arg(context, &args[1], typecheck::Type::Number);
    let msg = context.infer_expr(&args[2]);
    typecheck::Type::Sub(Box::new(msg))
}

fn check_sub_media_query_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    check_sub_change_call(context, name, args, "sub/media-query")
}

fn check_sub_dom_ref_resize_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    check_sub_change_call(context, name, args, "sub/dom-ref/resize")
}

fn check_sub_change_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
    subscription_kind: &str,
) -> typecheck::Type {
    if args.len() != 3 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            3,
            args.len(),
        );
        return typecheck::Type::Sub(Box::new(context.fresh()));
    }

    require_call_arg(context, &args[0], typecheck::Type::String);
    require_call_arg(context, &args[1], typecheck::Type::String);
    let tag = context.infer_expr(&args[2]);
    let msg = subscription_tag_message_type(
        context,
        Some(subscription_kind),
        "onChange",
        tag,
        args[2].span,
    );
    typecheck::Type::Sub(Box::new(msg))
}

fn check_sub_window_event_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 3 && args.len() != 4 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            3,
            args.len(),
        );
        return typecheck::Type::Sub(Box::new(context.fresh()));
    }

    require_call_arg(context, &args[0], typecheck::Type::String);
    require_call_arg(context, &args[1], typecheck::Type::String);
    let tag = context.infer_expr(&args[2]);
    if let Some(options) = args.get(3) {
        context.infer_expr(options);
    }
    let msg = subscription_tag_message_type(
        context,
        Some("sub/window/event"),
        "onEvent",
        tag,
        args[2].span,
    );
    typecheck::Type::Sub(Box::new(msg))
}

fn check_sub_window_event_with_call(
    context: &mut typecheck::TypecheckCallContext<'_>,
    name: &str,
    args: &[Expr],
) -> typecheck::Type {
    if args.len() != 4 {
        context.arity_error(
            args.first().map_or(Span::default(), |arg| arg.span),
            name,
            4,
            args.len(),
        );
        return typecheck::Type::Sub(Box::new(context.fresh()));
    }

    require_call_arg(context, &args[0], typecheck::Type::String);
    require_call_arg(context, &args[1], typecheck::Type::String);
    let tag = context.infer_expr(&args[2]);
    let config_ty = context.infer_expr(&args[3]);
    context.unify(
        typecheck::Type::Record(BTreeMap::new()),
        config_ty,
        args[3].span,
    );
    let msg = subscription_tag_message_type(
        context,
        Some("sub/window/event"),
        "onEvent",
        tag,
        args[2].span,
    );
    typecheck::Type::Sub(Box::new(msg))
}

fn require_call_arg(
    context: &mut typecheck::TypecheckCallContext<'_>,
    arg: &Expr,
    expected: typecheck::Type,
) {
    let actual = context.infer_expr(arg);
    context.unify(expected, actual, arg.span);
}

fn subscription_tag_message_type(
    context: &mut typecheck::TypecheckCallContext<'_>,
    subscription_kind: Option<&str>,
    field: &str,
    tag_ty: typecheck::Type,
    span: Span,
) -> typecheck::Type {
    match context.resolve(tag_ty) {
        typecheck::Type::Var(_) | typecheck::Type::Keyword(None) => context.fresh(),
        typecheck::Type::Keyword(Some(tag)) => context.command_message_tag_type(
            subscription_command_kind(subscription_kind),
            field,
            &tag,
            &BTreeMap::new(),
        ),
        other => {
            let found = context.format_type(&other);
            context.diagnostic(
                span,
                format!(
                    "subscription continuation :{} must be a keyword tag, found {}",
                    field, found
                ),
            );
            context.fresh()
        }
    }
}

fn subscription_command_kind(kind: Option<&str>) -> Option<&'static str> {
    match kind {
        Some("sub/timer/every") => Some("timer/every"),
        Some("sub/dom-ref/resize") => Some("dom-ref/resize-watch"),
        Some("sub/window/event") => Some("window/event-watch"),
        Some("sub/media-query") => Some("media-query/watch"),
        _ => kind.and_then(|kind| match kind {
            "none" => Some("none"),
            "batch" => Some("batch"),
            _ => None,
        }),
    }
}

fn check_browser_html_template(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    node: &HtmlNode,
) {
    check_browser_html_node(context, node);
}

fn check_browser_html_node(context: &mut typecheck::HtmlTemplateCheckContext<'_>, node: &HtmlNode) {
    match node {
        HtmlNode::Element(element) => {
            for attr in &element.attrs {
                match &attr.value {
                    HtmlAttrValue::Dynamic { expr, .. } => {
                        if attr.name.starts_with("on:") {
                            let message_ty = context.with_locals(
                                [("event".to_string(), browser_dom_event_type())],
                                |context| context.infer_expr(expr),
                            );
                            if let Some(expected_msg) = context.update_message_type() {
                                require_browser_event_message_matches(
                                    context,
                                    expected_msg,
                                    message_ty,
                                    expr.span,
                                    &attr.name,
                                );
                            }
                        } else {
                            let attr_ty = context.infer_expr(expr);
                            if attr.name == "ref" {
                                require_browser_ref_attr(context, attr_ty, expr.span);
                            } else if attr.name == "class" {
                                require_browser_class_attr(context, attr_ty, expr.span);
                            } else if attr.name == "style" {
                                require_browser_style_attr(context, attr_ty, expr.span);
                            } else if attr.name == "innerHTML" {
                                require_browser_inner_html_attr(context, attr_ty, expr.span);
                            } else if is_boolean_html_attr(&attr.name) {
                                context.unify(attr_ty, typecheck::Type::Bool, expr.span);
                            }
                        }
                    }
                    HtmlAttrValue::Bool(_) | HtmlAttrValue::Static(_) => {
                        validate_static_browser_html_attr(
                            context,
                            &attr.name,
                            &attr.value,
                            attr.span,
                        );
                    }
                }
            }
            for child in &element.children {
                check_browser_html_node(context, child);
            }
        }
        HtmlNode::Expr { expr, .. } => check_browser_html_expr(context, expr),
        HtmlNode::Text { .. } => {}
    }
}

fn validate_static_browser_html_attr(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    name: &str,
    value: &HtmlAttrValue,
    span: Span,
) {
    match value {
        HtmlAttrValue::Bool(true) => match name {
            "ref" => context.diagnostic(
                span,
                "ref attribute requires a value; use ref=\"name\" or ref={...}",
            ),
            "class" => context.diagnostic(
                span,
                "class attribute requires a value; use class=\"...\" or class={...}",
            ),
            "style" => context.diagnostic(
                span,
                "style attribute requires a value; use style=\"...\" or style={...}",
            ),
            _ => {}
        },
        HtmlAttrValue::Static(static_value) => {
            if name == "innerHTML" {
                context.diagnostic(
                    span,
                    "innerHTML requires TrustedHtml; use innerHTML={...} with an explicit sanitizer or unsafe-cast boundary",
                );
            }

            if name == "ref" && static_value.is_empty() {
                context.diagnostic(span, "ref attribute requires a non-empty ref name");
            }

            if is_boolean_html_attr(name)
                && !static_value.is_empty()
                && !static_value.eq_ignore_ascii_case(name)
            {
                context.diagnostic(
                    span,
                    format!(
                        "boolean attribute {} ignores string value {:?}; use bare {} or {}={{...}}",
                        name, static_value, name, name
                    ),
                );
            }
        }
        HtmlAttrValue::Bool(false) | HtmlAttrValue::Dynamic { .. } => {}
    }
}

fn check_browser_html_expr(context: &mut typecheck::HtmlTemplateCheckContext<'_>, expr: &Expr) {
    if let Some(spec) = BrowserHtmlForSpec::parse(expr) {
        let collection_ty = context.infer_expr(spec.collection);
        let item_ty = context.infer_iterable_element(collection_ty, spec.collection.span);
        let mut locals = vec![(spec.item.to_string(), item_ty)];
        if let Some(index) = spec.index {
            locals.push((index.to_string(), typecheck::Type::Number));
        }
        context.with_locals(locals, |context| {
            context.infer_expr(spec.key);
            check_browser_html_template(context, spec.template);
        });
        return;
    }

    if let Some(spec) = BrowserHtmlIfSpec::parse(expr) {
        let condition_ty = context.infer_expr(spec.condition);
        context.unify(condition_ty, typecheck::Type::Bool, spec.condition.span);
        check_browser_html_template(context, spec.then_template);
        check_browser_html_template(context, spec.else_template);
        return;
    }

    context.infer_expr(expr);
}

fn require_browser_ref_attr(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
    span: Span,
) {
    let resolved = context.resolve(ty);
    if browser_ref_attr_type_matches(context, resolved.clone(), span) {
        return;
    }

    let found = context.format_type(&resolved);
    context.diagnostic(
        span,
        format!(
            "ref attribute expects a string, keyword, or nil optional ref name, found {}",
            found
        ),
    );
}

fn browser_ref_attr_type_matches(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
    span: Span,
) -> bool {
    match context.resolve(ty) {
        typecheck::Type::Var(id) => {
            context.unify(typecheck::Type::Var(id), typecheck::Type::String, span);
            true
        }
        typecheck::Type::String | typecheck::Type::Keyword(_) | typecheck::Type::Nil => true,
        typecheck::Type::Option(inner) => browser_ref_attr_type_matches(context, *inner, span),
        _ => false,
    }
}

fn require_browser_class_attr(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
    span: Span,
) {
    let resolved = context.resolve(ty);
    if browser_class_attr_type_matches(context, resolved.clone(), span) {
        return;
    }

    let found = context.format_type(&resolved);
    context.diagnostic(
        span,
        format!(
            "class attribute expects a CSS class string, keyword, nil, bool, structured collection, or class flag map, found {}",
            found
        ),
    );
}

fn browser_class_attr_type_matches(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
    span: Span,
) -> bool {
    match context.resolve(ty) {
        typecheck::Type::Var(_)
        | typecheck::Type::String
        | typecheck::Type::Keyword(_)
        | typecheck::Type::Bool
        | typecheck::Type::Nil => true,
        typecheck::Type::Option(inner) => browser_class_attr_type_matches(context, *inner, span),
        typecheck::Type::List(inner)
        | typecheck::Type::Vector(inner)
        | typecheck::Type::Set(inner) => browser_class_attr_type_matches(context, *inner, span),
        typecheck::Type::Tuple(items) => items
            .into_iter()
            .all(|item| browser_class_attr_type_matches(context, item, span)),
        typecheck::Type::Record(fields) => fields
            .into_values()
            .all(|field_ty| browser_class_flag_type_matches(context, field_ty, span)),
        typecheck::Type::Map(key, value) => {
            browser_class_key_type_matches(context, *key)
                && browser_class_flag_type_matches(context, *value, span)
        }
        _ => false,
    }
}

fn browser_class_key_type_matches(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
) -> bool {
    matches!(
        context.resolve(ty),
        typecheck::Type::Var(_) | typecheck::Type::String | typecheck::Type::Keyword(_)
    )
}

fn browser_class_flag_type_matches(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
    span: Span,
) -> bool {
    match context.resolve(ty) {
        typecheck::Type::Var(id) => {
            context.unify(typecheck::Type::Var(id), typecheck::Type::Bool, span);
            true
        }
        typecheck::Type::Bool | typecheck::Type::Nil => true,
        typecheck::Type::Option(inner) => browser_class_flag_type_matches(context, *inner, span),
        _ => false,
    }
}

fn require_browser_inner_html_attr(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
    span: Span,
) {
    let resolved = context.resolve(ty);
    if browser_inner_html_attr_type_matches(context, resolved.clone(), span) {
        return;
    }

    let found = context.format_type(&resolved);
    context.diagnostic(
        span,
        format!(
            "innerHTML expects TrustedHtml from an explicit sanitizer or unsafe-cast boundary, found {}",
            found
        ),
    );
}

fn browser_inner_html_attr_type_matches(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
    span: Span,
) -> bool {
    match context.resolve(ty) {
        typecheck::Type::Var(id) => {
            let trusted = context.trusted_html_type();
            context.unify(typecheck::Type::Var(id), trusted, span);
            true
        }
        typecheck::Type::Nil => true,
        typecheck::Type::Option(inner) => {
            browser_inner_html_attr_type_matches(context, *inner, span)
        }
        resolved => resolved == context.trusted_html_type(),
    }
}

fn require_browser_style_attr(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
    span: Span,
) {
    let resolved = context.resolve(ty);
    if browser_style_attr_type_matches(context, resolved.clone()) {
        return;
    }

    let found = context.format_type(&resolved);
    context.diagnostic(
        span,
        format!(
            "style attribute expects a CSS string, nil, record, or map with style property values, found {}",
            found
        ),
    );
}

fn browser_style_attr_type_matches(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
) -> bool {
    match context.resolve(ty) {
        typecheck::Type::Var(_) | typecheck::Type::String | typecheck::Type::Nil => true,
        typecheck::Type::Option(inner) => browser_style_attr_type_matches(context, *inner),
        typecheck::Type::Record(fields) => fields
            .into_values()
            .all(|field_ty| browser_style_value_type_matches(context, field_ty)),
        typecheck::Type::Map(key, value) => {
            browser_style_key_type_matches(context, *key)
                && browser_style_value_type_matches(context, *value)
        }
        _ => false,
    }
}

fn browser_style_key_type_matches(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
) -> bool {
    match context.resolve(ty) {
        typecheck::Type::Var(_) | typecheck::Type::String | typecheck::Type::Keyword(_) => true,
        typecheck::Type::Option(inner) => browser_style_key_type_matches(context, *inner),
        _ => false,
    }
}

fn browser_style_value_type_matches(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    ty: typecheck::Type,
) -> bool {
    match context.resolve(ty) {
        typecheck::Type::Var(_)
        | typecheck::Type::String
        | typecheck::Type::Number
        | typecheck::Type::Bool
        | typecheck::Type::Nil => true,
        typecheck::Type::Option(inner) => browser_style_value_type_matches(context, *inner),
        _ => false,
    }
}

fn require_browser_event_message_matches(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    expected_msg: typecheck::Type,
    actual_msg: typecheck::Type,
    span: Span,
    event_name: &str,
) {
    if browser_event_message_matches(context, expected_msg.clone(), actual_msg.clone(), span) {
        return;
    }

    let expected = context.format_type_with_literals(&expected_msg);
    let actual = context.format_type_with_literals(&actual_msg);
    context.diagnostic(
        span,
        format!(
            "template event {} message has type {}, which is not part of update message type {}",
            event_name, actual, expected
        ),
    );
}

fn browser_event_message_matches(
    context: &mut typecheck::HtmlTemplateCheckContext<'_>,
    expected: typecheck::Type,
    actual: typecheck::Type,
    span: Span,
) -> bool {
    match context.resolve(actual) {
        typecheck::Type::Nil => true,
        typecheck::Type::Option(inner) => {
            browser_event_message_matches(context, expected, *inner, span)
        }
        typecheck::Type::Event(inner) => {
            browser_event_message_matches(context, expected, *inner, span)
        }
        actual => context.command_message_matches(expected, actual, span),
    }
}

fn browser_dom_event_type() -> typecheck::Type {
    let form_target = typecheck::Type::Record(BTreeMap::from([
        ("checked".to_string(), typecheck::Type::Bool),
        ("value".to_string(), typecheck::Type::String),
        ("valueAsNumber".to_string(), typecheck::Type::Number),
    ]));
    typecheck::Type::Record(BTreeMap::from([
        ("altKey".to_string(), typecheck::Type::Bool),
        ("clientX".to_string(), typecheck::Type::Number),
        ("clientY".to_string(), typecheck::Type::Number),
        ("ctrlKey".to_string(), typecheck::Type::Bool),
        ("currentTarget".to_string(), form_target.clone()),
        ("key".to_string(), typecheck::Type::String),
        ("metaKey".to_string(), typecheck::Type::Bool),
        ("shiftKey".to_string(), typecheck::Type::Bool),
        ("target".to_string(), form_target),
    ]))
}

struct BrowserHtmlForSpec<'a> {
    item: &'a str,
    index: Option<&'a str>,
    collection: &'a Expr,
    key: &'a Expr,
    template: &'a HtmlNode,
}

impl<'a> BrowserHtmlForSpec<'a> {
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

struct BrowserHtmlIfSpec<'a> {
    condition: &'a Expr,
    then_template: &'a HtmlNode,
    else_template: &'a HtmlNode,
}

impl<'a> BrowserHtmlIfSpec<'a> {
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
            | "itemscope"
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

fn event_prevent_rule() -> typecheck::IntrinsicCallRule {
    event_control_rule("Event.prevent")
}

fn event_stop_rule() -> typecheck::IntrinsicCallRule {
    event_control_rule("Event.stop")
}

fn event_prevent_stop_rule() -> typecheck::IntrinsicCallRule {
    event_control_rule("Event.prevent-stop")
}

fn event_control_rule(name: &'static str) -> typecheck::IntrinsicCallRule {
    typecheck::IntrinsicCallRule::new(
        name,
        vec![typecheck::IntrinsicCallOverload::new(
            vec![typecheck::IntrinsicParam::infer()],
            typecheck::CheckType::Event(Box::new(typecheck::CheckType::argument(0))),
        )],
    )
}

fn scope_view_rule() -> typecheck::IntrinsicCallRule {
    let child_state = typecheck::CheckType::fresh("child_state");
    typecheck::IntrinsicCallRule::new(
        "scope-view",
        vec![typecheck::IntrinsicCallOverload::new(
            vec![
                expect(typecheck::CheckType::Keyword),
                expect(typecheck::CheckType::function(
                    vec![child_state.clone()],
                    html_type(),
                )),
                expect(child_state),
            ],
            html_type(),
        )],
    )
}

fn render_to_string_rule() -> typecheck::IntrinsicCallRule {
    let state = typecheck::CheckType::fresh("state");
    typecheck::IntrinsicCallRule::new(
        "render-to-string",
        vec![
            typecheck::IntrinsicCallOverload::new(
                vec![expect(html_type())],
                typecheck::CheckType::String,
            ),
            typecheck::IntrinsicCallOverload::new(
                vec![
                    expect(typecheck::CheckType::function(
                        vec![state.clone()],
                        html_type(),
                    )),
                    expect(state),
                ],
                typecheck::CheckType::String,
            ),
        ],
    )
}

fn browser_current_url_rule() -> typecheck::IntrinsicCallRule {
    typecheck::IntrinsicCallRule::new(
        "browser-current-url",
        vec![typecheck::IntrinsicCallOverload::new(
            Vec::new(),
            typecheck::CheckType::String,
        )],
    )
}

fn history_replace_search_param_rule() -> typecheck::IntrinsicCallRule {
    fixed_string_args_rule("history-replace-search-param", 2, typecheck::CheckType::Nil)
}

fn history_write_route_rule() -> typecheck::IntrinsicCallRule {
    inferred_args_rule("history-write-route", 3, typecheck::CheckType::Nil)
}

fn browser_theme_initial_rule() -> typecheck::IntrinsicCallRule {
    fixed_string_args_rule("browser-theme-initial", 1, typecheck::CheckType::String)
}

fn browser_theme_toggle_rule() -> typecheck::IntrinsicCallRule {
    fixed_string_args_rule("browser-theme-toggle", 2, typecheck::CheckType::String)
}

fn clipboard_text_rule() -> typecheck::IntrinsicCallRule {
    inferred_args_rule("clipboard-text", 1, typecheck::CheckType::String)
}

fn clipboard_write_rule() -> typecheck::IntrinsicCallRule {
    fixed_string_args_rule("clipboard-write", 1, typecheck::CheckType::Nil)
}

fn browser_set_cookie_rule() -> typecheck::IntrinsicCallRule {
    fixed_string_args_rule("browser-set-cookie", 2, typecheck::CheckType::Nil)
}

fn auth_storage_load_rule() -> typecheck::IntrinsicCallRule {
    fixed_string_args_rule(
        "auth-storage-load",
        1,
        typecheck::CheckType::fresh("auth_storage"),
    )
}

fn auth_storage_persist_rule() -> typecheck::IntrinsicCallRule {
    inferred_args_rule("auth-storage-persist", 2, typecheck::CheckType::Nil)
}

fn selected_file_or_blob_rule() -> typecheck::IntrinsicCallRule {
    fixed_string_args_rule(
        "selected-file-or-blob",
        4,
        typecheck::CheckType::fresh("file_or_blob"),
    )
}

fn selected_file_by_test_id_rule() -> typecheck::IntrinsicCallRule {
    fixed_string_args_rule(
        "selected-file-by-test-id",
        1,
        typecheck::CheckType::fresh("file"),
    )
}

fn has_selected_file_rule() -> typecheck::IntrinsicCallRule {
    fixed_string_args_rule("has-selected-file", 1, typecheck::CheckType::Bool)
}

fn multipart_form_body_rule() -> typecheck::IntrinsicCallRule {
    inferred_args_rule(
        "multipart-form-body",
        2,
        typecheck::CheckType::fresh("form_body"),
    )
}

fn urlencoded_form_body_rule() -> typecheck::IntrinsicCallRule {
    inferred_args_rule("urlencoded-form-body", 2, typecheck::CheckType::String)
}

fn install_virtual_json_viewer_rule() -> typecheck::IntrinsicCallRule {
    typecheck::IntrinsicCallRule::new(
        "install-virtual-json-viewer",
        vec![typecheck::IntrinsicCallOverload::new(
            Vec::new(),
            typecheck::CheckType::Nil,
        )],
    )
}

fn history_replace_search_param_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/history-replace-search-param")
        .required_fields(["name", "value"])
        .field("name", typecheck::CheckType::String)
        .field(
            "value",
            typecheck::CheckType::option(typecheck::CheckType::String),
        )
}

fn history_write_route_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/history-write-route")
        .required_fields(["url", "op", "definition"])
        .field("url", typecheck::CheckType::String)
        .field(
            "op",
            typecheck::CheckType::option(typecheck::CheckType::String),
        )
        .field(
            "definition",
            typecheck::CheckType::option(typecheck::CheckType::String),
        )
}

fn browser_theme_load_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/theme-load")
        .required_fields(["key"])
        .field("key", typecheck::CheckType::String)
        .success_value(typecheck::CheckType::String)
        .require_success()
}

fn browser_theme_apply_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/theme-apply")
        .required_fields(["theme", "key"])
        .field("theme", typecheck::CheckType::String)
        .field("key", typecheck::CheckType::String)
        .success_value(typecheck::CheckType::String)
}

fn browser_clipboard_write_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/clipboard-write")
        .required_fields(["text"])
        .field("text", typecheck::CheckType::String)
}

fn browser_set_cookie_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/set-cookie")
        .required_fields(["name", "value"])
        .field("name", typecheck::CheckType::String)
        .field("value", typecheck::CheckType::String)
}

fn browser_history_push_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/history-push")
        .required_fields(["url"])
        .field("url", typecheck::CheckType::String)
}

fn browser_history_replace_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/history-replace")
        .required_fields(["url"])
        .field("url", typecheck::CheckType::String)
}

fn browser_location_assign_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/location-assign")
        .required_fields(["url"])
        .field("url", typecheck::CheckType::String)
}

fn browser_open_url_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/open-url")
        .required_fields(["url"])
        .field("url", typecheck::CheckType::String)
        .field("target", typecheck::CheckType::String)
}

fn browser_download_url_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/download-url")
        .required_fields(["url", "filename"])
        .field("url", typecheck::CheckType::String)
        .field("filename", typecheck::CheckType::String)
        .field("name", typecheck::CheckType::String)
}

fn browser_document_title_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/document-title")
        .required_fields(["title"])
        .field("title", typecheck::CheckType::String)
}

fn browser_scroll_to_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("browser/scroll-to")
        .required_fields(["x", "y"])
        .field("x", typecheck::CheckType::Number)
        .field("y", typecheck::CheckType::Number)
}

fn event_source_open_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("event-source/open")
        .required_fields(["id", "url", "eventType"])
        .field("id", typecheck::CheckType::String)
        .field("url", typecheck::CheckType::String)
        .field("eventType", typecheck::CheckType::String)
        .field("dispatchEvent", typecheck::CheckType::String)
        .field("refreshMs", typecheck::CheckType::Number)
        .field("intervalMs", typecheck::CheckType::Number)
        .field("logMessage", typecheck::CheckType::String)
}

fn event_source_close_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("event-source/close")
        .required_fields(["id"])
        .field("id", typecheck::CheckType::String)
        .field("url", typecheck::CheckType::String)
}

fn media_play_selector_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("media/play-selector")
        .required_fields(["selector"])
        .field("selector", typecheck::CheckType::String)
}

fn media_restore_current_time_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("media/restore-current-time")
        .required_fields(["selector", "currentTime"])
        .field("selector", typecheck::CheckType::String)
        .field("currentTime", typecheck::CheckType::Number)
}

fn media_sync_audio_element_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("media/sync-audio-element")
        .required_fields(["selector", "key", "url", "duration", "play"])
        .field("selector", typecheck::CheckType::String)
        .field("key", typecheck::CheckType::String)
        .field("url", typecheck::CheckType::String)
        .field("duration", typecheck::CheckType::Number)
        .field("play", typecheck::CheckType::Bool)
}

fn dom_document_set_attribute_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("dom/document-set-attribute")
        .required_fields(["name", "value"])
        .field("name", typecheck::CheckType::String)
        .field("value", typecheck::CheckType::String)
}

fn dom_breadcrumb_adaptive_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("dom/breadcrumb-adaptive")
}

fn dom_focus_selector_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("dom/focus-selector")
        .required_fields(["selector", "defer", "whenBody"])
        .field("selector", typecheck::CheckType::String)
        .field("defer", typecheck::CheckType::Bool)
        .field("whenBody", typecheck::CheckType::Bool)
}

fn storage_get_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("storage/get")
        .required_fields(["key"])
        .field("key", typecheck::CheckType::String)
        .field("format", browser_keyword_or_string_check_type())
        .field("parse", browser_keyword_or_string_check_type())
        .require_success()
        .success_value_from_payload_format_fields(["format", "parse"])
}

fn storage_set_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("storage/set")
        .required_fields(["key", "value"])
        .field("key", typecheck::CheckType::String)
        .success_value_from_field("value")
}

fn storage_remove_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("storage/remove")
        .required_fields(["key"])
        .field("key", typecheck::CheckType::String)
        .success_value(typecheck::CheckType::record([(
            "key",
            typecheck::CheckType::String,
        )]))
}

fn auth_storage_load_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("auth-storage/load")
        .required_fields(["sourceUrl"])
        .field("sourceUrl", typecheck::CheckType::String)
        .require_success()
        .success_value(typecheck::CheckType::Js)
}

fn auth_storage_persist_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("auth-storage/persist")
        .required_fields(["sourceUrl", "entries"])
        .field("sourceUrl", typecheck::CheckType::String)
}

fn file_download_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("file/download")
        .required_fields(["name", "content"])
        .field("name", typecheck::CheckType::String)
        .field("content", typecheck::CheckType::String)
        .field("mime", typecheck::CheckType::String)
        .success_value(typecheck::CheckType::record([
            ("name", typecheck::CheckType::String),
            ("content", typecheck::CheckType::String),
            ("mime", typecheck::CheckType::String),
        ]))
}

fn file_import_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("file/import")
        .field("accept", typecheck::CheckType::String)
        .field("multiple", typecheck::CheckType::Bool)
        .field("format", browser_keyword_or_string_check_type())
        .field("parse", browser_keyword_or_string_check_type())
        .require_success()
        .success_value_from_payload_format_fields(["format", "parse"])
        .supported_continuations(["onCancel"])
}

fn file_read_selected_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("file/read-selected")
        .required_fields(["ref"])
        .field("ref", typecheck::CheckType::String)
        .field("multiple", typecheck::CheckType::Bool)
        .field("clear", typecheck::CheckType::Bool)
        .field("format", browser_keyword_or_string_check_type())
        .field("parse", browser_keyword_or_string_check_type())
        .require_success()
        .success_value_from_payload_format_fields(["format", "parse"])
        .supported_continuations(["onCancel"])
}

fn file_read_blob_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("file/read-blob")
        .required_fields(["blob"])
        .field("blob", typecheck::CheckType::Js)
        .field("format", browser_keyword_or_string_check_type())
        .field("parse", browser_keyword_or_string_check_type())
        .require_success()
        .success_value(typecheck::CheckType::String)
}

fn bluetooth_request_device_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("bluetooth/request-device")
        .one_of_fields(["options", "filters", "acceptAllDevices"])
        .field(
            "options",
            typecheck::CheckType::record(std::iter::empty::<(&str, typecheck::CheckType)>()),
        )
        .field(
            "filters",
            typecheck::CheckType::vector(browser_bluetooth_filter_check_type()),
        )
        .field(
            "optionalServices",
            typecheck::CheckType::vector(typecheck::CheckType::String),
        )
        .field("acceptAllDevices", typecheck::CheckType::Bool)
        .require_success()
        .success_value(typecheck::CheckType::fresh("device"))
}

fn bluetooth_connect_heart_rate_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("bluetooth/connect-heart-rate")
        .required_fields(["id", "onReading"])
        .one_of_fields(["options", "filters", "acceptAllDevices"])
        .field("id", typecheck::CheckType::String)
        .field(
            "options",
            typecheck::CheckType::record(std::iter::empty::<(&str, typecheck::CheckType)>()),
        )
        .field(
            "filters",
            typecheck::CheckType::vector(browser_bluetooth_filter_check_type()),
        )
        .field(
            "optionalServices",
            typecheck::CheckType::vector(typecheck::CheckType::String),
        )
        .field("acceptAllDevices", typecheck::CheckType::Bool)
        .field("service", typecheck::CheckType::String)
        .field("characteristic", typecheck::CheckType::String)
        .require_success()
        .success_value(browser_connected_payload_check_type())
        .supported_continuations(["onReading", "onDisconnected"])
        .continuation_message_field("onReading", "bpm", typecheck::CheckType::Number)
}

fn bluetooth_disconnect_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("bluetooth/disconnect")
        .required_fields(["id"])
        .field("id", typecheck::CheckType::String)
        .payloadless_success()
}

fn simulation_heart_rate_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("simulation/heart-rate")
        .required_fields(["id", "onReading"])
        .field("id", typecheck::CheckType::String)
        .field("ms", typecheck::CheckType::Number)
        .field("min", typecheck::CheckType::Number)
        .field("max", typecheck::CheckType::Number)
        .field("jitter", typecheck::CheckType::Number)
        .field("start", typecheck::CheckType::Number)
        .field("deviceName", typecheck::CheckType::String)
        .require_success()
        .success_value(browser_connected_payload_check_type())
        .supported_continuations(["onReading", "onDisconnected"])
        .continuation_message_field("onReading", "bpm", typecheck::CheckType::Number)
}

fn simulation_stop_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("simulation/stop")
        .required_fields(["id"])
        .field("id", typecheck::CheckType::String)
        .success_value(browser_id_payload_check_type())
}

fn canvas_draw_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("canvas/draw")
        .required_fields(["ref", "ops"])
        .field("ref", typecheck::CheckType::String)
        .field("width", typecheck::CheckType::Number)
        .field("height", typecheck::CheckType::Number)
        .field("cssWidth", typecheck::CheckType::Number)
        .field("cssHeight", typecheck::CheckType::Number)
        .field("setCssSize", typecheck::CheckType::Bool)
        .success_value(browser_canvas_draw_payload_check_type())
}

fn canvas_measure_text_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("canvas/measure-text")
        .required_fields(["ref"])
        .one_of_fields(["text", "texts"])
        .field("ref", typecheck::CheckType::String)
        .field("text", typecheck::CheckType::String)
        .field(
            "texts",
            typecheck::CheckType::vector(typecheck::CheckType::String),
        )
        .field("font", typecheck::CheckType::String)
        .require_success()
        .success_value(browser_text_measure_payload_check_type())
}

fn dom_ref_focus_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("dom-ref/focus")
        .required_fields(["ref"])
        .field("ref", typecheck::CheckType::String)
        .success_value(typecheck::CheckType::record([(
            "ref",
            typecheck::CheckType::String,
        )]))
}

fn dom_ref_click_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("dom-ref/click")
        .required_fields(["ref"])
        .field("ref", typecheck::CheckType::String)
        .success_value(typecheck::CheckType::record([(
            "ref",
            typecheck::CheckType::String,
        )]))
}

fn dom_ref_measure_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("dom-ref/measure")
        .required_fields(["ref"])
        .field("ref", typecheck::CheckType::String)
        .require_success()
        .success_value(browser_dom_ref_measure_payload_check_type())
}

fn dom_input_set_selection_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("dom/input-set-selection")
        .required_fields(["target", "start", "end"])
        .field("target", typecheck::CheckType::Js)
        .field("start", typecheck::CheckType::Number)
        .field("end", typecheck::CheckType::Number)
}

fn dom_scroll_into_view_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("dom/scroll-into-view")
        .one_of_fields(["selector", "testId", "id"])
        .field("selector", typecheck::CheckType::String)
        .field("testId", typecheck::CheckType::String)
        .field("id", typecheck::CheckType::String)
        .field("behavior", typecheck::CheckType::String)
        .field("block", typecheck::CheckType::String)
        .field("inline", typecheck::CheckType::String)
        .field("skipIfVisible", typecheck::CheckType::Bool)
        .field("smooth", typecheck::CheckType::Bool)
}

fn dom_ref_resize_watch_command_schema() -> typecheck::CommandSchemaRule {
    let rect = browser_rect_check_type();
    typecheck::CommandSchemaRule::new("dom-ref/resize-watch")
        .required_fields(["ref", "onChange"])
        .field("id", typecheck::CheckType::String)
        .field("ref", typecheck::CheckType::String)
        .reject_success_continuations()
        .supported_continuations(["onChange"])
        .continuation_message_field("onChange", "id", typecheck::CheckType::String)
        .continuation_message_field("onChange", "ref", typecheck::CheckType::String)
        .continuation_message_field("onChange", "x", typecheck::CheckType::Number)
        .continuation_message_field("onChange", "y", typecheck::CheckType::Number)
        .continuation_message_field("onChange", "width", typecheck::CheckType::Number)
        .continuation_message_field("onChange", "height", typecheck::CheckType::Number)
        .continuation_message_field("onChange", "top", typecheck::CheckType::Number)
        .continuation_message_field("onChange", "right", typecheck::CheckType::Number)
        .continuation_message_field("onChange", "bottom", typecheck::CheckType::Number)
        .continuation_message_field("onChange", "left", typecheck::CheckType::Number)
        .continuation_message_field("onChange", "value", rect)
}

fn dom_ref_resize_unwatch_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("dom-ref/resize-unwatch")
        .one_of_fields(["id", "ref"])
        .field("id", typecheck::CheckType::String)
        .field("ref", typecheck::CheckType::String)
        .success_value(browser_id_payload_check_type())
}

fn window_event_watch_command_schema() -> typecheck::CommandSchemaRule {
    let event = browser_window_event_check_type();
    typecheck::CommandSchemaRule::new("window/event-watch")
        .required_fields(["type", "onEvent"])
        .field("id", typecheck::CheckType::String)
        .field("type", typecheck::CheckType::String)
        .success_value(typecheck::CheckType::record([
            ("id", typecheck::CheckType::String),
            ("type", typecheck::CheckType::String),
        ]))
        .supported_continuations(["onEvent"])
        .continuation_message_field("onEvent", "type", typecheck::CheckType::String)
        .continuation_message_field("onEvent", "clientX", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "clientY", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "pageX", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "pageY", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "screenX", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "screenY", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "movementX", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "movementY", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "button", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "buttons", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "pointerId", typecheck::CheckType::Number)
        .continuation_message_field("onEvent", "pointerType", typecheck::CheckType::String)
        .continuation_message_field("onEvent", "isPrimary", typecheck::CheckType::Bool)
        .continuation_message_field("onEvent", "key", typecheck::CheckType::String)
        .continuation_message_field("onEvent", "code", typecheck::CheckType::String)
        .continuation_message_field("onEvent", "altKey", typecheck::CheckType::Bool)
        .continuation_message_field("onEvent", "ctrlKey", typecheck::CheckType::Bool)
        .continuation_message_field("onEvent", "metaKey", typecheck::CheckType::Bool)
        .continuation_message_field("onEvent", "shiftKey", typecheck::CheckType::Bool)
        .continuation_message_field("onEvent", "id", typecheck::CheckType::String)
        .continuation_message_field("onEvent", "value", event)
}

fn window_event_unwatch_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("window/event-unwatch")
        .one_of_fields(["id", "type"])
        .field("id", typecheck::CheckType::String)
        .field("type", typecheck::CheckType::String)
        .success_value(browser_id_payload_check_type())
}

fn media_query_watch_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("media-query/watch")
        .required_fields(["query", "onChange"])
        .field("id", typecheck::CheckType::String)
        .field("query", typecheck::CheckType::String)
        .reject_success_continuations()
        .supported_continuations(["onChange"])
        .continuation_message_field("onChange", "id", typecheck::CheckType::String)
        .continuation_message_field("onChange", "media", typecheck::CheckType::String)
        .continuation_message_field("onChange", "matches", typecheck::CheckType::Bool)
}

fn media_query_unwatch_command_schema() -> typecheck::CommandSchemaRule {
    typecheck::CommandSchemaRule::new("media-query/unwatch")
        .one_of_fields(["id", "query"])
        .field("id", typecheck::CheckType::String)
        .field("query", typecheck::CheckType::String)
        .success_value(browser_id_payload_check_type())
}

fn sub_timer_every_subscription_schema() -> typecheck::SubscriptionSchemaRule {
    typecheck::SubscriptionSchemaRule::new("sub/timer/every")
        .required_fields(["id", "ms", "msg"])
        .field("id", typecheck::CheckType::String)
        .field("ms", typecheck::CheckType::Number)
        .command_kind("timer/every")
}

fn sub_simulation_heart_rate_subscription_schema() -> typecheck::SubscriptionSchemaRule {
    typecheck::SubscriptionSchemaRule::new("sub/simulation/heart-rate")
        .required_fields(["id", "onReading"])
        .field("id", typecheck::CheckType::String)
        .field("ms", typecheck::CheckType::Number)
        .field("min", typecheck::CheckType::Number)
        .field("max", typecheck::CheckType::Number)
        .field("jitter", typecheck::CheckType::Number)
        .field("start", typecheck::CheckType::Number)
        .field("deviceName", typecheck::CheckType::String)
        .command_kind("simulation/heart-rate")
}

fn sub_bluetooth_connect_heart_rate_subscription_schema() -> typecheck::SubscriptionSchemaRule {
    typecheck::SubscriptionSchemaRule::new("sub/bluetooth/connect-heart-rate")
        .required_fields(["id", "onReading"])
        .field("id", typecheck::CheckType::String)
        .field(
            "options",
            typecheck::CheckType::record(std::iter::empty::<(&str, typecheck::CheckType)>()),
        )
        .field(
            "filters",
            typecheck::CheckType::vector(browser_bluetooth_filter_check_type()),
        )
        .field(
            "optionalServices",
            typecheck::CheckType::vector(typecheck::CheckType::String),
        )
        .field("acceptAllDevices", typecheck::CheckType::Bool)
        .command_kind("bluetooth/connect-heart-rate")
}

fn sub_dom_ref_resize_subscription_schema() -> typecheck::SubscriptionSchemaRule {
    typecheck::SubscriptionSchemaRule::new("sub/dom-ref/resize")
        .required_fields(["ref", "onChange"])
        .field("id", typecheck::CheckType::String)
        .field("ref", typecheck::CheckType::String)
        .command_kind("dom-ref/resize-watch")
}

fn sub_window_event_subscription_schema() -> typecheck::SubscriptionSchemaRule {
    typecheck::SubscriptionSchemaRule::new("sub/window/event")
        .required_fields(["type", "onEvent"])
        .field("id", typecheck::CheckType::String)
        .field("type", typecheck::CheckType::String)
        .command_kind("window/event-watch")
}

fn sub_media_query_subscription_schema() -> typecheck::SubscriptionSchemaRule {
    typecheck::SubscriptionSchemaRule::new("sub/media-query")
        .required_fields(["query", "onChange"])
        .field("id", typecheck::CheckType::String)
        .field("query", typecheck::CheckType::String)
        .command_kind("media-query/watch")
}

fn browser_rect_check_type() -> typecheck::CheckType {
    typecheck::CheckType::Record(browser_rect_check_fields())
}

fn browser_rect_check_fields() -> BTreeMap<String, typecheck::CheckType> {
    [
        ("x", typecheck::CheckType::Number),
        ("y", typecheck::CheckType::Number),
        ("width", typecheck::CheckType::Number),
        ("height", typecheck::CheckType::Number),
        ("top", typecheck::CheckType::Number),
        ("right", typecheck::CheckType::Number),
        ("bottom", typecheck::CheckType::Number),
        ("left", typecheck::CheckType::Number),
    ]
    .into_iter()
    .map(|(name, ty)| (name.to_string(), ty))
    .collect()
}

fn browser_dom_ref_measure_payload_check_type() -> typecheck::CheckType {
    let mut fields = browser_rect_check_fields();
    fields.insert("ref".to_string(), typecheck::CheckType::String);
    typecheck::CheckType::Record(fields)
}

fn browser_connected_payload_check_type() -> typecheck::CheckType {
    typecheck::CheckType::record([
        ("id", typecheck::CheckType::String),
        ("deviceName", typecheck::CheckType::String),
        ("connected", typecheck::CheckType::Bool),
    ])
}

fn browser_canvas_draw_payload_check_type() -> typecheck::CheckType {
    typecheck::CheckType::record([
        ("ref", typecheck::CheckType::String),
        ("width", typecheck::CheckType::Number),
        ("height", typecheck::CheckType::Number),
        ("cssWidth", typecheck::CheckType::Number),
        ("cssHeight", typecheck::CheckType::Number),
        ("pixelRatio", typecheck::CheckType::Number),
    ])
}

fn browser_text_measure_payload_check_type() -> typecheck::CheckType {
    typecheck::CheckType::record([
        ("ref", typecheck::CheckType::String),
        ("font", typecheck::CheckType::String),
        (
            "texts",
            typecheck::CheckType::vector(typecheck::CheckType::String),
        ),
        (
            "widths",
            typecheck::CheckType::vector(typecheck::CheckType::Number),
        ),
        (
            "measurements",
            typecheck::CheckType::vector(browser_text_measurement_check_type()),
        ),
    ])
}

fn browser_text_measurement_check_type() -> typecheck::CheckType {
    typecheck::CheckType::record([
        ("text", typecheck::CheckType::String),
        ("width", typecheck::CheckType::Number),
        ("actualBoundingBoxLeft", typecheck::CheckType::Number),
        ("actualBoundingBoxRight", typecheck::CheckType::Number),
        ("actualBoundingBoxAscent", typecheck::CheckType::Number),
        ("actualBoundingBoxDescent", typecheck::CheckType::Number),
    ])
}

fn browser_bluetooth_filter_check_type() -> typecheck::CheckType {
    typecheck::CheckType::record([
        (
            "services",
            typecheck::CheckType::vector(typecheck::CheckType::String),
        ),
        ("name", typecheck::CheckType::String),
        ("namePrefix", typecheck::CheckType::String),
    ])
}

fn browser_window_event_check_type() -> typecheck::CheckType {
    typecheck::CheckType::record([
        ("type", typecheck::CheckType::String),
        ("clientX", typecheck::CheckType::Number),
        ("clientY", typecheck::CheckType::Number),
        ("pageX", typecheck::CheckType::Number),
        ("pageY", typecheck::CheckType::Number),
        ("screenX", typecheck::CheckType::Number),
        ("screenY", typecheck::CheckType::Number),
        ("movementX", typecheck::CheckType::Number),
        ("movementY", typecheck::CheckType::Number),
        ("button", typecheck::CheckType::Number),
        ("buttons", typecheck::CheckType::Number),
        ("pointerId", typecheck::CheckType::Number),
        ("pointerType", typecheck::CheckType::String),
        ("isPrimary", typecheck::CheckType::Bool),
        ("key", typecheck::CheckType::String),
        ("code", typecheck::CheckType::String),
        ("altKey", typecheck::CheckType::Bool),
        ("ctrlKey", typecheck::CheckType::Bool),
        ("metaKey", typecheck::CheckType::Bool),
        ("shiftKey", typecheck::CheckType::Bool),
    ])
}

fn browser_id_payload_check_type() -> typecheck::CheckType {
    typecheck::CheckType::record([("id", typecheck::CheckType::String)])
}

fn browser_keyword_or_string_check_type() -> typecheck::CheckType {
    typecheck::CheckType::Union(vec![
        typecheck::CheckType::Keyword,
        typecheck::CheckType::String,
    ])
}

fn history_replace_search_param_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/history-replace-search-param")
        .required_fields(["name", "value"])
}

fn history_write_route_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/history-write-route").required_fields([
        "url",
        "op",
        "definition",
    ])
}

fn browser_theme_load_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/theme-load")
        .required_fields(["key"])
        .require_success()
}

fn browser_theme_apply_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/theme-apply").required_fields(["theme", "key"])
}

fn browser_clipboard_write_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/clipboard-write").required_fields(["text"])
}

fn browser_set_cookie_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/set-cookie").required_fields(["name", "value"])
}

fn browser_history_push_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/history-push").required_fields(["url"])
}

fn browser_history_replace_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/history-replace").required_fields(["url"])
}

fn browser_location_assign_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/location-assign").required_fields(["url"])
}

fn browser_open_url_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/open-url").required_fields(["url"])
}

fn browser_download_url_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/download-url")
        .required_fields(["url", "filename"])
}

fn browser_document_title_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/document-title").required_fields(["title"])
}

fn browser_scroll_to_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("browser/scroll-to").required_fields(["x", "y"])
}

fn event_source_open_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("event-source/open").required_fields([
        "id",
        "url",
        "eventType",
    ])
}

fn event_source_close_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("event-source/close").required_fields(["id"])
}

fn media_play_selector_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("media/play-selector").required_fields(["selector"])
}

fn media_restore_current_time_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("media/restore-current-time")
        .required_fields(["selector", "currentTime"])
}

fn media_sync_audio_element_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("media/sync-audio-element")
        .required_fields(["selector", "key", "url", "duration", "play"])
}

fn dom_document_set_attribute_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("dom/document-set-attribute")
        .required_fields(["name", "value"])
}

fn dom_breadcrumb_adaptive_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("dom/breadcrumb-adaptive")
}

fn dom_focus_selector_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("dom/focus-selector")
        .required_fields(["selector", "defer", "whenBody"])
}

fn storage_get_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("storage/get")
        .required_fields(["key"])
        .require_success()
}

fn storage_set_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("storage/set").required_fields(["key", "value"])
}

fn storage_remove_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("storage/remove").required_fields(["key"])
}

fn auth_storage_load_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("auth-storage/load")
        .required_fields(["sourceUrl"])
        .require_success()
}

fn auth_storage_persist_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("auth-storage/persist")
        .required_fields(["sourceUrl", "entries"])
}

fn file_download_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("file/download").required_fields(["name", "content"])
}

fn file_import_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("file/import")
        .require_success()
        .supported_continuations(["onCancel"])
}

fn file_read_selected_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("file/read-selected")
        .required_fields(["ref"])
        .require_success()
        .supported_continuations(["onCancel"])
}

fn file_read_blob_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("file/read-blob")
        .required_fields(["blob"])
        .require_success()
}

fn bluetooth_request_device_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("bluetooth/request-device")
        .require_success()
        .one_of_fields(["options", "filters", "acceptAllDevices"])
}

fn bluetooth_connect_heart_rate_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("bluetooth/connect-heart-rate")
        .required_fields(["id", "onReading"])
        .require_success()
        .one_of_fields(["options", "filters", "acceptAllDevices"])
        .supported_continuations(["onReading", "onDisconnected"])
}

fn bluetooth_disconnect_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("bluetooth/disconnect")
        .required_fields(["id"])
        .payloadless_success()
}

fn simulation_heart_rate_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("simulation/heart-rate")
        .required_fields(["id", "onReading"])
        .require_success()
        .supported_continuations(["onReading", "onDisconnected"])
}

fn simulation_stop_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("simulation/stop").required_fields(["id"])
}

fn canvas_draw_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("canvas/draw").required_fields(["ref", "ops"])
}

fn canvas_measure_text_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("canvas/measure-text")
        .required_fields(["ref"])
        .require_success()
        .one_of_fields(["text", "texts"])
}

fn dom_ref_focus_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("dom-ref/focus").required_fields(["ref"])
}

fn dom_ref_click_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("dom-ref/click").required_fields(["ref"])
}

fn dom_ref_measure_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("dom-ref/measure")
        .required_fields(["ref"])
        .require_success()
}

fn dom_input_set_selection_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("dom/input-set-selection")
        .required_fields(["target", "start", "end"])
}

fn dom_scroll_into_view_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("dom/scroll-into-view")
        .one_of_fields(["selector", "testId", "id"])
}

fn dom_ref_resize_watch_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("dom-ref/resize-watch")
        .required_fields(["ref", "onChange"])
        .reject_success_continuations()
        .supported_continuations(["onChange"])
}

fn dom_ref_resize_unwatch_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("dom-ref/resize-unwatch").one_of_fields(["id", "ref"])
}

fn window_event_watch_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("window/event-watch")
        .required_fields(["type", "onEvent"])
        .supported_continuations(["onEvent"])
}

fn window_event_unwatch_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("window/event-unwatch").one_of_fields(["id", "type"])
}

fn media_query_watch_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("media-query/watch")
        .required_fields(["query", "onChange"])
        .reject_success_continuations()
        .supported_continuations(["onChange"])
}

fn media_query_unwatch_effect_schema() -> effects::EffectCommandSchemaRule {
    effects::EffectCommandSchemaRule::new("media-query/unwatch").one_of_fields(["id", "query"])
}

fn sub_none_effect_schema() -> effects::EffectSubscriptionSchemaRule {
    effects::EffectSubscriptionSchemaRule::new("none")
}

fn sub_batch_effect_schema() -> effects::EffectSubscriptionSchemaRule {
    effects::EffectSubscriptionSchemaRule::new("batch").collection_fields(["subscriptions", "subs"])
}

fn sub_timer_every_effect_schema() -> effects::EffectSubscriptionSchemaRule {
    effects::EffectSubscriptionSchemaRule::new("sub/timer/every")
        .required_fields(["id", "ms", "msg"])
}

fn sub_simulation_heart_rate_effect_schema() -> effects::EffectSubscriptionSchemaRule {
    effects::EffectSubscriptionSchemaRule::new("sub/simulation/heart-rate")
        .required_fields(["id", "onReading"])
}

fn sub_bluetooth_connect_heart_rate_effect_schema() -> effects::EffectSubscriptionSchemaRule {
    effects::EffectSubscriptionSchemaRule::new("sub/bluetooth/connect-heart-rate")
        .required_fields(["id", "onReading"])
}

fn sub_dom_ref_resize_effect_schema() -> effects::EffectSubscriptionSchemaRule {
    effects::EffectSubscriptionSchemaRule::new("sub/dom-ref/resize")
        .required_fields(["ref", "onChange"])
}

fn sub_window_event_effect_schema() -> effects::EffectSubscriptionSchemaRule {
    effects::EffectSubscriptionSchemaRule::new("sub/window/event")
        .required_fields(["type", "onEvent"])
}

fn sub_media_query_effect_schema() -> effects::EffectSubscriptionSchemaRule {
    effects::EffectSubscriptionSchemaRule::new("sub/media-query")
        .required_fields(["query", "onChange"])
}

fn fixed_string_args_rule(
    name: &'static str,
    arity: usize,
    result: typecheck::CheckType,
) -> typecheck::IntrinsicCallRule {
    typecheck::IntrinsicCallRule::new(
        name,
        vec![typecheck::IntrinsicCallOverload::new(
            (0..arity)
                .map(|_| expect(typecheck::CheckType::String))
                .collect(),
            result,
        )],
    )
}

fn inferred_args_rule(
    name: &'static str,
    arity: usize,
    result: typecheck::CheckType,
) -> typecheck::IntrinsicCallRule {
    typecheck::IntrinsicCallRule::new(
        name,
        vec![typecheck::IntrinsicCallOverload::new(
            (0..arity)
                .map(|_| typecheck::IntrinsicParam::infer())
                .collect(),
            result,
        )],
    )
}

fn html_type() -> typecheck::CheckType {
    typecheck::CheckType::named("Html")
}

fn trusted_html_type() -> typecheck::CheckType {
    typecheck::CheckType::named("TrustedHtml")
}

fn browser_boot_type() -> typecheck::CheckType {
    typecheck::CheckType::record([
        ("currentUrl", typecheck::CheckType::String),
        ("host", typecheck::CheckType::String),
        ("path", typecheck::CheckType::String),
        ("query", typecheck::CheckType::String),
    ])
}

fn expect(ty: typecheck::CheckType) -> typecheck::IntrinsicParam {
    typecheck::IntrinsicParam::expect(ty)
}

fn sub_none_emit_rule() -> js_emit::SymbolReadEmitRule {
    js_emit::SymbolReadEmitRule::new("Sub.none", "__closkellNone").none_const()
}

fn cmd_storage_get_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.storage/get", emit_cmd_storage_get)
}

fn cmd_storage_set_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.storage/set", emit_cmd_storage_set)
}

fn cmd_storage_set_silent_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.storage/set-silent", emit_cmd_storage_set_silent)
}

fn cmd_dom_ref_click_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.dom-ref/click", emit_cmd_dom_ref_click)
}

fn cmd_dom_ref_focus_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.dom-ref/focus", emit_cmd_dom_ref_focus)
}

fn cmd_file_read_selected_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.file/read-selected", emit_cmd_file_read_selected)
}

fn cmd_file_download_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.file/download", emit_cmd_file_download)
}

fn cmd_canvas_draw_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.canvas/draw", emit_cmd_canvas_draw)
}

fn cmd_dom_ref_measure_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.dom-ref/measure", emit_cmd_dom_ref_measure)
}

fn cmd_bluetooth_connect_heart_rate_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new(
        "Cmd.bluetooth/connect-heart-rate",
        emit_cmd_bluetooth_connect_heart_rate,
    )
}

fn cmd_bluetooth_disconnect_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.bluetooth/disconnect", emit_cmd_bluetooth_disconnect)
}

fn cmd_simulation_heart_rate_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.simulation/heart-rate", emit_cmd_simulation_heart_rate)
}

fn cmd_simulation_stop_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.simulation/stop", emit_cmd_simulation_stop)
}

fn sub_batch_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Sub.batch", emit_sub_batch)
}

fn sub_timer_every_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Sub.timer/every", emit_sub_timer_every)
}

fn sub_media_query_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Sub.media-query", emit_sub_media_query)
}

fn sub_window_event_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Sub.window/event", emit_sub_window_event)
}

fn sub_window_event_with_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Sub.window/event-with", emit_sub_window_event_with)
}

fn sub_dom_ref_resize_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Sub.dom-ref/resize", emit_sub_dom_ref_resize)
}

fn cmd_dom_ref_resize_watch_emit_rule() -> js_emit::CustomCallEmitRule {
    js_emit::CustomCallEmitRule::new("Cmd.dom-ref/resize-watch", emit_cmd_dom_ref_resize_watch)
}

fn emit_cmd_storage_get(context: &mut js_emit::CustomCallEmitContext<'_>, args: &[Expr]) -> String {
    context.add_runtime_effect("storage/get");
    if args.len() != 4 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"storage/get\"), key: {}, format: {}, toMessage: {}, onError: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2]),
        context.emit_expr(&args[3])
    )
}

fn emit_cmd_storage_set(context: &mut js_emit::CustomCallEmitContext<'_>, args: &[Expr]) -> String {
    context.add_runtime_effect("storage/set");
    if args.len() != 3 && args.len() != 4 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    let on_error = args
        .get(3)
        .map(|arg| format!(", onError: {}", context.emit_expr(arg)))
        .unwrap_or_default();
    format!(
        "{{ kind: Symbol.for(\"storage/set\"), key: {}, value: {}, msg: {}{} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2]),
        on_error
    )
}

fn emit_cmd_storage_set_silent(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    context.add_runtime_effect("storage/set");
    if args.len() != 3 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"storage/set\"), key: {}, value: {}, onError: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2])
    )
}

fn emit_cmd_dom_ref_click(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    emit_cmd_dom_ref_action(context, args, "dom-ref/click")
}

fn emit_cmd_dom_ref_focus(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    emit_cmd_dom_ref_action(context, args, "dom-ref/focus")
}

fn emit_cmd_dom_ref_action(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
    kind: &str,
) -> String {
    context.add_runtime_effect(kind);
    if args.len() != 2 && args.len() != 3 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    let on_error = args
        .get(2)
        .map(|arg| format!(", onError: {}", context.emit_expr(arg)))
        .unwrap_or_default();
    format!(
        "{{ kind: Symbol.for(\"{}\"), ref: {}, msg: {}{} }}",
        kind,
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        on_error
    )
}

fn emit_cmd_file_read_selected(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    context.add_runtime_effect("file/read-selected");
    if args.len() != 5 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"file/read-selected\"), ref: {}, format: {}, toMessage: {}, onError: {}, onCancel: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2]),
        context.emit_expr(&args[3]),
        context.emit_expr(&args[4])
    )
}

fn emit_cmd_file_download(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    context.add_runtime_effect("file/download");
    if args.len() != 5 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"file/download\"), name: {}, content: {}, mime: {}, msg: {}, onError: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2]),
        context.emit_expr(&args[3]),
        context.emit_expr(&args[4])
    )
}

fn emit_cmd_canvas_draw(context: &mut js_emit::CustomCallEmitContext<'_>, args: &[Expr]) -> String {
    context.add_runtime_effect("canvas/draw");
    if args.len() != 5 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"canvas/draw\"), ref: {}, cssWidth: {}, cssHeight: {}, devicePixelRatio: true, ops: {}, onError: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2]),
        context.emit_expr(&args[3]),
        context.emit_expr(&args[4])
    )
}

fn emit_cmd_dom_ref_measure(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    context.add_runtime_effect("dom-ref/measure");
    if args.len() != 3 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"dom-ref/measure\"), ref: {}, toMessage: {}, onError: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2])
    )
}

fn emit_cmd_bluetooth_connect_heart_rate(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    context.add_runtime_effect("bluetooth/connect-heart-rate");
    if args.len() != 6 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"bluetooth/connect-heart-rate\"), id: {}, ...{}, toMessage: {}, onReading: {}, onDisconnected: {}, onError: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2]),
        context.emit_expr(&args[3]),
        context.emit_expr(&args[4]),
        context.emit_expr(&args[5])
    )
}

fn emit_cmd_bluetooth_disconnect(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    context.add_runtime_effect("bluetooth/disconnect");
    if args.len() != 2 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"bluetooth/disconnect\"), id: {}, msg: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1])
    )
}

fn emit_cmd_simulation_heart_rate(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    context.add_runtime_effect("simulation/heart-rate");
    if args.len() != 5 && args.len() != 6 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    let on_disconnected = if args.len() == 6 {
        format!(", onDisconnected: {}", context.emit_expr(&args[4]))
    } else {
        String::new()
    };
    let error_arg = if args.len() == 6 { &args[5] } else { &args[4] };
    format!(
        "{{ kind: Symbol.for(\"simulation/heart-rate\"), id: {}, ...{}, toMessage: {}, onReading: {}{}, onError: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2]),
        context.emit_expr(&args[3]),
        on_disconnected,
        context.emit_expr(error_arg)
    )
}

fn emit_cmd_simulation_stop(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    context.add_runtime_effect("simulation/stop");
    if args.len() != 1 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"simulation/stop\"), id: {} }}",
        context.emit_expr(&args[0])
    )
}

fn emit_sub_batch(context: &mut js_emit::CustomCallEmitContext<'_>, args: &[Expr]) -> String {
    if args.len() != 1 {
        return "{ kind: Symbol.for(\"batch\"), subscriptions: [] }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"batch\"), subscriptions: {} }}",
        context.emit_expr(&args[0])
    )
}

fn emit_cmd_dom_ref_resize_watch(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    if args.len() == 4 {
        if let Some(to_message) = emit_resize_to_message(context, &args[2]) {
            context.add_runtime_effect("dom-ref/resize-watch/direct");
            return format!(
                "{{ kind: Symbol.for(\"dom-ref/resize-watch/direct\"), id: {}, ref: {}, m: {}, onError: {} }}",
                context.emit_expr(&args[0]),
                context.emit_expr(&args[1]),
                to_message,
                context.emit_expr(&args[3])
            );
        }
    }
    context.add_runtime_effect("dom-ref/resize-watch");
    if args.len() != 4 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"dom-ref/resize-watch\"), id: {}, ref: {}, onChange: {}, onError: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2]),
        context.emit_expr(&args[3])
    )
}

fn emit_sub_timer_every(context: &mut js_emit::CustomCallEmitContext<'_>, args: &[Expr]) -> String {
    context.add_runtime_effect("timer/every");
    context.add_runtime_effect("timer/cancel");
    if args.len() != 3 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"timer/every\"), s: \"timer/cancel\", id: {}, ms: {}, msg: {} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2])
    )
}

fn emit_sub_media_query(context: &mut js_emit::CustomCallEmitContext<'_>, args: &[Expr]) -> String {
    if args.len() == 3 {
        if let Some(to_message) = emit_media_query_to_message(context, &args[2]) {
            context.add_runtime_effect("media-query/watch/direct");
            context.add_runtime_effect("media-query/unwatch/direct");
            return format!(
                "{{ kind: Symbol.for(\"media-query/watch/direct\"), s: \"media-query/unwatch/direct\", id: {}, query: {}, m: {} }}",
                context.emit_expr(&args[0]),
                context.emit_expr(&args[1]),
                to_message
            );
        }
    }
    emit_sub_change(
        context,
        args,
        "media-query/watch",
        "query",
        "media-query/unwatch",
    )
}

fn emit_sub_dom_ref_resize(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    emit_sub_change(
        context,
        args,
        "dom-ref/resize-watch",
        "ref",
        "dom-ref/resize-unwatch",
    )
}

fn emit_sub_change(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
    kind: &str,
    target_field: &str,
    stop_kind: &str,
) -> String {
    context.add_runtime_effect(kind);
    context.add_runtime_effect(stop_kind);
    if args.len() != 3 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ kind: Symbol.for(\"{}\"), s: \"{}\", id: {}, {}: {}, onChange: {} }}",
        kind,
        stop_kind,
        context.emit_expr(&args[0]),
        target_field,
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2])
    )
}

fn emit_sub_window_event(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    if args.len() == 3 {
        if let Some(to_message) = emit_window_event_to_message(context, &args[2]) {
            return emit_direct_window_event_subscription(
                context, &args[0], &args[1], to_message, None, false, false,
            );
        }
    }
    context.add_runtime_effect("window/event-watch");
    context.add_runtime_effect("window/event-unwatch");
    if args.len() != 3 && args.len() != 4 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    let options = args
        .get(3)
        .map(|arg| format!(", options: {}", context.emit_expr(arg)))
        .unwrap_or_default();
    format!(
        "{{ kind: Symbol.for(\"window/event-watch\"), s: \"window/event-unwatch\", id: {}, type: {}, onEvent: {}{} }}",
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2]),
        options
    )
}

fn emit_sub_window_event_with(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    args: &[Expr],
) -> String {
    if args.len() == 4 {
        if let (Some(to_message), Some(options)) = (
            emit_window_event_to_message(context, &args[2]),
            static_window_event_options(&args[3]),
        ) {
            return emit_direct_window_event_subscription(
                context,
                &args[0],
                &args[1],
                to_message,
                options.options,
                options.prevent_default,
                options.stop_propagation,
            );
        }
    }
    context.add_runtime_effect("window/event-watch");
    context.add_runtime_effect("window/event-unwatch");
    if args.len() != 4 {
        return "{ kind: Symbol.for(\"none\") }".to_string();
    }
    format!(
        "{{ ...{}, kind: Symbol.for(\"window/event-watch\"), s: \"window/event-unwatch\", id: {}, type: {}, onEvent: {} }}",
        context.emit_expr(&args[3]),
        context.emit_expr(&args[0]),
        context.emit_expr(&args[1]),
        context.emit_expr(&args[2])
    )
}

fn emit_direct_window_event_subscription(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    id: &Expr,
    event_type: &Expr,
    to_message: String,
    options: Option<&Expr>,
    prevent_default: bool,
    stop_propagation: bool,
) -> String {
    context.add_runtime_effect("window/event-watch/direct");
    context.add_runtime_effect("window/event-unwatch/direct");
    let options = options
        .map(|expr| format!(", options: {}", context.emit_expr(expr)))
        .unwrap_or_default();
    let prevent_default = if prevent_default { ", p: true" } else { "" };
    let stop_propagation = if stop_propagation { ", q: true" } else { "" };
    format!(
        "{{ kind: Symbol.for(\"window/event-watch/direct\"), s: \"window/event-unwatch/direct\", id: {}, type: {}, m: {}{}{}{} }}",
        context.emit_expr(id),
        context.emit_expr(event_type),
        to_message,
        options,
        prevent_default,
        stop_propagation
    )
}

fn emit_window_event_to_message(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    message: &Expr,
) -> Option<String> {
    let kind = literal_string_value(message)?;
    let reads = context.message_field_reads(kind)?;
    if reads.value_escapes || reads.top_fields.contains("id") {
        return None;
    }
    let body = window_event_message_object(kind, reads)?;
    Some(format!("(event) => ({})", body))
}

fn window_event_message_object(kind: &str, reads: &js_emit::MessageFieldReads) -> Option<String> {
    let mut fields = vec![format!("kind: Symbol.for(\"{}\")", escape_js(kind))];
    for field in &reads.top_fields {
        if field == "id" {
            fields.push("id".to_string());
        } else {
            fields.push(format!(
                "{}: {}",
                property_key(field),
                window_event_field_expr(field)?
            ));
        }
    }
    if !reads.value_fields.is_empty() {
        let mut value_fields = Vec::new();
        for field in &reads.value_fields {
            value_fields.push(format!(
                "{}: {}",
                property_key(field),
                window_event_field_expr(field)?
            ));
        }
        fields.push(format!("value: {{ {} }}", value_fields.join(", ")));
    }
    Some(format!("{{ {} }}", fields.join(", ")))
}

fn emit_media_query_to_message(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    message: &Expr,
) -> Option<String> {
    let kind = literal_string_value(message)?;
    let reads = context.message_field_reads(kind)?;
    if reads.value_escapes || !reads.value_fields.is_empty() || reads.top_fields.contains("id") {
        return None;
    }
    let mut fields = vec![format!("kind: Symbol.for(\"{}\")", escape_js(kind))];
    for field in &reads.top_fields {
        let value = match field.as_str() {
            "media" => "mediaQuery.media",
            "matches" => "mediaQuery.matches",
            _ => return None,
        };
        fields.push(format!("{}: {}", property_key(field), value));
    }
    Some(format!("(mediaQuery) => ({{ {} }})", fields.join(", ")))
}

fn emit_resize_to_message(
    context: &mut js_emit::CustomCallEmitContext<'_>,
    message: &Expr,
) -> Option<String> {
    let kind = literal_string_value(message)?;
    let reads = context.message_field_reads(kind)?;
    if reads.value_escapes {
        return None;
    }
    let mut fields = vec![format!("kind: Symbol.for(\"{}\")", escape_js(kind))];
    for field in &reads.top_fields {
        let value = match field.as_str() {
            "id" => "id",
            _ => resize_field_expr(field)?,
        };
        fields.push(format!("{}: {}", property_key(field), value));
    }
    if !reads.value_fields.is_empty() {
        let mut value_fields = Vec::new();
        for field in &reads.value_fields {
            value_fields.push(format!(
                "{}: {}",
                property_key(field),
                resize_field_expr(field)?
            ));
        }
        fields.push(format!("value: {{ {} }}", value_fields.join(", ")));
    }
    let params = if reads.top_fields.contains("id") {
        "entry, node, id"
    } else {
        "entry, node"
    };
    Some(format!("({}) => ({{ {} }})", params, fields.join(", ")))
}

fn resize_field_expr(field: &str) -> Option<&'static str> {
    match field {
        "x" => Some("entry?.contentRect?.x ?? 0"),
        "y" => Some("entry?.contentRect?.y ?? 0"),
        "width" => Some(
            "entry?.contentRect?.width ?? node.clientWidth ?? node.offsetWidth ?? node.width ?? 0",
        ),
        "height" => Some(
            "entry?.contentRect?.height ?? node.clientHeight ?? node.offsetHeight ?? node.height ?? 0",
        ),
        "top" => Some("entry?.contentRect?.top ?? entry?.contentRect?.y ?? 0"),
        "left" => Some("entry?.contentRect?.left ?? entry?.contentRect?.x ?? 0"),
        "right" => Some(
            "entry?.contentRect?.right ?? ((entry?.contentRect?.x ?? 0) + (entry?.contentRect?.width ?? node.clientWidth ?? node.offsetWidth ?? node.width ?? 0))",
        ),
        "bottom" => Some(
            "entry?.contentRect?.bottom ?? ((entry?.contentRect?.y ?? 0) + (entry?.contentRect?.height ?? node.clientHeight ?? node.offsetHeight ?? node.height ?? 0))",
        ),
        _ => None,
    }
}

struct StaticWindowEventOptions<'a> {
    options: Option<&'a Expr>,
    prevent_default: bool,
    stop_propagation: bool,
}

fn static_window_event_options(expr: &Expr) -> Option<StaticWindowEventOptions<'_>> {
    let ExprKind::Map(entries) = &expr.kind else {
        return None;
    };
    let mut options = None;
    let mut prevent_default = false;
    let mut stop_propagation = false;
    for (key, value) in entries {
        let key = object_key_name(key)?;
        match key.as_str() {
            "options" => options = Some(value),
            "preventDefault" => {
                let ExprKind::Bool(value) = &value.kind else {
                    return None;
                };
                prevent_default = *value;
            }
            "stopPropagation" => {
                let ExprKind::Bool(value) = &value.kind else {
                    return None;
                };
                stop_propagation = *value;
            }
            _ => return None,
        }
    }
    Some(StaticWindowEventOptions {
        options,
        prevent_default,
        stop_propagation,
    })
}

fn window_event_field_expr(field: &str) -> Option<&'static str> {
    match field {
        "type" => Some("event.type || \"\""),
        "clientX" => Some("event.clientX || 0"),
        "clientY" => Some("event.clientY || 0"),
        "pageX" => Some("event.pageX || 0"),
        "pageY" => Some("event.pageY || 0"),
        "screenX" => Some("event.screenX || 0"),
        "screenY" => Some("event.screenY || 0"),
        "movementX" => Some("event.movementX || 0"),
        "movementY" => Some("event.movementY || 0"),
        "button" => Some("event.button || 0"),
        "buttons" => Some("event.buttons || 0"),
        "pointerId" => Some("event.pointerId || 0"),
        "pointerType" => Some("event.pointerType || \"\""),
        "isPrimary" => Some("!!event.isPrimary"),
        "key" => Some("event.key || \"\""),
        "code" => Some("event.code || \"\""),
        "altKey" => Some("!!event.altKey"),
        "ctrlKey" => Some("!!event.ctrlKey"),
        "metaKey" => Some("!!event.metaKey"),
        "shiftKey" => Some("!!event.shiftKey"),
        "targetEditable" => Some(
            "(() => { const el = event.target; const tag = String(el?.tagName || \"\").toLowerCase(); const fileBrowser = el?.getAttribute?.(\"data-testid\") === \"file-browser\"; return tag === \"input\" || tag === \"textarea\" || tag === \"select\" || (!!el?.isContentEditable && !fileBrowser); })()",
        ),
        _ => None,
    }
}

fn literal_string_value(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::String(value) => Some(value),
        ExprKind::Keyword(value) => Some(value),
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

fn scope_view_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "scope-view",
        Vec::new(),
        "__closkellScopeView(\"scope\", () => ({ mount() {}, update() {}, dispose() {}, root: null }), undefined)",
        vec![js_emit::IntrinsicCallEmitForm::new(
            3,
            "__closkellScopeView({0}, {1}, {2})",
        )],
    )
    .html_runtime()
    .runtime_import("scopeView", "__closkellScopeView")
}

fn render_to_string_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "render-to-string",
        Vec::new(),
        "__closkellRenderToString(null)",
        vec![
            js_emit::IntrinsicCallEmitForm::new(1, "__closkellRenderToString({0})"),
            js_emit::IntrinsicCallEmitForm::new(2, "__closkellRenderToString({0}, {1})"),
        ],
    )
    .html_runtime()
    .runtime_import("renderToString", "__closkellRenderToString")
}

fn browser_current_url_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "browser-current-url",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            0,
            "globalThis.location?.href ?? \"\"",
        )],
    )
}

fn history_replace_search_param_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "history-replace-search-param",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            2,
            "((__name, __value) => { try { const __next = new URL(globalThis.location?.href ?? \"http://localhost/\"); if (__value === null || __value === undefined || String(__value) === \"\") __next.searchParams.delete(__name); else __next.searchParams.set(__name, String(__value)); globalThis.history?.replaceState?.(null, \"\", `${__next.pathname}${__next.search}${__next.hash}`); } catch {} return null; })({0}, {1})",
        )],
    )
}

fn history_write_route_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "history-write-route",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            3,
            r#"((__url, __op, __definition) => {
  try {
    const __params = new URLSearchParams();
    if (__url !== null && __url !== undefined && String(__url) !== "") __params.set("url", String(__url));
    if (__definition !== null && __definition !== undefined && String(__definition) !== "") __params.set("definition", String(__definition));
    if (__op !== null && __op !== undefined && String(__op) !== "") __params.set("op", String(__op));
    const __query = __params.toString();
    const __next = __query ? `${globalThis.location?.pathname ?? "/"}?${__query}` : (globalThis.location?.pathname ?? "/");
    if (__next !== `${globalThis.location?.pathname ?? "/"}${globalThis.location?.search ?? ""}`) globalThis.history?.replaceState?.(null, "", __next);
  } catch {}
  return null;
})({0}, {1}, {2})"#,
        )],
    )
}

fn browser_theme_initial_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "browser-theme-initial",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            1,
            "((__key) => { const __stored = globalThis.sessionStorage?.getItem(__key) ?? globalThis.localStorage?.getItem(__key); const __theme = __stored === \"light\" ? \"light\" : \"dark\"; globalThis.document?.documentElement?.classList?.toggle(\"dark\", __theme === \"dark\"); try { globalThis.sessionStorage?.setItem(__key, __theme); globalThis.localStorage?.setItem(__key, __theme); } catch {} return __theme; })({0})",
        )],
    )
}

fn browser_theme_toggle_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "browser-theme-toggle",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            2,
            "((__current, __key) => { const __theme = __current === \"dark\" ? \"light\" : \"dark\"; globalThis.document?.documentElement?.classList?.toggle(\"dark\", __theme === \"dark\"); try { globalThis.sessionStorage?.setItem(__key, __theme); globalThis.localStorage?.setItem(__key, __theme); } catch {} return __theme; })({0}, {1})",
        )],
    )
}

fn clipboard_text_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "clipboard-text",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            1,
            "({0}?.clipboardData?.getData?.(\"text/plain\") ?? \"\")",
        )],
    )
}

fn clipboard_write_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "clipboard-write",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            1,
            "((__text) => { void globalThis.navigator?.clipboard?.writeText?.(String(__text)); return null; })({0})",
        )],
    )
}

fn browser_set_cookie_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "browser-set-cookie",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            2,
            "((__name, __value) => { if (globalThis.document) globalThis.document.cookie = `${encodeURIComponent(__name)}=${encodeURIComponent(__value)}; path=/`; return null; })({0}, {1})",
        )],
    )
}

fn auth_storage_load_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "auth-storage-load",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            1,
            r#"((__sourceUrl) => {
  const __key = `better-swagger-auth:${String(__sourceUrl)}`;
  const __raw = globalThis.localStorage?.getItem(__key) ?? globalThis.sessionStorage?.getItem(__key);
  if (!__raw) return {};
  try {
    const __parsed = JSON.parse(__raw);
    const __now = Date.now();
    const __valid = (Array.isArray(__parsed) ? __parsed : []).filter((__entry) => !__entry?.expiresAt || __entry.expiresAt > __now);
    const __entries = Object.fromEntries(__valid.map((__entry) => [__entry.schemeId, __entry]));
    if (__valid.length !== (Array.isArray(__parsed) ? __parsed.length : 0) || globalThis.sessionStorage?.getItem(__key)) {
      globalThis.localStorage?.setItem(__key, JSON.stringify(Object.values(__entries)));
      globalThis.sessionStorage?.removeItem(__key);
    }
    return __entries;
  } catch {
    return {};
  }
})({0})"#,
        )],
    )
}

fn auth_storage_persist_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "auth-storage-persist",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            2,
            r#"((__sourceUrl, __entries) => {
  const __key = `better-swagger-auth:${String(__sourceUrl)}`;
  try {
    globalThis.localStorage?.setItem(__key, JSON.stringify(Object.values(__entries ?? {})));
  } catch {}
  return null;
})({0}, {1})"#,
        )],
    )
}

fn selected_file_or_blob_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "selected-file-or-blob",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            4,
            r#"((__testId, __content, __name, __type) => { const __input = globalThis.document?.querySelector?.(`[data-testid="${__testId}"]`); const __selected = __input?.files?.[0]; if (__selected) return __selected; if (typeof File === "function") return new File([String(__content)], String(__name), { type: String(__type) }); return new Blob([String(__content)], { type: String(__type) }); })({0}, {1}, {2}, {3})"#,
        )],
    )
}

fn selected_file_by_test_id_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "selected-file-by-test-id",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            1,
            r#"((__testId) => { const __safe = String(__testId).replace(/\\/g, "\\\\").replace(/"/g, "\\\""); return globalThis.document?.querySelector?.(`[data-testid="${__safe}"]`)?.files?.[0] ?? null; })({0})"#,
        )],
    )
}

fn has_selected_file_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "has-selected-file",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            1,
            r#"((__testId) => { const __safe = String(__testId).replace(/\\/g, "\\\\").replace(/"/g, "\\\""); return Boolean(globalThis.document?.querySelector?.(`[data-testid="${__safe}"]`)?.files?.[0]); })({0})"#,
        )],
    )
}

fn multipart_form_body_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "multipart-form-body",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            2,
            r#"((__fields, __values) => { const __form = new FormData(); const __attr = (__value) => String(__value).replace(/\\/g, "\\\\").replace(/"/g, "\\\""); const __byTestId = (__id) => globalThis.document?.querySelector?.(`[data-testid="${__attr(__id)}"]`); for (const __field of Array.isArray(__fields) ? __fields : []) { const __name = String(__field?.name ?? ""); if (!__name) continue; if (__field?.kind === "file") { const __file = __byTestId(`request-body-multipart-${__name}`)?.files?.[0]; if (__file) __form.append(__name, __file, __file.name); } else { const __value = (__values && Object.prototype.hasOwnProperty.call(__values, __name)) ? __values[__name] : (__byTestId(`request-body-field-${__name}`)?.value ?? ""); const __text = String(__value ?? "").trim(); if (__text) __form.append(__name, __text); } } return __form; })({0}, {1})"#,
        )],
    )
}

fn urlencoded_form_body_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "urlencoded-form-body",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            2,
            r#"((__fields, __values) => { const __params = new URLSearchParams(); for (const __field of Array.isArray(__fields) ? __fields : []) { if (__field?.kind === "file") continue; const __name = String(__field?.name ?? ""); if (!__name) continue; const __text = String((__values && Object.prototype.hasOwnProperty.call(__values, __name)) ? __values[__name] : "").trim(); if (__text) __params.append(__name, __text); } return __params.toString(); })({0}, {1})"#,
        )],
    )
}

fn install_virtual_json_viewer_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "install-virtual-json-viewer",
        Vec::new(),
        "undefined",
        vec![js_emit::IntrinsicCallEmitForm::new(
            0,
            r#"(() => {
  if (globalThis.customElements?.get?.("virtual-json-viewer")) return null;
  const LINE_HEIGHT = 20;
  const OVERSCAN = 15;
  const escapeHtml = (__text) => String(__text)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  const prettyText = (__raw) => {
    const __trimmed = String(__raw ?? "").trim();
    if (!__trimmed) return "";
    try { return JSON.stringify(JSON.parse(__trimmed), null, 2); }
    catch { return String(__raw ?? ""); }
  };
  const highlightLine = (__line) => {
    const __tokens = [];
    const __pattern = /("(?:\\.|[^"\\])*")(\s*:)?|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|\btrue\b|\bfalse\b|\bnull\b/g;
    let __last = 0;
    for (const __match of String(__line).matchAll(__pattern)) {
      __tokens.push(escapeHtml(String(__line).slice(__last, __match.index)));
      const __token = __match[0];
      const __escaped = escapeHtml(__token);
      if (__match[1] && __match[2]) __tokens.push(`<span class="hljs-attr">${escapeHtml(__match[1])}</span>${escapeHtml(__match[2])}`);
      else if (__match[1]) __tokens.push(`<span class="hljs-string">${__escaped}</span>`);
      else if (/^-?\d/.test(__token)) __tokens.push(`<span class="hljs-number">${__escaped}</span>`);
      else __tokens.push(`<span class="hljs-literal">${__escaped}</span>`);
      __last = (__match.index ?? 0) + __token.length;
    }
    __tokens.push(escapeHtml(String(__line).slice(__last)));
    return __tokens.join("");
  };
  class VirtualJsonViewer extends HTMLElement {
    constructor() {
      super();
      this.__lines = [""];
      this.__scroll = null;
      this.__spacer = null;
      this.__rows = null;
      this.__onScroll = () => this.__updateRows();
    }
    static get observedAttributes() { return ["data-json", "data-version", "max-height", "highlight"]; }
    connectedCallback() { this.__renderShell(); this.__rebuild(); }
    disconnectedCallback() { this.__scroll?.removeEventListener("scroll", this.__onScroll); }
    attributeChangedCallback() { if (this.isConnected) this.__rebuild(); }
    __bindScroll() {
      if (!this.__scroll) return;
      this.__scroll.removeEventListener("scroll", this.__onScroll);
      this.__scroll.addEventListener("scroll", this.__onScroll, { passive: true });
    }
    __renderShell() {
      if (this.__scroll) return;
      this.classList.add("block");
      this.innerHTML = `<div class="virtual-json-scroll overflow-auto rounded-lg border border-zinc-200 bg-zinc-50 font-mono text-[13px] dark:border-zinc-800 dark:bg-zinc-900/80"><div class="virtual-json-spacer relative w-full"><div class="virtual-json-rows absolute inset-x-0 top-0"></div></div></div>`;
      this.__scroll = this.querySelector(".virtual-json-scroll");
      this.__spacer = this.querySelector(".virtual-json-spacer");
      this.__rows = this.querySelector(".virtual-json-rows");
      this.__scroll.style.maxHeight = this.getAttribute("max-height") || "24rem";
      this.__bindScroll();
    }
    __rebuild() {
      this.__renderShell();
      this.__bindScroll();
      this.__scroll.style.maxHeight = this.getAttribute("max-height") || "24rem";
      this.__scroll.scrollTop = 0;
      const __text = prettyText(this.getAttribute("data-json") ?? "");
      const __highlight = this.getAttribute("highlight") !== "false";
      this.__lines = (__text ? __text.split("\n") : [""]).map((__line) => __highlight ? highlightLine(__line) : escapeHtml(__line));
      this.__spacer.style.height = `${this.__lines.length * LINE_HEIGHT}px`;
      this.__updateRows();
    }
    __updateRows() {
      if (!this.__scroll || !this.__rows) return;
      const __height = this.__scroll.clientHeight || 384;
      const __first = Math.max(0, Math.floor(this.__scroll.scrollTop / LINE_HEIGHT) - OVERSCAN);
      const __count = Math.ceil(__height / LINE_HEIGHT) + OVERSCAN * 2;
      const __last = Math.min(this.__lines.length, __first + __count);
      let __html = "";
      for (let __index = __first; __index < __last; __index += 1) {
        __html += `<div class="absolute left-0 top-0 flex w-full px-3 leading-5 whitespace-pre" style="height:${LINE_HEIGHT}px;transform:translateY(${__index * LINE_HEIGHT}px)"><span class="mr-3 w-8 shrink-0 text-right text-zinc-400 select-none dark:text-zinc-600">${__index + 1}</span><span>${this.__lines[__index] ?? ""}</span></div>`;
      }
      this.__rows.innerHTML = __html;
    }
  }
  globalThis.customElements?.define?.("virtual-json-viewer", VirtualJsonViewer);
  return null;
})()"#,
        )],
    )
}

const BROWSER_FRAMEWORK_SYMBOLS: &[&str] = &[
    "scope-view",
    "render-to-string",
    "Cmd.storage/get",
    "Cmd.storage/set",
    "Cmd.storage/set-silent",
    "Cmd.dom-ref/click",
    "Cmd.dom-ref/focus",
    "Cmd.dom-ref/measure",
    "Cmd.dom-ref/resize-watch",
    "Cmd.file/read-selected",
    "Cmd.file/download",
    "Cmd.canvas/draw",
    "Cmd.bluetooth/connect-heart-rate",
    "Cmd.bluetooth/disconnect",
    "Cmd.simulation/heart-rate",
    "Cmd.simulation/stop",
    "browser-current-url",
    "browser-theme-initial",
    "browser-theme-toggle",
    "history-replace-search-param",
    "history-write-route",
    "auth-storage-load",
    "auth-storage-persist",
    "clipboard-text",
    "clipboard-write",
    "browser-set-cookie",
    "selected-file-or-blob",
    "selected-file-by-test-id",
    "has-selected-file",
    "multipart-form-body",
    "urlencoded-form-body",
    "install-virtual-json-viewer",
];

const EVENT_MUTATION_SYMBOLS: &[&str] = &["event.preventDefault", "event.stopPropagation"];
const EVENT_MUTATION_PREFIXES: &[&str] = &["event.preventDefault.", "event.stopPropagation."];

const BROWSER_API_SYMBOLS: &[&str] = &[
    "window",
    "document",
    "navigator",
    "history",
    "localStorage",
    "sessionStorage",
    "fetch",
    "setTimeout",
    "setInterval",
    "clearTimeout",
    "clearInterval",
    "requestAnimationFrame",
    "cancelAnimationFrame",
    "browser-current-url",
    "browser-theme-initial",
    "browser-theme-toggle",
    "history-replace-search-param",
    "history-write-route",
    "auth-storage-load",
    "auth-storage-persist",
    "clipboard-write",
    "browser-set-cookie",
    "selected-file-or-blob",
    "selected-file-by-test-id",
    "has-selected-file",
    "multipart-form-body",
];

const BROWSER_API_PREFIXES: &[&str] = &[
    "window.",
    "document.",
    "navigator.",
    "location.",
    "history.",
    "localStorage.",
    "sessionStorage.",
    "fetch.",
    "setTimeout.",
    "setInterval.",
    "clearTimeout.",
    "clearInterval.",
    "requestAnimationFrame.",
    "cancelAnimationFrame.",
];

struct AppRuntimeRegistration {
    import_name: &'static str,
    kinds: &'static [&'static str],
}

const APP_RUNTIME_REGISTRATIONS: &[AppRuntimeRegistration] = &[
    AppRuntimeRegistration {
        import_name: "registerCompiledAnimationCommandHandlers",
        kinds: &["animation/frame", "animation/cancel"],
    },
    AppRuntimeRegistration {
        import_name: "registerAuthStorageCommandHandlers",
        kinds: &["auth-storage/persist", "auth-storage/load"],
    },
    AppRuntimeRegistration {
        import_name: "registerBluetoothCommandHandlers",
        kinds: &["bluetooth/request-device"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledBluetoothHeartRateCommandHandlers",
        kinds: &[
            "bluetooth/connect-heart-rate",
            "bluetooth/disconnect",
            "sub/bluetooth/connect-heart-rate",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerBrowserCommandHandlers",
        kinds: &[
            "browser/history-replace-search-param",
            "browser/history-write-route",
            "browser/theme-load",
            "browser/theme-apply",
            "browser/clipboard-write",
            "browser/set-cookie",
            "browser/history-push",
            "browser/history-replace",
            "browser/location-assign",
            "browser/open-url",
            "browser/download-url",
            "browser/scroll-to",
            "event-source/open",
            "event-source/close",
            "media/play-selector",
            "media/restore-current-time",
            "media/sync-audio-element",
            "dom/breadcrumb-adaptive",
            "dom/focus-selector",
            "dom/document-set-attribute",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledCanvasDrawCommandHandlers",
        kinds: &["canvas/draw"],
    },
    AppRuntimeRegistration {
        import_name: "registerCanvasMeasureTextCommandHandlers",
        kinds: &["canvas/measure-text"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledDomRefCommandHandlers",
        kinds: &[
            "dom-ref/focus",
            "dom-ref/click",
            "dom-ref/measure",
            "dom/input-set-selection",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledDomResizeCommandHandlers",
        kinds: &[
            "dom-ref/resize-watch",
            "dom-ref/resize-unwatch",
            "sub/dom-ref/resize",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledDirectDomResizeCommandHandlers",
        kinds: &["dom-ref/resize-watch/direct"],
    },
    AppRuntimeRegistration {
        import_name: "registerDomScrollCommandHandlers",
        kinds: &["dom/scroll-into-view"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledFileDownloadCommandHandlers",
        kinds: &["file/download"],
    },
    AppRuntimeRegistration {
        import_name: "registerFileImportCommandHandlers",
        kinds: &["file/import"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledFileReadSelectedCommandHandlers",
        kinds: &["file/read-selected", "file/read-blob"],
    },
    AppRuntimeRegistration {
        import_name: "registerHttpCommandHandlers",
        kinds: &["http/request"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledMediaQueryCommandHandlers",
        kinds: &[
            "media-query/watch",
            "media-query/unwatch",
            "sub/media-query",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledDirectMediaQueryCommandHandlers",
        kinds: &["media-query/watch/direct", "media-query/unwatch/direct"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledRandomCommandHandlers",
        kinds: &["random/number"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledSimulationCommandHandlers",
        kinds: &["simulation/heart-rate", "sub/simulation/heart-rate"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledSimulationStopCommandHandlers",
        kinds: &["simulation/stop"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledStorageReadWriteCommandHandlers",
        kinds: &["storage/get", "storage/set"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledStorageRemoveCommandHandlers",
        kinds: &["storage/remove"],
    },
    AppRuntimeRegistration {
        import_name: "registerTaskCommandHandlers",
        kinds: &["task/perform"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledTimerCommandHandlers",
        kinds: &[
            "timer/after",
            "timer/every",
            "timer/cancel",
            "sub/timer/every",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledTimeCommandHandlers",
        kinds: &["time/now"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledWindowEventCommandHandlers",
        kinds: &[
            "window/event-watch",
            "window/event-unwatch",
            "sub/window/event",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledDirectWindowEventCommandHandlers",
        kinds: &["window/event-watch/direct", "window/event-unwatch/direct"],
    },
];

pub fn collect_runtime_registrations<'a, I>(effect_sets: I) -> BTreeSet<&'static str>
where
    I: IntoIterator<Item = &'a BTreeSet<String>>,
{
    let mut registrations = BTreeSet::new();
    for effects in effect_sets {
        collect_runtime_registrations_from_effects(effects, &mut registrations);
    }
    if registrations.contains("registerCompiledSimulationCommandHandlers") {
        registrations.remove("registerCompiledSimulationStopCommandHandlers");
    }
    registrations
}

pub fn runtime_registration_effect_kinds(import_name: &str) -> Option<&'static [&'static str]> {
    APP_RUNTIME_REGISTRATIONS
        .iter()
        .find(|registration| registration.import_name == import_name)
        .map(|registration| registration.kinds)
}

pub fn known_runtime_registrations() -> HashSet<&'static str> {
    APP_RUNTIME_REGISTRATIONS
        .iter()
        .map(|registration| registration.import_name)
        .collect()
}

pub fn wrap_browser_app_module(
    emitted: &mut js_emit::EmitResult,
    options: &BrowserAppOptions,
    runtime_registrations: &BTreeSet<&'static str>,
    init_takes_boot: bool,
    has_subscriptions: bool,
) {
    let prelude = app_bootstrap_prelude(options, runtime_registrations, has_subscriptions);
    let postlude = app_bootstrap_postlude(
        options,
        runtime_registrations,
        init_takes_boot,
        has_subscriptions,
    );
    let inserted_lines = prelude.lines().count();
    for mapping in &mut emitted.source_mappings {
        mapping.generated_line += inserted_lines;
    }
    if !emitted.code.ends_with('\n') {
        emitted.code.push('\n');
    }
    emitted.code = format!("{}{}{}", prelude, emitted.code, postlude);
}

fn collect_runtime_registrations_from_effects(
    effects: &BTreeSet<String>,
    registrations: &mut BTreeSet<&'static str>,
) {
    for registration in APP_RUNTIME_REGISTRATIONS {
        if registration
            .kinds
            .iter()
            .any(|kind| effects.contains(*kind))
        {
            registrations.insert(registration.import_name);
        }
    }
}

fn app_runtime_registration_alias(import_name: &str) -> String {
    let mut chars = import_name.chars();
    let Some(first) = chars.next() else {
        return "__closkellRegister".to_string();
    };
    format!("__closkell{}{}", first.to_ascii_uppercase(), chars.as_str())
}

fn app_bootstrap_prelude(
    options: &BrowserAppOptions,
    runtime_registrations: &BTreeSet<&'static str>,
    has_subscriptions: bool,
) -> String {
    let mut code = String::new();
    let app_runner = if has_subscriptions {
        "startCompiledApp"
    } else {
        "startCompiledAppWithoutSubscriptions"
    };
    let mut imports = vec![format!("{} as __closkellStartApp", app_runner)];
    for import_name in runtime_registrations {
        imports.push(format!(
            "{} as {}",
            import_name,
            app_runtime_registration_alias(import_name)
        ));
    }
    if !imports.is_empty() {
        code.push_str("import { ");
        code.push_str(&imports.join(", "));
        code.push_str(" } from \"@closkell/runtime\";\n");
    }
    if let Some(css) = &options.css {
        code.push_str("import ");
        code.push_str(&json_string(css));
        code.push_str(";\n");
    }
    code
}

fn app_bootstrap_postlude(
    options: &BrowserAppOptions,
    runtime_registrations: &BTreeSet<&'static str>,
    init_takes_boot: bool,
    has_subscriptions: bool,
) -> String {
    let mut code = String::new();
    code.push_str("const __closkellRoot = document.getElementById(");
    code.push_str(&json_string(&options.root_id));
    code.push_str(");\n");
    code.push_str("if (!__closkellRoot) {\n");
    code.push_str("  throw new Error(");
    code.push_str(&json_string(&format!(
        "Root element #{} was not found.",
        options.root_id
    )));
    code.push_str(");\n");
    code.push_str("}\n");
    code.push_str(
        "const __closkellHandlerContext = { env: {}, host: globalThis, disposers: [] };\n",
    );
    code.push_str("const __closkellHandlers = {};\n");
    let registrations = runtime_registrations
        .iter()
        .map(|import_name| app_runtime_registration_alias(import_name))
        .collect::<Vec<_>>();
    for registration in registrations {
        code.push_str(&registration);
        code.push_str("(__closkellHandlers, __closkellHandlerContext);\n");
    }
    code.push_str("Object.defineProperty(__closkellHandlers, \"dispose\", { value() { for (const dispose of __closkellHandlerContext.disposers.splice(0)) dispose(); } });\n");
    code.push_str("export const __closkellApp = __closkellStartApp({\n");
    code.push_str("  root: __closkellRoot,\n");
    code.push_str("  init,\n");
    code.push_str("  update,\n");
    code.push_str("  view,\n");
    if init_takes_boot {
        code.push_str("  boot: { currentUrl: globalThis.location?.href ?? \"\" },\n");
    }
    if has_subscriptions {
        code.push_str("  subscriptions,\n");
    }
    code.push_str("  handlers: __closkellHandlers\n");
    code.push_str("});\n\n");
    code
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn browser_runtime_registrations_are_selected_from_effects() {
        let effects = BTreeSet::from([
            "storage/get".to_string(),
            "simulation/heart-rate".to_string(),
            "simulation/stop".to_string(),
        ]);

        let registrations = collect_runtime_registrations([&effects]);

        assert!(registrations.contains("registerCompiledStorageReadWriteCommandHandlers"));
        assert!(registrations.contains("registerCompiledSimulationCommandHandlers"));
        assert!(
            !registrations.contains("registerCompiledSimulationStopCommandHandlers"),
            "simulation start handler owns the stop handler registration"
        );
    }

    #[test]
    fn browser_effect_options_reject_direct_host_access() {
        let source = syntax::parse_source(
            "(def bad fetch)\n\
             (defn view [state] #html <button on:click={(event.preventDefault)}>Load</button>)",
        );
        let report = effects::validate_purity_with_options(
            &source,
            &HashSet::new(),
            browser_effect_options(),
        );

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("fetch")),
            "{:?}",
            report.diagnostics
        );
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
    fn browser_framework_rejection_flags_html_templates_and_helpers() {
        let source = syntax::parse_source(
            "(defn view [state] #html <p>{state.title}</p>)\n\
             (def command (Cmd.dom-ref/focus \"search\" :focused))",
        );
        let report = effects::validate_purity_with_options(
            &source,
            &HashSet::new(),
            reject_browser_framework_access(effects::EffectOptions::default()),
        );

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("#html")),
            "{:?}",
            report.diagnostics
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Cmd.dom-ref/focus")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn browser_typecheck_rejection_flags_templates_and_types() {
        let source = syntax::parse_source(
            "(ann view (Fn [String] Html))\n\
             (defn view [label] #html <p>{label}</p>)",
        );
        let result = typecheck::check_source_with_module_imports_and_options(
            &source,
            &[],
            &[],
            reject_browser_typecheck_access(typecheck::CheckOptions::default()),
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Html")),
            "{:?}",
            result.diagnostics
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("#html")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn browser_typecheck_records_imported_html_calls_inside_keyed_templates() {
        let dependency = syntax::parse_source(
            "(ann operationPanel (Fn [{:expandedOp String} {:id String :method String}] Html))\n\
             (defn operationPanel [state item] #html <article>{item.method}</article>)",
        );
        let dependency_result = typecheck::check_source_with_module_imports_and_options(
            &dependency,
            &[],
            &[],
            browser_typecheck_options(),
        );
        assert!(
            dependency_result.diagnostics.is_empty(),
            "{:?}",
            dependency_result.diagnostics
        );
        let operation_panel = dependency_result
            .bindings
            .iter()
            .find(|binding| binding.name == "operationPanel")
            .expect("dependency should export operationPanel")
            .import_as("operationPanel");
        let source = syntax::parse_source(
            "(import \"./operation.clsk\" [operationPanel])\n\
             (defn view [state entry]\n  #html <section>{(for [item entry.value :key item.id] #html <div>{(operationPanel state item)}</div>)}</section>)",
        );
        let checked = typecheck::check_source_with_module_imports_and_options(
            &source,
            &[operation_panel],
            &[],
            browser_typecheck_options(),
        );

        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        assert!(
            checked.expr_types.values().any(|ty| ty == "Html"),
            "nested imported component call should be recorded as Html:\n{:?}",
            checked.expr_types
        );
        let view_schema = checked
            .bindings
            .iter()
            .find(|binding| binding.name == "view")
            .expect("view should be exported")
            .schema();
        assert!(
            view_schema.contains(":method String"),
            "keyed item type should include imported component argument requirements:\n{}",
            view_schema
        );
    }

    #[test]
    fn browser_emit_intrinsics_lower_render_to_string() {
        let source = syntax::parse_source(
            "(defn view [state]\n  #html <main>{state.title}</main>)\n\
             (def html (render-to-string view {:title \"Pulse\"}))",
        );
        let mut options = js_emit::EmitOptions::default();
        add_browser_emit_intrinsics(&mut options);
        let emitted =
            js_emit::emit_module_with_types_and_options(&source, BTreeMap::new(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(
            emitted
                .code
                .contains("renderToString as __closkellRenderToString")
        );
        assert!(emitted.code.contains("__closkellRenderToString(view,"));
    }

    #[test]
    fn browser_emit_intrinsics_lower_scope_view() {
        let source = syntax::parse_source(
            "(defn child-view [state]\n  #html <button>{state.count}</button>)\n\
             (defn view [state]\n  #html <main>{(scope-view :log child-view state.log)}</main>)",
        );
        let mut options = js_emit::EmitOptions::default();
        add_browser_emit_intrinsics(&mut options);
        let emitted =
            js_emit::emit_module_with_types_and_options(&source, BTreeMap::new(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("scopeView as __closkellScopeView"));
        assert!(
            emitted
                .code
                .contains("() => __closkellScopeView(Symbol.for(\"log\"), child_view, state.log)")
        );
    }

    #[test]
    fn browser_emit_intrinsics_lower_subscriptions() {
        let source = syntax::parse_source(
            "(defn subscriptions [state]\n  (Sub.batch [(if state.running?\n                   (Sub.timer/every \"clock\" 250 {:kind :tick})\n                   Sub.none)\n              (Sub.media-query \"mobile\" \"(max-width: 700px)\" :media-changed)\n              (Sub.window/event \"dev\" \"keydown\" :key {:passive true})]))",
        );
        let mut options = js_emit::EmitOptions::default();
        add_browser_emit_intrinsics(&mut options);
        let emitted =
            js_emit::emit_module_with_types_and_options(&source, BTreeMap::new(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("kind: Symbol.for(\"batch\")"));
        assert!(emitted.code.contains("timer/every"));
        assert!(emitted.code.contains("media-query"));
        assert!(emitted.code.contains("window/event"));
        assert!(emitted.code.contains("kind: Symbol.for(\"none\")"));
        assert!(!emitted.code.contains("Sub."));

        let source = syntax::parse_source(
            "(defn subscriptions [state]\n  (Sub.window/event-with \"drag\" \"pointermove\" :move {:preventDefault true :options {:passive false}}))",
        );
        let mut options = js_emit::EmitOptions::default();
        add_browser_emit_intrinsics(&mut options);
        let emitted =
            js_emit::emit_module_with_types_and_options(&source, BTreeMap::new(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("window/event"));
        assert!(emitted.code.contains("preventDefault: true"));
        assert!(emitted.code.contains("options:"));
        assert!(!emitted.code.contains("Sub."));
    }

    #[test]
    fn browser_emit_intrinsics_lower_resize_watch_command() {
        let source = syntax::parse_source(
            "(defn update [state msg]\n  (match msg\n    {:kind :resized :width width :value {:height height}} [state Cmd.none]\n    _ [state Cmd.none]))\n\
             (defn init [] [{} (Cmd.dom-ref/resize-watch \"chart\" \"heart-chart\" :resized :failed)])",
        );
        let mut options = js_emit::EmitOptions::default();
        add_browser_emit_intrinsics(&mut options);
        let emitted =
            js_emit::emit_module_with_types_and_options(&source, BTreeMap::new(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("dom-ref/resize-watch/direct"));
        assert!(emitted.code.contains("m: (entry, node) =>"));
        assert!(emitted.code.contains("width: entry?.contentRect?.width"));
        assert!(emitted.code.contains("value: { height:"));
        assert!(!emitted.code.contains("Cmd.dom-ref/resize-watch"));
    }

    #[test]
    fn browser_emit_intrinsics_lower_command_helpers() {
        let source = syntax::parse_source(
            "(def auth (auth-storage-load \"/docs\"))\n\
             (def persisted (auth-storage-persist \"/docs\" auth))\n\
             (defn update [state msg]\n  [state (Cmd.batch [Cmd.none\n                         (Cmd.dom-ref/measure \"track\" (fn [rect] {:kind :measured :left rect.left}) :measure-failed)\n                         (Cmd.bluetooth/connect-heart-rate \"hr\" {:filters [{:services [\"heart_rate\"]}]} (Msg.mapper :connected :info) :heart-rate :disconnected :failed)\n                         (Cmd.simulation/heart-rate \"sim\" {:ms 1000 :min 90 :max 160 :jitter 3 :start 120 :deviceName \"Sim\"} (Msg.mapper :connected :info) :heart-rate :failed)])])",
        );
        let mut options = js_emit::EmitOptions::default();
        add_browser_emit_intrinsics(&mut options);
        let emitted =
            js_emit::emit_module_with_types_and_options(&source, BTreeMap::new(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("better-swagger-auth"));
        assert!(
            emitted
                .code
                .contains("kind: Symbol.for(\"dom-ref/measure\")")
        );
        assert!(
            emitted
                .code
                .contains("kind: Symbol.for(\"bluetooth/connect-heart-rate\")")
        );
        assert!(
            emitted
                .code
                .contains("kind: Symbol.for(\"simulation/heart-rate\")")
        );
        assert!(!emitted.code.contains("Cmd."));
        assert!(!emitted.code.contains("Msg."));
    }

    #[test]
    fn browser_file_form_and_viewer_intrinsics_are_frontend_owned() {
        let source = syntax::parse_source(
            "(def file (selected-file-or-blob \"upload\" \"{}\" \"body.json\" \"application/json\"))\n\
             (def selected (selected-file-by-test-id \"upload\"))\n\
             (def has-file (has-selected-file \"upload\"))\n\
             (def multipart (multipart-form-body [{:name \"file\" :kind \"file\"}] {}))\n\
             (def urlencoded (urlencoded-form-body [] {}))\n\
             (def viewer (install-virtual-json-viewer))",
        );
        let checked = typecheck::check_source_with_module_imports_and_options(
            &source,
            &[],
            &[],
            browser_typecheck_options(),
        );

        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);

        let mut options = js_emit::EmitOptions::default();
        add_browser_emit_intrinsics(&mut options);
        let emitted =
            js_emit::emit_module_with_types_and_options(&source, BTreeMap::new(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("new File"));
        assert!(emitted.code.contains("querySelector"));
        assert!(emitted.code.contains("new FormData"));
        assert!(emitted.code.contains("new URLSearchParams"));
        assert!(emitted.code.contains("virtual-json-viewer"));
    }

    #[test]
    fn browser_app_wrapper_injects_runtime_root_and_css() {
        let mut emitted = js_emit::EmitResult {
            code: "function init() { return [{}, { kind: Symbol.for(\"none\") }]; }\nfunction update(state) { return [state, { kind: Symbol.for(\"none\") }]; }\nfunction view(state) { return state; }\nconst banner = \"export function text should stay literal\";\n".to_string(),
            diagnostics: Vec::new(),
            source_mappings: vec![js_emit::SourceMapping {
                generated_line: 1,
                generated_column: 0,
                source_offset: 0,
            }],
            runtime_effects: BTreeSet::new(),
            exports: BTreeMap::new(),
        };
        let registrations = BTreeSet::from(["registerCompiledStorageReadWriteCommandHandlers"]);

        wrap_browser_app_module(
            &mut emitted,
            &BrowserAppOptions {
                root_id: "app".to_string(),
                css: Some("./styles.css".to_string()),
            },
            &registrations,
            false,
            false,
        );

        assert!(emitted.code.contains("import \"./styles.css\";"));
        assert!(emitted.code.contains("document.getElementById(\"app\")"));
        assert!(
            emitted
                .code
                .contains("registerCompiledStorageReadWriteCommandHandlers")
        );
        assert!(!emitted.code.contains("export function init"));
        assert!(
            emitted
                .code
                .contains("\"export function text should stay literal\"")
        );
        assert!(emitted.source_mappings[0].generated_line > 1);
    }
}
