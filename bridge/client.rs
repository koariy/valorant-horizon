use super::{Game, native_world_draw::NativeWorldDraw};
use valorant_horizon_engine::{
    gameplay::{Action, Request, Snapshot, geometry::sub, smoke::sphere},
    server::CHANNEL,
    state,
};
impl Game<'_> {
    pub(super) fn vh_send(&mut self, action: Action) {
        if !self.native_window() {
            return;
        }
        let data = {
            let Ok(mut s) = state().lock() else {
                return;
            };
            s.sequence = s.sequence.saturating_add(1);
            if matches!(&action,Action::Activate(slot) if (s.selected==0 && *slot==3)||(((s.selected==1||s.selected==2) && *slot<3)||(s.selected==3 && *slot!=1)||(s.selected==4 && *slot!=1)||(s.selected==5 && *slot==0)||(s.selected==6)||(s.selected==7)||(s.selected==8 && *slot==3)||(s.selected==9)||(s.selected==10 && (*slot==1||*slot==2))||(s.selected==11)))
            {
                s.pending_equipped = Some(s.sequence);
            }
            serde_json::to_vec(&Request {
                sequence: s.sequence,
                expected_agent: s.selected,
                action,
            })
        };
        if let Ok(data) = data
            && let Err(error) = self.native_net_send_to_server(CHANNEL, &data)
            && let Ok(mut s) = state().lock()
        {
            s.diagnostic = Some(error);
        }
    }
    pub(super) fn vh_snapshot(&mut self, channel: &str, data: &[u8]) {
        if channel != CHANNEL || data.len() > 30_000 {
            return;
        }
        if let Ok(snapshot) = serde_json::from_slice::<Snapshot>(data)
            && let Ok(mut s) = state().lock()
        {
            if snapshot.agent >= s.registry.agents().len() {
                return;
            }
            s.selected = snapshot.agent;
            if s.pending_equipped
                .is_some_and(|seq| snapshot.sequence >= seq)
            {
                s.pending_equipped = None;
            }
            for i in 0..4 {
                s.runtime[i].charges = snapshot.charges[i];
                s.runtime[i].remaining = snapshot.cooldowns[i];
                s.runtime[i].active = snapshot.active[i];
            }
            s.diagnostic = snapshot.error.clone();
            s.snapshot = Some(snapshot);
        }
    }
    pub(super) fn vh_world(&self, draw: &mut NativeWorldDraw) {
        let snapshot = state().lock().ok().and_then(|s| s.snapshot.clone());
        let Some(snapshot) = snapshot else {
            return;
        };
        if let Ok(mut d) = horizon_client_render::valorant_spatial::audio_planes().lock() {
            *d = snapshot
                .divides
                .iter()
                .map(|d| horizon_client_render::valorant_spatial::Divide {
                    point: d.point,
                    normal: d.normal,
                    ticks: d.ticks,
                })
                .collect();
        }
        if let Ok(mut p) = horizon_client_render::valorant_spatial::listener().lock() {
            *p = draw.camera;
        }
        for shield in &snapshot.shields {
            for q in sphere(sub(shield.center, draw.camera), shield.radius) {
                self.native_world_quad(
                    draw,
                    "horizon/white",
                    q,
                    [0., 0., 16., 16.],
                    if shield.active {
                        [50, 160, 220, 255]
                    } else {
                        [45, 155, 200, 255]
                    },
                );
            }
        }
        for star in &snapshot.stars {
            for quad in sphere(sub(star.position, draw.camera), 0.22) {
                self.native_world_quad(
                    draw,
                    "horizon/white",
                    quad,
                    [0., 0., 16., 16.],
                    [235, 175, 65, 255],
                );
            }
        }
        for d in &snapshot.divides {
            use valorant_horizon_engine::gameplay::geometry::{add, dot, scale};
            let base = sub(
                draw.camera,
                scale(d.normal, dot(sub(draw.camera, d.point), d.normal)),
            );
            let tangent = scale([-d.normal[2], 0., d.normal[0]], 512.);
            let points = [
                add(sub(base, tangent), [0., -512., 0.]),
                add(sub(base, tangent), [0., 512., 0.]),
                add(add(base, tangent), [0., 512., 0.]),
                add(add(base, tangent), [0., -512., 0.]),
            ]
            .map(|p| sub(p, draw.camera).map(|v| v as f32));
            self.native_world_quad(
                draw,
                "horizon/white",
                points,
                [0., 0., 16., 16.],
                [95, 35, 150, 255],
            );
            self.native_world_quad(
                draw,
                "horizon/white",
                [points[3], points[2], points[1], points[0]],
                [0., 0., 16., 16.],
                [95, 35, 150, 255],
            );
        }
        for zone in &snapshot.zones {
            let mut visual_bounds = zone.bounds;
            if matches!(
                zone.kind,
                valorant_horizon_engine::gameplay::zones::ZoneKind::Whirlpool
            ) {
                visual_bounds.max[1] = visual_bounds.min[1] + 0.2;
            }
            for quad in
                valorant_horizon_engine::gameplay::zones::box_quads(visual_bounds, draw.camera)
            {
                self.native_world_quad(
                    draw,
                    "horizon/white",
                    quad,
                    [0.0, 0.0, 16.0, 16.0],
                    if matches!(
                        zone.kind,
                        valorant_horizon_engine::gameplay::zones::ZoneKind::WaterWall
                    ) {
                        [35, 165, 220, 255]
                    } else if matches!(
                        zone.kind,
                        valorant_horizon_engine::gameplay::zones::ZoneKind::Whirlpool
                    ) {
                        [35, 165, 220, 150]
                    } else if matches!(
                        zone.kind,
                        valorant_horizon_engine::gameplay::zones::ZoneKind::Gravity
                            | valorant_horizon_engine::gameplay::zones::ZoneKind::Nova
                    ) {
                        [120, 75, 210, 160]
                    } else if matches!(
                        zone.kind,
                        valorant_horizon_engine::gameplay::zones::ZoneKind::Stim
                            | valorant_horizon_engine::gameplay::zones::ZoneKind::Acid
                    ) {
                        [90, 210, 160, 140]
                    } else if matches!(
                        zone.kind,
                        valorant_horizon_engine::gameplay::zones::ZoneKind::Orbital
                    ) {
                        [255, 60, 15, 190]
                    } else if matches!(
                        zone.kind,
                        valorant_horizon_engine::gameplay::zones::ZoneKind::Slow
                    ) {
                        [105, 205, 240, 180]
                    } else {
                        [255, 105, 25, 255]
                    },
                );
            }
        }
        if let Some(preview) = &snapshot.preview {
            for quad in sphere(sub(preview.center, draw.camera), preview.radius) {
                self.native_world_quad(
                    draw,
                    "horizon/white",
                    quad,
                    [0.0, 0.0, 16.0, 16.0],
                    preview.color,
                );
            }
        }
        for object in &snapshot.objects {
            for quad in
                valorant_horizon_engine::gameplay::zones::box_quads(object.bounds(), draw.camera)
            {
                self.native_world_quad(
                    draw,
                    "horizon/white",
                    quad,
                    [0.0, 0.0, 16.0, 16.0],
                    match object.kind {
                        valorant_horizon_engine::gameplay::objects::ObjectKind::Drone
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::ReconBolt => {
                            [60, 170, 240, 255]
                        }
                        valorant_horizon_engine::gameplay::objects::ObjectKind::Leer => {
                            [180, 45, 210, 255]
                        }
                        valorant_horizon_engine::gameplay::objects::ObjectKind::Soul => {
                            [180, 60, 220, 180]
                        }
                        valorant_horizon_engine::gameplay::objects::ObjectKind::Shade => {
                            [70, 45, 100, 220]
                        }
                        valorant_horizon_engine::gameplay::objects::ObjectKind::BoomBot => {
                            [240, 125, 30, 255]
                        }
                        valorant_horizon_engine::gameplay::objects::ObjectKind::BlastPack => {
                            [80, 200, 90, 255]
                        }
                    },
                );
            }
        }
        for o in &snapshot.objects {
            use valorant_horizon_engine::gameplay::{
                geometry::{add, scale},
                objects::ObjectKind as K,
            };
            if matches!(o.kind, K::Drone) {
                let angle = draw.game_time as f64 * 0.8;
                let dir = [angle.sin(), 0., angle.cos()];
                for side in [-0.3, 0.3] {
                    let at = sub(add(o.position, [side, 0.22, 0.]), draw.camera);
                    for q in valorant_horizon_engine::gameplay::projectiles::beam(
                        add(at, scale(dir, -0.2)),
                        add(at, scale(dir, 0.2)),
                        0.03,
                    ) {
                        self.native_world_quad(
                            draw,
                            "horizon/white",
                            q,
                            [0., 0., 16., 16.],
                            [180, 220, 240, 255],
                        );
                    }
                }
            }
            if matches!(o.kind, K::ReconBolt) {
                let phase = f64::from(60u32.saturating_sub(o.ticks) % 20) / 20.;
                for q in sphere(
                    sub(add(o.position, [0., 0.1, 0.]), draw.camera),
                    0.2 + phase * 4.,
                ) {
                    self.native_world_quad(
                        draw,
                        "horizon/white",
                        q,
                        [0., 0., 16., 16.],
                        [50, 160, 255, (80. * (1. - phase)) as u8],
                    );
                }
            }
        }
        for f in &snapshot.toxins {
            let quads = if f.sphere {
                sphere(
                    sub(
                        valorant_horizon_engine::gameplay::geometry::scale(
                            valorant_horizon_engine::gameplay::geometry::add(
                                f.bounds.min,
                                f.bounds.max,
                            ),
                            0.5,
                        ),
                        draw.camera,
                    ),
                    (f.bounds.max[0] - f.bounds.min[0]) * 0.5,
                )
            } else {
                valorant_horizon_engine::gameplay::zones::box_quads(f.bounds, draw.camera)
            };
            for q in quads {
                self.native_world_quad(
                    draw,
                    "horizon/white",
                    q,
                    [0., 0., 16., 16.],
                    [75, 180, 20, 255],
                );
            }
        }
        for smoke in &snapshot.smokes {
            // Subtract f64 camera first: preserve precision near the world border.
            for quad in sphere(sub(smoke.center, draw.camera), smoke.radius) {
                self.native_world_quad(
                    draw,
                    "horizon/white",
                    quad,
                    [0.0, 0.0, 16.0, 16.0],
                    smoke.color,
                );
            }
        }
        if snapshot.agent == 0
            && snapshot.active[3] > 0
            && let Some(player) = self.native_player()
        {
            for i in 0..snapshot.knives {
                let angle =
                    f64::from(i) * std::f64::consts::TAU / 5.0 + draw.game_time as f64 * 0.03;
                let center = sub(
                    [
                        player.x + angle.cos() * 0.7,
                        player.y + 1.3,
                        player.z + angle.sin() * 0.7,
                    ],
                    draw.camera,
                );
                for quad in
                    valorant_horizon_engine::gameplay::projectiles::blade(center, [0.0, 1.0, 0.0])
                {
                    self.native_world_quad(
                        draw,
                        "horizon/white",
                        quad,
                        [0.0, 0.0, 16.0, 16.0],
                        [215, 250, 255, 255],
                    );
                }
            }
        }
        for b in &snapshot.beams {
            for q in valorant_horizon_engine::gameplay::projectiles::beam(
                sub(b.from, draw.camera),
                sub(b.to, draw.camera),
                0.35,
            ) {
                self.native_world_quad(
                    draw,
                    "horizon/white",
                    q,
                    [0., 0., 16., 16.],
                    [75, 170, 255, 210],
                );
            }
        }
        for projectile in &snapshot.projectiles {
            use valorant_horizon_engine::gameplay::projectiles::ProjectileKind as P;
            let color = match projectile.kind {
                P::Incendiary { .. } => [255, 140, 15, 255],
                P::SnakeBite | P::PoisonEmitter => [95, 225, 30, 255],
                P::SovaBolt { .. } | P::ReconDart => [80, 195, 255, 255],
                P::Meddle => [180, 100, 220, 255],
                P::StormSurge | P::Reckoning => [35, 175, 230, 230],
                P::Cloudburst => [180, 220, 240, 255],
                P::Knife { .. } => [215, 250, 255, 255],
                P::SlowOrb => [105, 205, 240, 255],
                P::HotHands => [255, 95, 15, 255],
                P::Curveball { .. } => [255, 190, 45, 255],
                P::DarkCover { .. } | P::Paranoia => [85, 55, 135, 255],
                P::BlastPack => [70, 210, 100, 255],
                P::Grenade { .. } => [190, 80, 220, 255],
                P::Rocket => [250, 125, 35, 255],
            };
            let center = sub(projectile.position, draw.camera);
            let quads = if matches!(projectile.kind, P::Reckoning) {
                use valorant_horizon_engine::gameplay::geometry::{Box3, add};
                valorant_horizon_engine::gameplay::zones::box_quads(
                    Box3 {
                        min: add(projectile.position, [-2.5, 0., -2.5]),
                        max: add(projectile.position, [2.5, 3., 2.5]),
                    },
                    draw.camera,
                )
            } else if matches!(
                projectile.kind,
                P::Knife { .. } | P::SovaBolt { .. } | P::ReconDart
            ) {
                valorant_horizon_engine::gameplay::projectiles::blade(center, projectile.velocity)
            } else {
                sphere(center, 0.15)
            };
            for quad in quads {
                self.native_world_quad(draw, "horizon/white", quad, [0.0, 0.0, 16.0, 16.0], color);
            }
        }
    }
    pub(super) fn vh_fog(&self, camera: [f64; 3], fog: &mut horizon_client::fog::FogData) {
        let vision = state()
            .lock()
            .ok()
            .and_then(|s| s.snapshot.as_ref().map(|s| s.vision));
        if vision.is_some_and(|v| v.flash > 0) {
            fog.color = [1.0, 0.97, 0.85, 1.0];
            fog.environmental_start = 0.0;
            fog.environmental_end = 0.01;
            fog.render_distance_start = 0.0;
            fog.render_distance_end = 0.01;
            fog.sky_end = 0.01;
            fog.cloud_end = 0.01;
            return;
        }
        if vision.is_some_and(|v| v.nearsight > 0) {
            fog.color = [0.025, 0.015, 0.05, 1.0];
            fog.environmental_start = 0.2;
            fog.environmental_end = 4.0;
            fog.render_distance_start = 0.2;
            fog.render_distance_end = 4.0;
            fog.sky_end = 4.0;
            fog.cloud_end = 4.0;
        }
        if state().lock().ok().is_some_and(|s| {
            s.snapshot
                .as_ref()
                .is_some_and(|s| s.toxins.iter().any(|f| f.contains(camera)))
        }) {
            fog.color = [0.12, 0.3, 0.015, 1.];
            fog.environmental_start = 0.;
            fog.environmental_end = 1.;
            fog.render_distance_start = 0.;
            fog.render_distance_end = 1.;
            fog.sky_end = 1.;
            fog.cloud_end = 1.;
            return;
        }
        let water = state().lock().ok().is_some_and(|s| {
            s.snapshot.as_ref().is_some_and(|s| {
                let sphere = s.shields.iter().any(|v| {
                    valorant_horizon_engine::gameplay::geometry::distance(camera, v.center)
                        < v.radius
                });
                let wall = s.zones.iter().any(|z| {
                    matches!(
                        z.kind,
                        valorant_horizon_engine::gameplay::zones::ZoneKind::WaterWall
                    ) && (0..3)
                        .all(|i| camera[i] >= z.bounds.min[i] && camera[i] <= z.bounds.max[i])
                });
                sphere || wall
            })
        });
        if water {
            fog.color = [0.035, 0.3, 0.4, 1.];
            fog.environmental_start = 0.;
            fog.environmental_end = 0.5;
            fog.render_distance_start = 0.;
            fog.render_distance_end = 0.5;
            fog.sky_end = 0.5;
            fog.cloud_end = 0.5;
            return;
        }
        let smoke = state().lock().ok().and_then(|s| {
            s.snapshot.as_ref().and_then(|s| {
                s.smokes
                    .iter()
                    .find(|smoke| smoke.contains(camera))
                    .cloned()
            })
        });
        if let Some(smoke) = smoke {
            fog.color = [
                f32::from(smoke.color[0]) / 255.0,
                f32::from(smoke.color[1]) / 255.0,
                f32::from(smoke.color[2]) / 255.0,
                1.0,
            ];
            fog.environmental_start = 0.0;
            fog.environmental_end = 0.35;
            fog.render_distance_start = 0.0;
            fog.render_distance_end = 0.35;
            fog.sky_end = 0.35;
            fog.cloud_end = 0.35;
        }
    }
}
