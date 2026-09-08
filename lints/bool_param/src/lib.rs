use whisker_rust::{RustLintPass, RustLintPassAdapter};
use whisker_types::{DecoratedNode, Diagnostic, LintPass, RuleId, RuleOptions, Severity};

const RULE_ID: RuleId = RuleId::new("lint.bool-param");

/// Flags `bool` parameters in function signatures and `bool` fields in
/// struct definitions
///
/// Boolean parameters and fields obscure intent at call sites and in data
/// models. An enum with meaningful variant names makes the code
/// self-documenting and prevents accidental transposition of arguments.
///
/// The `foreign-attributes` option names the attribute macros that fix a
/// signature, and the rule skips a function that carries one. It is the same
/// option `lint.repeated-primitive-params` reads, and it means the same
/// thing; a project that sets one usually sets both.
#[derive(Default)]
pub struct BoolParam {
    foreign_attributes: Vec<String>,
}

impl BoolParam {
    /// Creates a boxed [`LintPass`] suitable for the whisker pipeline
    ///
    /// The pass reads no options. A caller that wants the
    /// `foreign-attributes` exemption goes through whisker, which configures
    /// every pass it constructs.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pass = BoolParam::into_lint_pass();
    /// ```
    pub fn into_lint_pass() -> Box<dyn LintPass> {
        Box::new(RustLintPassAdapter::new(Self::default()))
    }
}

/// Returns whether the given node is a `primitive_type` with text `"bool"`
fn is_bool_type(node: &DecoratedNode<'_>) -> bool {
    node.kind() == "primitive_type" && node.text() == "bool"
}

/// Returns whether an attribute on the signature is one of `foreign`
///
/// An attribute is a sibling that precedes the item, so the walk goes
/// backwards from the item and stops at the first sibling that is neither an
/// attribute nor a comment. A doc comment between two attributes therefore
/// does not end the run.
///
/// A configured name matches the last segment of the attribute's path, so
/// `shard` covers both `#[shard]` and `#[topcoat::shard]`. They are one
/// macro, and which one a file writes depends on its imports.
fn carries_a_foreign_attribute(node: &DecoratedNode<'_>, foreign: &[String]) -> bool {
    if foreign.is_empty() {
        return false;
    }

    let Some(parent) = node.parent() else {
        return false;
    };
    let siblings = parent.named_children();
    let Some(position) = siblings
        .iter()
        .position(|sibling| sibling.id() == node.id())
    else {
        return false;
    };

    for sibling in siblings[..position].iter().rev() {
        let attribute = match sibling.kind() {
            "attribute_item" => sibling.named_child(0),
            "line_comment" => continue,
            "block_comment" => continue,
            _ => return false,
        };
        let Some(attribute) = attribute else { continue };
        let Some(path) = attribute.named_child(0) else {
            continue;
        };

        let name = path.text();
        let name = name.rsplit("::").next().unwrap_or(name).trim();
        if foreign.iter().any(|candidate| candidate == name) {
            return true;
        }
    }

    false
}

impl RustLintPass for BoolParam {
    fn configure(&mut self, options: &RuleOptions) {
        self.foreign_attributes = options
            .names(RULE_ID, "foreign-attributes")
            .unwrap_or_default()
            .to_vec();
    }

    fn check_function_item(&mut self, node: &DecoratedNode<'_>) -> Vec<Diagnostic> {
        if carries_a_foreign_attribute(node, &self.foreign_attributes) {
            return Vec::new();
        }

        let Some(parameters) = node.child_by_field_name("parameters") else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        for param in parameters.named_children() {
            if param.kind() != "parameter" {
                continue;
            }
            let Some(ty) = param.child_by_field_name("type") else {
                continue;
            };
            if is_bool_type(&ty) {
                diagnostics.push(Diagnostic::new(
                    RULE_ID,
                    Severity::Warn,
                    "parameter has type `bool`; use an enum with meaningful variants".into(),
                    ty.span(),
                ));
            }
        }
        diagnostics
    }

    fn check_struct_item(&mut self, node: &DecoratedNode<'_>) -> Vec<Diagnostic> {
        let Some(body) = node.child_by_field_name("body") else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        for child in body.named_children() {
            let ty = match child.kind() {
                "field_declaration" => child.child_by_field_name("type"),
                _ => None,
            };
            let Some(ty) = ty else {
                continue;
            };
            if is_bool_type(&ty) {
                diagnostics.push(Diagnostic::new(
                    RULE_ID,
                    Severity::Warn,
                    "struct field has type `bool`; use an enum with meaningful variants".into(),
                    ty.span(),
                ));
            }
        }
        diagnostics
    }
}

