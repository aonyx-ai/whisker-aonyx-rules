//! Sources that every rule reading [`OPTION`] has to agree about
//!
//! `bool_param` reported an `extern` signature that
//! `repeated_primitive_params` skipped, on the same reasoning and under
//! the same option. Both implementations were reviewed and both looked
//! right. What differed was the tests: one rule had `extern` cases and the
//! other never did, so nothing failed.
//!
//! Sharing the predicate stops the code from drifting. This stops the
//! coverage from drifting. Each rule runs the same sources and has to
//! reach the same verdict, so a case added here is a case both rules
//! answer for.
//!
//! Every source holds one `bool` parameter and two `String` parameters, so
//! it trips both rules when nothing exempts it. A source that only one rule
//! looks at would pass for the other by never being visited. That is the
//! false comfort this module exists to remove. An `extern` block is
//! therefore absent: `bool_param` visits no `function_signature_item`, so
//! that case belongs to [`crosses_a_boundary`]'s own tests.
//!
//! [`OPTION`]: crate::OPTION
//! [`crosses_a_boundary`]: crate::crosses_a_boundary

/// The attributes a project is assumed to have named for [`EXEMPT`]
///
/// # Examples
///
/// ```
/// use boundary::corpus::ATTRIBUTES;
///
/// assert!(ATTRIBUTES.contains(&"shard"));
/// ```
pub const ATTRIBUTES: &[&str] = &["shard"];

/// Signatures on a boundary, which every rule reading the option ignores
///
/// # Examples
///
/// ```
/// use boundary::corpus::EXEMPT;
///
/// assert!(!EXEMPT.is_empty());
/// ```
pub const EXEMPT: &[&str] = &[
    "pub extern \"C\" fn f(flag: bool, path: String, text: String) {}",
    "#[shard]\npub fn f(flag: bool, path: String, text: String) {}",
    "#[topcoat::shard]\npub fn f(flag: bool, path: String, text: String) {}",
    "impl S {\n    #[shard]\n    fn f(flag: bool, path: String, text: String) {}\n}",
    "#[shard]\n/// Doc\n#[inline]\nfn f(flag: bool, path: String, text: String) {}",
];

/// Signatures on no boundary, which every such rule still reports
///
/// A rule that exempts too much reports nothing and looks like a rule that
/// found no fault, so the corpus pins both directions.
///
/// # Examples
///
/// ```
/// use boundary::corpus::REPORTED;
///
/// assert!(!REPORTED.is_empty());
/// ```
pub const REPORTED: &[&str] = &[
    "pub fn f(flag: bool, path: String, text: String) {}",
    "#[inline]\npub fn f(flag: bool, path: String, text: String) {}",
    "#[shard]\nfn a(x: String) {}\nfn b(flag: bool, path: String, text: String) {}",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_exempt_source_trips_both_rules_when_nothing_exempts_it() {
        for source in EXEMPT {
            assert!(source.contains("bool"), "no bool parameter in: {source}");
            assert!(
                source.matches("String").count() >= 2,
                "fewer than two String parameters in: {source}"
            );
        }
    }

    #[test]
    fn every_reported_source_trips_both_rules() {
        for source in REPORTED {
            assert!(source.contains("bool"), "no bool parameter in: {source}");
            assert!(
                source.matches("String").count() >= 2,
                "fewer than two String parameters in: {source}"
            );
        }
    }
}
