use std::path::PathBuf;
use std::process::Command;

use ferrovia_core::{Config, PluginConfig, PluginSpec, optimize};
use serde_json::json;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace member")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn roundtrip_simple_svg_is_stable() {
    let svg = std::fs::read_to_string(workspace_root().join("tests/fixtures/roundtrip/simple.svg"))
        .expect("fixture");
    let result = optimize(&svg, &Config::default()).expect("optimize");
    assert_eq!(result.data, svg.trim().to_string());
}

#[test]
fn serializer_canonicalizes_attribute_quotes_to_double_quotes() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><text x='1' y='2'>x</text></svg>"#;
    let result = optimize(svg, &Config::default()).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><text x="1" y="2">x</text></svg>"#
    );
}

#[test]
fn serializer_escapes_double_quotes_inside_attribute_values() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect class='a"b' width="1" height="1"/></svg>"#;
    let result = optimize(svg, &Config::default()).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><rect class="a&quot;b" width="1" height="1"/></svg>"#
    );
}

#[test]
fn serializer_escapes_quotes_and_markup_inside_text_nodes() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><text>fill="remove" and rotate='auto' &amp; &lt;tag&gt;</text></svg>"#;
    let result = optimize(svg, &Config::default()).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><text>fill=&quot;remove&quot; and rotate=&apos;auto&apos; &amp; &lt;tag&gt;</text></svg>"#
    );
}

#[test]
fn serializer_preserves_mixed_text_indentation_around_children() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><text x="0" y="0">"#,
        "\n  Hello\n  ",
        r#"<set attributeName="fill" to="red"/>"#,
        "\n</text></svg>",
    );
    let result = optimize(svg, &Config::default()).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><text x="0" y="0">"#,
            "\n  Hello\n  ",
            r#"<set attributeName="fill" to="red"/>"#,
            "\n",
            r#"</text></svg>"#,
        )
    );
}

#[test]
fn serializer_preserves_whitespace_between_animation_children_in_text() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><text x="0" y="0">"#,
        "\n  Hello\n  ",
        r#"<set attributeName="fill" to="red"/>"#,
        "\n  ",
        r#"<set attributeName="stroke" to="blue"/>"#,
        "\n</text></svg>",
    );
    let result = optimize(svg, &Config::default()).expect("optimize");
    assert_eq!(result.data, svg);
}

#[test]
fn serializer_trims_outer_script_indentation_but_keeps_inner_lines() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><script>"#,
        "\n  function f() {\n    return 1;\n  }\n",
        r#"</script></svg>"#,
    );
    let result = optimize(svg, &Config::default()).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><script>"#,
            "function f() {\n    return 1;\n  }",
            r#"</script></svg>"#,
        )
    );
}

#[test]
fn removes_supported_structural_nodes() {
    let svg =
        std::fs::read_to_string(workspace_root().join("tests/fixtures/oracle/remove-comments.svg"))
            .expect("fixture");

    let config = Config {
        plugins: vec![
            PluginSpec::Name("removeXMLProcInst".to_string()),
            PluginSpec::Name("removeDoctype".to_string()),
            PluginSpec::Configured(PluginConfig {
                name: "removeComments".to_string(),
                params: Some(json!({ "preservePatterns": ["^!"] })),
                enabled: true,
            }),
            PluginSpec::Name("removeMetadata".to_string()),
            PluginSpec::Name("removeTitle".to_string()),
        ],
        ..Config::default()
    };

    let result = optimize(&svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><!--!keep legal--><desc>World</desc><g><text>Hi</text></g></svg>"#
    );
}

#[test]
fn matches_svgo_oracle_for_supported_fixture() {
    assert_oracle_fixture("remove-comments");
}

#[test]
fn matches_svgo_oracle_for_remove_desc_empty() {
    assert_oracle_fixture("remove-desc-empty");
}

#[test]
fn matches_svgo_oracle_for_remove_dimensions() {
    assert_oracle_fixture("remove-dimensions");
}

#[test]
fn matches_svgo_oracle_for_remove_xmlns() {
    assert_oracle_fixture("remove-xmlns");
}

