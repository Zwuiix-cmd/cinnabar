//! The production painter retains catalogs, layouts and text across unchanged editor frames.

use super::*;

struct NoImages;
impl ui_engine::sources::RemoteSource for NoImages {
    /// The original editor fixture has no remote artwork.
    fn state(&self, _: &str) -> ui_engine::sources::RemoteImage {
        ui_engine::sources::RemoteImage::Failed
    }
}

/// Creates a production painter without local game carriers or a graphics device.
fn engine() -> FormEngine {
    use ui_engine::{engine::screen::ScreenRegistry, sources::ImageWork};
    static REGISTRY: std::sync::OnceLock<ScreenRegistry> = std::sync::OnceLock::new();
    let bytes = assets::encode_ui_catalog([1; 32], &[], &[], &[], &[]).unwrap();
    FormEngine::new(
        Arc::new(assets::RuntimeUiAssets::decode(&bytes).unwrap()),
        Catalog::default(),
        0,
        REGISTRY.get_or_init(|| ScreenRegistry::new(Context::desktop())),
        Arc::new(NoImages),
        ImageWork::inline(),
    )
}

/// Paints one editor frame through the same path used by the client.
fn paint(
    ui: &mut ModUi,
    engine: &FormEngine,
    font: &assets::RuntimeFontCatalog,
    layouts: &mut ui::TextLayoutCache,
    now: f64,
) {
    let icons = ui.mod_hud_editor_icons(|_, _| None).unwrap();
    ui.paint_mod_hud_editor(
        Some(engine),
        &mut PaintFrame {
            layouts,
            font,
            metrics: ui_engine::text_metrics::TextMetrics::for_viewport(
                [1280, 720],
                ui::DpiScale::new(1.).unwrap(),
                Some(2),
            ),
            solid_page: 0,
            safe_area: ui::SafeArea::ZERO,
            content: [1280., 720.],
            translate: &|_| None,
            language: [0; 3],
            now,
            nodes: &mut Vec::new(),
            next: &mut 1,
            renderers: &mut [],
        },
        &icons,
    );
    assert!(ui.mod_hud_editor_open());
}

#[test]
fn amendment_hidden_artwork_and_steady_frames_retain_catalog_bindings_and_text() {
    let engine = engine();
    let font = ui_engine::test_support::fixture_font();
    let mut layouts = ui::TextLayoutCache::new(64, 1 << 24);
    let mut ui = ModUi::default();
    ui.set_mod_panel(
        crate::PanelUiOwner::Environment(0),
        Some(&crate::test_support::sample_panel()),
    )
    .unwrap();
    ui.set_mod_panel_open(true);
    let mut hud = Hud {
        inspector_size: Some([180., 120.]),
        cards: vec![ui::mod_hud::Card {
            id: "disabled".into(),
            title: "Hidden module".into(),
            visible: false,
            ..Default::default()
        }],
        ..Default::default()
    };
    ui.open_mod_hud_editor(&hud).unwrap();
    ui.mod_hud_editor.as_mut().unwrap().selected = Some(0);
    paint(&mut ui, &engine, &font, &mut layouts, 1.);
    let editor = ui.mod_hud_editor.as_ref().unwrap();
    assert!(
        !editor.screen.nodes().iter().any(|node| matches!(&node.draw,
        json_ui::Draw::Text { text, .. } if text == "Hidden module"))
    );
    let catalog = Arc::clone(editor.catalog.as_ref().unwrap());
    let data = Arc::clone(&editor.data);
    let passes = editor.screen.passes;
    let shaped = layouts.built_layout_count();
    paint(&mut ui, &engine, &font, &mut layouts, 2.);
    let editor = ui.mod_hud_editor.as_ref().unwrap();
    assert!(Arc::ptr_eq(&catalog, editor.catalog.as_ref().unwrap()));
    assert!(Arc::ptr_eq(&data, &editor.data));
    assert_eq!(editor.screen.passes, passes);
    assert_eq!(layouts.built_layout_count(), shaped);
    for action in ["hud.grid", "hud.snap", "hud.visibility:0"] {
        ui.mod_hud_editor.as_mut().unwrap().action(action, &mut []);
        paint(&mut ui, &engine, &font, &mut layouts, 3.);
        assert!(Arc::ptr_eq(
            &catalog,
            ui.mod_hud_editor
                .as_ref()
                .unwrap()
                .catalog
                .as_ref()
                .unwrap()
        ));
    }
    assert!(
        ui.mod_hud_editor
            .as_ref()
            .unwrap()
            .screen
            .nodes()
            .iter()
            .any(|node| matches!(&node.draw,
        json_ui::Draw::Text { text, .. } if text == "Hidden module"))
    );
    hud.inspector_size = Some([220., 160.]);
    ui.update_mod_hud_editor(&hud).unwrap();
    paint(&mut ui, &engine, &font, &mut layouts, 4.);
    assert!(Arc::ptr_eq(
        &catalog,
        ui.mod_hud_editor
            .as_ref()
            .unwrap()
            .catalog
            .as_ref()
            .unwrap()
    ));
}

