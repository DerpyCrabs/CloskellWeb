pub fn reject_server_framework_access(options: effects::EffectOptions) -> effects::EffectOptions {
    let mut options = options;
    for prefix in ["Route.", "Response.", "Server."] {
        options = options.forbid_symbol(
            effects::SymbolPattern::prefix(prefix),
            "`{symbol}` is a server framework helper; check or build with --target server",
        );
    }
    options
}

pub fn reject_server_typecheck_access(options: typecheck::CheckOptions) -> typecheck::CheckOptions {
    let mut options = options;
    for name in SERVER_FRAMEWORK_TYPES {
        options = options.forbid_type(
            *name,
            "{type} is a server framework type; check or build with --target server",
        );
    }
    for prefix in ["Route.", "Response.", "Server."] {
        options = options.forbid_symbol(
            typecheck::CheckSymbolPattern::prefix(prefix),
            "{symbol} is a server framework helper; check or build with --target server",
        );
    }
    options
}

const SERVER_FRAMEWORK_TYPES: &[&str] = &[
    "ServerBoot",
    "HttpError",
    "Request",
    "Response",
    "Route",
    "ServerResource",
    "ServerResources",
];

pub fn server_typecheck_options() -> typecheck::CheckOptions {
    add_server_intrinsic_calls(add_server_typecheck_types(
        typecheck::CheckOptions::default(),
    ))
}

pub fn add_server_typecheck_types(options: typecheck::CheckOptions) -> typecheck::CheckOptions {
    options
        .named_type_alias("ServerBoot", server_boot_type())
        .named_type("HttpError", 0)
        .named_type("Request", 0)
        .named_type("Response", 0)
        .named_type("Route", 2)
        .named_type("ServerResource", 1)
        .named_type("ServerResources", 1)
}

pub fn add_server_intrinsic_calls(options: typecheck::CheckOptions) -> typecheck::CheckOptions {
    SERVER_INTRINSIC_CALLS
        .iter()
        .fold(options, |options, builder| {
            options.intrinsic_call(builder())
        })
}

pub fn add_server_emit_intrinsics(options: &mut js_emit::EmitOptions) {
    options
        .intrinsic_calls
        .extend(SERVER_EMIT_INTRINSICS.iter().map(|builder| builder()));
}

type ServerIntrinsicBuilder = fn() -> typecheck::IntrinsicCallRule;
type ServerEmitBuilder = fn() -> js_emit::IntrinsicCallEmitRule;

const SERVER_INTRINSIC_CALLS: &[ServerIntrinsicBuilder] = &[
    route_rule,
    route_get_rule,
    route_post_rule,
    route_put_rule,
    route_patch_rule,
    route_delete_rule,
    response_json_rule,
    response_text_rule,
    response_empty_rule,
    response_redirect_rule,
    response_file_rule,
    response_status_rule,
    server_resource_rule,
    server_resources_rule,
];

const SERVER_EMIT_INTRINSICS: &[ServerEmitBuilder] = &[
    route_emit_rule,
    route_get_emit_rule,
    route_post_emit_rule,
    route_put_emit_rule,
    route_patch_emit_rule,
    route_delete_emit_rule,
    response_json_emit_rule,
    response_text_emit_rule,
    response_empty_emit_rule,
    response_redirect_emit_rule,
    response_file_emit_rule,
    response_status_emit_rule,
    server_resource_emit_rule,
    server_resources_emit_rule,
];

fn route_rule() -> typecheck::IntrinsicCallRule {
    let request = typecheck::CheckType::fresh("request");
    let task = response_task();
    typecheck::IntrinsicCallRule::new(
        "Route.route",
        vec![typecheck::IntrinsicCallOverload::new(
            vec![
                expect(typecheck::CheckType::String),
                expect(typecheck::CheckType::String),
                expect(request.clone()),
                expect(typecheck::CheckType::function(
                    vec![request.clone()],
                    task.clone(),
                )),
            ],
            typecheck::CheckType::apply("Route", vec![request, task]),
        )],
    )
}

fn route_get_rule() -> typecheck::IntrinsicCallRule {
    route_method_rule("Route.get")
}

