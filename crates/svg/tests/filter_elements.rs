//! Raster effects as SVG `<filter>` elements for an element styled elsewhere (`filter_elements`),
//! and back (`effects_of_filters`): the elements are the ones the export writes, reading them back
//! gives the effects again, and what isn't such a fragment reads back as nothing, safely.
// Integration tests: unwrapping and panicking on failure is fine here, unlike in shipped code (AGENTS.md › Robustness).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use vectorcraft_doc::{Appearance, Document, Effect, Node};
use vectorcraft_geom::{Rect, shapes};
use vectorcraft_svg::{ExportOptions, FilterOptions, effects_of_filters, export, filter_elements};

fn fx(id: &str, params: Value) -> Effect {
    Effect { id: id.into(), params, visible: true }
}

/// One of each effect the export writes a filter for, with every parameter the import reads back.
fn every_kind() -> Vec<Effect> {
    vec![
        fx("blur.gaussian", json!({ "radius": 4.0 })),
        fx("stylize.dropShadow", json!({ "x": 3.0, "y": 4.0, "blur": 5.0, "color": "#112233", "opacity": 60.0, "mode": "multiply" })),
        fx(
            "stylize.dropShadow",
            json!({ "x": -2.0, "y": 1.0, "blur": 2.0, "shade": "darkness", "darkness": 40.0, "opacity": 50.0, "mode": "normal" }),
        ),
        fx("stylize.outerGlow", json!({ "blur": 6.0, "color": "#ffcc00", "opacity": 75.0, "mode": "screen" })),
        fx("stylize.innerGlow", json!({ "blur": 3.0, "color": "#ffffff", "opacity": 80.0, "mode": "normal", "source": "edge" })),
        fx("stylize.innerGlow", json!({ "blur": 3.0, "color": "#00ff00", "opacity": 70.0, "mode": "normal", "source": "center" })),
        fx("stylize.feather", json!({ "radius": 2.5 })),
    ]
}

/// The `<filter …>…</filter>` elements of an SVG text, in order.
fn filters_in(svg: &str) -> Vec<String> {
    svg.match_indices("<filter ").map(|(i, _)| svg[i..].split_inclusive("</filter>").next().unwrap().to_string()).collect()
}

/// A filter element with its id taken out.
fn without_id(element: &str) -> String {
    let rest = element.strip_prefix("<filter id=\"").unwrap();
    rest.split_once('"').unwrap().1.to_string()
}

/// For each kind of effect, alone and all of them together, at several resolutions and numbers of
/// decimals, the elements are the export's: region, primitives and order.
#[test]
fn the_elements_are_the_ones_the_export_writes() {
    let mut cases: Vec<Vec<Effect>> = every_kind().into_iter().map(|e| vec![e]).collect();
    cases.push(every_kind());
    for effects in cases {
        for (ppi, decimals) in [(72.0, 3), (300.0, 2), (7.0, 0)] {
            let mut d = Document::new(200.0, 200.0);
            d.raster_effects_ppi = ppi;
            let l = d.layers[0].id;
            let mut a = Appearance::default_art();
            a.effects = effects.clone();
            let id = d.alloc_id();
            let n = Node::path(id, shapes::rectangle(Rect::new(50.0, 50.0, 150.0, 150.0)), a);
            let bounds = n.visual_bounds();
            d.insert(Some(l), 0, n).unwrap();
            let exported = filters_in(&export(&d, &ExportOptions { decimals, ..ExportOptions::default() }));
            let ours = filter_elements(&effects, "fx", &FilterOptions { bounds, raster_ppi: ppi, decimals });
            // The export nests the last effect outermost, so it writes its filters last first.
            let exported: Vec<String> = exported.iter().rev().map(|e| without_id(e)).collect();
            let ours: Vec<String> = ours.iter().map(|e| without_id(&e.svg)).collect();
            assert_eq!(ours, exported, "{effects:?} at {ppi} ppi, {decimals} decimals");
        }
    }
}

