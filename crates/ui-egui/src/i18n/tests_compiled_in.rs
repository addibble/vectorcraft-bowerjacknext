//! The languages the interface offers, with or without the `translations` feature. Run without it
//! with `cargo test -p vectorcraft-ui-egui --no-default-features --lib i18n::tests_compiled_in`.

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
        // The table keeps every language, but those left out claim no catalog.
        assert!(LANGUAGES.iter().skip(1).all(|l| !Lang(l).complete_menus()));
        assert!(lang_from_tag("de_DE.UTF-8").is_none(), "the system locale falls back to English");
        assert_eq!(tr(Lang::EN, "Layers"), "Layers");
    }
}
