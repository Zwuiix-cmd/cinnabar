//! Scrollbar ownership keeps inventory gesture edges and cancellation intact.

use super::*;

/// Supplies a scrollable view with an independently positioned thumb and track.
fn frame() -> ui_engine::frame::EngineFrame {
    let rect = json_ui::RectOut {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 100.0,
    };
    let region = json_ui::HitRegion {
        key: "scroll".into(),
        name: "scroll".into(),
        kind: json_ui::HitKind::ScrollView,
        rect,
        clip: rect,
        layer: 0,
        order: 0,
        pressed: None,
        control_name: None,
        collection_index: None,
        collection: None,
        enabled: true,
        checked: None,
        max_length: None,
        group_index: None,
        renderer: None,
        drag_axes: [false; 2],
        sound: None,
        input: Default::default(),
        focus: None,
        collections: Vec::new(),
        widget: Default::default(),
        modal_root: None,
        transform: None,
    };
    ui_engine::frame::EngineFrame {
        hits: Arc::from([region]),
        report: json_ui::LayoutReport {
            scrolls: [(
                "scroll".to_owned(),
                json_ui::ScrollMetrics {
                    content: 200.0,
                    viewport: 100.0,
                    viewport_rect: Some([0.0, 0.0, 100.0, 100.0]),
                    track: Some([95.0, 0.0, 5.0, 100.0]),
                    thumb: Some([95.0, 0.0, 5.0, 50.0]),
                    box_drag: json_ui::Draggable::Vertical,
                    track_button: Some("track".into()),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        },
        cancel_target: None,
        origin: [0.0; 2],
        scale: 1.0,
        panel: None,
        edit_texts: Vec::new(),
        top: Vec::new(),
    }
}

#[test]
fn engine_scrollbar_drag_moves_the_container_without_clicking_a_slot() {
    let frame = frame();
    let mut runtime = UiRuntime::new(1);
    assert!(scroll_container(
        &mut runtime,
        &frame,
        [97.0, 25.0],
        &[],
        true,
        false,
    ));
    assert_eq!(
        runtime.screen_state().container_scroll_drag,
        Some(("scroll".into(), 25.0))
    );
    assert!(scroll_container(
        &mut runtime,
        &frame,
        [97.0, 35.0],
        &[],
        false,
        false,
    ));
    assert_eq!(runtime.screen_state().container_scroll["scroll"], 20.0);
    assert!(scroll_container(
        &mut runtime,
        &frame,
        [97.0, 45.0],
        &[],
        false,
        false,
    ));
    assert_eq!(runtime.screen_state().container_scroll["scroll"], 40.0);
    assert!(scroll_container(
        &mut runtime,
        &frame,
        [97.0, 45.0],
        &[],
        false,
        true,
    ));
    assert!(runtime.screen_state().container_scroll_drag.is_none());
}

#[test]
fn wheel_scrolling_leaves_inventory_button_edges_available() {
    let mut runtime = UiRuntime::new(1);
    assert!(!scroll_container(
        &mut runtime,
        &frame(),
        [50.0, 50.0],
        &[(-20.0, MouseScrollUnit::Pixel)],
        false,
        true,
    ));
    assert!(runtime.screen_state().container_scroll["scroll"] > 0.0);
}

#[test]
fn a_complete_click_does_not_retain_scrollbar_capture() {
    let mut runtime = UiRuntime::new(1);
    assert!(scroll_container(
        &mut runtime,
        &frame(),
        [97.0, 25.0],
        &[],
        true,
        true
    ));
    assert!(runtime.screen_state().container_scroll_drag.is_none());
    assert!(!scroll_container(
        &mut runtime,
        &frame(),
        [97.0, 35.0],
        &[],
        false,
        false
    ));
}

#[test]
fn a_thumb_outside_the_track_and_viewport_can_be_dragged() {
    let mut frame = frame();
    let metrics = frame.report.scrolls.get_mut("scroll").unwrap();
    metrics.track = Some([95.0, 50.0, 5.0, 50.0]);
    metrics.viewport_rect = Some([0.0, 50.0, 90.0, 50.0]);
    let mut runtime = UiRuntime::new(1);
    assert!(scroll_container(
        &mut runtime,
        &frame,
        [97.0, 25.0],
        &[],
        true,
        false
    ));
    assert!(runtime.screen_state().container_scroll_drag.is_some());
}

#[test]
fn losing_window_focus_cancels_scrollbar_capture() {
    let (mut app, entity) = ordered::personal_app(2);
    app.update();
    app.world_mut()
        .resource_mut::<UiRuntimeResource>()
        .screen_state_mut()
        .container_scroll_drag = Some(("scroll".into(), 25.0));
    app.world_mut().get_mut::<Window>(entity).unwrap().focused = false;
    app.update();
    assert!(
        app.world()
            .resource::<UiRuntimeResource>()
            .screen_state()
            .container_scroll_drag
            .is_none()
    );
}

#[test]
fn withdrawing_the_pointer_cancels_scrollbar_capture() {
    let mut player = client_state::PlayerRuntime(player_state::PlayerState::new(1));
    let mut runtime = client_ui::test_support::inventory_session(&mut player);
    runtime.toggle_inventory(&mut player, None);
    runtime.observe_inventory_window(&mut player);
    let presentation = presentation(&player, &runtime);
    runtime.screen_state_mut().container_scroll_drag = Some(("scroll".into(), 25.0));
    let mut window = window(Vec2::ZERO);
    window.set_cursor_position(None);
    pointer_edge(
        &mut player,
        &mut runtime,
        &window,
        &presentation,
        false,
        true,
    );
    assert!(runtime.screen_state().container_scroll_drag.is_none());
}

#[test]
fn hidden_disabled_and_clipped_scrollbars_do_not_capture_presses() {
    for case in 0..3 {
        let mut frame = frame();
        match case {
            0 => Arc::make_mut(&mut frame.hits)[0].enabled = false,
            1 => frame.report.scrolls.get_mut("scroll").unwrap().bar_visible = Some(false),
            _ => Arc::make_mut(&mut frame.hits)[0].clip.w = 90.0,
        }
        let mut runtime = UiRuntime::new(1);
        assert!(!scroll_container(
            &mut runtime,
            &frame,
            [97.0, 25.0],
            &[],
            true,
            false,
        ));
        assert!(runtime.screen_state().container_scroll_drag.is_none());
    }
}

#[test]
fn releasing_a_moved_thumb_applies_its_final_position() {
    let mut runtime = UiRuntime::new(1);
    let frame = frame();
    assert!(scroll_container(
        &mut runtime,
        &frame,
        [97.0, 25.0],
        &[],
        true,
        false,
    ));
    assert!(scroll_container(
        &mut runtime,
        &frame,
        [97.0, 35.0],
        &[],
        false,
        true,
    ));
    assert_eq!(
        runtime.screen_state().container_scroll.get("scroll"),
        Some(&20.0)
    );
    assert!(runtime.screen_state().container_scroll_drag.is_none());
}