/// A short distance must not let its wider annotation cover the selected widget or nearby keys.
#[test]
fn measurement_captions_keep_selected_and_neighboring_artwork_clear() {
    let mut ui = ModUi::default();
    ui.set_mod_panel(
        crate::PanelUiOwner::Environment(0),
        Some(&crate::test_support::sample_panel()),
    )
    .unwrap();
    ui.set_mod_panel_open(true);
    let hud: Hud = serde_json::from_value(serde_json::json!({"cards":[
        {"id":"fps","anchor":"top_left","offset":[12,37],"width":56,"row_height":18,"rows":[{"label":"FPS","value":"144"}]},
        {"id":"keys","anchor":"top_left","offset":[0,60],"width":70,"row_height":26,"rows":[{"label":"W","value":""},{"label":"A S D","value":""},{"label":"SPACE","value":""}]}
    ]})).unwrap();
    ui.open_mod_hud_editor(&hud).unwrap();
    let editor = ui.mod_hud_editor.as_mut().unwrap();
    editor.viewport = [400., 240.];
    editor.selected = Some(0);
    let data = editor.data();
    for axis in 0..2 {
        assert_eq!(
            data.global_value(&format!("#measurement_{axis}_visible")),
            Some(&Scalar::Bool(true))
        );
        let Scalar::Json(value) = data
            .global_value(&format!("#measurement_{axis}_pill"))
            .unwrap()
        else {
            panic!("caption position")
        };
        let point: [f64; 2] = serde_json::from_value(value.clone()).unwrap();
        for (index, card) in editor.draft.cards.iter().enumerate() {
            let at = editor.card_origin(index);
            let size = cards::dimensions(card);
            assert!(
                point[0] + MEASUREMENT_PILL[0] <= at[0]
                    || point[0] >= at[0] + size[0]
                    || point[1] + MEASUREMENT_PILL[1] <= at[1]
                    || point[1] >= at[1] + size[1],
                "caption covers {}",
                card.id
            );
        }
    }
}

