use super::Game;
use valorant_horizon_engine::{DEFAULTS, ID, NAMES, presentation::ui::Tab, state};
impl Game<'_> {
    pub(super) fn vh_ui_key(&self, key: &str) {
        let Ok(mut s) = state().lock() else {
            return;
        };
        if let Some(slot) = s.rebind.take() {
            if key != "key.keyboard.escape"
                && ![
                    "key.keyboard.v",
                    "key.keyboard.b",
                    "key.keyboard.r",
                    "key.keyboard.f1",
                ]
                .contains(&key)
                && !NAMES
                    .iter()
                    .enumerate()
                    .any(|(i, n)| i != slot && self.native_mod_key(ID, n, DEFAULTS[i]) == key)
            {
                s.ui.binding = Some((slot, key.into()));
            }
            return;
        }
        match key {
            "key.keyboard.escape" => {
                if s.ui.search {
                    s.ui.search = false;
                } else {
                    s.ui.close = true;
                }
                return;
            }
            "key.keyboard.f2" => {
                s.ui.tab = Tab::Controls;
                s.ui.search = false;
                return;
            }
            "key.keyboard.f3" => {
                s.ui.tab = Tab::Presentation;
                s.ui.search = false;
                return;
            }
            "key.keyboard.f4" => {
                s.ui.tab = Tab::Agents;
                return;
            }
            _ => {}
        }
        match s.ui.tab {
            Tab::Agents => {
                if key == "key.keyboard.slash" {
                    s.ui.search = true;
                    return;
                }
                if s.ui.search {
                    if key == "key.keyboard.backspace" {
                        s.ui.query.pop();
                        s.ui.page = 0;
                    }
                    if key == "key.keyboard.enter" {
                        s.ui.search = false;
                    }
                    return;
                }
                let names: Vec<_> = s.registry.agents().iter().map(|a| a.name).collect();
                let results = s.ui.results(&names);
                match key {
                    "key.keyboard.tab" => {
                        s.ui.role = s.ui.role.next();
                        s.ui.page = 0;
                    }
                    "key.keyboard.left" => s.ui.page = s.ui.page.saturating_sub(1),
                    "key.keyboard.right" => {
                        s.ui.page = (s.ui.page + 1).min(results.len().saturating_sub(1) / 10)
                    }
                    _ => {
                        if let Some(n) = key
                            .strip_prefix("key.keyboard.")
                            .and_then(|k| k.parse::<usize>().ok())
                            .filter(|n| *n < 10)
                        {
                            let row = if n == 0 { 9 } else { n - 1 };
                            s.ui.select = results.get(s.ui.page * 10 + row).copied();
                        }
                    }
                }
            }
            Tab::Controls => {
                if let Some(n) = key
                    .strip_prefix("key.keyboard.")
                    .and_then(|k| k.parse::<usize>().ok())
                    .filter(|n| (1..=4).contains(n))
                {
                    s.rebind = Some(n - 1);
                }
            }
            Tab::Presentation => {
                if let Some(n) = key
                    .strip_prefix("key.keyboard.")
                    .and_then(|k| k.parse::<usize>().ok())
                    .filter(|n| (1..=5).contains(n))
                {
                    let mut p = s.presentation;
                    s.ui.adjust(n - 1, &mut p);
                    s.presentation = p;
                }
            }
        }
    }
    pub(super) fn vh_ui_click(&self, mouse: [f32; 2], size: [f32; 2]) {
        let Ok(mut s) = state().lock() else {
            return;
        };
        let view = valorant_horizon_engine::presentation::ui::Viewport::new(size);
        let mouse = view.point(mouse);
        let size = view.size;
        let w = (size[0] - 16.).clamp(1., 560.);
        let x = (size[0] - w) * 0.5;
        let y = 8.;
        let [mx, my] = mouse;
        if mx < x + 10. || mx > x + w - 10. {
            return;
        }
        if (y + 24.0..y + 42.).contains(&my) {
            let tab = ((mx - x - 10.) / ((w - 20.) / 3.)) as usize;
            s.ui.tab = match tab {
                0 => Tab::Agents,
                1 => Tab::Controls,
                _ => Tab::Presentation,
            };
            s.ui.search = false;
            s.rebind = None;
            return;
        }
        match s.ui.tab {
            Tab::Agents => {
                if (y + 48.0..y + 67.).contains(&my) {
                    if mx < x + w * 0.68 {
                        s.ui.search = true;
                    } else {
                        s.ui.role = s.ui.role.next();
                        s.ui.page = 0;
                        s.ui.search = false;
                    }
                    return;
                }
                let names: Vec<_> = s.registry.agents().iter().map(|a| a.name).collect();
                let results = s.ui.results(&names);
                let page = s.ui.page.min(results.len().saturating_sub(1) / 10);
                if (y + 74.0..y + 199.).contains(&my) {
                    let row = ((my - y - 74.) / 25.) as usize;
                    let col = usize::from(mx > x + w * 0.5);
                    s.ui.select = results.get(page * 10 + row * 2 + col).copied();
                }
                if (y + 201.0..y + 220.).contains(&my) {
                    if mx < x + w * 0.5 {
                        s.ui.page = s.ui.page.saturating_sub(1);
                    } else {
                        s.ui.page = (page + 1).min(results.len().saturating_sub(1) / 10);
                    }
                }
            }
            Tab::Controls => {
                if (y + 55.0..y + 167.).contains(&my) {
                    s.rebind = Some(((my - y - 55.) / 28.) as usize);
                }
            }
            Tab::Presentation => {
                if (y + 51.0..y + 186.).contains(&my) {
                    let row = ((my - y - 51.) / 27.) as usize;
                    let mut p = s.presentation;
                    s.ui.adjust(row, &mut p);
                    s.presentation = p;
                }
            }
        }
    }
}
