use super::*;
/// Constant-work local obstacle avoidance. No global search or chunk generation.
pub fn step(w: &impl World, o: &mut Object, direction: V3, speed: f64) {
    o.velocity[1] = (o.velocity[1] - 0.06).max(-0.6);
    let down = sweep(w, o.bounds(), [0., o.velocity[1], 0.]);
    o.position = add(o.position, down);
    if down[1].abs() < 0.01 {
        o.velocity[1] = 0.;
    }
    let d = horizontal(direction);
    for angle in [0_f64, 0.785, -0.785, 1.57, -1.57] {
        let v = scale(
            [
                d[0] * angle.cos() - d[2] * angle.sin(),
                0.,
                d[0] * angle.sin() + d[2] * angle.cos(),
            ],
            speed,
        );
        let delta = sweep(w, o.bounds(), v);
        if length(delta) >= speed * 0.9 {
            o.position = add(o.position, delta);
            o.velocity[0] = v[0];
            o.velocity[2] = v[2];
            return;
        }
        let up = [0., 0.5, 0.];
        if o.velocity[1] == 0.
            && clear_box(w, o.bounds().translated(up))
            && length(sweep(w, o.bounds().translated(up), v)) >= speed * 0.9
            && block_hit(
                w,
                add(add(o.position, up), v),
                add(add(o.position, v), [0., -0.3, 0.]),
                0.,
            )
            .is_some()
        {
            o.position = add(add(o.position, up), v);
            o.velocity[0] = v[0];
            o.velocity[2] = v[2];
            return;
        }
    }
}
