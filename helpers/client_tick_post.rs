fn valorant_client_tick(game: &mut crate::game::Game<'_>) {
    use valorant_horizon_engine::{ID, gameplay::Action, state};
    if !game.native_window() || game.native_player().is_none() {
        return;
    }
    let selection = {
        let Ok(mut s) = state().lock() else {
            return;
        };
        if !s.loaded {
            s.restore(&game.native_setting(ID, "agent").unwrap_or_default());
        }
        s.heartbeat = (s.heartbeat + 1) % 40;
        (s.heartbeat == 1).then_some(s.selected)
    };
    if let Some(index) = selection {
        game.vh_send(Action::Select(index));
    }
    let stabilize = game.native_playing()
        && state().lock().ok().is_some_and(|s| {
            !s.menu
                && s.heartbeat % 5 == 0
                && s.held.contains("key.keyboard.r")
                && s.snapshot.as_ref().is_some_and(|x| x.stabilize)
        });
    if stabilize {
        game.vh_send(Action::Stabilize(true));
    }
    let reclaim = game.native_playing()
        && state().lock().ok().is_some_and(|s| {
            !s.menu
                && s.selected == 21
                && s.heartbeat % 5 == 0
                && s.held.contains("key.keyboard.r")
                && s.snapshot
                    .as_ref()
                    .is_some_and(|x| x.kit[0] > 0 && !x.stabilize)
        });
    if reclaim {
        game.vh_send(Action::Recall);
    }
    let aiming = valorant_horizon_engine::state()
        .lock()
        .ok()
        .and_then(|s| s.snapshot.clone())
        .is_some_and(|s| {
            (s.agent == 3 || s.agent == 9)
                && s.equipped
                && s.preview.is_some_and(|p| p.radius > 1.0)
        });
    if game.native_playing() && aiming {
        if game.native_mapping("key.attack").is_some_and(|m| m.0) {
            game.vh_send(Action::Fire(false));
        } else if game.native_mapping("key.use").is_some_and(|m| m.0) {
            game.vh_send(Action::Fire(true));
        }
    }
}
