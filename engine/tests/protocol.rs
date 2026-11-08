use valorant_horizon_engine::{
    agents::reyna::Soul,
    gameplay::{geometry::*, objects::*, projectiles::*, smoke::*, zones::*, *},
    server::{Player, Session},
};
#[test]
fn maximal_snapshot_fits_native_payload_budget() {
    let at = [-29999999.12345679, 319.1234567890123, 29999999.12345679];
    let mut s = Session::default();
    let mut p = Player {
        agent: 5,
        ..Default::default()
    };
    p.reyna.souls = vec![
        Soul {
            position: at,
            ticks: u32::MAX
        };
        16
    ];
    p.error = Some("Ability unavailable, obstructed, or agent not implemented".into());
    s.players.insert(u128::MAX, p);
    s.objects = vec![
        Object {
            id: u64::MAX,
            owner: u128::MAX,
            kind: ObjectKind::ReconBolt,
            position: at,
            health: 6.,
            ticks: u32::MAX,
            velocity: at,
            target: Some(Entity::Player(u128::MAX))
        };
        MAX_OBJECTS
    ];
    s.projectiles = vec![
        Projectile {
            owner: u128::MAX,
            kind: ProjectileKind::DarkCover { target: at },
            position: at,
            velocity: at,
            ticks: u32::MAX,
            guided: false,
            victims: vec![]
        };
        MAX_PROJECTILES
    ];
    s.smokes = vec![
        Smoke {
            owner: u128::MAX,
            center: at,
            radius: 2.8,
            ticks: u32::MAX,
            color: [255; 4]
        };
        MAX_SMOKES
    ];
    s.zones = vec![
        Zone {
            owner: u128::MAX,
            kind: ZoneKind::Incendiary,
            ticks: u32::MAX,
            bounds: Box3 { min: at, max: at }
        };
        MAX_ZONES
    ];
    s.toxins = vec![
        toxin::Field {
            owner: u128::MAX,
            bounds: Box3 { min: at, max: at },
            sphere: false
        };
        toxin::MAX_FIELDS
    ];
    s.players.get_mut(&u128::MAX).unwrap().astra.stars = vec![
            valorant_horizon_engine::agents::astra::Star {
                owner: u128::MAX,
                position: at,
                ticks: u32::MAX
            };
            valorant_horizon_engine::agents::astra::MAX_STARS
        ];
    s.divides = vec![
        divide::Divide {
            point: at,
            normal: at,
            ticks: u32::MAX
        };
        2
    ];
    s.shields = vec![
        shields::Shield {
            owner: u128::MAX,
            center: at,
            radius: 3.,
            health: 30.,
            ticks: u32::MAX,
            active: true,
            spent: true
        };
        shields::MAX_SHIELDS
    ];
    let mut snapshot = s.snapshot(u128::MAX).unwrap();
    snapshot.contacts = vec![
        recon::Contact {
            entity: u32::MAX,
            ticks: u32::MAX
        };
        recon::MAX_CONTACTS
    ];
    snapshot.beams = vec![
        recon::Beam {
            from: at,
            to: at,
            ticks: u32::MAX
        };
        recon::MAX_BEAMS
    ];
    snapshot.trails = snapshot.beams.iter().take(2).cloned().collect();
    snapshot.vision.deafen = u32::MAX;
    snapshot.bow_charge = Some(u32::MAX);
    snapshot.kit = [u32::MAX; 4];
    snapshot.bounces = u8::MAX;
    snapshot.fury_shots = u8::MAX;
    let bytes = serde_json::to_vec(&snapshot).unwrap();
    assert!(bytes.len() < 29000, "{}", bytes.len());
    let decoded: Snapshot = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded.trails.len(), 2);
    assert_eq!(decoded.objects.len(), snapshot.objects.len());
    assert_eq!(decoded.objects[0].velocity, at);
    assert_eq!(decoded.projectiles.len(), MAX_PROJECTILES);
}
