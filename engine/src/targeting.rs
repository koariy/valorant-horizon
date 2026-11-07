/// Fail closed before asking the platform for line of sight. This is a geometry
/// primitive; server authority and collision sweeps are required for combat.
pub fn in_range(from: [f64; 3], to: [f64; 3], range: f64) -> bool {
    range.is_finite()
        && range >= 0.0
        && from.iter().chain(&to).all(|v| v.is_finite())
        && from
            .iter()
            .zip(to)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            <= range * range
}
