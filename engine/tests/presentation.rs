use valorant_horizon_engine::{
    gameplay::{
        geometry::*,
        smoke::Smoke,
        zones::{Zone, ZoneKind},
    },
    presentation::{
        mesh::{Frame, Scene},
        ui::*,
        *,
    },
    server::{Player, Session},
};
fn snapshot() -> valorant_horizon_engine::gameplay::Snapshot {
    let mut s = Session::default();
    s.players.insert(1, Player::default());
    s.snapshot(1).unwrap()
}
#[test]
fn malformed_and_partial_presentation_settings_are_safe() {
    let d = Settings::decode("broken");
    assert_eq!(d.quality, Quality::Medium);
    let s = Settings::decode(r#"{"quality":"High","scale":100,"x":-10,"y":99}"#);
    assert_eq!(s.scale, 1.5);
    assert_eq!(s.x, 0.);
    assert_eq!(s.y, 1.);
    let s = Settings {
        scale: f32::NAN,
        x: f32::INFINITY,
        y: f32::NEG_INFINITY,
        ..Default::default()
    };
    let result = Settings::decode(&s.encode());
    assert_eq!(result.scale, 1.);
    assert_eq!(result.x, 0.5);
}
#[test]
fn settings_roundtrip_and_layout_stay_on_screen() {
    for quality in [Quality::Low, Quality::Medium, Quality::High] {
        for scale in [0.7, 1., 1.5] {
            let s = Settings {
                quality,
                scale,
                x: 1.,
                y: 1.,
                sound: false,
            };
            let r = Settings::decode(&s.encode());
            assert_eq!(r.quality, quality);
            assert!(!r.sound);
            let origin = r.origin([800., 600.], [294., 104.]);
            assert!(origin[0] + 294. * scale <= 800.01);
            assert!(origin[1] + 104. * scale <= 600.01);
        }
    }
}
#[test]
fn menu_filters_are_case_insensitive_composable_and_bounded() {
    let names = [
        "Jett",
        "Sage",
        "Phoenix",
        "Omen",
        "Raze",
        "Reyna",
        "Brimstone",
        "Viper",
        "Astra",
        "Harbor",
        "Clove",
        "Sova",
        "Cypher",
        "Fade",
        "Neon",
        "Yoru",
        "Iso",
        "Waylay",
        "KAY/O",
        "Breach",
        "Skye",
        "Gekko",
    ];
    let mut m = Menu {
        query: "SK".into(),
        ..Default::default()
    };
    assert_eq!(m.results(&names), vec![20]);
    m.role = Role::Duelist;
    assert!(m.results(&names).is_empty());
    m.query = "o".into();
    assert!(m.results(&names).contains(&14));
    m.search = true;
    for _ in 0..100 {
        m.typed('é');
    }
    assert!(m.query.chars().count() <= 32);
    let n = m.query.len();
    m.typed('\n');
    assert_eq!(m.query.len(), n);
}
#[test]
fn categories_cover_all_registered_agents_without_overlap() {
    let names = valorant_horizon_engine::agents::Registry::default();
    let names: Vec<_> = names.agents().iter().map(|a| a.name).collect();
    let mut all = vec![];
    for role in [
        Role::Duelist,
        Role::Controller,
        Role::Sentinel,
        Role::Initiator,
    ] {
        let m = Menu {
            role,
            ..Default::default()
        };
        all.extend(m.results(&names));
    }
    all.sort();
    assert_eq!(all, (0..22).collect::<Vec<_>>());
}
#[test]
fn menu_settings_cycle_stays_in_valid_range_and_marks_save() {
    let mut m = Menu::default();
    let mut s = Settings::default();
    for _ in 0..100 {
        for row in 0..5 {
            m.adjust(row, &mut s);
            assert!((0.65..=1.5).contains(&s.scale));
            assert!((0. ..=1.).contains(&s.x));
            assert!((0. ..=1.).contains(&s.y));
        }
    }
    assert!(m.dirty);
}
#[test]
fn small_screen_transform_has_matching_mouse_coordinates() {
    for size in [[192., 144.], [320., 240.], [1280., 720.]] {
        let v = Viewport::new(size);
        let pt = [v.size[0] * 0.7, 100.];
        let physical = pt.map(|x| x * v.scale);
        let got = v.point(physical);
        assert!((got[0] - pt[0]).abs() < 0.001);
        assert!(v.size[1] >= 239.999);
    }
}
#[test]
fn original_icons_distinguish_functional_categories() {
    let heal = icon("Healing Orb", false);
    let smoke = icon("Cloudburst", false);
    let flash = icon("Curveball", false);
    assert_ne!(heal, smoke);
    assert_ne!(smoke, flash);
    for glyph in [heal, smoke, flash, icon("Thrash", true)] {
        assert!(glyph.iter().any(|r| *r != 0));
    }
}
fn quad(z: f32) -> [[f32; 3]; 4] {
    [[-1., -1., z], [-1., 1., z], [1., 1., z], [1., -1., z]]
}
#[test]
fn alpha_faces_sort_behind_opaque_then_back_to_front() {
    let mut f = Frame::new(Quality::Low);
    f.push(quad(2.), [1, 2, 3, 100]);
    f.push(quad(5.), [1, 2, 3, 100]);
    f.push(quad(1.), [1, 2, 3, 255]);
    let faces = f.sorted();
    assert_eq!(faces[0].color[3], 255);
    assert_eq!(faces[1].points[0][2], 5.);
    assert_eq!(faces[2].points[0][2], 2.);
}
#[test]
fn quality_geometry_and_particle_work_are_strictly_bounded() {
    for q in [Quality::Low, Quality::Medium, Quality::High] {
        let mut frame = Frame::new(q);
        for _ in 0..q.quads() + 100 {
            frame.push(quad(1.), [1; 4]);
        }
        assert_eq!(frame.faces.len(), q.quads());
        assert_eq!(frame.dropped, 100);
        let mut scene = Scene::default();
        scene.burst([0.; 3], [255; 4], q, usize::MAX);
        assert_eq!(scene.particles.len(), q.particles());
        let mut f = Frame::new(q);
        scene.draw(&mut f, [0.; 3], 0.5);
        assert!(f.faces.len() <= q.particles() * 4);
    }
}
#[test]
fn invalid_and_far_geometry_is_culled_but_near_crossing_wall_remains() {
    let mut f = Frame::new(Quality::Low);
    f.push([[f32::NAN; 3]; 4], [255; 4]);
    f.push(quad(1000.), [255; 4]);
    assert!(f.faces.is_empty());
    f.push(
        [
            [-1000., -1000., 1.],
            [-1000., 1000., 1.],
            [1000., 1000., 1.],
            [1000., -1000., 1.],
        ],
        [255; 4],
    );
    assert_eq!(f.faces.len(), 1);
}
#[test]
fn smoke_core_stays_opaque_at_every_quality_and_final_tick() {
    for q in [Quality::Low, Quality::Medium, Quality::High] {
        let mut f = Frame::new(q);
        f.smoke(
            &Smoke {
                owner: 1,
                center: [0.; 3],
                radius: 3.,
                ticks: 1,
                color: [150, 180, 220, 60],
            },
            [0.; 3],
            100.,
        );
        assert!(f.faces.iter().any(|a| a.color[3] == 255));
        assert!(f.faces.iter().any(|a| a.color[3] > 0 && a.color[3] < 255));
        assert!(f.faces.len() < q.quads());
    }
}
#[test]
fn smoke_camera_subtraction_preserves_world_border_precision() {
    let mut f = Frame::new(Quality::High);
    f.smoke(
        &Smoke {
            owner: 1,
            center: [30_000_000.5, 64., 0.],
            radius: 1.,
            ticks: 100,
            color: [255; 4],
        },
        [30_000_000., 64., 0.],
        0.,
    );
    let max = f
        .faces
        .iter()
        .filter(|f| f.color[3] == 255)
        .flat_map(|f| f.points)
        .map(|p| p[0])
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((max - 1.5).abs() < 0.001);
}
#[test]
fn cosmetic_pool_expires_and_clears_on_disconnect() {
    let mut scene = Scene::default();
    let s = snapshot();
    scene.burst([0.; 3], [255; 4], Quality::High, 256);
    for _ in 0..24 {
        scene.tick(&s, None, Quality::Low);
    }
    assert!(scene.particles.is_empty());
    scene.burst([0.; 3], [255; 4], Quality::High, 256);
    scene.clear();
    assert!(scene.particles.is_empty());
    assert!(scene.cues.is_empty());
}
#[test]
fn audio_transitions_do_not_replay_on_identical_snapshots() {
    let mut scene = Scene::default();
    let mut s = snapshot();
    scene.tick(&s, Some([0.; 3]), Quality::Medium);
    assert!(scene.cues.is_empty());
    s.charges[0] = s.charges[0].saturating_sub(1);
    scene.tick(&s, Some([0.; 3]), Quality::Medium);
    assert!(scene.cues.contains(&Cue::Cast));
    scene.tick(&s, Some([0.; 3]), Quality::Medium);
    assert!(scene.cues.is_empty());
    s.vision.flash = 10;
    scene.tick(&s, Some([0.; 3]), Quality::Medium);
    assert!(scene.cues.contains(&Cue::Flash));
    scene.tick(&s, Some([0.; 3]), Quality::Medium);
    assert!(scene.cues.is_empty());
}
#[test]
fn agent_change_and_remote_camera_never_create_spurious_dash_or_cast() {
    let mut scene = Scene::default();
    let mut s = snapshot();
    scene.tick(&s, Some([0.; 3]), Quality::Medium);
    s.agent = 2;
    s.charges = [0; 4];
    scene.tick(&s, Some([50.; 3]), Quality::Medium);
    assert!(scene.cues.is_empty());
    assert!(scene.particles.is_empty());
    s.camera = Some([100.; 3]);
    scene.tick(&s, Some([100.; 3]), Quality::Medium);
    s.camera = None;
    scene.tick(&s, Some([0.; 3]), Quality::Medium);
    assert!(scene.particles.is_empty());
}
#[test]
fn dash_and_ultimate_are_bounded_events_and_quality_does_not_mutate_snapshot() {
    let mut scene = Scene::default();
    let mut s = snapshot();
    scene.tick(&s, Some([0.; 3]), Quality::High);
    s.active[3] = 10;
    let before = serde_json::to_vec(&s).unwrap();
    scene.tick(&s, Some([2., 0., 0.]), Quality::Low);
    assert!(!scene.particles.is_empty());
    assert!(scene.cues.contains(&Cue::Ultimate));
    assert_eq!(serde_json::to_vec(&s).unwrap(), before);
    assert!(scene.particles.len() <= 48);
}
#[test]
fn every_zone_kind_has_bounded_finite_geometry() {
    let kinds = [
        ZoneKind::ElectricWall,
        ZoneKind::ElectricPulse,
        ZoneKind::Seize,
        ZoneKind::Nightfall,
        ZoneKind::CyberCage,
        ZoneKind::WaterWall,
        ZoneKind::Whirlpool,
        ZoneKind::Gravity,
        ZoneKind::Nova,
        ZoneKind::Acid,
        ZoneKind::Stim,
        ZoneKind::Orbital,
        ZoneKind::Incendiary,
        ZoneKind::FireWall,
        ZoneKind::FirePool,
        ZoneKind::Slow,
    ];
    for q in [Quality::Low, Quality::Medium, Quality::High] {
        let mut s = snapshot();
        s.zones = kinds
            .map(|kind| Zone {
                owner: 1,
                kind,
                bounds: Box3 {
                    min: [-3., 0., -3.],
                    max: [3., 3., 3.],
                },
                ticks: 100,
            })
            .into();
        let before = serde_json::to_vec(&s).unwrap();
        let mut f = Frame::new(q);
        f.zones(&s, [0.; 3], 10.5);
        assert!(!f.faces.is_empty());
        assert!(f.faces.len() <= q.quads());
        assert!(
            f.faces
                .iter()
                .flat_map(|f| f.points)
                .flatten()
                .all(f32::is_finite)
        );
        assert_eq!(serde_json::to_vec(&s).unwrap(), before);
    }
}
#[test]
fn authoritative_cosmetic_emission_has_fixed_budget() {
    let mut b = EmissionBudget::default();
    for _ in 0..32 {
        assert!(b.allow(100));
    }
    for _ in 0..100 {
        assert!(!b.allow(100));
    }
    assert!(b.allow(101));
}

// Exercise the shipped SDK input adapter with only the key lookup boundary replaced.
struct Game<'a>(std::marker::PhantomData<&'a ()>);
impl Game<'_> {
    fn native_mod_key(&self, _id: &str, _slot: &str, default: &str) -> String {
        default.into()
    }
}
#[path = "../../bridge/ui_input.rs"]
mod input_adapter;
#[test]
fn shipped_menu_adapter_routes_search_categories_binding_and_scaled_clicks() {
    use valorant_horizon_engine::{State, state};
    let game = Game(Default::default());
    *state().lock().unwrap() = State::default();
    game.vh_ui_key("key.keyboard.slash");
    {
        let mut s = state().lock().unwrap();
        for c in "GEKKO".chars() {
            s.ui.typed(c);
        }
    }
    game.vh_ui_key("key.keyboard.enter");
    game.vh_ui_key("key.keyboard.1");
    assert_eq!(state().lock().unwrap().ui.select, Some(21));
    {
        let mut s = state().lock().unwrap();
        s.ui.query.clear();
        s.ui.select = None;
    }
    game.vh_ui_key("key.keyboard.tab");
    assert_eq!(state().lock().unwrap().ui.role, Role::Duelist);
    game.vh_ui_key("key.keyboard.f2");
    game.vh_ui_key("key.keyboard.1");
    game.vh_ui_key("key.keyboard.q"); // Other slot already owns Q.
    assert!(state().lock().unwrap().ui.binding.is_none());
    game.vh_ui_key("key.keyboard.1");
    game.vh_ui_key("key.keyboard.r");
    assert!(state().lock().unwrap().ui.binding.is_none());
    game.vh_ui_key("key.keyboard.1");
    game.vh_ui_key("key.keyboard.f");
    assert_eq!(
        state().lock().unwrap().ui.binding,
        Some((0, "key.keyboard.f".into()))
    );
    game.vh_ui_key("key.keyboard.f3");
    let viewport = Viewport::new([192., 144.]);
    let physical = [30., 65.].map(|v| v * viewport.scale);
    game.vh_ui_click(physical, [192., 144.]);
    let s = state().lock().unwrap();
    assert_eq!(s.presentation.quality, Quality::High);
    assert!(s.ui.dirty);
}
#[test]
fn surface_and_reused_frame_buffers_remain_bounded_across_quality_changes() {
    let mut f = Frame::new(Quality::High);
    let kinds = [
        ZoneKind::FirePool,
        ZoneKind::WaterWall,
        ZoneKind::ElectricWall,
        ZoneKind::Acid,
        ZoneKind::Orbital,
    ];
    for i in 0..1000 {
        let z = Zone {
            owner: 1,
            kind: kinds[i % kinds.len()],
            bounds: Box3 {
                min: [-3., 0., -3.],
                max: [3., 3., 3.],
            },
            ticks: 5,
        };
        f.surface(&z, [0.; 3], i as f64);
    }
    assert!(f.faces.len() <= Quality::High.quads());
    f.reset(Quality::Low);
    assert!(f.faces.is_empty());
    assert!(f.faces.capacity() <= Quality::Low.quads() * 2);
    for _ in 0..12 {
        f.smoke(
            &Smoke {
                owner: 1,
                center: [0.; 3],
                radius: 3.,
                ticks: 1,
                color: [255; 4],
            },
            [0.; 3],
            1.,
        );
    }
    assert_eq!(f.dropped, 0);
}
