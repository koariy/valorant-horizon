use super::{geometry::*, *};
use serde::{Deserialize, Serialize};
pub const BLOCK: &str = "minecraft:frosted_ice";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OwnedBlock {
    pub position: [i32; 3],
    pub expires: i64,
}
pub fn cells(a: &Actor, rotated: bool) -> Vec<[i32; 3]> {
    let forward = unit([a.direction[0], 0.0, a.direction[2]]);
    let center = add(a.position, scale(forward, 3.0)).map(|v| v.floor() as i32);
    let along_x = (forward[2].abs() >= forward[0].abs()) != rotated;
    let mut out = Vec::new();
    for i in -1..=2 {
        for y in 0..3 {
            out.push([
                center[0] + if along_x { i } else { 0 },
                center[1] + y,
                center[2] + if along_x { 0 } else { i },
            ]);
        }
    }
    out
}
pub fn place(w: &mut impl World, a: &Actor, rotated: bool, owned: &mut Vec<OwnedBlock>) -> bool {
    let cells = cells(a, rotated);
    let actors = w.actors();
    if owned.len() + cells.len() > 192 {
        return false;
    }
    for cell in &cells {
        if !w
            .block(*cell)
            .is_some_and(|s| ["minecraft:air", "minecraft:cave_air"].contains(&s.as_str()))
        {
            return false;
        }
        let b = Box3 {
            min: cell.map(f64::from),
            max: add(cell.map(f64::from), [1.0; 3]),
        };
        if actors.iter().any(|a| a.eligible && a.bounds.intersects(b)) {
            return false;
        }
    }
    for cell in cells
        .iter()
        .filter(|c| c[1] == a.position[1].floor() as i32)
    {
        if w.boxes([cell[0], cell[1] - 1, cell[2]])
            .is_none_or(|b| b.is_empty())
        {
            return false;
        }
    }
    let old_len = owned.len();
    owned.extend(cells.iter().map(|p| OwnedBlock {
        position: *p,
        expires: w.now().saturating_add(600),
    }));
    // Persist intent before changing the world. Failure cannot consume a charge or place a wall.
    if !w.save_blocks(owned) {
        owned.truncate(old_len);
        return false;
    }
    let mut placed = Vec::new();
    for cell in cells {
        if !w.set_block(cell, BLOCK) {
            let mut pending = Vec::new();
            for p in placed {
                if w.block(p).as_deref() == Some(BLOCK) && !w.set_block(p, "minecraft:air") {
                    pending.push(OwnedBlock {
                        position: p,
                        expires: w.now(),
                    });
                }
            }
            owned.truncate(old_len);
            owned.extend(pending);
            w.save_blocks(owned);
            return false;
        }
        placed.push(cell);
    }
    true
}
pub fn cleanup(w: &mut impl World, owned: &mut Vec<OwnedBlock>) {
    let len = owned.len();
    owned.retain(|b| {
        let Some(block) = w.block(b.position) else {
            return true;
        };
        if block != BLOCK {
            return false;
        }
        b.expires > w.now() || !w.set_block(b.position, "minecraft:air")
    });
    if owned.len() != len {
        w.save_blocks(owned);
    }
}
