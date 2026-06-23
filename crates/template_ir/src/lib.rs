use std::collections::{BTreeMap, BTreeSet};

use syntax::{ExprKind, HtmlAttrValue, HtmlNode, SourceFile, format_expr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Template {
    pub nodes: Vec<Node>,
    pub slots: Vec<Slot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedTemplate {
    pub name: String,
    pub template: Template,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: usize,
    pub parent: Option<usize>,
    pub kind: NodeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Element {
        tag: String,
        static_attrs: Vec<(String, String)>,
    },
    Text(String),
    DynamicText,
    KeyedListMarker,
    ConditionalMarker,
    ComponentMarker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    pub id: usize,
    pub node_id: usize,
    pub kind: SlotKind,
    pub expr: String,
    pub reads: Vec<String>,
    pub component_uses: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotKind {
    Text,
    Attr(String),
    Event(String),
    Ref,
    KeyedList {
        item: String,
        index: Option<String>,
        key: String,
    },
    Conditional,
    Component {
        name: String,
    },
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

pub fn lower_source(source: &SourceFile) -> Vec<Template> {
    let components = collect_template_defns(source);
    let read_summaries = collect_read_summaries(source);
    source
        .forms
        .iter()
        .filter_map(|form| match &form.kind {
            ExprKind::HtmlTemplate(node) => Some(lower_template_with_components(
                node,
                &components,
                &read_summaries,
            )),
            _ => None,
        })
        .collect()
}

pub fn lower_named_templates(source: &SourceFile) -> Vec<NamedTemplate> {
    let components = collect_template_defns(source);
    let read_summaries = collect_read_summaries(source);
    let mut anonymous_index = 0;
    source
        .forms
        .iter()
        .filter_map(|form| {
            let (name, template_expr) = match &form.kind {
                ExprKind::HtmlTemplate(node) => {
                    let name = format!("template{}", anonymous_index);
                    anonymous_index += 1;
                    (
                        name,
                        TemplateExpr {
                            node: node.as_ref(),
                            read_aliases: BTreeMap::new(),
                        },
                    )
                }
                ExprKind::List(items) if items.len() >= 3 && matches_symbol(&items[0], "def") => {
                    let ExprKind::Symbol(name) = &items[1].kind else {
                        return None;
                    };
                    (name.clone(), template_expr(&items[2], &read_summaries)?)
                }
                ExprKind::List(items) if items.len() >= 4 && matches_symbol(&items[0], "defn") => {
                    let ExprKind::Symbol(name) = &items[1].kind else {
                        return None;
                    };
                    (name.clone(), template_expr(items.last()?, &read_summaries)?)
                }
                _ => return None,
            };
            Some(NamedTemplate {
                name,
                template: lower_template_with_components_and_aliases(
                    template_expr.node,
                    &components,
                    &read_summaries,
                    &template_expr.read_aliases,
                ),
            })
        })
        .collect()
}

pub fn lower_template(root: &HtmlNode) -> Template {
    lower_template_with_components(root, &BTreeSet::new(), &BTreeMap::new())
}

fn lower_template_with_components(
    root: &HtmlNode,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Template {
    lower_template_with_components_and_aliases(root, components, read_summaries, &BTreeMap::new())
}

fn lower_template_with_components_and_aliases(
    root: &HtmlNode,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
    read_aliases: &ReadAliases,
) -> Template {
    let mut lowerer = Lowerer {
        components,
        read_summaries,
        read_aliases,
        nodes: Vec::new(),
        slots: Vec::new(),
    };
    lowerer.lower_node(root, None);
    Template {
        nodes: lowerer.nodes,
        slots: lowerer.slots,
    }
}

struct Lowerer<'a> {
    components: &'a BTreeSet<String>,
    read_summaries: &'a BTreeMap<String, ReadSummary>,
    read_aliases: &'a ReadAliases,
    nodes: Vec<Node>,
    slots: Vec<Slot>,
}

impl Lowerer<'_> {
    fn lower_node(&mut self, node: &HtmlNode, parent: Option<usize>) -> usize {
        match node {
            HtmlNode::Element(element) => {
                let node_id = self.next_node_id();
                let mut static_attrs = Vec::new();

                for attr in &element.attrs {
                    if attr.name == "ref" {
                        match &attr.value {
                            HtmlAttrValue::Bool(true) => {}
                            HtmlAttrValue::Bool(false) => {}
                            HtmlAttrValue::Static(value) => {
                                self.push_static_slot(node_id, SlotKind::Ref, value);
                            }
                            HtmlAttrValue::Dynamic { expr, .. } => {
                                self.push_slot(node_id, SlotKind::Ref, expr);
                            }
                        }
                        continue;
                    }

                    match &attr.value {
                        HtmlAttrValue::Bool(true) => {
                            static_attrs.push((attr.name.clone(), "true".to_string()));
                        }
                        HtmlAttrValue::Bool(false) => {
                            static_attrs.push((attr.name.clone(), "false".to_string()));
                        }
                        HtmlAttrValue::Static(value) => {
                            static_attrs.push((attr.name.clone(), value.clone()));
                        }
                        HtmlAttrValue::Dynamic { expr, .. } => {
                            let kind = if let Some(event) = attr.name.strip_prefix("on:") {
                                SlotKind::Event(event.to_string())
                            } else {
                                SlotKind::Attr(attr.name.clone())
                            };
                            self.push_slot(node_id, kind, expr);
                        }
                    }
                }

                self.nodes.push(Node {
                    id: node_id,
                    parent,
                    kind: NodeKind::Element {
                        tag: element.tag.clone(),
                        static_attrs,
                    },
                });

                for child in &element.children {
                    self.lower_node(child, Some(node_id));
                }
                node_id
            }
            HtmlNode::Text { text, .. } => {
                let node_id = self.next_node_id();
                self.nodes.push(Node {
                    id: node_id,
                    parent,
                    kind: NodeKind::Text(text.clone()),
                });
                node_id
            }
            HtmlNode::Expr { expr, .. } => {
                let node_id = self.next_node_id();
                if let Some(spec) = ForSpec::parse(expr) {
                    self.nodes.push(Node {
                        id: node_id,
                        parent,
                        kind: NodeKind::KeyedListMarker,
                    });
                    let id = self.slots.len();
                    self.slots.push(Slot {
                        id,
                        node_id,
                        kind: SlotKind::KeyedList {
                            item: spec.item.to_string(),
                            index: spec.index.map(str::to_string),
                            key: syntax::format_expr(spec.key),
                        },
                        expr: syntax::format_expr(expr),
                        reads: expand_reads(
                            spec.reads(self.components, self.read_summaries),
                            self.read_aliases,
                        ),
                        component_uses: collect_html_component_uses(spec.template, self.components),
                    });
                    return node_id;
                }
                if let Some(spec) = IfSpec::parse(expr, self.components) {
                    self.nodes.push(Node {
                        id: node_id,
                        parent,
                        kind: NodeKind::ConditionalMarker,
                    });
                    let id = self.slots.len();
                    self.slots.push(Slot {
                        id,
                        node_id,
                        kind: SlotKind::Conditional,
                        expr: syntax::format_expr(expr),
                        reads: expand_reads(
                            spec.reads(self.components, self.read_summaries),
                            self.read_aliases,
                        ),
                        component_uses: spec.component_uses(self.components),
                    });
                    return node_id;
                }
                if let Some(spec) = ComponentSpec::parse(expr, self.components) {
                    self.nodes.push(Node {
                        id: node_id,
                        parent,
                        kind: NodeKind::ComponentMarker,
                    });
                    let id = self.slots.len();
                    self.slots.push(Slot {
                        id,
                        node_id,
                        kind: SlotKind::Component {
                            name: spec.name.to_string(),
                        },
                        expr: syntax::format_expr(expr),
                        reads: expand_reads(
                            component_call_reads(expr, &spec, self.read_summaries),
                            self.read_aliases,
                        ),
                        component_uses: component_spec_uses(&spec),
                    });
                    return node_id;
                }

                self.nodes.push(Node {
                    id: node_id,
                    parent,
                    kind: NodeKind::DynamicText,
                });
                self.push_slot(node_id, SlotKind::Text, expr);
                node_id
            }
        }
    }

    fn next_node_id(&self) -> usize {
        self.nodes.len()
    }

    fn push_slot(&mut self, node_id: usize, kind: SlotKind, expr: &syntax::Expr) {
        let id = self.slots.len();
        let reads = match &kind {
            SlotKind::Event(_) => Vec::new(),
            SlotKind::Text
            | SlotKind::Attr(_)
            | SlotKind::Ref
            | SlotKind::Conditional
            | SlotKind::Component { .. }
            | SlotKind::KeyedList { .. } => expand_reads(
                collect_template_reads(expr, self.read_summaries),
                self.read_aliases,
            ),
        };
        self.slots.push(Slot {
            id,
            node_id,
            kind,
            expr: format_expr(expr),
            reads,
            component_uses: Vec::new(),
        });
    }

    fn push_static_slot(&mut self, node_id: usize, kind: SlotKind, value: &str) {
        let id = self.slots.len();
        self.slots.push(Slot {
            id,
            node_id,
            kind,
            expr: value.to_string(),
            reads: Vec::new(),
            component_uses: Vec::new(),
        });
    }
}

struct ForSpec<'a> {
    item: &'a str,
    index: Option<&'a str>,
    collection: &'a syntax::Expr,
    key: &'a syntax::Expr,
    template: &'a HtmlNode,
}

impl<'a> ForSpec<'a> {
    fn parse(expr: &'a syntax::Expr) -> Option<Self> {
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

    fn reads(
        &self,
        components: &BTreeSet<String>,
        read_summaries: &BTreeMap<String, ReadSummary>,
    ) -> Vec<String> {
        let mut reads = BTreeSet::new();
        reads.extend(collect_template_reads(self.collection, read_summaries));
        reads.extend(collect_template_reads(self.key, read_summaries));
        collect_html_reads(self.template, &mut reads, components, read_summaries);
        let item_prefix = format!("{}.", self.item);
        reads.retain(|read| read != self.item && !read.starts_with(&item_prefix));
        if let Some(index) = self.index {
            reads.retain(|read| read != index);
        }
        reads.into_iter().collect()
    }
}

struct IfSpec<'a> {
    condition: &'a syntax::Expr,
    then_branch: TemplateBranch<'a>,
    else_branch: TemplateBranch<'a>,
}

struct ComponentSpec<'a> {
    name: &'a str,
    args: &'a [syntax::Expr],
}

enum TemplateBranch<'a> {
    Html(&'a HtmlNode),
    If(Box<IfSpec<'a>>),
    Component {
        expr: &'a syntax::Expr,
        spec: ComponentSpec<'a>,
    },
}

impl<'a> TemplateBranch<'a> {
    fn parse(expr: &'a syntax::Expr, components: &BTreeSet<String>) -> Option<Self> {
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
    fn parse(expr: &'a syntax::Expr, components: &BTreeSet<String>) -> Option<Self> {
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

    fn reads(
        &self,
        components: &BTreeSet<String>,
        read_summaries: &BTreeMap<String, ReadSummary>,
    ) -> Vec<String> {
        let mut reads = BTreeSet::new();
        reads.extend(collect_template_reads(self.condition, read_summaries));
        collect_branch_reads(&self.then_branch, &mut reads, components, read_summaries);
        collect_branch_reads(&self.else_branch, &mut reads, components, read_summaries);
        reads.into_iter().collect()
    }

    fn component_uses(&self, components: &BTreeSet<String>) -> Vec<String> {
        let mut uses = BTreeSet::new();
        collect_branch_component_uses(&self.then_branch, components, &mut uses);
        collect_branch_component_uses(&self.else_branch, components, &mut uses);
        uses.into_iter().collect()
    }
}

fn collect_branch_component_uses(
    branch: &TemplateBranch<'_>,
    components: &BTreeSet<String>,
    uses: &mut BTreeSet<String>,
) {
    match branch {
        TemplateBranch::Html(node) => collect_html_component_uses_inner(node, components, uses),
        TemplateBranch::If(spec) => {
            collect_branch_component_uses(&spec.then_branch, components, uses);
            collect_branch_component_uses(&spec.else_branch, components, uses);
        }
        TemplateBranch::Component { spec, .. } => {
            collect_component_spec_uses(spec, uses);
        }
    }
}

fn collect_html_component_uses(node: &HtmlNode, components: &BTreeSet<String>) -> Vec<String> {
    let mut uses = BTreeSet::new();
    collect_html_component_uses_inner(node, components, &mut uses);
    uses.into_iter().collect()
}

fn collect_html_component_uses_inner(
    node: &HtmlNode,
    components: &BTreeSet<String>,
    uses: &mut BTreeSet<String>,
) {
    match node {
        HtmlNode::Element(element) => {
            for child in &element.children {
                collect_html_component_uses_inner(child, components, uses);
            }
        }
        HtmlNode::Expr { expr, .. } => {
            if let Some(spec) = ForSpec::parse(expr) {
                collect_html_component_uses_inner(spec.template, components, uses);
                return;
            }
            if let Some(spec) = IfSpec::parse(expr, components) {
                collect_branch_component_uses(&spec.then_branch, components, uses);
                collect_branch_component_uses(&spec.else_branch, components, uses);
                return;
            }
            if let Some(spec) = ComponentSpec::parse(expr, components) {
                collect_component_spec_uses(&spec, uses);
            }
        }
        HtmlNode::Text { .. } => {}
    }
}

fn collect_component_spec_uses(spec: &ComponentSpec<'_>, uses: &mut BTreeSet<String>) {
    if spec.name == "scope-view" {
        if let Some(view_name) = spec.args.get(1).and_then(symbol_name) {
            uses.insert(view_name.to_string());
            return;
        }
    }
    uses.insert(spec.name.to_string());
}

fn component_spec_uses(spec: &ComponentSpec<'_>) -> Vec<String> {
    let mut uses = BTreeSet::new();
    collect_component_spec_uses(spec, &mut uses);
    uses.into_iter().collect()
}

fn collect_branch_reads(
    branch: &TemplateBranch<'_>,
    reads: &mut BTreeSet<String>,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) {
    match branch {
        TemplateBranch::Html(node) => collect_html_reads(node, reads, components, read_summaries),
        TemplateBranch::If(spec) => {
            reads.extend(collect_template_reads(spec.condition, read_summaries));
            collect_branch_reads(&spec.then_branch, reads, components, read_summaries);
            collect_branch_reads(&spec.else_branch, reads, components, read_summaries);
        }
        TemplateBranch::Component { expr, spec } => {
            reads.extend(component_call_reads(expr, spec, read_summaries));
        }
    }
}

fn collect_html_reads(
    node: &HtmlNode,
    reads: &mut BTreeSet<String>,
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
                    reads.extend(collect_template_reads(expr, read_summaries));
                }
            }
            for child in &element.children {
                collect_html_reads(child, reads, components, read_summaries);
            }
        }
        HtmlNode::Expr { expr, .. } => {
            if let Some(spec) = ForSpec::parse(expr) {
                reads.extend(spec.reads(components, read_summaries));
                return;
            }
            if let Some(spec) = IfSpec::parse(expr, components) {
                reads.extend(spec.reads(components, read_summaries));
                return;
            }
            if let Some(spec) = ComponentSpec::parse(expr, components) {
                reads.extend(component_call_reads(expr, &spec, read_summaries));
                return;
            }
            reads.extend(collect_template_reads(expr, read_summaries));
        }
        HtmlNode::Text { .. } => {}
    }
}