#[test]
fn matches_svgo_oracle_for_remove_empty_containers() {
    assert_oracle_fixture("remove-empty-containers");
}

#[test]
fn matches_svgo_oracle_for_move_group_attrs_to_elems() {
    assert_oracle_fixture("move-group-attrs-to-elems");
}

#[test]
fn matches_svgo_oracle_for_move_elems_attrs_to_group() {
    assert_oracle_fixture("move-elems-attrs-to-group");
}

#[test]
fn matches_svgo_oracle_for_collapse_groups() {
    assert_oracle_fixture("collapse-groups");
}

#[test]
fn matches_svgo_oracle_for_cleanup_enable_background() {
    assert_oracle_fixture("cleanup-enable-background");
}

#[test]
fn matches_svgo_oracle_for_remove_non_inheritable_group_attrs() {
    assert_oracle_fixture("remove-non-inheritable-group-attrs");
}

#[test]
fn matches_svgo_oracle_for_remove_useless_stroke_and_fill() {
    assert_oracle_fixture("remove-useless-stroke-and-fill");
}

#[test]
fn matches_svgo_oracle_for_remove_unknowns_and_defaults() {
    assert_oracle_fixture("remove-unknowns-and-defaults");
}

#[test]
fn matches_svgo_oracle_for_remove_unknowns_and_defaults_foreign_description() {
    assert_oracle_fixture("remove-unknowns-and-defaults-foreign-description");
}

#[test]
fn remove_unknowns_and_defaults_removes_default_zero_coordinates_on_use_and_image() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">"#,
        r##"<use xlink:href="#shape" x="0" y="0"/>"##,
        r#"<image xlink:href="img.png" x="0" y="0" width="10" height="10"/>"#,
        r#"</svg>"#,
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("removeUnknownsAndDefaults".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">"#,
            r##"<use xlink:href="#shape"/>"##,
            r#"<image xlink:href="img.png" width="10" height="10"/>"#,
            r#"</svg>"#,
        )
    );
}

#[test]
fn remove_unknowns_and_defaults_drops_inherited_stop_presentation_values() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<linearGradient stop-color="inherit" stop-opacity="inherit">"#,
        r#"<stop offset="1" stop-color="inherit" stop-opacity="inherit"/>"#,
        r#"</linearGradient></svg>"#,
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("removeUnknownsAndDefaults".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
            r#"<linearGradient stop-color="inherit" stop-opacity="inherit"><stop offset="1"/></linearGradient>"#,
            r#"</svg>"#,
        )
    );
}

#[test]
fn remove_unknowns_and_defaults_strips_unknown_set_event_attrs() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<set attributeName="visibility" to="hidden" dur="1s" onend="g()"/>"#,
        r#"</svg>"#,
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("removeUnknownsAndDefaults".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><set attributeName="visibility" to="hidden" dur="1s"/></svg>"#
    );
}

#[test]
fn matches_svgo_oracle_for_merge_styles() {
    assert_oracle_fixture("merge-styles");
}

#[test]
fn matches_svgo_oracle_for_inline_styles() {
    assert_oracle_fixture("inline-styles");
}

#[test]
fn matches_svgo_oracle_for_minify_styles() {
    assert_oracle_fixture("minify-styles");
}

#[test]
fn matches_svgo_oracle_for_cleanup_ids() {
    assert_oracle_fixture("cleanup-ids");
}

#[test]
fn matches_svgo_oracle_for_cleanup_numeric_values() {
    assert_oracle_fixture("cleanup-numeric-values");
}

#[test]
fn matches_svgo_oracle_for_convert_colors() {
    assert_oracle_fixture("convert-colors");
}

#[test]
fn matches_svgo_oracle_for_convert_ellipse_to_circle() {
    assert_oracle_fixture("convert-ellipse-to-circle");
}

#[test]
fn matches_svgo_oracle_for_convert_shape_to_path() {
    assert_oracle_fixture("convert-shape-to-path");
}

#[test]
fn matches_svgo_oracle_for_remove_hidden_elems() {
    assert_oracle_fixture("remove-hidden-elems");
}

#[test]
fn matches_svgo_oracle_for_convert_transform() {
    assert_oracle_fixture("convert-transform");
}