#[test]
fn elements_are_numbered_in_the_order_they_apply_and_read_back_as_the_effects() {
    let effects = every_kind();
    let elements = filter_elements(&effects, "card-7", &FilterOptions::default());
    let ids: Vec<&str> = elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["card-7-1", "card-7-2", "card-7-3", "card-7-4", "card-7-5", "card-7-6", "card-7-7"]);
    assert!(elements.iter().all(|e| e.svg.starts_with(&format!("<filter id=\"{}\"", e.id))));
    let fragment: String = elements.iter().map(|e| e.svg.as_str()).collect();
    assert_eq!(effects_of_filters(&fragment).unwrap(), effects);
    // In the other order, the other order back.
    let reversed: String = elements.iter().rev().map(|e| e.svg.as_str()).collect();
    assert_eq!(effects_of_filters(&reversed).unwrap(), effects.iter().rev().cloned().collect::<Vec<_>>());
}

#[test]
fn a_prefix_is_made_an_id_that_url_references_as_it_is() {
    let blur = [fx("blur.gaussian", json!({ "radius": 4.0 }))];
    for (prefix, id) in [("a b)c#d", "a_b_c_d-1"), ("", "f-1"), ("7up", "f7up-1"), ("-x", "f-x-1"), ("é", "_-1")] {
        let elements = filter_elements(&blur, prefix, &FilterOptions::default());
        assert_eq!(elements[0].id, id);
        assert_eq!(effects_of_filters(&elements[0].svg).unwrap(), blur, "{prefix:?}");
    }
}

#[test]
fn hidden_effects_and_effects_without_a_filter_are_left_out() {
    let mut hidden = fx("blur.gaussian", json!({ "radius": 4.0 }));
    hidden.visible = false;
    // A geometry effect, a pixel effect SVG has no filter for, and a feather.
    let effects =
        [hidden, fx("distort.roughen", json!({})), fx("blur.radial", json!({ "amount": 10.0 })), fx("stylize.feather", json!({ "radius": 2.0 }))];
    let elements = filter_elements(&effects, "f", &FilterOptions::default());
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].id, "f-1");
    assert!(elements[0].svg.contains("feComposite"), "the feather: {}", elements[0].svg);
}

#[test]
fn what_is_not_an_effect_reads_back_as_none() {
    let blur = r#"<filter id="b"><feGaussianBlur stdDeviation="2"/></filter>"#;
    // A filter written by hand that is one: a blur of σ 2 is a Gaussian Blur of radius 4.
    assert_eq!(effects_of_filters(blur).unwrap(), [fx("blur.gaussian", json!({ "radius": 4.0 }))]);
    for bad in [
        // A filter the canvas has no effect for.
        r#"<filter id="t"><feTurbulence baseFrequency="0.05"/></filter>"#.to_string(),
        // No id, a shared id, another element, nothing, or no XML at all.
        r#"<filter><feGaussianBlur stdDeviation="2"/></filter>"#.to_string(),
        format!("{blur}{}", blur.replace("stdDeviation=\"2\"", "stdDeviation=\"5\"")),
        format!("{blur}<rect id=\"r\"/>"),
        String::new(),
        "<filter id=".to_string(),
        // Markup that leaves the fragment's place to add elements of its own.
        format!("{blur}</defs><g filter=\"url(#b)\"><rect width=\"9\" height=\"9\"/></g><defs>"),
        // More filters than are read.
        (0..257).map(|i| blur.replace("id=\"b\"", &format!("id=\"b{i}\""))).collect(),
    ] {
        assert_eq!(effects_of_filters(&bad), None, "{bad:.120}");
    }
}

/// Elements nested deeper than the import reads are refused, not read by recursion until the stack
/// runs out.
#[test]
fn deep_nesting_reads_back_as_none() {
    let deep = format!(r#"<filter id="a"><g id="d">{}{}</g></filter>"#, "<g>".repeat(5000), "</g>".repeat(5000));
    assert_eq!(effects_of_filters(&deep), None);
    let after = format!(r#"<filter id="a"><feGaussianBlur stdDeviation="2"/></filter>{}{}"#, "<g>".repeat(5000), "</g>".repeat(5000));
    assert_eq!(effects_of_filters(&after), None);
}

/// A primitive's link to a file reads nothing: /dev/zero, read, would never end.
#[cfg(unix)]
#[test]
fn a_link_to_a_file_is_not_read() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let r = effects_of_filters(r#"<filter id="i"><feImage href="/dev/zero"/></filter>"#);
        let _ = tx.send(r);
    });
    assert!(rx.recv_timeout(std::time::Duration::from_secs(20)).expect("effects_of_filters returned").is_none());
}
