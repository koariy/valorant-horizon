use super::{Game, native_world_draw::NativeWorldDraw};
use valorant_horizon_engine::{
    gameplay::{Action, Request, Snapshot, geometry::sub},
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
            if matches!(&action,Action::Activate(slot) if (s.selected==0 && *slot==3)||(((s.selected==1||s.selected==2) && *slot<3)||(s.selected==3 && *slot!=1)||(s.selected==4 && *slot!=1)||(s.selected==5 && *slot==0)||(s.selected==6)||(s.selected==7)||(s.selected==8 && *slot==3)||(s.selected==9)||(s.selected==10 && (*slot==1||*slot==2))||(s.selected>=11)))
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
            horizon_client_render::valorant_spatial::set_deafened(snapshot.vision.deafen > 0);
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
        let (snapshot, quality, mesh) = state()
            .lock()
            .ok()
            .map(|mut s| (s.snapshot.clone(), s.presentation.quality, s.mesh.take()))
            .unwrap_or_default();
        let Some(snapshot) = snapshot else {
            return;
        };
        let sphere = |c, r| {
            valorant_horizon_engine::presentation::mesh::sphere_mesh(
                c,
                r,
                if r < 0.8 {
                    valorant_horizon_engine::presentation::Quality::Low
                } else {
                    quality
                },
            )
        };
        let mut frame = mesh
            .unwrap_or_else(|| valorant_horizon_engine::presentation::mesh::Frame::new(quality));
        frame.reset(quality);
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
        for smoke in &snapshot.smokes {
            frame.smoke(
                smoke,
                draw.camera,
                draw.game_time as f64 + f64::from(draw.partial),
            );
        }
        for shield in &snapshot.shields {
            for q in sphere(sub(shield.center, draw.camera), shield.radius) {
                frame.push(
                    q,
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
                frame.push(quad, [235, 175, 65, 255]);
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
            frame.push(points, [95, 35, 150, 255]);
            frame.push(
                [points[3], points[2], points[1], points[0]],
                [95, 35, 150, 255],
            );
        }
        for zone in &snapshot.zones {
            frame.surface(
                zone,
                draw.camera,
                draw.game_time as f64 + f64::from(draw.partial),
            );
        }
        if let Some(preview) = &snapshot.preview {
            for quad in sphere(sub(preview.center, draw.camera), preview.radius) {
                frame.push(quad, preview.color);
            }
        }
        for object in &snapshot.objects {
            use valorant_horizon_engine::gameplay::{
                geometry::{add, scale, unit},
                objects::ObjectKind as K,
                projectiles::beam,
                smoke::orb,
            };
            if matches!(
                object.kind,
                K::Wingman
                    | K::Dizzy
                    | K::Thrash
                    | K::Plasma
                    | K::Mosh
                    | K::MoshPatch
                    | K::WingmanGlobule
                    | K::DizzyGlobule
                    | K::ThrashGlobule
            ) {
                let color = match object.kind {
                    K::Wingman | K::WingmanGlobule => [245, 205, 80, 230],
                    K::Dizzy | K::DizzyGlobule | K::Plasma => [85, 135, 250, 230],
                    K::Thrash | K::ThrashGlobule => [225, 110, 205, 230],
                    _ => [95, 220, 80, 220],
                };
                let center = sub(add(object.position, [0., 0.2, 0.]), draw.camera);
                if matches!(object.kind, K::MoshPatch) {
                    let center = sub(add(object.position, [0., 0.03, 0.]), draw.camera);
                    for n in 0..16 {
                        let point = |i: u32| {
                            let angle = f64::from(i) * std::f64::consts::TAU / 16.;
                            add(center, [angle.cos() * 3.5, 0., angle.sin() * 3.5])
                                .map(|v| v as f32)
                        };
                        let q = [
                            center.map(|v| v as f32),
                            point(n),
                            point(n + 1),
                            center.map(|v| v as f32),
                        ];
                        for q in [q, [q[3], q[2], q[1], q[0]]] {
                            frame.push(q, [95, 220, 80, if object.ticks < 10 { 160 } else { 90 }]);
                        }
                    }
                } else if matches!(object.kind, K::Plasma) {
                    for q in beam(
                        center,
                        add(center, scale(unit(object.velocity), -0.3)),
                        0.06,
                    ) {
                        frame.push(q, color);
                    }
                } else if matches!(object.kind, K::Wingman) {
                    let b = valorant_horizon_engine::gameplay::geometry::Box3 {
                        min: add(object.position, [-0.2, 0., -0.2]),
                        max: add(object.position, [0.2, 0.5, 0.2]),
                    };
                    for q in valorant_horizon_engine::gameplay::zones::box_quads(b, draw.camera) {
                        frame.push(q, color);
                    }
                } else {
                    for q in orb(
                        center,
                        if matches!(object.kind, K::Dizzy | K::Thrash) {
                            0.32
                        } else {
                            0.16
                        },
                    ) {
                        frame.push(q, color);
                    }
                    if matches!(object.kind, K::Thrash) {
                        for q in valorant_horizon_engine::gameplay::projectiles::blade(
                            center,
                            object.velocity,
                        ) {
                            frame.push(q, color);
                        }
                    }
                }
                continue;
            }
            if matches!(object.kind, K::SkyeTiger | K::SkyeHawk | K::SkyeSeeker) {
                if matches!(object.kind, K::SkyeTiger) {
                    let dir =
                        valorant_horizon_engine::gameplay::geometry::horizontal(object.velocity);
                    let dir = if dir == [0.; 3] { [0., 0., 1.] } else { dir };
                    let right = [dir[2], 0., -dir[0]];
                    for (lo, hi) in [
                        ([-0.24, 0., -0.3], [0.24, 0.32, 0.3]),
                        ([-0.2, 0.2, 0.15], [0.2, 0.5, 0.4]),
                    ] {
                        let b =
                            valorant_horizon_engine::gameplay::geometry::Box3 { min: lo, max: hi };
                        for q in valorant_horizon_engine::gameplay::zones::box_quads(b, [0.; 3]) {
                            let q = q.map(|v| {
                                sub(
                                    add(
                                        object.position,
                                        add(
                                            scale(right, f64::from(v[0])),
                                            add(
                                                [0., f64::from(v[1]), 0.],
                                                scale(dir, f64::from(v[2])),
                                            ),
                                        ),
                                    ),
                                    draw.camera,
                                )
                                .map(|n| n as f32)
                            });
                            frame.push(q, [100, 185, 75, 240]);
                        }
                    }
                } else if matches!(object.kind, K::SkyeHawk) {
                    let p = sub(object.position, draw.camera);
                    let dir =
                        valorant_horizon_engine::gameplay::geometry::horizontal(object.velocity);
                    let side = [dir[2], 0., -dir[0]];
                    for q in beam(add(p, scale(side, -0.45)), add(p, scale(side, 0.45)), 0.07) {
                        frame.push(q, [155, 220, 95, 240]);
                    }
                    for q in orb(p, 0.16) {
                        frame.push(q, [245, 215, 120, 245]);
                    }
                } else {
                    for q in orb(sub(add(object.position, [0., 0.25, 0.]), draw.camera), 0.3) {
                        frame.push(q, [150, 215, 65, 225]);
                    }
                }
                continue;
            }
            if matches!(object.kind, K::SeismicFront | K::WallCharge) {
                if matches!(object.kind, K::SeismicFront) {
                    for q in beam(
                        sub(add(object.position, [0., 0.1, 0.]), draw.camera),
                        sub(add(object.velocity, [0., 0.1, 0.]), draw.camera),
                        0.16,
                    ) {
                        frame.push(q, [255, 145, 45, 220]);
                    }
                } else {
                    for q in orb(sub(object.position, draw.camera), 0.15) {
                        frame.push(q, [255, 155, 55, 230]);
                    }
                }
                continue;
            }
            if matches!(
                object.kind,
                K::Fragment | K::FlashDrive | K::SuppressionBlade | K::KayoCore
            ) {
                let center = add(
                    object.position,
                    [
                        0.,
                        if matches!(object.kind, K::KayoCore) {
                            0.9
                        } else {
                            0.1
                        },
                        0.,
                    ],
                );
                let color = if matches!(object.kind, K::FlashDrive) {
                    [240, 235, 180, 240]
                } else {
                    [150, 90, 240, 210]
                };
                for q in orb(
                    sub(center, draw.camera),
                    if matches!(object.kind, K::KayoCore) {
                        0.45
                    } else {
                        0.15
                    },
                ) {
                    frame.push(q, color);
                }
                continue;
            }
            if matches!(object.kind, K::LightCluster | K::LightBeacon | K::LightPath) {
                if matches!(object.kind, K::LightPath) {
                    let width = ((30_f64 - f64::from(object.ticks)) / 5.).clamp(0.1, 3.);
                    for q in beam(
                        sub(object.position, draw.camera),
                        sub(object.velocity, draw.camera),
                        width,
                    ) {
                        frame.push(q, [250, 220, 140, 100]);
                    }
                    let b = valorant_horizon_engine::gameplay::geometry::Box3 {
                        min: add(object.position, [-0.3, -0.8, -0.15]),
                        max: add(object.position, [0.3, 1., 0.15]),
                    };
                    for q in valorant_horizon_engine::gameplay::zones::box_quads(b, draw.camera) {
                        frame.push(q, [255, 230, 185, 170]);
                    }
                } else {
                    for q in orb(sub(add(object.position, [0., 0.15, 0.]), draw.camera), 0.22) {
                        frame.push(q, [255, 230, 160, 230]);
                    }
                }
                continue;
            }
            if matches!(object.kind, K::EnergyWall) {
                let normal =
                    valorant_horizon_engine::gameplay::geometry::horizontal(object.velocity);
                let side = [-normal[2] * 2., 0., normal[0] * 2.];
                let p = [
                    sub(object.position, side),
                    add(sub(object.position, side), [0., 3., 0.]),
                    add(add(object.position, side), [0., 3., 0.]),
                    add(object.position, side),
                ]
                .map(|p| sub(p, draw.camera).map(|v| v as f32));
                frame.push(p, [130, 60, 225, 140]);
                frame.push([p[3], p[2], p[1], p[0]], [130, 60, 225, 140]);
                continue;
            }
            if matches!(object.kind, K::IsoOrb | K::IsoAura) {
                if object.health <= 0. {
                    continue;
                }
                let aura = matches!(object.kind, K::IsoAura);
                let center = add(object.position, [0., if aura { 0.9 } else { 0.2 }, 0.]);
                for q in orb(sub(center, draw.camera), if aura { 1.1 } else { 0.2 }) {
                    frame.push(q, [160, 70, 240, if aura { 95 } else { 240 }]);
                }
                continue;
            }
            if matches!(object.kind, K::Echo) {
                if object.health <= 0. {
                    continue;
                }
                let facing =
                    valorant_horizon_engine::gameplay::geometry::horizontal(object.velocity);
                let right = [facing[2], 0., -facing[0]];
                let parts = [
                    ([-0.25, 0., -0.12], [-0.02, 0.7, 0.12], [20, 25, 65, 255]),
                    ([0.02, 0., -0.12], [0.25, 0.7, 0.12], [20, 25, 65, 255]),
                    ([-0.27, 0.7, -0.17], [0.27, 1.4, 0.17], [35, 50, 175, 255]),
                    ([-0.43, 0.65, -0.12], [-0.28, 1.4, 0.12], [40, 55, 195, 255]),
                    ([0.28, 0.65, -0.12], [0.43, 1.4, 0.12], [40, 55, 195, 255]),
                    ([-0.22, 1.4, -0.22], [0.22, 1.8, 0.22], [150, 165, 180, 255]),
                ];
                for (min, max, color) in parts {
                    for q in valorant_horizon_engine::gameplay::zones::box_quads(
                        valorant_horizon_engine::gameplay::geometry::Box3 { min, max },
                        [0.; 3],
                    ) {
                        let q = q.map(|p| {
                            sub(
                                add(
                                    object.position,
                                    [
                                        right[0] * f64::from(p[0]) + facing[0] * f64::from(p[2]),
                                        f64::from(p[1]),
                                        right[2] * f64::from(p[0]) + facing[2] * f64::from(p[2]),
                                    ],
                                ),
                                draw.camera,
                            )
                            .map(|v| v as f32)
                        });
                        frame.push(q, color);
                    }
                }
                continue;
            }
            if matches!(object.kind, K::Gatecrash | K::Blindside) {
                let center = add(object.position, [0., 0.15, 0.]);
                for q in orb(
                    sub(center, draw.camera),
                    if matches!(object.kind, K::Gatecrash) {
                        0.22
                    } else {
                        0.14
                    },
                ) {
                    frame.push(q, [45, 75, 240, 230]);
                }
                continue;
            }
            if matches!(object.kind, K::Haunt | K::HauntOrb | K::SeizeOrb) {
                let center = add(object.position, [0., 0.25, 0.]);
                for q in orb(sub(center, draw.camera), 0.28) {
                    frame.push(q, [40, 15, 65, 255]);
                }
                if matches!(object.kind, K::Haunt) {
                    let pupil = add(center, scale(unit(sub(draw.camera, center)), 0.27));
                    for q in orb(sub(pupil, draw.camera), 0.09) {
                        frame.push(q, [160, 225, 230, 255]);
                    }
                    for side in [-1., 0., 1.] {
                        for q in beam(
                            sub(center, draw.camera),
                            sub(add(object.position, [side * 0.4, -0.1, 0.2]), draw.camera),
                            0.025,
                        ) {
                            frame.push(q, [105, 30, 145, 220]);
                        }
                    }
                }
                continue;
            }
            if matches!(object.kind, K::Prowler) {
                let forward = unit([object.velocity[0], 0., object.velocity[2]]);
                let side = [-forward[2], 0., forward[0]];
                let head = add(add(object.position, [0., 0.32, 0.]), scale(forward, 0.25));
                for q in orb(sub(head, draw.camera), 0.12) {
                    frame.push(q, [155, 220, 230, 255]);
                }
                for n in [-1., 1.] {
                    let from = add(object.position, add([0., 0.15, 0.], scale(side, n * 0.2)));
                    let tip = add(from, add(scale(forward, 0.5), scale(side, n * 0.25)));
                    for q in beam(sub(from, draw.camera), sub(tip, draw.camera), 0.04) {
                        frame.push(q, [45, 15, 70, 255]);
                    }
                }
            }
            if matches!(
                object.kind,
                valorant_horizon_engine::gameplay::objects::ObjectKind::Trapwire
            ) {
                for q in valorant_horizon_engine::gameplay::projectiles::beam(
                    sub(object.position, draw.camera),
                    sub(object.velocity, draw.camera),
                    0.025,
                ) {
                    frame.push(q, [60, 200, 220, 255]);
                }
                continue;
            }

            for quad in
                valorant_horizon_engine::gameplay::zones::box_quads(object.bounds(), draw.camera)
            {
                frame.push(quad,
                    match object.kind {
                        valorant_horizon_engine::gameplay::objects::ObjectKind::Wingman | valorant_horizon_engine::gameplay::objects::ObjectKind::Dizzy | valorant_horizon_engine::gameplay::objects::ObjectKind::Thrash | valorant_horizon_engine::gameplay::objects::ObjectKind::Plasma | valorant_horizon_engine::gameplay::objects::ObjectKind::Mosh | valorant_horizon_engine::gameplay::objects::ObjectKind::MoshPatch | valorant_horizon_engine::gameplay::objects::ObjectKind::WingmanGlobule | valorant_horizon_engine::gameplay::objects::ObjectKind::DizzyGlobule | valorant_horizon_engine::gameplay::objects::ObjectKind::ThrashGlobule => [100,180,220,220],

                        valorant_horizon_engine::gameplay::objects::ObjectKind::SkyeTiger | valorant_horizon_engine::gameplay::objects::ObjectKind::SkyeHawk | valorant_horizon_engine::gameplay::objects::ObjectKind::SkyeSeeker => [150,215,65,225],

                        valorant_horizon_engine::gameplay::objects::ObjectKind::SeismicFront | valorant_horizon_engine::gameplay::objects::ObjectKind::WallCharge => [255,145,45,220],

                        valorant_horizon_engine::gameplay::objects::ObjectKind::Fragment |
                        valorant_horizon_engine::gameplay::objects::ObjectKind::FlashDrive |
                        valorant_horizon_engine::gameplay::objects::ObjectKind::SuppressionBlade |
                        valorant_horizon_engine::gameplay::objects::ObjectKind::KayoCore => [150,90,240,210],

                        valorant_horizon_engine::gameplay::objects::ObjectKind::LightCluster
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::LightBeacon
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::LightPath => {
                            [255, 230, 160, 200]
                        }

                        valorant_horizon_engine::gameplay::objects::ObjectKind::IsoOrb
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::IsoAura
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::Undercut
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::EnergyWall => {
                            [145, 65, 235, 180]
                        }
                        valorant_horizon_engine::gameplay::objects::ObjectKind::Echo
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::DormantEcho
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::Gatecrash
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::Blindside => {
                            [50, 65, 225, 255]
                        }
                        valorant_horizon_engine::gameplay::objects::ObjectKind::RelayBolt => {
                            [80, 215, 255, 255]
                        }
                        valorant_horizon_engine::gameplay::objects::ObjectKind::HauntOrb
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::SeizeOrb
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::Haunt
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::Prowler => {
                            [80, 30, 110, 255]
                        }
                        valorant_horizon_engine::gameplay::objects::ObjectKind::Trapwire
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::Spycam
                        | valorant_horizon_engine::gameplay::objects::ObjectKind::CageDevice => {
                            [70, 210, 220, 255]
                        }
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
                        frame.push(q, [180, 220, 240, 255]);
                    }
                }
            }
            if matches!(o.kind, K::ReconBolt) {
                let phase = f64::from(60u32.saturating_sub(o.ticks) % 20) / 20.;
                for q in sphere(
                    sub(add(o.position, [0., 0.1, 0.]), draw.camera),
                    0.2 + phase * 4.,
                ) {
                    frame.push(q, [50, 160, 255, (80. * (1. - phase)) as u8]);
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
                .collect::<Vec<_>>()
            } else {
                valorant_horizon_engine::gameplay::zones::box_quads(f.bounds, draw.camera)
            };
            for q in quads {
                frame.push(q, [75, 180, 20, 255]);
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
                    frame.push(quad, [215, 250, 255, 255]);
                }
            }
        }
        for t in &snapshot.trails {
            for q in valorant_horizon_engine::gameplay::projectiles::beam(
                sub(t.from, draw.camera),
                sub(t.to, draw.camera),
                0.045,
            ) {
                frame.push(q, [110, 45, 160, 220]);
            }
        }
        for b in &snapshot.beams {
            for q in valorant_horizon_engine::gameplay::projectiles::beam(
                sub(b.from, draw.camera),
                sub(b.to, draw.camera),
                0.35,
            ) {
                frame.push(q, [75, 170, 255, 210]);
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
                sphere(center, 0.15).collect::<Vec<_>>()
            };
            for quad in quads {
                frame.push(quad, color);
            }
        }
        frame.zones(
            &snapshot,
            draw.camera,
            draw.game_time as f64 + f64::from(draw.partial),
        );
        if let Ok(s) = state().lock() {
            s.scene.draw(&mut frame, draw.camera, draw.partial);
        }
        for face in frame.sorted() {
            self.native_world_quad(
                draw,
                "horizon/white",
                face.points,
                [0., 0., 16., 16.],
                face.color,
            );
        }
        if let Ok(mut s) = state().lock() {
            s.mesh = Some(frame);
        }
    }
    pub(super) fn vh_fog(&self, camera: [f64; 3], fog: &mut horizon_client::fog::FogData) {
        let cage = state().lock().ok().is_some_and(|s| {
            s.snapshot.as_ref().is_some_and(|s| {
                s.zones.iter().any(|z| {
                    matches!(
                        z.kind,
                        valorant_horizon_engine::gameplay::zones::ZoneKind::CyberCage
                            | valorant_horizon_engine::gameplay::zones::ZoneKind::ElectricWall
                    ) && z.touches(valorant_horizon_engine::gameplay::geometry::Box3 {
                        min: camera.map(|n| n - 0.01),
                        max: camera.map(|n| n + 0.01),
                    })
                })
            })
        });
        if cage {
            fog.color = [0.35, 0.55, 0.6, 1.];
            fog.environmental_start = 0.;
            fog.environmental_end = 0.25;
            fog.render_distance_start = 0.;
            fog.render_distance_end = 0.25;
            return;
        }
        let vision = state()
            .lock()
            .ok()
            .and_then(|s| s.snapshot.as_ref().map(|s| s.vision));
        if vision.is_some_and(|v| v.splash > 0) {
            fog.color = [0.12, 0.3, 0.85, 1.];
            fog.environmental_start = 0.;
            fog.environmental_end = 0.02;
            fog.render_distance_start = 0.;
            fog.render_distance_end = 0.02;
            fog.sky_end = 0.02;
            fog.cloud_end = 0.02;
            return;
        }
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