#[test]
fn matches_svgo_oracle_for_convert_path_data() {
    assert_oracle_fixture("convert-path-data");
}

#[test]
fn matches_svgo_oracle_for_merge_paths() {
    assert_oracle_fixture("merge-paths");
}

#[test]
fn matches_svgo_oracle_for_merge_paths_force() {
    assert_oracle_fixture("merge-paths-force");
}

#[test]
fn inline_styles_leaves_multi_match_selector_by_default() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.hero{fill:red}</style>"#,
        r#"<rect class="hero" width="10" height="10"/><circle class="hero" r="5"/></svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("inlineStyles".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><rect class="hero" width="10" height="10"/><circle class="hero" r="5"/></svg>"#
    );
}

#[test]
fn cleanup_ids_rewrites_begin_references() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path id="shape" d="M0 0"/><animate begin="shape.begin"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("cleanupIds".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path id="a" d="M0 0"/><animate begin="a.begin"/></svg>"#
    );
}

#[test]
fn cleanup_ids_preserves_begin_list_spacing_when_rewriting_ids() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<path id="shape" d="M0 0"><animate id="pulse" begin="0s; pulse.end + 1s"/></path>"#,
        r#"<animate begin="shape.begin; pulse.end + 2s"/></svg>"#,
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("cleanupIds".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
            r#"<path id="b" d="M0 0"><animate id="a" begin="0s; a.end + 1s"/></path>"#,
            r#"<animate begin="b.begin; a.end + 2s"/></svg>"#,
        )
    );
}

#[test]
fn cleanup_ids_only_rewrites_first_matching_begin_segment_per_id() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<set id="syncBase" attributeName="display" begin="0s" dur="indefinite" to="inline"/>"#,
        r#"<animate begin="syncBase.begin + 1s; syncBase.begin + 4s"/></svg>"#,
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("cleanupIds".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
            r#"<set id="a" attributeName="display" begin="0s" dur="indefinite" to="inline"/>"#,
            r#"<animate begin="a.begin + 1s; syncBase.begin + 4s"/></svg>"#,
        )
    );
}

#[test]
fn cleanup_ids_minifies_in_reference_encounter_order() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">"#,
        r#"<path id="fadeIn" d="M0 0"><animate id="fadeInAnim" begin="indefinite"/></path>"#,
        r#"<path id="fadeOut" d="M1 1"><animate id="fadeOutAnim" begin="indefinite"/></path>"#,
        r##"<a xlink:href="#fadeInAnim"><text>In</text></a>"##,
        r##"<a xlink:href="#fadeOutAnim"><text>Out</text></a>"##,
        r#"</svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("cleanupIds".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">"#,
            r#"<path d="M0 0"><animate id="a" begin="indefinite"/></path>"#,
            r#"<path d="M1 1"><animate id="b" begin="indefinite"/></path>"#,
            r##"<a xlink:href="#a"><text>In</text></a>"##,
            r##"<a xlink:href="#b"><text>Out</text></a>"##,
            r#"</svg>"#
        )
    );
}

#[test]
fn cleanup_numeric_values_rounds_and_strips_default_px() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10.1234 20.9876"><rect x="10.5000px" y="2.54cm" width="0.5000" version="1.1"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("cleanupNumericValues".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10.123 20.988"><rect x="10.5" y="96" width=".5" version="1.1"/></svg>"#
    );
}

#[test]
fn convert_colors_skips_current_color_conversion_inside_masks() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><mask id="m"><rect fill="blue"/></mask><rect fill="red" stroke="none"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Configured(PluginConfig {
            name: "convertColors".to_string(),
            params: Some(json!({ "currentColor": true })),
            enabled: true,
        })],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r##"<svg xmlns="http://www.w3.org/2000/svg"><mask id="m"><rect fill="#00f"/></mask><rect fill="currentColor" stroke="none"/></svg>"##
    );
}

#[test]
fn convert_ellipse_to_circle_rewrites_matching_radii() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><ellipse cx="5" cy="5" rx="10" ry="10"/><ellipse rx="10" ry="12"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertEllipseToCircle".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><circle cx="5" cy="5" r="10"/><ellipse rx="10" ry="12"/></svg>"#
    );
}

