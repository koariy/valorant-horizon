fn valorant_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    if !game.native_window() {
        return;
    }
    use valorant_horizon_engine::{DEFAULTS, ID, NAMES, gameplay::Action, state};
    let first = state()
        .lock()
        .ok()
        .is_some_and(|mut s| s.key_transition(key, pressed));
    if pressed && !first {
        return;
    }
    let bindings: [String; 4] =
        std::array::from_fn(|i| game.native_mod_key(ID, NAMES[i], DEFAULTS[i]));
    if !pressed {
        if let Ok(mut s) = state().lock() {
            if key == "key.mouse.left" {
                s.key_transition("ability.attack", false);
            }
            if key == "key.mouse.right" {
                s.key_transition("ability.use", false);
            }
        }
        if key == "key.mouse.left" {
            game.vh_send(Action::Release(4));
        }
        if let Some(slot) = bindings.iter().position(|k| k == key) {
            game.vh_send(Action::Release(slot));
        }
        return;
    }
    let after_death = game.screen == crate::game::Screen::Dead
        && state().lock().ok().is_some_and(|s| {
            s.snapshot
                .as_ref()
                .is_some_and(|x| x.agent == 10 && x.after_death)
        });
    if after_death {
        let action = if key == bindings[2] {
            Some(Action::Activate(2))
        } else {
            match key {
                "key.keyboard.left" => Some(Action::Aim(-1, 0)),
                "key.keyboard.right" => Some(Action::Aim(1, 0)),
                "key.keyboard.up" => Some(Action::Aim(0, -1)),
                "key.keyboard.down" => Some(Action::Aim(0, 1)),
                "key.keyboard.z" => Some(Action::Fire(false)),
                "key.keyboard.r" => Some(Action::Fire(true)),
                "key.keyboard.b" => Some(Action::Cancel),
                _ => None,
            }
        };
        if let Some(action) = action {
            game.vh_send(action);
        }
        return;
    }
    if !game.native_playing() {
        return;
    }

    let mut action = None;
    let mut save = None;
    {
        let Ok(mut s) = state().lock() else {
            return;
        };
        if !s.loaded {
            s.restore(&game.native_setting(ID, "agent").unwrap_or_default());
        }
        s.consumed = None;
        if let Some(slot) = s.rebind.take() {
            s.consumed = Some(key.to_owned());
            if ![
                "key.keyboard.escape",
                "key.keyboard.v",
                "key.keyboard.b",
                "key.keyboard.r",
                "key.keyboard.f1",
            ]
            .contains(&key)
                && !bindings
                    .iter()
                    .enumerate()
                    .any(|(i, k)| i != slot && k == key)
            {
                game.native_set_mod_key(ID, NAMES[slot], Some(key));
            }
            return;
        }
        if key == "key.keyboard.v" {
            s.menu = !s.menu;
            s.consumed = Some(key.into());
            return;
        }
        if s.menu {
            s.consumed = Some(key.into());
            if let Some(i) = DEFAULTS.iter().position(|k| *k == key) {
                s.rebind = Some(i);
                return;
            }
            if key == "key.keyboard.equal" && s.select(11) {
                action = Some(Action::Select(11));
                save = Some("sova");
            }
            if key == "key.keyboard.minus" && s.select(10) {
                action = Some(Action::Select(10));
                save = Some("clove");
            }
            if let Some(i) = key
                .strip_prefix("key.keyboard.")
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|i| if i == 0 { Some(9) } else { i.checked_sub(1) })
                && s.select(i)
            {
                action = Some(Action::Select(i));
                save = Some(s.registry.agents()[i].id);
            }
        } else if key == "key.keyboard.r" && s.selected == 8 {
            s.consumed = Some(key.into());
            action = Some(Action::Recall);
        } else if key == "key.keyboard.b" {
            s.consumed = Some(key.into());
            action = Some(Action::Cancel);
        } else if let Some(i) = bindings.iter().position(|k| k == key) {
            s.consumed = Some(key.into());
            action = Some(Action::Activate(i));
        }
    }
    if let Some(id) = save {
        game.native_set_setting(ID, "agent", id);
    }
    if let Some(action) = action {
        game.vh_send(action);
    }
}