impl<'a> ComponentSpec<'a> {
    fn parse(expr: &'a syntax::Expr, components: &BTreeSet<String>) -> Option<Self> {
        let ExprKind::List(items) = &expr.kind else {
            return None;
        };
        let Some((head, args)) = items.split_first() else {
            return None;
        };
        let ExprKind::Symbol(name) = &head.kind else {
            return None;
        };
        if name != "scope-view" && !components.contains(name) {
            return None;
        }

        Some(Self { name, args })
    }
}

fn collect_template_reads(
    expr: &syntax::Expr,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    collect_template_reads_inner(expr, &mut symbols, read_summaries);
    symbols.into_iter().collect()
}

fn collect_template_reads_inner(
    expr: &syntax::Expr,
    reads: &mut BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) {
    match &expr.kind {
        ExprKind::Symbol(name) => {
            reads.insert(name.clone());
        }
        ExprKind::List(items) => {
            if let Some(read) = projectable_read_path(expr, &ReadAliases::new()) {
                reads.insert(read);
                return;
            }
            if let Some((head, args)) = items.split_first() {
                if let ExprKind::Symbol(name) = &head.kind {
                    if let Some(summary) = read_summaries.get(name) {
                        reads.extend(project_call_reads(summary, args, read_summaries));
                        return;
                    }
                    for item in args {
                        collect_template_reads_inner(item, reads, read_summaries);
                    }
                    return;
                }
            }
            for item in items {
                collect_template_reads_inner(item, reads, read_summaries);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_template_reads_inner(item, reads, read_summaries);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_template_reads_inner(key, reads, read_summaries);
                collect_template_reads_inner(value, reads, read_summaries);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => {
            collect_template_reads_inner(inner, reads, read_summaries)
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
    expr: &syntax::Expr,
    spec: &ComponentSpec<'_>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    if spec.name == "scope-view" {
        return scope_view_reads(spec, read_summaries);
    }
    let Some(summary) = read_summaries.get(spec.name) else {
        return collect_template_reads(expr, read_summaries);
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
    args: &[syntax::Expr],
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

fn collect_template_defns(source: &SourceFile) -> BTreeSet<String> {
    source
        .forms
        .iter()
        .filter_map(|form| {
            let ExprKind::List(items) = &form.kind else {
                return None;
            };
            if items.len() == 3 && matches_symbol(&items[0], "def") {
                let ExprKind::Symbol(name) = &items[1].kind else {
                    return None;
                };
                if template_shape_expr(&items[2]).is_some() {
                    return Some(name.clone());
                }
            }

            if items.len() >= 4 && matches_symbol(&items[0], "defn") {
                let ExprKind::Symbol(name) = &items[1].kind else {
                    return None;
                };
                if items.last().and_then(template_shape_expr).is_some() {
                    return Some(name.clone());
                }
            }

            None
        })
        .collect()
}

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
            .flat_map(typecheck::free_symbols)
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

fn collect_summary_body_reads(
    body: &syntax::Expr,
    components: &BTreeSet<String>,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Vec<String> {
    if let Some(template) = template_expr(body, read_summaries) {
        let mut reads = BTreeSet::new();
        collect_html_reads(template.node, &mut reads, components, read_summaries);
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

fn params_from_vector(expr: &syntax::Expr) -> Option<Vec<String>> {
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

struct TemplateExpr<'a> {
    node: &'a HtmlNode,
    read_aliases: ReadAliases,
}

fn template_shape_expr(expr: &syntax::Expr) -> Option<&HtmlNode> {
    match &expr.kind {
        ExprKind::HtmlTemplate(node) => Some(node.as_ref()),
        ExprKind::List(items) if items.len() == 3 && matches_symbol(&items[0], "let") => {
            let ExprKind::HtmlTemplate(node) = &items[2].kind else {
                return None;
            };
            Some(node.as_ref())
        }
        _ => None,
    }
}

fn template_expr<'a>(
    expr: &'a syntax::Expr,
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> Option<TemplateExpr<'a>> {
    match &expr.kind {
        ExprKind::HtmlTemplate(node) => Some(TemplateExpr {
            node: node.as_ref(),
            read_aliases: BTreeMap::new(),
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
    bindings: &[syntax::Expr],
    read_summaries: &BTreeMap<String, ReadSummary>,
) -> ReadAliases {
    let mut aliases = BTreeMap::new();
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

fn can_project_read_alias(value: &syntax::Expr, aliases: &ReadAliases) -> bool {
    projectable_read_path(value, aliases).is_some()
}

fn collect_pattern_read_aliases(
    pattern: &syntax::Expr,
    base_reads: &[String],
    can_project: bool,
    aliases: &mut ReadAliases,
) {
    collect_pattern_read_aliases_inner(pattern, base_reads, can_project, &[], aliases);
}

fn collect_pattern_read_aliases_inner(
    pattern: &syntax::Expr,
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
                let Some(name) = pattern_key_name(key) else {
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

fn pattern_key_name(expr: &syntax::Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Keyword(name) | ExprKind::Symbol(name) | ExprKind::String(name) => {
            Some(name.clone())
        }
        _ => None,
    }
}

fn projectable_read_path(expr: &syntax::Expr, aliases: &ReadAliases) -> Option<String> {
    match &expr.kind {
        ExprKind::Symbol(read) if read_path_projectable(read, aliases) => Some(read.clone()),
        ExprKind::List(items) => projectable_indexed_read_path(items, aliases),
        _ => None,
    }
}

fn projectable_indexed_read_path(items: &[syntax::Expr], aliases: &ReadAliases) -> Option<String> {
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

fn literal_vector_index(expr: &syntax::Expr) -> Option<usize> {
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

fn matches_symbol(expr: &syntax::Expr, expected: &str) -> bool {
    matches!(&expr.kind, ExprKind::Symbol(name) if name == expected)
}

fn symbol_name(expr: &syntax::Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Symbol(name) => Some(name),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowers_dynamic_attrs_events_and_text() {
        let source = syntax::parse_source(
            "#html <button class={classes.start} on:click={Msg.Start}>{label}</button>",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_source(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].nodes.len(), 2);
        assert_eq!(templates[0].slots.len(), 3);
        assert_eq!(
            templates[0].slots[0].kind,
            SlotKind::Attr("class".to_string())
        );
        assert_eq!(
            templates[0].slots[1].kind,
            SlotKind::Event("click".to_string())
        );
        assert_eq!(templates[0].slots[2].kind, SlotKind::Text);
        assert_eq!(templates[0].slots[0].reads, vec!["classes.start"]);
        assert!(templates[0].slots[1].reads.is_empty());
        assert_eq!(templates[0].slots[2].reads, vec!["label"]);
    }

    #[test]
    fn lowers_ref_slots_without_static_attrs() {
        let source =
            syntax::parse_source("#html <canvas ref=\"heart-chart\" class=\"chart\"></canvas>");
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_source(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(
            templates[0].nodes[0].kind,
            NodeKind::Element {
                tag: "canvas".to_string(),
                static_attrs: vec![("class".to_string(), "chart".to_string())]
            }
        );
        assert_eq!(templates[0].slots[0].kind, SlotKind::Ref);
        assert_eq!(templates[0].slots[0].expr, "heart-chart");
        assert!(templates[0].slots[0].reads.is_empty());
    }

    #[test]
    fn lowers_keyed_loop_slots() {
        let source = syntax::parse_source(
            "#html <section>{(for [entry state.entries index :key entry.id] #html <article data-index={index}>{entry.label}</article>)}</section>",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_source(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].nodes.len(), 2);
        assert_eq!(templates[0].nodes[1].kind, NodeKind::KeyedListMarker);
        assert_eq!(
            templates[0].slots[0].kind,
            SlotKind::KeyedList {
                item: "entry".to_string(),
                index: Some("index".to_string()),
                key: "entry.id".to_string()
            }
        );
        assert_eq!(templates[0].slots[0].reads, vec!["state.entries"]);
    }

    #[test]
    fn lowers_conditional_template_slots() {
        let source = syntax::parse_source(
            "#html <section>{(if state.connected? #html <strong>Live</strong> #html <em>Idle</em>)}</section>",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_source(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].nodes.len(), 2);
        assert_eq!(templates[0].nodes[1].kind, NodeKind::ConditionalMarker);
        assert_eq!(templates[0].slots[0].kind, SlotKind::Conditional);
        assert_eq!(templates[0].slots[0].reads, vec!["state.connected?"]);
    }

    #[test]
    fn lowers_nested_conditional_template_slots() {
        let source = syntax::parse_source(
            "#html <section>{(if state.metrics? #html <strong>Metrics</strong> (if state.log? #html <em>Log</em> #html <span>Live</span>))}</section>",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_source(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].nodes.len(), 2);
        assert_eq!(templates[0].nodes[1].kind, NodeKind::ConditionalMarker);
        assert_eq!(templates[0].slots[0].kind, SlotKind::Conditional);
        assert_eq!(
            templates[0].slots[0].reads,
            vec!["state.log?", "state.metrics?"]
        );
    }

    #[test]
    fn lowers_conditional_component_branches() {
        let source = syntax::parse_source(
            "(defn live-pane [state] #html <section>{state.latestBpm}</section>)\n\
             (defn log-pane [state] #html <section>{state.selectedLogId}</section>)\n\
             (defn view [state]\n  #html <main>{(if (= state.detailView \"live\") (live-pane state) (log-pane state))}</main>)",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_named_templates(&source);
        assert_eq!(templates.len(), 3);
        assert_eq!(templates[2].name, "view");
        assert_eq!(
            templates[2].template.nodes[1].kind,
            NodeKind::ConditionalMarker
        );
        assert_eq!(templates[2].template.slots[0].kind, SlotKind::Conditional);
        assert_eq!(
            templates[2].template.slots[0].reads,
            vec!["state.detailView", "state.latestBpm", "state.selectedLogId"]
        );
        assert_eq!(
            templates[2].template.slots[0].component_uses,
            vec!["live-pane", "log-pane"]
        );
    }

    #[test]
    fn conditional_reads_ignore_nested_keyed_loop_locals() {
        let source = syntax::parse_source(
            "#html <section>{(if state.show? #html <div>{(for [zone state.zones :key zone.id] #html <i>{zone.name}</i>)}</div> #html <p>None</p>)}</section>",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_source(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].slots[0].kind, SlotKind::Conditional);
        assert_eq!(
            templates[0].slots[0].reads,
            vec!["state.show?", "state.zones"]
        );
    }

    #[test]
    fn lowers_component_template_slots() {
        let source = syntax::parse_source(
            "(defn summary-card [summary] #html <article>{summary.label}</article>)\n#html <section>{(summary-card state.summary)}</section>",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_source(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].nodes[1].kind, NodeKind::ComponentMarker);
        assert_eq!(
            templates[0].slots[0].kind,
            SlotKind::Component {
                name: "summary-card".to_string()
            }
        );
        assert_eq!(templates[0].slots[0].reads, vec!["state.summary.label"]);
    }

    #[test]
    fn lowers_named_template_definitions() {
        let source = syntax::parse_source(
            "(defn summary-card [summary] #html <article>{summary.label}</article>)\n\
             (defn view [state] #html <section>{(summary-card state.summary)}</section>)",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_named_templates(&source);
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].name, "summary-card");
        assert_eq!(templates[1].name, "view");
        assert_eq!(
            templates[1].template.slots[0].kind,
            SlotKind::Component {
                name: "summary-card".to_string()
            }
        );
        assert_eq!(
            templates[1].template.slots[0].reads,
            vec!["state.summary.label"]
        );
    }

    #[test]
    fn lowers_helper_call_reads_to_state_paths() {
        let source = syntax::parse_source(
            "(defn connection-label [state]\n  (if state.connected? (if state.simulated? \"Simulated\" \"Bluetooth\") \"Disconnected\"))\n\
             (defn view [state]\n  #html <h2>{(connection-label state)}</h2>)",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_named_templates(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(
            templates[0].template.slots[0].reads,
            vec!["state.connected?", "state.simulated?"]
        );
    }

    #[test]
    fn lowers_let_wrapped_named_templates_with_source_reads() {
        let source = syntax::parse_source(
            "(defn stat-tile [label value] #html <strong>{value}</strong>)\n\
             (defn view [state]\n  (let [avg (average-bpm state.readings)\n        selected (selected-log state.entries state.selectedLogId)]\n    #html <section>\n            {(stat-tile \"Avg\" avg)}\n            <p>{selected.durationMs}</p>\n            {(for [entry state.entries :key entry.id]\n               #html <button on:click={{:kind :select :id entry.id}}>{entry.label}</button>)}\n          </section>))",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_named_templates(&source);
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[1].name, "view");
        assert_eq!(templates[1].template.slots[0].reads, vec!["state.readings"]);
        assert_eq!(
            templates[1].template.slots[1].reads,
            vec!["state.entries", "state.selectedLogId"]
        );
        assert_eq!(templates[1].template.slots[2].reads, vec!["state.entries"]);
    }

    #[test]
    fn lowers_let_wrapped_named_templates_with_pattern_source_reads() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (let [{:reading {:bpm bpm}\n\
                      :samples (cons head rest)} state.payload]\n\
                 #html <section data-bpm={bpm} data-head={head} data-count={(count rest)}></section>))",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_named_templates(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "view");
        assert_eq!(
            templates[0].template.slots[0].reads,
            vec!["state.payload.reading.bpm"]
        );
        assert_eq!(
            templates[0].template.slots[1].reads,
            vec!["state.payload.samples.0"]
        );
        assert_eq!(
            templates[0].template.slots[2].reads,
            vec!["state.payload.samples"]
        );
    }

    #[test]
    fn lowers_static_indexed_let_aliases_to_precise_state_reads() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (let [first-entry (first state.entries)\n\
                     second-entry (nth state.entries 1)]\n\
                 #html <section data-first={first-entry.label} data-second={second-entry.label}></section>))",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_named_templates(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "view");
        assert_eq!(
            templates[0].template.slots[0].reads,
            vec!["state.entries.0.label"]
        );
        assert_eq!(
            templates[0].template.slots[1].reads,
            vec!["state.entries.1.label"]
        );
    }

    #[test]
    fn keeps_dynamic_indexed_let_aliases_on_collection_and_index_reads() {
        let source = syntax::parse_source(
            "(defn view [state]\n\
               (let [entry (nth state.entries state.selectedIndex)]\n\
                 #html <section data-label={entry.label}></section>))",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_named_templates(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "view");
        assert_eq!(
            templates[0].template.slots[0].reads,
            vec!["state.entries", "state.selectedIndex"]
        );
    }

    #[test]
    fn lowers_let_aliases_through_helper_read_summaries() {
        let source = syntax::parse_source(
            "(defn selected-log [entries selected-id]\n  (find entries (fn [entry] (= entry.id selected-id))))\n\
             (defn display-readings [state]\n  (let [entry (selected-log state.entries state.selectedLogId)]\n    (if (= state.detailView \"log\")\n        entry.readings\n        state.readings)))\n\
             (defn view [state]\n  (let [readings (display-readings state)]\n    #html <main data-readings={(count readings)}></main>))",
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);

        let templates = lower_named_templates(&source);
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "view");
        assert_eq!(
            templates[0].template.slots[0].reads,
            vec![
                "state.detailView",
                "state.entries",
                "state.readings",
                "state.selectedLogId"
            ]
        );
    }
}