#[test]
fn convert_shape_to_path_removes_short_polyline() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><polyline points="0,0"/><polygon points="0,0 10,0 10,10"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertShapeToPath".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0 10 0 10 10z"/></svg>"#
    );
}

fn assert_oracle_fixture(name: &str) {
    let root = workspace_root();
    let svg_path = root.join(format!("tests/fixtures/oracle/{name}.svg"));
    let config_path = root.join(format!("tests/fixtures/oracle/{name}.config.json"));
    let node_modules = root.join("node_modules/svgo");

    if !node_modules.exists() {
        eprintln!("skipping oracle test because svgo is not installed");
        return;
    }

    let expected = Command::new("node")
        .arg(root.join("scripts/run-svgo-oracle.mjs"))
        .arg(&svg_path)
        .arg(&config_path)
        .current_dir(&root)
        .output()
        .expect("run oracle");

    assert!(expected.status.success(), "oracle failed: {expected:?}");

    let config =
        serde_json::from_str::<Config>(&std::fs::read_to_string(config_path).expect("config"))
            .expect("parse config");
    let actual =
        optimize(&std::fs::read_to_string(svg_path).expect("svg"), &config).expect("optimize");

    assert_eq!(
        actual.data,
        String::from_utf8(expected.stdout).expect("utf8")
    );
}

#[test]
fn preset_default_honors_boolean_overrides() {
    let svg = r#"<?xml version="1.0"?><!DOCTYPE svg><svg><!--keep me--><metadata>meta</metadata><desc>Created with Sketch.</desc></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Configured(PluginConfig {
            name: "preset-default".to_string(),
            params: Some(json!({
                "overrides": {
                    "removeComments": false
                }
            })),
            enabled: true,
        })],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, r"<svg><!--keep me--></svg>");
}

#[test]
fn preset_default_runs_convert_path_data() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M10 10 L20 10 L20 25"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("preset-default".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M10 10h10v15"/></svg>"#
    );
}

#[test]
fn preset_default_runs_merge_paths() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><g>"#,
        r#"<path fill="red" d="M0 0H10"/><path fill="red" d="M20 0H30"/></g></svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("preset-default".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><g fill="red"><path d="M0 0h10M20 0h10"/></g></svg>"#
    );
}

#[test]
fn preset_default_drops_unused_style_scaffold_and_root_defaults() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" id="svg-root" width="100%" height="100%" viewBox="0 0 10 10">"#,
        r#"<defs><style>#test-body-content .final{fill:red}.hideme{display:none}</style></defs>"#,
        r#"<g id="test-body-content"><text class="hideme">x</text></g></svg>"#,
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("preset-default".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"/>"#);
}

#[test]
fn preset_default_preserves_whitespace_text_nodes_left_by_removed_children() {
    let svg = concat!(
        r##"<svg xmlns="http://www.w3.org/2000/svg"><a xmlns:xlink="http://www.w3.org/1999/xlink" xlink:href="#x">"##,
        "\n  ",
        r#"<path d="M0 0"/>"#,
        "\n  ",
        r#"<text>x</text>"#,
        "\n",
        r#"</a></svg>"#,
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("preset-default".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg"><a xmlns:xlink="http://www.w3.org/1999/xlink" xlink:href="#x">"##,
            "\n  \n  ",
            r#"<text>x</text>"#,
            "\n",
            r#"</a></svg>"#
        )
    );
}

#[test]
fn remove_unused_ns_ignores_detached_prefixed_attrs() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">"#,
        r#"<defs><font-face><font-face-src><font-face-uri xlink:href="font.svg#ascii"/></font-face-src></font-face></defs>"#,
        r#"<rect width="10" height="10"/></svg>"#
    );
    let config = Config {
        plugins: vec![
            PluginSpec::Name("removeUselessDefs".to_string()),
            PluginSpec::Name("removeUnusedNS".to_string()),
        ],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="10" height="10"/></svg>"#
    );
}

