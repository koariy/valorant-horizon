// SDK adapter for client-only HUD and native mod screen. All layouts are GUI pixels.
use super::Game;
use valorant_horizon_engine::{
    DEFAULTS, ID, NAMES,
    presentation::ui::{Tab, icon},
    state,
};
impl Game<'_> {
    pub(super) fn vh_ui_draw(&self, batch: &mut crate::hud::HudBatch) {
        let Ok(s) = state().lock() else {
            return;
        };
        let view =
            valorant_horizon_engine::presentation::ui::Viewport::new([batch.width, batch.height]);
        let start = batch.vertices.len();
        let w = (view.size[0] - 16.).clamp(1., 560.);
        let x = (view.size[0] - w) * 0.5;
        let y = 8.;
        self.native_fill(batch, [x, y, w, (view.size[1] - 16.).max(1.)], 0xf20c141d);
        self.native_fill(batch, [x, y, 3., (view.size[1] - 16.).max(1.)], 0xffff4655);
        self.native_text(
            batch,
            "VALORANT HORIZON",
            [x + 12., y + 8.],
            0xf4f7ed,
            false,
        );
        for (i, label) in ["AGENTS", "CONTROLS", "PRESENTATION"].iter().enumerate() {
            let active = matches!(
                (i, s.ui.tab),
                (0, Tab::Agents) | (1, Tab::Controls) | (2, Tab::Presentation)
            );
            let bx = x + 10. + i as f32 * (w - 20.) / 3.;
            self.native_fill(
                batch,
                [bx, y + 24., (w - 26.) / 3., 18.],
                if active { 0xff953744 } else { 0xff202d39 },
            );
            self.native_text(batch, label, [bx + 4., y + 29.], 0xffffff, false);
        }
        match s.ui.tab {
            Tab::Agents => {
                let names: Vec<_> = s.registry.agents().iter().map(|a| a.name).collect();
                let results = s.ui.results(&names);
                let page = s.ui.page.min(results.len().saturating_sub(1) / 10);
                self.native_fill(
                    batch,
                    [x + 10., y + 48., (w - 24.) * 0.65, 19.],
                    if s.ui.search { 0xff394652 } else { 0xff202d39 },
                );
                self.native_text(
                    batch,
                    &format!(
                        "Search: {}{}",
                        s.ui.query,
                        if s.ui.search { "_" } else { "" }
                    ),
                    [x + 14., y + 54.],
                    0xe6e9e4,
                    false,
                );
                self.native_text(
                    batch,
                    s.ui.role.name(),
                    [x + w * 0.69, y + 54.],
                    0xff8c94,
                    false,
                );
                for (row, index) in results.iter().skip(page * 10).take(10).enumerate() {
                    let bx = x + 10. + (row % 2) as f32 * (w - 20.) * 0.5;
                    let by = y + 74. + (row / 2) as f32 * 25.;
                    self.native_fill(
                        batch,
                        [bx, by, (w - 24.) * 0.5, 22.],
                        if *index == s.selected {
                            0xff42665f
                        } else {
                            0xff202d39
                        },
                    );
                    self.native_text(
                        batch,
                        &format!("[{}] {}", (row + 1) % 10, names[*index]),
                        [bx + 5., by + 4.],
                        0xffffff,
                        false,
                    );
                    self.native_fill(batch, [bx + 5., by + 17., 24., 1.], 0xffff4655);
                }
                self.native_text(
                    batch,
                    &format!(
                        "<  {}/{}  >    Tab: category | /: search",
                        page + 1,
                        results.len().saturating_sub(1) / 10 + 1
                    ),
                    [x + 12., y + 204.],
                    0xbac7ce,
                    false,
                );
            }
            Tab::Controls => {
                let lines = s.lines(&std::array::from_fn(|i| {
                    self.native_mod_key(ID, NAMES[i], DEFAULTS[i])
                }));
                for i in 0..4 {
                    let by = y + 55. + i as f32 * 28.;
                    self.native_fill(batch, [x + 10., by, w - 20., 24.], 0xff202d39);
                    let text = format!("[{}] {}", i + 1, lines[i + 1]);
                    let text: String = text
                        .chars()
                        .take(((w - 32.) / 6.).max(1.) as usize)
                        .collect();
                    self.native_text(batch, &text, [x + 15., by + 7.], 0xffffff, false);
                }
                self.native_text(
                    batch,
                    if s.rebind.is_some() {
                        "Press a unique key; Esc cancels"
                    } else {
                        "Click a slot to rebind. B cancel | R utility"
                    },
                    [x + 12., y + 178.],
                    0xffa4aa,
                    false,
                );
            }
            Tab::Presentation => {
                let p = s.presentation;
                let rows = [
                    format!("Quality: {:?}", p.quality),
                    format!("HUD scale: {:.0}%", p.scale * 100.),
                    format!("HUD horizontal: {:.0}%", p.x * 100.),
                    format!("HUD vertical: {:.0}%", p.y * 100.),
                    format!(
                        "Local feedback sounds: {}",
                        if p.sound { "On" } else { "Off" }
                    ),
                ];
                for (i, text) in rows.iter().enumerate() {
                    let by = y + 51. + i as f32 * 27.;
                    self.native_fill(batch, [x + 10., by, w - 20., 23.], 0xff202d39);
                    self.native_text(
                        batch,
                        &format!("[{}] {}", i + 1, text),
                        [x + 15., by + 7.],
                        0xffffff,
                        false,
                    );
                }
                self.native_text(
                    batch,
                    "Click/1-5: change | Saved locally",
                    [x + 12., y + 194.],
                    0xbac7ce,
                    false,
                );
            }
        }
        self.native_text(
            batch,
            "Esc: close (world continues)",
            [x + 12., (view.size[1] - 19.).max(0.)],
            0xbac7ce,
            false,
        );
        batch.scale_since(start, 0., 0., view.scale, view.scale);
    }
    pub(super) fn vh_hud(&self, batch: &mut crate::hud::HudBatch) {
        let Ok(s) = state().lock() else {
            return;
        };
        if s.menu {
            return;
        }
        let names = self.vh_ability_names(s.selected);
        let mut p = s.presentation;
        p.scale = p.scale.min((batch.width / 300.).max(0.1));
        let [x, y] = p.origin([batch.width, batch.height], [294., 104.]);
        let start = batch.vertices.len();
        self.native_fill(batch, [x, y, 294., 104.], 0xb80c141d);
        self.native_fill(batch, [x, y, 294., 2.], 0xffff4655);
        self.native_text(
            batch,
            &format!(
                "{}   [V] AGENTS / SETTINGS",
                s.registry.agents()[s.selected].name
            ),
            [x + 7., y + 7.],
            0xf0f5ed,
            false,
        );
        for (i, name) in names.iter().enumerate() {
            let r = s.runtime[i];
            let bx = x + 6. + i as f32 * 72.;
            let ready = r.charges > 0;
            let color = if r.active > 0 {
                0xff63e1c0
            } else if ready {
                0xffedf4ee
            } else {
                0xff687883
            };
            self.native_fill(batch, [bx, y + 22., 66., 56.], 0xd51b2935);
            let glyph = icon(name, i == 3);
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        self.native_fill(
                            batch,
                            [
                                bx + 4. + col as f32 * 1.5,
                                y + 26. + row as f32 * 1.5,
                                1.5,
                                1.5,
                            ],
                            color,
                        );
                    }
                }
            }
            let binding = self.native_mod_key(ID, NAMES[i], DEFAULTS[i]);
            let key = binding
                .trim_start_matches("key.keyboard.")
                .trim_start_matches("key.mouse.");
            self.native_text(
                batch,
                &format!("{}  x{}", key, r.charges),
                [bx + 3., y + 43.],
                color & 0xffffff,
                false,
            );
            let short: String = name.chars().take(10).collect();
            self.native_text(batch, &short, [bx + 3., y + 55.], 0xf2f4eb, false);
            let ability = s.registry.agents()[s.selected].slots[i];
            let progress = if r.remaining > 0 {
                1. - r.remaining as f32 / ability.map_or(1, |a| a.cooldown.max(1)) as f32
            } else if ready {
                1.
            } else {
                0.
            };
            self.native_fill(
                batch,
                [bx + 3., y + 73., 60. * progress.clamp(0., 1.), 2.],
                color,
            );
            if r.remaining > 0 {
                self.native_text(
                    batch,
                    &format!("{:.1}s", r.remaining as f32 / 20.),
                    [bx + 25., y + 28.],
                    0xff9ba4,
                    false,
                );
            }
        }
        let bindings = std::array::from_fn(|i| self.native_mod_key(ID, NAMES[i], DEFAULTS[i]));
        let lines = s.lines(&bindings);
        for (i, line) in lines.iter().skip(5).take(2).enumerate() {
            let short: String = line.chars().take(46).collect();
            self.native_text(
                batch,
                &short,
                [x + 7., y + 82. + i as f32 * 10.],
                0xc5d5dc,
                false,
            );
        }
        batch.scale_since(start, x, y, p.scale, p.scale);
    }
    fn vh_ability_names(&self, index: usize) -> [&'static str; 4] {
        use valorant_horizon_engine::{advanced, agents::*};
        advanced::names(index).unwrap_or_else(|| match index {
            0 => jett::NAMES,
            1 => sage::NAMES,
            2 => phoenix::NAMES,
            3 => omen::NAMES,
            4 => raze::NAMES,
            5 => reyna::NAMES,
            6 => brimstone::NAMES,
            7 => viper::NAMES,
            8 => astra::NAMES,
            9 => harbor::NAMES,
            10 => clove::NAMES,
            11 => sova::NAMES,
            _ => ["?"; 4],
        })
    }
}
