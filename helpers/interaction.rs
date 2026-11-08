fn valorant_fire(
    game: &mut crate::game::Game<'_>,
    button: i32,
    canceled: &mut bool,
    swing_hand: &mut bool,
) {
    if !game.native_window() || !game.native_playing() {
        return;
    }
    use valorant_horizon_engine::{
        gameplay::{Action, geometry::*},
        state,
    };
    if state()
        .lock()
        .ok()
        .is_some_and(|s| s.menu || s.rebind.is_some())
    {
        if button == 0 || button == 1 {
            *canceled = true;
            *swing_hand = false;
        }
        return;
    }
    let waiting = state()
        .lock()
        .ok()
        .is_some_and(|s| (s.selected == 8 || s.selected >= 11) && s.pending_equipped.is_some());
    if waiting {
        *canceled = true;
        *swing_hand = false;
        return;
    }
    let snapshot = state().lock().ok().and_then(|s| s.snapshot.clone());
    if snapshot
        .as_ref()
        .is_some_and(|s| s.camera.is_some() || ((s.agent == 11 || s.agent >= 12) && s.equipped))
    {
        *canceled = true;
        *swing_hand = false;
        if button == 0 || button == 1 {
            let fresh = state().lock().ok().is_some_and(|mut s| {
                s.key_transition(
                    if button == 0 {
                        "ability.attack"
                    } else {
                        "ability.use"
                    },
                    true,
                )
            });
            if fresh {
                game.vh_send(Action::Fire(button == 1));
            }
        }
        return;
    }
    if snapshot.as_ref().is_some_and(|s| s.vision.phased) {
        *canceled = true;
        *swing_hand = false;
        return;
    }
    let equipped = state().lock().ok().is_some_and(|s| {
        s.pending_equipped.is_some() || s.snapshot.as_ref().is_some_and(|s| s.equipped)
    });
    let object = snapshot.as_ref().is_some_and(|s| {
        let Some(from) = game.eye_position() else {
            return false;
        };
        let dir = crate::game::view_vector(game.yaw, game.pitch);
        let end = add(from, scale(dir, 3.0));
        let wall = game
            .native_raycast(3.0)
            .map(|hit| distance(from, [hit.x, hit.y, hit.z]) / 3.0)
            .unwrap_or(1.0);
        let wall = wall
            .min(game.mob_hit().map_or(1.0, |(_, d, _)| d / 3.0))
            .min(
                game.remote_object_hit(true)
                    .map_or(1.0, |h| h.distance / 3.0),
            );
        s.objects
            .iter()
            .any(|o| o.health > 0. && o.hit(from, end, 0.).is_some_and(|t| t < wall))
    });
    if (equipped || object) && (button == 0 || button == 1) {
        *canceled = true;
        *swing_hand = false;
        game.vh_send(Action::Fire(button == 1));
    }
}
