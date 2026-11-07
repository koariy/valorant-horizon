mod support;
use support::*;
use valorant_horizon_engine::gameplay::{geometry::*, remote_view::RemoteView, *};
#[test]
fn invalid_pose_and_limits_fail_closed() {
    let w = TestWorld::default();
    assert!(RemoteView::new(&w, [f64::NAN; 3], 5.).is_none());
    assert!(RemoteView::new(&w, [f64::MAX; 3], 5.).is_none());
    assert!(RemoteView::new(&w, [0., 2., 0.], 65.).is_none());
    assert!(RemoteView::new(&w, [0., 0., 0.], 5.).is_none());
}
#[test]
fn bounded_motion_does_not_move_owner() {
    let w = TestWorld::default();
    let original = w.actors[0].position;
    let mut view = RemoteView::new(&w, [0., 2., 0.], 2.).unwrap();
    for _ in 0..100 {
        view.step(&w, [0., 0., 100.]);
    }
    assert!(distance(view.origin, view.position) <= 2.0001);
    assert_eq!(w.actors[0].position, original);
    assert!(!view.look([f64::INFINITY, 0., 0.]));
    assert!(!view.step(&w, [f64::NAN, 0., 0.]));
}
#[test]
fn camera_does_not_tunnel_through_wall() {
    let w = TestWorld {
        wall: Some(Box3 {
            min: [-2., 0., 1.],
            max: [2., 5., 1.2],
        }),
        ..Default::default()
    };
    let mut view = RemoteView::new(&w, [0., 2., 0.], 10.).unwrap();
    for _ in 0..10 {
        view.step(&w, [0., 0., 1.]);
    }
    assert!(view.position[2] < 1.);
}
#[test]
fn remote_targeting_uses_remote_eye_and_checks_occlusion() {
    let mut w = TestWorld {
        wall: Some(Box3 {
            min: [-2., 0., 0.5],
            max: [2., 5., 0.8],
        }),
        ..Default::default()
    };
    w.actors[1] = actor(2, [0., 1., 2.]);
    let a = w.actors[0].clone();
    assert!(aim(&w, &a, 10., false).is_none());
    let mut view = RemoteView::new(&w, [0., 2., 5.], 10.).unwrap();
    view.look([0., 0., -1.]);
    assert_eq!(view.target(&w, &a, 10.).unwrap().entity, Entity::Player(2));
    assert!(view.target(&w, &a, f64::NAN).is_none());
}
