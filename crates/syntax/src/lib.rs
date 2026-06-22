use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn join(self, other: Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceFile {
    pub forms: Vec<Expr>,
    pub diagnostics: Vec<Diagnostic>,
}

impl SourceFile {
    pub fn pretty(&self) -> String {
        self.forms
            .iter()
            .map(format_expr)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Nil,
    Bool(bool),
    Number(String),
    String(String),
    Keyword(String),
    Symbol(String),
    List(Vec<Expr>),
    Vector(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Set(Vec<Expr>),
    Quote(Box<Expr>),
    QuasiQuote(Box<Expr>),
    Unquote(Box<Expr>),
    UnquoteSplicing(Box<Expr>),
    HtmlTemplate(Box<HtmlNode>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum HtmlNode {
    Element(HtmlElement),
    Text { text: String, span: Span },
    Expr { expr: Box<Expr>, span: Span },
}

impl HtmlNode {
    pub fn span(&self) -> Span {
        match self {
            HtmlNode::Element(element) => element.span,
            HtmlNode::Text { span, .. } => *span,
            HtmlNode::Expr { span, .. } => *span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HtmlElement {
    pub tag: String,
    pub attrs: Vec<HtmlAttr>,
    pub children: Vec<HtmlNode>,
    pub self_closing: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HtmlAttr {
    pub name: String,
    pub value: HtmlAttrValue,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HtmlAttrValue {
    Bool(bool),
    Static(String),
    Dynamic { expr: Box<Expr>, span: Span },
}

pub fn parse_source(input: &str) -> SourceFile {
    Parser::new(input).parse_source()
}

pub fn format_expr(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Nil => "nil".to_string(),
        ExprKind::Bool(value) => value.to_string(),
        ExprKind::Number(value) => value.clone(),
        ExprKind::String(value) => format!("\"{}\"", escape_string(value)),
        ExprKind::Keyword(name) => format!(":{}", name),
        ExprKind::Symbol(name) => name.clone(),
        ExprKind::List(items) => format_sequence("(", items, ")"),
        ExprKind::Vector(items) => format_sequence("[", items, "]"),
        ExprKind::Map(entries) => {
            let mut parts = Vec::new();
            for (key, value) in entries {
                parts.push(format_expr(key));
                parts.push(format_expr(value));
            }
            format!("{{{}}}", parts.join(" "))
        }
        ExprKind::Set(items) => format!("#{}", format_sequence("{", items, "}")),
        ExprKind::Quote(inner) => format!("'{}", format_expr(inner)),
        ExprKind::QuasiQuote(inner) => format!("`{}", format_expr(inner)),
        ExprKind::Unquote(inner) => format!("~{}", format_expr(inner)),
        ExprKind::UnquoteSplicing(inner) => format!("~@{}", format_expr(inner)),
        ExprKind::HtmlTemplate(node) => format!("#html {}", format_html_node(node)),
    }
}

pub fn format_html_node(node: &HtmlNode) -> String {
    match node {
        HtmlNode::Text { text, .. } => text.clone(),
        HtmlNode::Expr { expr, .. } => format!("{{{}}}", format_expr(expr)),
        HtmlNode::Element(element) => {
            let mut rendered = String::new();
            rendered.push('<');
            rendered.push_str(&element.tag);
            for attr in &element.attrs {
                rendered.push(' ');
                rendered.push_str(&attr.name);
                match &attr.value {
                    HtmlAttrValue::Bool(true) => {}
                    HtmlAttrValue::Bool(false) => rendered.push_str("={false}"),
                    HtmlAttrValue::Static(value) => {
                        rendered.push_str("=\"");
                        rendered.push_str(&escape_string(value));
                        rendered.push('"');
                    }
                    HtmlAttrValue::Dynamic { expr, .. } => {
                        rendered.push_str("={");
                        rendered.push_str(&format_expr(expr));
                        rendered.push('}');
                    }
                }
            }

            if element.self_closing && element.children.is_empty() {
                rendered.push_str(" />");
                return rendered;
            }

            rendered.push('>');
            for child in &element.children {
                rendered.push_str(&format_html_node(child));
            }
            rendered.push_str("</");
            rendered.push_str(&element.tag);
            rendered.push('>');
            rendered
        }
    }
}

pub fn render_diagnostics(input: &str, diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let (line, column) = line_column(input, diagnostic.span.start);
            format!(
                "{:?} at {}:{}: {}",
                diagnostic.severity, line, column, diagnostic.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn line_column(input: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for (index, ch) in input.char_indices() {
        if index >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn format_sequence(open: &str, items: &[Expr], close: &str) -> String {
    format!(
        "{}{}{}",
        open,
        items.iter().map(format_expr).collect::<Vec<_>>().join(" "),
        close
    )
}

fn escape_string(value: &str) -> String {
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

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_source(mut self) -> SourceFile {
        let mut forms = Vec::new();
        while {
            self.skip_ws_and_comments();
            !self.eof()
        } {
            match self.parse_expr() {
                Some(expr) => forms.push(expr),
                None => self.recover_one(),
            }
        }

        SourceFile {
            forms,
            diagnostics: self.diagnostics,
        }
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        self.skip_ws_and_comments();
        let start = self.pos;
        let ch = self.peek_char()?;

        match ch {
            '(' => self.parse_sequence('(', ')', ExprKind::List),
            '[' => self.parse_sequence('[', ']', ExprKind::Vector),
            '{' => self.parse_map(),
            '"' => Some(self.parse_string()),
            ':' => Some(self.parse_keyword()),
            '\'' => self.parse_prefixed(start, '\'', ExprKind::Quote),
            '`' => self.parse_prefixed(start, '`', ExprKind::QuasiQuote),
            '~' => {
                self.bump_char();
                if self.consume("@") {
                    self.parse_prefixed_after_consumed(start, ExprKind::UnquoteSplicing)
                } else {
                    self.parse_prefixed_after_consumed(start, ExprKind::Unquote)
                }
            }
            '#' if self.starts_with("#html") && self.after_keyword_boundary("#html") => {
                self.consume("#html");
                self.skip_ws_and_comments();
                let node = match self.parse_html_node() {
                    Some(node) => node,
                    None => {
                        self.error_here("expected an HTML element after #html");
                        HtmlNode::Text {
                            text: String::new(),
                            span: Span::new(self.pos, self.pos),
                        }
                    }
                };
                let span = Span::new(start, node.span().end);
                Some(Expr::new(ExprKind::HtmlTemplate(Box::new(node)), span))
            }
            '#' if self.starts_with("#{") => self.parse_set(),
            ')' | ']' | '}' => {
                self.diagnostics.push(Diagnostic::error(
                    Span::new(start, start + ch.len_utf8()),
                    format!("unexpected closing delimiter `{}`", ch),
                ));
                self.bump_char();
                None
            }
            _ => Some(self.parse_atom()),
        }
    }

    fn parse_sequence(
        &mut self,
        open: char,
        close: char,
        build: fn(Vec<Expr>) -> ExprKind,
    ) -> Option<Expr> {
        let start = self.pos;
        self.expect_char(open);
        let mut items = Vec::new();

        loop {
            self.skip_ws_and_comments();
            if self.eof() {
                self.diagnostics.push(Diagnostic::error(
                    Span::new(start, self.pos),
                    format!("unterminated `{}` sequence", open),
                ));
                break;
            }
            if self.peek_char() == Some(close) {
                self.bump_char();
                break;
            }
            if let Some(expr) = self.parse_expr() {
                items.push(expr);
            } else {
                self.recover_one();
            }
        }

        Some(Expr::new(build(items), Span::new(start, self.pos)))
    }

    fn parse_map(&mut self) -> Option<Expr> {
        let start = self.pos;
        self.expect_char('{');
        let mut entries = Vec::new();

        loop {
            self.skip_ws_and_comments();
            if self.eof() {
                self.diagnostics.push(Diagnostic::error(
                    Span::new(start, self.pos),
                    "unterminated map literal",
                ));
                break;
            }
            if self.peek_char() == Some('}') {
                self.bump_char();
                break;
            }

            let Some(key) = self.parse_expr() else {
                self.recover_one();
                continue;
            };

            self.skip_ws_and_comments();
            if self.peek_char() == Some('}') || self.eof() {
                self.diagnostics
                    .push(Diagnostic::error(key.span, "map key is missing a value"));
                if self.peek_char() == Some('}') {
                    self.bump_char();
                }
                break;
            }

            let Some(value) = self.parse_expr() else {
                self.recover_one();
                continue;
            };
            entries.push((key, value));
        }

        Some(Expr::new(
            ExprKind::Map(entries),
            Span::new(start, self.pos),
        ))
    }

    fn parse_set(&mut self) -> Option<Expr> {
        let start = self.pos;
        self.consume("#{");
        let mut items = Vec::new();

        loop {
            self.skip_ws_and_comments();
            if self.eof() {
                self.diagnostics.push(Diagnostic::error(
                    Span::new(start, self.pos),
                    "unterminated set literal",
                ));
                break;
            }
            if self.peek_char() == Some('}') {
                self.bump_char();
                break;
            }
            if let Some(expr) = self.parse_expr() {
                items.push(expr);
            } else {
                self.recover_one();
            }
        }

        Some(Expr::new(ExprKind::Set(items), Span::new(start, self.pos)))
    }

    fn parse_string(&mut self) -> Expr {
        let start = self.pos;
        self.expect_char('"');
        let mut value = String::new();
        let mut terminated = false;

        while let Some(ch) = self.bump_char() {
            match ch {
                '"' => {
                    terminated = true;
                    break;
                }
                '\\' => match self.bump_char() {
                    Some('n') => value.push('\n'),
                    Some('r') => value.push('\r'),
                    Some('t') => value.push('\t'),
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some(other) => {
                        value.push(other);
                        self.diagnostics.push(Diagnostic::warning(
                            Span::new(self.pos.saturating_sub(other.len_utf8()), self.pos),
                            format!("unknown escape sequence `\\{}`", other),
                        ));
                    }
                    None => break,
                },
                other => value.push(other),
            }
        }

        if !terminated {
            self.diagnostics.push(Diagnostic::error(
                Span::new(start, self.pos),
                "unterminated string literal",
            ));
        }

        Expr::new(ExprKind::String(value), Span::new(start, self.pos))
    }

    fn parse_keyword(&mut self) -> Expr {
        let start = self.pos;
        self.expect_char(':');
        let name = self.take_token();
        if name.is_empty() {
            self.diagnostics.push(Diagnostic::error(
                Span::new(start, self.pos),
                "expected keyword name after `:`",
            ));
        }
        Expr::new(ExprKind::Keyword(name), Span::new(start, self.pos))
    }

    fn parse_atom(&mut self) -> Expr {
        let start = self.pos;
        let token = self.take_token();
        if token.is_empty() {
            self.recover_one();
            return Expr::new(ExprKind::Nil, Span::new(start, self.pos));
        }

        let kind = match token.as_str() {
            "nil" => ExprKind::Nil,
            "true" => ExprKind::Bool(true),
            "false" => ExprKind::Bool(false),
            _ if is_number(&token) => ExprKind::Number(token),
            _ => ExprKind::Symbol(token),
        };
        Expr::new(kind, Span::new(start, self.pos))
    }

    fn parse_prefixed(
        &mut self,
        start: usize,
        marker: char,
        build: fn(Box<Expr>) -> ExprKind,
    ) -> Option<Expr> {
        self.expect_char(marker);
        self.parse_prefixed_after_consumed(start, build)
    }

    fn parse_prefixed_after_consumed(
        &mut self,
        start: usize,
        build: fn(Box<Expr>) -> ExprKind,
    ) -> Option<Expr> {
        match self.parse_expr() {
            Some(expr) => {
                let end = expr.span.end;
                Some(Expr::new(build(Box::new(expr)), Span::new(start, end)))
            }
            None => {
                self.diagnostics.push(Diagnostic::error(
                    Span::new(start, self.pos),
                    "expected expression after reader prefix",
                ));
                None
            }
        }
    }

    fn parse_html_node(&mut self) -> Option<HtmlNode> {
        if self.starts_with("<") {
            self.parse_html_element()
        } else if self.starts_with("{") {
            Some(self.parse_html_expr_node())
        } else {
            self.parse_html_text()
        }
    }

    fn parse_html_element(&mut self) -> Option<HtmlNode> {
        let start = self.pos;
        self.expect_char('<');

        if self.consume("/") {
            self.diagnostics.push(Diagnostic::error(
                Span::new(start, self.pos),
                "unexpected closing tag",
            ));
            return None;
        }

        let tag = self.take_html_name();
        if tag.is_empty() {
            self.diagnostics.push(Diagnostic::error(
                Span::new(start, self.pos),
                "expected tag name",
            ));
        }

        let mut attrs = Vec::new();
        let mut children = Vec::new();
        let mut self_closing = false;

        loop {
            self.skip_html_ws();
            if self.eof() {
                self.diagnostics.push(Diagnostic::error(
                    Span::new(start, self.pos),
                    "unterminated opening tag",
                ));
                break;
            }
            if self.consume("/>") {
                self_closing = true;
                let element = HtmlElement {
                    tag,
                    attrs,
                    children,
                    self_closing,
                    span: Span::new(start, self.pos),
                };
                return Some(HtmlNode::Element(element));
            }
            if self.consume(">") {
                break;
            }

            if let Some(attr) = self.parse_html_attr() {
                attrs.push(attr);
            } else {
                self.recover_html_attr();
            }
        }

        while !self.eof() {
            if self.starts_with("</") {
                let close_start = self.pos;
                self.consume("</");
                self.skip_html_ws();
                let closing_tag = self.take_html_name();
                self.skip_html_ws();
                if !self.consume(">") {
                    self.diagnostics.push(Diagnostic::error(
                        Span::new(close_start, self.pos),
                        "expected `>` after closing tag",
                    ));
                }
                if closing_tag != tag {
                    self.diagnostics.push(Diagnostic::error(
                        Span::new(close_start, self.pos),
                        format!("expected closing tag </{}>, found </{}>", tag, closing_tag),
                    ));
                }
                let element = HtmlElement {
                    tag,
                    attrs,
                    children,
                    self_closing,
                    span: Span::new(start, self.pos),
                };
                return Some(HtmlNode::Element(element));
            }

            match self.parse_html_node() {
                Some(child) => children.push(child),
                None => self.recover_one(),
            }
        }

        self.diagnostics.push(Diagnostic::error(
            Span::new(start, self.pos),
            format!("missing closing tag </{}>", tag),
        ));

        Some(HtmlNode::Element(HtmlElement {
            tag,
            attrs,
            children,
            self_closing,
            span: Span::new(start, self.pos),
        }))
    }

    fn parse_html_attr(&mut self) -> Option<HtmlAttr> {
        self.skip_html_ws();
        let start = self.pos;
        let name = self.take_html_name();
        if name.is_empty() {
            self.diagnostics.push(Diagnostic::error(
                Span::new(start, self.pos),
                "expected attribute name",
            ));
            return None;
        }

        self.skip_html_ws();
        let value = if self.consume("=") {
            self.skip_html_ws();
            if self.starts_with("\"") {
                let expr = self.parse_string();
                match expr.kind {
                    ExprKind::String(value) => HtmlAttrValue::Static(value),
                    _ => unreachable!("parse_string always returns a string expression"),
                }
            } else if self.starts_with("{") {
                let (expr, span) = self.parse_html_braced_expr();
                HtmlAttrValue::Dynamic {
                    expr: Box::new(expr),
                    span,
                }
            } else {
                HtmlAttrValue::Static(self.take_unquoted_attr_value())
            }
        } else {
            HtmlAttrValue::Bool(true)
        };

        Some(HtmlAttr {
            name,
            value,
            span: Span::new(start, self.pos),
        })
    }

    fn parse_html_expr_node(&mut self) -> HtmlNode {
        let (expr, span) = self.parse_html_braced_expr();
        HtmlNode::Expr {
            expr: Box::new(expr),
            span,
        }
    }

    fn parse_html_braced_expr(&mut self) -> (Expr, Span) {
        let start = self.pos;
        self.expect_char('{');
        let mut forms = Vec::new();

        loop {
            self.skip_ws_and_comments();
            if self.eof() {
                self.diagnostics.push(Diagnostic::error(
                    Span::new(start, self.pos),
                    "unterminated template expression",
                ));
                break;
            }
            if self.consume("}") {
                break;
            }
            if let Some(expr) = self.parse_expr() {
                forms.push(expr);
            } else {
                self.recover_one();
            }
        }

        let span = Span::new(start, self.pos);
        let expr = match forms.len() {
            0 => Expr::new(ExprKind::Nil, span),
            1 => forms.remove(0),
            _ => Expr::new(ExprKind::List(forms), span),
        };
        (expr, span)
    }

    fn parse_html_text(&mut self) -> Option<HtmlNode> {
        let start = self.pos;
        let mut text = String::new();
        while let Some(ch) = self.peek_char() {
            if ch == '<' || ch == '{' {
                break;
            }
            text.push(ch);
            self.bump_char();
        }
        if text.is_empty() {
            return None;
        }
        Some(HtmlNode::Text {
            text,
            span: Span::new(start, self.pos),
        })
    }

    fn take_token(&mut self) -> String {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if is_token_delimiter(ch) {
                break;
            }
            self.bump_char();
        }
        self.input[start..self.pos].to_string()
    }

    fn take_html_name(&mut self) -> String {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
                self.bump_char();
            } else {
                break;
            }
        }
        self.input[start..self.pos].to_string()
    }

    fn take_unquoted_attr_value(&mut self) -> String {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() || matches!(ch, '>' | '/') {
                break;
            }
            self.bump_char();
        }
        self.input[start..self.pos].to_string()
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while matches!(self.peek_char(), Some(ch) if ch.is_whitespace()) {
                self.bump_char();
            }

            if self.peek_char() == Some(';') {
                while let Some(ch) = self.bump_char() {
                    if ch == '\n' {
                        break;
                    }
                }
                continue;
            }

            break;
        }
    }

    fn skip_html_ws(&mut self) {
        while matches!(self.peek_char(), Some(ch) if ch.is_whitespace()) {
            self.bump_char();
        }
    }

    fn recover_one(&mut self) {
        self.bump_char();
    }

    fn recover_html_attr(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() || ch == '>' {
                break;
            }
            self.bump_char();
        }
    }

    fn error_here(&mut self, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(
            Span::new(self.pos, self.pos),
            message.into(),
        ));
    }

    fn expect_char(&mut self, expected: char) {
        let found = self.bump_char();
        if found != Some(expected) {
            self.diagnostics.push(Diagnostic::error(
                Span::new(self.pos, self.pos),
                format!("expected `{}`", expected),
            ));
        }
    }

    fn consume(&mut self, expected: &str) -> bool {
        if self.starts_with(expected) {
            self.pos += expected.len();
            true
        } else {
            false
        }
    }

    fn starts_with(&self, expected: &str) -> bool {
        self.input[self.pos..].starts_with(expected)
    }

    fn after_keyword_boundary(&self, keyword: &str) -> bool {
        let after = self.pos + keyword.len();
        self.input[after..]
            .chars()
            .next()
            .is_none_or(|ch| ch.is_whitespace() || ch == '<')
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
}

fn is_token_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '"' | ';')
}

fn is_number(token: &str) -> bool {
    if matches!(token, "+" | "-" | "." | "+." | "-.") {
        return false;
    }
    if token.contains('_') {
        let chars = token.chars().collect::<Vec<_>>();
        for (index, ch) in chars.iter().enumerate() {
            if *ch == '_' {
                let previous = index.checked_sub(1).and_then(|idx| chars.get(idx));
                let next = chars.get(index + 1);
                if !matches!(previous, Some(ch) if ch.is_ascii_digit())
                    || !matches!(next, Some(ch) if ch.is_ascii_digit())
                {
                    return false;
                }
            }
        }
        return token.replace('_', "").parse::<f64>().is_ok();
    }
    token.parse::<f64>().is_ok()
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => formatter.write_str("error"),
            Severity::Warning => formatter.write_str("warning"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_lisp_forms() {
        let source = parse_source("(def answer (+ 40 2))\n{:ok true :items [1 2 nil]}");

        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);
        assert_eq!(
            source.pretty(),
            "(def answer (+ 40 2))\n{:ok true :items [1 2 nil]}"
        );
    }

    #[test]
    fn parses_numeric_separators() {
        let source = parse_source("(def recovery-ms 60_000)\n(def ratio 0.12)\n(def gap -30_000)");

        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);
        assert_eq!(
            source.pretty(),
            "(def recovery-ms 60_000)\n(def ratio 0.12)\n(def gap -30_000)"
        );
    }

    #[test]
    fn parses_goal_html_sample() {
        let input = r#"#html <button class={classes.start}
                  disabled={not connected?}
                  on:click={Msg.Start}>
          {label}
        </button>"#;
        let source = parse_source(input);

        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);
        let rendered = source.pretty();
        assert!(rendered.contains("#html <button"));
        assert!(rendered.contains("disabled={(not connected?)}"));
        assert!(rendered.contains("on:click={Msg.Start}"));
        assert!(rendered.contains("{label}"));
    }

    #[test]
    fn reports_unterminated_html() {
        let source = parse_source("#html <div>{name}");

        assert!(source.has_errors());
        assert!(
            source
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("missing closing tag"))
        );
    }

    #[test]
    fn reports_odd_map_entry() {
        let source = parse_source("{:a 1 :b}");

        assert!(source.has_errors());
        assert!(
            source
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("missing a value"))
        );
    }
}