#[test]
fn sort_attrs_supports_alphabetical_xmlns_order() {
    let svg = r#"<svg foo="bar" xmlns="http://www.w3.org/2000/svg" height="10" baz="quux" width="10" hello="world"><rect x="0" y="0" width="100" height="100" stroke-width="1" stroke-linejoin="round" fill="red" stroke="orange" xmlns="http://www.w3.org/2000/svg"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Configured(PluginConfig {
            name: "sortAttrs".to_string(),
            params: Some(json!({ "xmlnsOrder": "alphabetical" })),
            enabled: true,
        })],
        js2svg: ferrovia_core::Js2Svg {
            pretty: true,
            indent: 4,
        },
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data.trim(),
        r#"<svg width="10" height="10" baz="quux" foo="bar" hello="world" xmlns="http://www.w3.org/2000/svg">
    <rect width="100" height="100" x="0" y="0" fill="red" stroke="orange" stroke-linejoin="round" stroke-width="1" xmlns="http://www.w3.org/2000/svg"/>
</svg>"#
    );
}

#[test]
fn remove_empty_containers_removes_empty_defs_and_referencing_use() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><defs id="gone"/><use href="#gone"/><mask id="keep"/></svg>"##;
    let config = Config {
        plugins: vec![PluginSpec::Name("removeEmptyContainers".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><mask id="keep"/></svg>"#
    );
}

#[test]
fn remove_empty_containers_preserves_switch_child_and_filtered_group() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><style>g.keep{filter:url(#fx)}</style><switch><g/></switch><g class="keep"/><g/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("removeEmptyContainers".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style>g.keep{filter:url(#fx)}</style><switch><g/></switch><g class="keep"/></svg>"#
    );
}

#[test]
fn move_group_attrs_to_elems_keeps_group_transform_when_url_reference_is_present() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g transform="scale(2)" clip-path="url(#clip)"><path d="M0 0"/></g></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("moveGroupAttrsToElems".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><g transform="scale(2)" clip-path="url(#clip)"><path d="M0 0"/></g></svg>"#
    );
}

#[test]
fn move_group_attrs_to_elems_repeats_for_nested_groups_created_in_same_pass() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><g transform="translate(20 50)"><g>"#,
        r#"<path d="M120 200L170 200"/><path d="M120 167L170 167"/>"#,
        r#"</g></g></svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("moveGroupAttrsToElems".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><g><g>"#,
            r#"<path d="M120 200L170 200" transform="translate(20 50)"/>"#,
            r#"<path d="M120 167L170 167" transform="translate(20 50)"/>"#,
            r#"</g></g></svg>"#
        )
    );
}

#[test]
fn move_elems_attrs_to_group_deoptimizes_when_style_exists() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.x"#,
        "{fill:red}",
        r#"</style><g><path class="x" fill="red" d="M0 0"/><circle class="x" fill="red" cx="5" cy="5" r="5"/></g></svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("moveElemsAttrsToGroup".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, svg);
}

#[test]
fn collapse_groups_preserves_group_with_animation_child() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g><animate attributeName="opacity"/><path d="M0 0"/></g></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("collapseGroups".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, svg);
}

#[test]
fn collapse_groups_merges_single_child_group_when_safe() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g><g fill="red" transform="scale(2)"><path d="M0 0" transform="rotate(5)"/></g></g></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("collapseGroups".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0" transform="scale(2) rotate(5)" fill="red"/></svg>"#
    );
}

#[test]
fn cleanup_enable_background_removes_attr_and_style_without_filters() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" enable-background="new 0 0 10 10" style="enable-background:new 0 0 10 10;fill:red"><rect width="10" height="10"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("cleanupEnableBackground".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg" style="fill:red"><rect width="10" height="10"/></svg>"#
    );
}

#[test]
fn cleanup_enable_background_keeps_new_for_mask_with_matching_dimensions() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><filter id="fx"/><mask width="10" height="10" enable-background="new 0 0 10 10" style="fill:red;enable-background:new 0 0 10 10"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("cleanupEnableBackground".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><filter id="fx"/><mask width="10" height="10" enable-background="new" style="fill:red;enable-background:new"/></svg>"#
    );
}