fn route_post_rule() -> typecheck::IntrinsicCallRule {
    route_method_rule("Route.post")
}

fn route_put_rule() -> typecheck::IntrinsicCallRule {
    route_method_rule("Route.put")
}

fn route_patch_rule() -> typecheck::IntrinsicCallRule {
    route_method_rule("Route.patch")
}

fn route_delete_rule() -> typecheck::IntrinsicCallRule {
    route_method_rule("Route.delete")
}

fn route_method_rule(name: &'static str) -> typecheck::IntrinsicCallRule {
    let task = response_task();
    typecheck::IntrinsicCallRule::new(
        name,
        vec![typecheck::IntrinsicCallOverload::new(
            vec![
                expect(typecheck::CheckType::String),
                expect(typecheck::CheckType::function(
                    vec![typecheck::CheckType::named("Request")],
                    task.clone(),
                )),
            ],
            typecheck::CheckType::apply(
                "Route",
                vec![typecheck::CheckType::named("Request"), task],
            ),
        )],
    )
}

fn response_json_rule() -> typecheck::IntrinsicCallRule {
    response_with_optional_status_rule("Response.json", typecheck::IntrinsicParam::Infer)
}

fn response_text_rule() -> typecheck::IntrinsicCallRule {
    response_with_optional_status_rule("Response.text", expect(typecheck::CheckType::String))
}

fn response_empty_rule() -> typecheck::IntrinsicCallRule {
    typecheck::IntrinsicCallRule::new(
        "Response.empty",
        vec![
            typecheck::IntrinsicCallOverload::new(vec![], response_type()),
            typecheck::IntrinsicCallOverload::new(
                vec![expect(typecheck::CheckType::Number)],
                response_type(),
            ),
        ],
    )
}

fn response_redirect_rule() -> typecheck::IntrinsicCallRule {
    response_with_optional_status_rule("Response.redirect", expect(typecheck::CheckType::String))
}

fn response_file_rule() -> typecheck::IntrinsicCallRule {
    typecheck::IntrinsicCallRule::new(
        "Response.file",
        vec![
            typecheck::IntrinsicCallOverload::new(
                vec![expect(typecheck::CheckType::String)],
                response_type(),
            ),
            typecheck::IntrinsicCallOverload::new(
                vec![
                    expect(typecheck::CheckType::String),
                    typecheck::IntrinsicParam::Infer,
                ],
                response_type(),
            ),
        ],
    )
}

fn response_status_rule() -> typecheck::IntrinsicCallRule {
    typecheck::IntrinsicCallRule::new(
        "Response.status",
        vec![typecheck::IntrinsicCallOverload::new(
            vec![
                expect(typecheck::CheckType::Number),
                expect(response_type()),
            ],
            response_type(),
        )],
    )
}

fn server_resource_rule() -> typecheck::IntrinsicCallRule {
    let payload = typecheck::CheckType::fresh("payload");
    let msg = typecheck::CheckType::fresh("msg");
    typecheck::IntrinsicCallRule::new(
        "Server.resource",
        vec![
            typecheck::IntrinsicCallOverload::new(
                vec![
                    expect(typecheck::CheckType::String),
                    typecheck::IntrinsicParam::Infer,
                ],
                typecheck::CheckType::apply(
                    "ServerResource",
                    vec![typecheck::CheckType::fresh("msg")],
                ),
            ),
            typecheck::IntrinsicCallOverload::new(
                vec![
                    expect(typecheck::CheckType::String),
                    typecheck::IntrinsicParam::Infer,
                    expect(typecheck::CheckType::function(vec![payload], msg.clone())),
                ],
                typecheck::CheckType::apply("ServerResource", vec![msg]),
            ),
        ],
    )
}

fn server_resources_rule() -> typecheck::IntrinsicCallRule {
    let msg = typecheck::CheckType::fresh("msg");
    let resource = typecheck::CheckType::apply("ServerResource", vec![msg.clone()]);
    typecheck::IntrinsicCallRule::new(
        "Server.resources",
        vec![
            typecheck::IntrinsicCallOverload::new(
                vec![expect(typecheck::CheckType::vector(resource.clone()))],
                typecheck::CheckType::apply("ServerResources", vec![msg.clone()]),
            ),
            typecheck::IntrinsicCallOverload::new(
                vec![expect(typecheck::CheckType::list(resource))],
                typecheck::CheckType::apply("ServerResources", vec![msg]),
            ),
        ],
    )
}