/// Authored editor typography stays centered inside the distance pill rather than filling its band.
#[test]
fn compact_editor_typography_centers_measurements_in_their_backgrounds() {
    let mut ui = ModUi::default();
    ui.set_mod_panel(
        crate::PanelUiOwner::Environment(0),
        Some(&crate::test_support::sample_panel()),
    )
    .unwrap();
    ui.set_mod_panel_open(true);
    let hud: Hud = serde_json::from_str(r#"{"editor_style":{"font":"sans","text_scale":0.5,"measurement":[0.1,0.2,0.3,1]},"cards":[{"id":"fps","width":56,"offset":[37,41],"rows":[{"label":"FPS","value":"144"}]}]}"#).unwrap();
    ui.open_mod_hud_editor(&hud).unwrap();
    ui.mod_hud_editor.as_mut().unwrap().selected = Some(0);
    let engine = engine();
    let font = ui_engine::test_support::fixture_font();
    let font = font
        .with_named_font(ui::mod_panel::SANS_FONT_NAME, &font)
        .unwrap();
    paint(
        &mut ui,
        &engine,
        &font,
        &mut ui::TextLayoutCache::new(64, 1 << 24),
        1.,
    );
    let editor = ui.mod_hud_editor.as_ref().unwrap();
    let mut labels = 0;
    for node in editor.screen.nodes() {
        if let json_ui::Draw::Text {
            text,
            scale,
            options,
            ..
        } = &node.draw
            && text.ends_with(" px")
        {
            labels += 1;
            assert_eq!(*scale, 0.5);
            assert_eq!(
                options.font_type.as_deref(),
                Some(ui::mod_panel::SANS_FONT_NAME)
            );
            assert!(node.dest.h < MEASUREMENT_PILL[1]);
            assert!(
                node.dest.y >= node.clip.y
                    && node.dest.y + node.dest.h <= node.clip.y + node.clip.h
            );
        }
    }
    assert_eq!(labels, 2);
}

/// Native selection chrome reserves its full measured caption before placing the inspector and guides.
#[test]
fn measured_selection_caption_stays_compact_and_clear_of_the_inspector() {
    let engine = engine();
    let font = ui_engine::test_support::fixture_font();
    let font = font
        .with_named_font(ui::mod_panel::SANS_FONT_NAME, &font)
        .unwrap();
    for width in [56., 130., 400.] {
        for scale in [0.25, 1.] {
            let hud: Hud = serde_json::from_value(serde_json::json!({
                "editor_style":{"selection_caption":true,"font":"sans","text_scale":0.5,"top_clearance":44},
                "inspector_size":[104,204],
                "cards":[{"id":"fps","editor_label":"FPS","width":width,"scale":scale,"offset":[12,37],"rows":[{"label":"","value":"144 FPS"}]}]
            })).unwrap();
            let mut ui = ModUi::default();
            ui.set_mod_panel(
                crate::PanelUiOwner::Environment(0),
                Some(&crate::test_support::sample_panel()),
            )
            .unwrap();
            ui.set_mod_panel_open(true);
            ui.open_mod_hud_editor(&hud).unwrap();
            ui.mod_hud_editor.as_mut().unwrap().selected = Some(0);
            paint(
                &mut ui,
                &engine,
                &font,
                &mut ui::TextLayoutCache::new(64, 1 << 24),
                1.,
            );
            let editor = ui.mod_hud_editor.as_ref().unwrap();
            let caption = editor.caption_bounds.unwrap();
            assert_eq!(
                caption[2], 64.,
                "caption followed widget width {width} at scale {scale}"
            );
            let inspector = [editor.inspector.at[0], editor.inspector.at[1], 104., 204.];
            let overlaps = |a: [f64; 4], b: [f64; 4]| {
                a[0] < b[0] + b[2] && a[0] + a[2] > b[0] && a[1] < b[1] + b[3] && a[1] + a[3] > b[1]
            };
            assert!(
                !overlaps(caption, inspector),
                "inspector covers the caption"
            );
            assert!(editor.screen.nodes().iter().any(|node| node.key.contains("selection_caption") && matches!(&node.draw,json_ui::Draw::Custom { data,.. } if data.get("radius").and_then(serde_json::Value::as_f64)==Some(3.5)) && node.dest.w == 64.));
            for axis in 0..2 {
                if editor
                    .data
                    .global_value(&format!("#measurement_{axis}_visible"))
                    != Some(&Scalar::Bool(true))
                {
                    continue;
                }
                let Scalar::Json(value) = editor
                    .data
                    .global_value(&format!("#measurement_{axis}_pill"))
                    .unwrap()
                else {
                    panic!("caption position")
                };
                let point: [f64; 2] = serde_json::from_value(value.clone()).unwrap();
                let pill = [point[0], point[1], MEASUREMENT_PILL[0], MEASUREMENT_PILL[1]];
                assert!(!overlaps(pill, caption) && !overlaps(pill, inspector));
                assert!(pill[1] >= 44., "measurement overlaps the authored toolbar");
            }
        }
    }
}

#[test]
fn guest_slider_fill_follows_panel_values_without_rebuilding_or_churning_data() {
    let engine = engine();
    let font = ui_engine::test_support::fixture_font();
    let mut layouts = ui::TextLayoutCache::new(64, 1 << 24);
    let mut ui = ModUi::default();
    let mut panel = crate::test_support::sample_panel();
    let index = panel
        .controls
        .iter()
        .position(|c| matches!(c, ui::mod_panel::Control::Slider { .. }))
        .unwrap();
    let fill = format!("#row_{index}_fill");
    ui.set_mod_panel(crate::PanelUiOwner::Environment(0), Some(&panel))
        .unwrap();
    ui.set_mod_panel_open(true);
    ui.open_mod_hud_editor(&Hud::default()).unwrap();
    paint(&mut ui, &engine, &font, &mut layouts, 1.);
    let editor = ui.mod_hud_editor.as_ref().unwrap();
    assert_eq!(
        editor.data.global_value(&fill),
        Some(&Scalar::Num(f64::from(0.35_f32)))
    );
    let catalog = Arc::clone(editor.catalog.as_ref().unwrap());
    let data = Arc::clone(&editor.data);
    paint(&mut ui, &engine, &font, &mut layouts, 2.);
    assert!(Arc::ptr_eq(
        &data,
        &ui.mod_hud_editor.as_ref().unwrap().data
    ));
    if let ui::mod_panel::Control::Slider { value, .. } = &mut panel.controls[index] {
        *value = 80.;
    }
    // A same-shape guest republish keeps the editor open and only moves the fill.
    ui.set_mod_panel(crate::PanelUiOwner::Environment(0), Some(&panel))
        .unwrap();
    assert!(ui.mod_hud_editor_open());
    paint(&mut ui, &engine, &font, &mut layouts, 3.);
    let editor = ui.mod_hud_editor.as_ref().unwrap();
    assert_eq!(
        editor.data.global_value(&fill),
        Some(&Scalar::Num(f64::from(0.8_f32)))
    );
    assert!(Arc::ptr_eq(&catalog, editor.catalog.as_ref().unwrap()));
    let data = Arc::clone(&editor.data);
    paint(&mut ui, &engine, &font, &mut layouts, 4.);
    assert!(Arc::ptr_eq(
        &data,
        &ui.mod_hud_editor.as_ref().unwrap().data
    ));
}
