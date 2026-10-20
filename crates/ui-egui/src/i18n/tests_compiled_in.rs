//! The languages the interface offers, with or without the `translations` feature. The decision is
//! tested both ways in every build; the build's own configuration is checked as built (without the
//! feature: `cargo test -p vectorcraft-ui-egui --no-default-features --lib i18n::tests_compiled_in`).

use super::*;

/// With or without the `translations` feature, the languages the interface offers are exactly the
/// ones whose catalog is in the binary: every language with it, English alone without it, and a
/// locale for a left-out language resolves to English.
#[test]
fn the_offered_languages_are_the_compiled_in_ones() {
    let offered: Vec<&str> = Lang::all().map(Lang::code).collect();
    if cfg!(feature = "translations") {
        assert_eq!(offered.len(), LANGUAGES.len());
        assert!(LANGUAGES.iter().all(|l| l.code == "en" || !l.source.is_empty()));
        assert_eq!(Lang::from_code("de").map(Lang::code), Some("de"));
        assert!(Lang::from_code("ja").is_some_and(Lang::complete_menus));
    } else {
        assert_eq!(offered, ["en"]);
        assert!(Lang::from_code("de").is_none());
        // The table keeps every language, but those left out claim no catalog and carry none.
        assert!(LANGUAGES.iter().skip(1).all(|l| !Lang(l).complete_menus()));
        assert!(LANGUAGES.iter().all(|l| l.source.is_empty()), "a catalog compiled in without the feature");
        assert!(lang_from_tag("de_DE.UTF-8").is_none(), "the system locale falls back to English");
    }
}

/// The decision itself, both ways, in whichever build runs it: English always, the others only with
/// the catalogs.
#[test]
fn english_alone_is_offered_without_the_catalogs() {
    assert!(offered("en", false) && offered("en", true));
    assert!(!offered("ja", false) && offered("ja", true));
}
