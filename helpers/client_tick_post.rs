fn valorant_client_tick(game: &mut crate::game::Game<'_>) {
    use valorant_horizon_engine::{ID, gameplay::Action, state};
    if !game.native_window() || game.native_player().is_none() {
        return;
    }
    let pos = game.native_player().map(|p| [p.x, p.y, p.z]);
    let (save, binding, select, close, cues) = {
        let Ok(mut s) = state().lock() else {
            return;
        };
        if !s.presentation_loaded {
            s.presentation = valorant_horizon_engine::presentation::Settings::decode(
                &game.native_setting(ID, "presentation").unwrap_or_default(),
            );
            s.presentation_loaded = true;
        }
        if s.menu && game.native_mod_screen() != Some(ID) {
            s.menu = false;
            s.rebind = None;
            s.held.clear();
        }
        if let Some(snapshot) = s.snapshot.clone() {
            let q = s.presentation.quality;
            s.scene.tick(&snapshot, pos, q);
        }
        let save = s.ui.dirty.then(|| s.presentation.encode());
        s.ui.dirty = false;
        let select = s.ui.select.take().filter(|i| s.select(*i));
        let close = std::mem::take(&mut s.ui.close) || select.is_some();
        if close {
            s.menu = false;
            s.rebind = None;
            s.held.clear();
        }
        let cues = if s.presentation.sound {
            s.scene
                .cues
                .iter()
                .take(2)
                .map(|c| c.sound(s.selected))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        (save, s.ui.binding.take(), select, close, cues)
    };
    if let Some(at) = pos {
        for (sound, volume, pitch) in cues {
            game.native_play_local_sound(sound, at, volume, pitch, 0);
        }
    }
    if let Some(text) = save {
        game.native_set_setting(ID, "presentation", &text);
    }
    if let Some((slot, key)) = binding {
        game.native_set_mod_key(ID, valorant_horizon_engine::NAMES[slot], Some(&key));
    }
    if let Some(index) = select {
        let (id, sound) = state()
            .lock()
            .ok()
            .map(|s| (s.registry.agents()[index].id, s.presentation.sound))
            .unwrap_or(("jett", false));
        game.native_set_setting(ID, "agent", id);
        game.vh_send(Action::Select(index));
        if sound {
            if let Some(at) = pos {
                game.native_play_local_sound("minecraft:block.note_block.pling", at, 0.25, 1.2, 0);
            }
        }
    }
    if close {
        game.native_mod_screen_close(ID);
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