#[test]
fn remove_non_inheritable_group_attrs_preserves_inheritable_and_group_specific_attrs() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g clip="rect(0 0 0 0)" fill="red" opacity="0.5"><path d="M0 0"/></g></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name(
            "removeNonInheritableGroupAttrs".to_string(),
        )],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><g fill="red" opacity="0.5"><path d="M0 0"/></g></svg>"#
    );
}

#[test]
fn remove_useless_stroke_and_fill_deoptimizes_when_style_exists() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.x"#,
        "{stroke:none}",
        r#"</style><path class="x" d="M0 0" stroke="red" stroke-width="0" fill="none"/></svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("removeUselessStrokeAndFill".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, svg);
}

#[test]
fn remove_useless_stroke_and_fill_removes_shape_when_remove_none_is_enabled() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0" fill="none"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Configured(PluginConfig {
            name: "removeUselessStrokeAndFill".to_string(),
            params: Some(json!({ "removeNone": true })),
            enabled: true,
        })],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#);
}

#[test]
fn remove_unknowns_and_defaults_removes_root_defaults_and_unknown_svg_content() {
    let svg = r#"<?xml version="1.0" standalone="no"?><svg xmlns="http://www.w3.org/2000/svg" x="0" y="0" width="100%" height="100%" preserveAspectRatio="xMidYMid meet" zoomAndPan="magnify" version="1.1" baseProfile="none" contentScriptType="application/ecmascript" contentStyleType="text/css" foo="bar"><g bogus="1"><unknown-child/><rect x="0" y="0" width="1" height="1"/></g></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("removeUnknownsAndDefaults".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg"><g><rect width="1" height="1"/></g></svg>"#
    );
}

#[test]
fn remove_unknowns_and_defaults_preserves_foreign_object_subtree_and_data_attrs() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><foreignObject x="0" y="0" width="10" height="10" unknown="x"><div xmlns="http://www.w3.org/1999/xhtml" data-test="1" foo="bar"><span aria-hidden="true">Hi</span></div></foreignObject></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("removeUnknownsAndDefaults".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, svg);
}

#[test]
fn remove_unknowns_and_defaults_strips_unknown_unprefixed_children_from_foreign_description_nodes() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<d:SVGTestCase xmlns:d="http://www.w3.org/2000/02/svg/testsuite/description/">"#,
        r#"<d:testDescription xmlns="http://www.w3.org/1999/xhtml"><p>Test</p></d:testDescription>"#,
        r#"</d:SVGTestCase></svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("removeUnknownsAndDefaults".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
            r#"<d:SVGTestCase xmlns:d="http://www.w3.org/2000/02/svg/testsuite/description/">"#,
            r#"<d:testDescription xmlns="http://www.w3.org/1999/xhtml"/></d:SVGTestCase></svg>"#,
        )
    );
}

#[test]
fn merge_styles_drops_media_attr_on_single_style_like_svgo() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style media="screen">.a"#,
        "{fill:red}",
        r#"</style></svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("mergeStyles".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.a"#,
            "{fill:red}",
            r#"</style></svg>"#
        )
    );
}

#[test]
fn merge_styles_uses_cdata_when_any_merged_style_uses_cdata() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style><![CDATA[.a"#,
        "{fill:red}",
        r#"]]></style><style>.b"#,
        "{fill:blue}",
        r#"</style></svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("mergeStyles".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><style><![CDATA[.a"#,
            "{fill:red}.b",
            "{fill:blue}",
            r#"]]></style></svg>"#
        )
    );
}

#[test]
fn remove_hidden_elems_keeps_hidden_group_with_visible_descendant() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g visibility="hidden"><g visibility="visible"><rect width="10" height="10"/></g></g></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("removeHiddenElems".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, svg);
}

#[test]
fn remove_hidden_elems_removes_unreferenced_marker_and_empty_path() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><marker id="m" display="none"><path d="M0 0"/></marker><path d=""/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("removeHiddenElems".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#);
}