fn response_with_optional_status_rule(
    name: &'static str,
    body: typecheck::IntrinsicParam,
) -> typecheck::IntrinsicCallRule {
    typecheck::IntrinsicCallRule::new(
        name,
        vec![
            typecheck::IntrinsicCallOverload::new(vec![body.clone()], response_type()),
            typecheck::IntrinsicCallOverload::new(
                vec![body, expect(typecheck::CheckType::Number)],
                response_type(),
            ),
        ],
    )
}

fn response_task() -> typecheck::CheckType {
    typecheck::CheckType::task(typecheck::CheckType::named("HttpError"), response_type())
}

fn response_type() -> typecheck::CheckType {
    typecheck::CheckType::named("Response")
}

fn server_boot_type() -> typecheck::CheckType {
    typecheck::CheckType::record([
        (
            "argv",
            typecheck::CheckType::vector(typecheck::CheckType::String),
        ),
        ("cwd", typecheck::CheckType::String),
        ("env", typecheck::CheckType::Js),
        ("mode", typecheck::CheckType::String),
        ("runtime", typecheck::CheckType::String),
    ])
}

fn expect(ty: typecheck::CheckType) -> typecheck::IntrinsicParam {
    typecheck::IntrinsicParam::expect(ty)
}

fn route_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "Route.route",
        vec!["server/route".to_string()],
        "{ kind: Symbol.for(\"server/route\"), method: \"GET\", path: \"\", request: {}, handler: () => ({ kind: Symbol.for(\"task/fail\"), error: \"invalid route\" }) }",
        vec![js_emit::IntrinsicCallEmitForm::new(
            4,
            "{ kind: Symbol.for(\"server/route\"), method: {0}, path: {1}, request: {2}, handler: {3} }",
        )],
    )
}

fn route_get_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    route_method_emit_rule("Route.get", "GET")
}

fn route_post_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    route_method_emit_rule("Route.post", "POST")
}

fn route_put_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    route_method_emit_rule("Route.put", "PUT")
}

fn route_patch_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    route_method_emit_rule("Route.patch", "PATCH")
}

fn route_delete_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    route_method_emit_rule("Route.delete", "DELETE")
}

fn route_method_emit_rule(
    name: &'static str,
    method: &'static str,
) -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        name,
        vec!["server/route".to_string()],
        format!(
            "{{ kind: Symbol.for(\"server/route\"), method: \"{}\", path: \"\", request: {{ kind: Symbol.for(\"request\") }}, handler: () => ({{ kind: Symbol.for(\"task/fail\"), error: \"invalid route\" }}) }}",
            method
        ),
        vec![js_emit::IntrinsicCallEmitForm::new(
            2,
            format!(
                "{{ kind: Symbol.for(\"server/route\"), method: \"{}\", path: {{0}}, request: {{ kind: Symbol.for(\"request\") }}, handler: {{1}} }}",
                method
            ),
        )],
    )
}

fn response_json_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    response_emit_rule(
        "Response.json",
        "server/response/json",
        "response/json",
        "json",
        "200",
        "{ kind: Symbol.for(\"response/json\"), body: null, status: 500, headers: {} }",
    )
}

fn response_text_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    response_emit_rule(
        "Response.text",
        "server/response/text",
        "response/text",
        "text",
        "200",
        "{ kind: Symbol.for(\"response/text\"), body: \"\", status: 500, headers: {} }",
    )
}

fn response_empty_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "Response.empty",
        vec!["server/response/empty".to_string()],
        "{ kind: Symbol.for(\"response/empty\"), status: 500, headers: {} }",
        vec![
            js_emit::IntrinsicCallEmitForm::new(
                0,
                "{ kind: Symbol.for(\"response/empty\"), status: 204, headers: {} }",
            ),
            js_emit::IntrinsicCallEmitForm::new(
                1,
                "{ kind: Symbol.for(\"response/empty\"), status: {0}, headers: {} }",
            ),
        ],
    )
}