#[cfg(feature = "plugin")]
impl whisker_rust::DeclaresRules for BoolParam {
    fn rules(&self) -> Vec<RuleId> {
        vec![RuleId::new("lint.bool-param")]
    }
}

whisker_rust::export_lints![BoolParam::default()];

#[cfg(test)]
mod tests {
    use whisker_testing::{assert_diagnostic, assert_no_diagnostics, execute, parse};
    use whisker_types::{Language, LintPass, RuleOption, Severity};

    use super::*;

    fn adapt() -> Box<dyn LintPass> {
        BoolParam::into_lint_pass()
    }

    fn run(source: &str) -> Vec<Diagnostic> {
        let tree = parse(source, Language::Rust);
        let mut passes = vec![adapt()];
        execute(&tree, &mut passes)
    }

    /// Runs the rule as whisker runs it, with `foreign-attributes` set
    fn run_with_foreign_attributes(source: &str, foreign: &[&str]) -> Vec<Diagnostic> {
        let tree = parse(source, Language::Rust);
        let options = RuleOptions::new(vec![RuleOption::new(
            RULE_ID.as_str().to_owned(),
            "foreign-attributes".to_owned(),
            foreign.iter().map(|name| (*name).to_owned()).collect(),
        )]);

        let mut pass = adapt();
        pass.configure(&options);
        let mut passes = vec![pass];

        execute(&tree, &mut passes)
    }

    #[test]
    fn bool_local_variable_not_flagged() {
        let diagnostics = run("fn foo() { let x: bool = true; }");

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn bool_param_in_function_flagged() {
        let diagnostics = run("fn foo(x: bool) {}");

        assert_eq!(diagnostics.len(), 1);
        assert_diagnostic(&diagnostics[0])
            .has_rule_id("lint.bool-param")
            .has_severity(Severity::Warn)
            .message_contains("parameter has type `bool`");
    }

    #[test]
    fn bool_param_behind_a_configured_attribute_not_flagged() {
        let diagnostics = run_with_foreign_attributes("#[shard]\nfn foo(x: bool) {}", &["shard"]);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn bool_param_behind_a_configured_attribute_written_in_full_not_flagged() {
        let diagnostics =
            run_with_foreign_attributes("#[topcoat::shard]\nfn foo(x: bool) {}", &["shard"]);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn bool_param_behind_an_attribute_nobody_configured_flagged() {
        let diagnostics = run("#[shard]\nfn foo(x: bool) {}");

        assert_eq!(diagnostics.len(), 1);
        assert_diagnostic(&diagnostics[0]).message_contains("parameter has type `bool`");
    }

    #[test]
    fn bool_param_behind_another_attribute_flagged() {
        let diagnostics = run_with_foreign_attributes("#[inline]\nfn foo(x: bool) {}", &["shard"]);

        assert_eq!(diagnostics.len(), 1);
        assert_diagnostic(&diagnostics[0]).message_contains("parameter has type `bool`");
    }

    #[test]
    fn bool_struct_field_behind_a_configured_attribute_flagged() {
        let diagnostics =
            run_with_foreign_attributes("#[shard]\nstruct Config { verbose: bool }", &["shard"]);

        assert_eq!(diagnostics.len(), 1);
        assert_diagnostic(&diagnostics[0]).message_contains("struct field has type `bool`");
    }

    #[test]
    fn bool_return_type_not_flagged() {
        let diagnostics = run("fn foo() -> bool { true }");

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn bool_struct_field_flagged() {
        let diagnostics = run("struct Config { verbose: bool }");

        assert_eq!(diagnostics.len(), 1);
        assert_diagnostic(&diagnostics[0])
            .has_rule_id("lint.bool-param")
            .has_severity(Severity::Warn)
            .message_contains("struct field has type `bool`");
    }

    #[test]
    fn multiple_bool_params_each_flagged() {
        let diagnostics = run("fn foo(a: bool, b: bool) {}");

        assert_eq!(diagnostics.len(), 2);
        assert_diagnostic(&diagnostics[0])
            .has_rule_id("lint.bool-param")
            .has_severity(Severity::Warn)
            .message_contains("parameter has type `bool`");
        assert_diagnostic(&diagnostics[1])
            .has_rule_id("lint.bool-param")
            .has_severity(Severity::Warn)
            .message_contains("parameter has type `bool`");
    }

    #[test]
    fn non_bool_param_not_flagged() {
        let diagnostics = run("fn foo(x: i32, y: String) {}");

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn trait_send() {
        fn assert_send<T: Send>() {}
        assert_send::<BoolParam>();
    }

    #[test]
    fn trait_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<BoolParam>();
    }

    #[test]
    fn trait_unpin() {
        fn assert_unpin<T: Unpin>() {}
        assert_unpin::<BoolParam>();
    }
}