#[test]
fn remove_hidden_elems_removes_hidden_defs_target_and_corresponding_use() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><defs><g id="gone" display="none"><path d="M0 0"/></g></defs><use href="#gone"/></svg>"##;
    let config = Config {
        plugins: vec![PluginSpec::Name("removeHiddenElems".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#);
}

#[test]
fn remove_hidden_elems_keeps_referenced_opacity_zero_path() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><defs><path id="p" opacity="0" d="M0 0"/></defs><use href="#p"/></svg>"##;
    let config = Config {
        plugins: vec![PluginSpec::Name("removeHiddenElems".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, svg);
}

#[test]
fn remove_hidden_elems_deoptimizes_non_rendering_removal_when_style_exists() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.x"#,
        "{fill:red}",
        r#"</style><marker id="m"><path d="M0 0"/></marker></svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("removeHiddenElems".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.x"#,
            "{fill:red}",
            r#"</style><marker id="m"/></svg>"#
        )
    );
}

#[test]
fn convert_transform_removes_identity_and_shortens_rotate_about_center() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g transform="translate(10 20) rotate(90) translate(-10 -20) scale(1 1) skewY(0)"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertTransform".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><g transform="rotate(90 10 20)"/></svg>"#
    );
}

#[test]
fn convert_transform_decomposes_axis_aligned_matrix_to_translate_and_scale() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g transform="matrix(.8 0 0 .8 40 0)"><path d="M0 0h10"/><path d="M20 0h10"/></g></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertTransform".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><g transform="translate(40)scale(.8)"><path d="M0 0h10"/><path d="M20 0h10"/></g></svg>"#
    );
}

#[test]
fn convert_path_data_normalizes_lines_and_collapses_repeated_axis_commands() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0L5 0L8 0L8 4L8 3"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0h8v4-1"/></svg>"#
    );
}

#[test]
fn convert_path_data_preserves_repeated_commands_when_marker_mid_matches_stylesheet() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.mid"#,
        "{marker-mid:url(#m)}",
        r#"</style><path class="mid" d="M0 0L5 0L8 0L8 4L8 3"/></svg>"#
    );
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.mid"#,
            "{marker-mid:url(#m)}",
            r#"</style><path class="mid" d="M0 0h5h3v4v-1"/></svg>"#
        )
    );
}

#[test]
fn convert_path_data_utilizes_absolute_when_shorter() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0 L10 0 L10 10 L0 10 Z"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0h10v10H0Z"/></svg>"#
    );
}

#[test]
fn convert_path_data_compacts_moveto_and_line_runs_like_svgo() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M90 258L390 180"/><path d="M-30 0L0 -60L30 0Z"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="m90 258 300-78"/><path d="M-30 0 0-60 30 0Z"/></svg>"#
    );
}

#[test]
fn convert_path_data_uses_smooth_curve_shorthand_when_first_control_is_current_point() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M90 258C90 258 216 120 390 198"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M90 258s126-138 300-60"/></svg>"#
    );
}

#[test]
fn convert_path_data_uses_smooth_curve_shorthand_after_reflected_control_point() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M260 131c0-15 12-28 28-28 15 0 27 13 27 28 0 15-12 28-27 28z"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M260 131c0-15 12-28 28-28 15 0 27 13 27 28s-12 28-27 28z"/></svg>"#
    );
}

#[test]
fn convert_path_data_utilizes_absolute_quadratic_when_shorter() {
    let svg =
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0q30 0 30-30q-30 0-30 30"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0q30 0 30-30Q0-30 0 0"/></svg>"#
    );
}

#[test]
fn convert_path_data_drops_redundant_close_segment_before_z() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M135 55h25h-25z"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M135 55h25z"/></svg>"#
    );
}

#[test]
fn convert_path_data_keeps_zero_length_stub_when_it_is_the_only_draw_command() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M40 0h0"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M40 0h0"/></svg>"#
    );
}

#[test]
fn convert_path_data_reduces_axis_aligned_degenerate_curves_to_lines() {
    let svg =
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M80 170C100 170 160 170 180 170Z"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M80 170h100Z"/></svg>"#
    );
}

#[test]
fn convert_path_data_compacts_repeated_curve_commands_into_single_run() {
    let svg =
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0c10 0 20 0 30 0c10 0 20 0 30 0"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("convertPathData".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0h30s20 0 30 0"/></svg>"#
    );
}

#[test]
fn convert_path_data_bakes_affine_transforms_into_non_arc_paths() {
    assert_oracle_fixture("convert-path-data-transform");
}