fn response_redirect_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    response_emit_rule(
        "Response.redirect",
        "server/response/redirect",
        "response/redirect",
        "redirect",
        "302",
        "{ kind: Symbol.for(\"response/redirect\"), location: \"/\", status: 302, headers: {} }",
    )
}

fn response_file_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "Response.file",
        vec!["server/response/file".to_string()],
        "{ kind: Symbol.for(\"response/file\"), path: \"\", options: {} }",
        vec![
            js_emit::IntrinsicCallEmitForm::new(
                1,
                "{ kind: Symbol.for(\"response/file\"), path: {0}, options: {} }",
            ),
            js_emit::IntrinsicCallEmitForm::new(
                2,
                "{ kind: Symbol.for(\"response/file\"), path: {0}, options: {1} }",
            ),
        ],
    )
}

fn response_status_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "Response.status",
        vec!["server/response/status".to_string()],
        "{ kind: Symbol.for(\"response/empty\"), status: 500, headers: {} }",
        vec![js_emit::IntrinsicCallEmitForm::new(
            2,
            "Object.assign({}, {1}, { status: {0} })",
        )],
    )
}

fn server_resource_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "Server.resource",
        vec!["server/resource".to_string()],
        "{ kind: Symbol.for(\"server/resource\"), resource: \"invalid\", config: {} }",
        vec![
            js_emit::IntrinsicCallEmitForm::new(
                2,
                "{ kind: Symbol.for(\"server/resource\"), resource: {0}, config: {1} }",
            ),
            js_emit::IntrinsicCallEmitForm::new(
                3,
                "{ kind: Symbol.for(\"server/resource\"), resource: {0}, config: {1}, onEvent: {2} }",
            ),
        ],
    )
}

fn server_resources_emit_rule() -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        "Server.resources",
        vec!["server/resources".to_string()],
        "{ kind: Symbol.for(\"server/resources\"), resources: [] }",
        vec![js_emit::IntrinsicCallEmitForm::new(
            1,
            "{ kind: Symbol.for(\"server/resources\"), resources: {0} }",
        )],
    )
}

fn response_emit_rule(
    name: &'static str,
    effect: &'static str,
    kind: &'static str,
    response_type: &'static str,
    default_status: &'static str,
    fallback: &'static str,
) -> js_emit::IntrinsicCallEmitRule {
    js_emit::IntrinsicCallEmitRule::new(
        name,
        vec![effect.to_string()],
        fallback,
        vec![
            js_emit::IntrinsicCallEmitForm::new(
                1,
                format!(
                    "{{ kind: Symbol.for(\"{}\"), {}: {{0}}, status: {}, headers: {{}} }}",
                    kind,
                    if response_type == "redirect" {
                        "location"
                    } else {
                        "body"
                    },
                    default_status
                ),
            ),
            js_emit::IntrinsicCallEmitForm::new(
                2,
                format!(
                    "{{ kind: Symbol.for(\"{}\"), {}: {{0}}, status: {{1}}, headers: {{}} }}",
                    kind,
                    if response_type == "redirect" {
                        "location"
                    } else {
                        "body"
                    }
                ),
            ),
        ],
    )
}

pub fn wrap_server_app_module(emitted: &mut js_emit::EmitResult, main_takes_boot: bool) {
    let postlude = server_app_bootstrap_postlude(main_takes_boot);
    if !emitted.code.ends_with('\n') {
        emitted.code.push('\n');
    }
    emitted.code.push_str(&postlude);
}

fn server_app_bootstrap_postlude(main_takes_boot: bool) -> String {
    let mut code = String::new();
    code.push_str("export const __closkellServerBoot = {\n");
    code.push_str("  argv: globalThis.process?.argv ?? [],\n");
    code.push_str("  env: globalThis.process?.env ?? {},\n");
    code.push_str("  cwd: globalThis.process?.cwd?.() ?? \"\",\n");
    code.push_str("  mode: globalThis.process?.env?.NODE_ENV ?? \"development\",\n");
    code.push_str("  runtime: \"node\"\n");
    code.push_str("};\n");
    if main_takes_boot {
        code.push_str(
            "export const __closkellServerResult = await main(__closkellServerBoot);\n\n",
        );
    } else {
        code.push_str("export const __closkellServerResult = await main();\n\n");
    }
    code
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn server_framework_rejection_flags_server_helpers() {
        let source = syntax::parse_source(
            "(def route (Route.get \"/\" handler))\n\
             (def response (Response.json {:ok true}))",
        );
        let report = effects::validate_purity_with_options(
            &source,
            &std::collections::HashSet::new(),
            reject_server_framework_access(effects::EffectOptions::default()),
        );

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Route.get")),
            "{:?}",
            report.diagnostics
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Response.json")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn server_typecheck_rejection_flags_server_types_and_helpers() {
        let source = syntax::parse_source(
            "(ann handler (Fn [Request] Response))\n\
             (defn handler [request] (Response.text \"ok\"))",
        );
        let result = typecheck::check_source_with_module_imports_and_options(
            &source,
            &[],
            &[],
            reject_server_typecheck_access(typecheck::CheckOptions::default()),
        );

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Request")),
            "{:?}",
            result.diagnostics
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Response.text")),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn server_typecheck_options_type_server_intrinsics() {
        let source = syntax::parse_source(
            "(ann handler (Fn [Request] (Task HttpError Response)))\n\
             (defn handler [request] (Task.succeed (Response.text \"ok\")))\n\
             (def route (Route.get \"/\" handler))\n\
             (def resources (Server.resources [(Server.resource \"events\" {} (fn [payload] {:kind :event :payload payload}))]))",
        );
        let result = typecheck::check_source_with_module_imports_and_options(
            &source,
            &[],
            &[],
            server_typecheck_options(),
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.forms[1].ty,
            "(Route Request (Task HttpError Response))"
        );
        assert!(result.forms[2].ty.starts_with("(ServerResources"));
    }

    #[test]
    fn server_emit_intrinsics_lower_server_helpers() {
        let source = syntax::parse_source(
            "(defn handler [request] (Task.succeed (Response.text \"ok\")))\n\
             (def route (Route.get \"/\" handler))\n\
             (def status (Response.status 201 (Response.json {:ok true})))\n\
             (def resources (Server.resources [(Server.resource \"events\" {} (fn [payload] {:kind :event :payload payload}))]))",
        );
        let mut options = js_emit::EmitOptions::default();
        add_server_emit_intrinsics(&mut options);
        let emitted =
            js_emit::emit_module_with_types_and_options(&source, Default::default(), options);

        assert!(emitted.diagnostics.is_empty(), "{:?}", emitted.diagnostics);
        assert!(emitted.code.contains("Symbol.for(\"server/route\")"));
        assert!(emitted.code.contains("Symbol.for(\"response/text\")"));
        assert!(emitted.code.contains("Object.assign({},"));
        assert!(emitted.code.contains("{ status: 201 }"));
        assert!(emitted.code.contains("Symbol.for(\"server/resources\")"));
        assert!(emitted.runtime_effects.contains("server/route"));
        assert!(emitted.runtime_effects.contains("server/resources"));
    }

    #[test]
    fn server_app_wrapper_invokes_direct_main() {
        let mut emitted = js_emit::EmitResult {
            code: "function main(boot) { return startFastify(boot); }\nconst banner = \"export const text should stay literal\";\n".to_string(),
            diagnostics: Vec::new(),
            source_mappings: vec![js_emit::SourceMapping {
                generated_line: 1,
                generated_column: 0,
                source_offset: 0,
            }],
            runtime_effects: BTreeSet::new(),
            exports: Default::default(),
        };

        wrap_server_app_module(&mut emitted, true);

        assert!(!emitted.code.contains("__closkellService"));
        assert!(emitted.code.contains("export const __closkellServerBoot"));
        assert!(
            emitted.code.contains(
                "export const __closkellServerResult = await main(__closkellServerBoot);"
            )
        );
        assert!(!emitted.code.contains("export function init"));
        assert!(
            emitted
                .code
                .contains("\"export const text should stay literal\"")
        );
        assert_eq!(emitted.source_mappings[0].generated_line, 1);
    }
}
