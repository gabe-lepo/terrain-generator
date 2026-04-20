use crate::config::*;
use crate::planet::PlanetConfig;
use noise::{NoiseFn, Perlin};

#[derive(Clone, Copy)]
pub enum StampKind {
    Volcano,
    Mesa,
    Crater,
    Peak,
}

#[derive(Clone)]
pub struct FeatureStamp {
    pub world_x: f64,
    pub world_z: f64,
    pub radius: f64,
    pub strength: f64,
    pub kind: StampKind,
}

pub fn stamp_contribution(x: f64, z: f64, stamp: &FeatureStamp) -> f64 {
    let d = dist(x, z, stamp);
    if d > stamp.radius {
        return 0.0;
    }

    match stamp.kind {
        StampKind::Volcano => {
            let t = 1.0 - (d / stamp.radius);
            let cone = t * stamp.strength;
            let crater_r = stamp.radius * 0.3;
            let crater_depth = if d < crater_r {
                let ct = 1.0 - (d / crater_r);
                ct * ct * stamp.strength * 0.6
            } else {
                0.0
            };

            cone - crater_depth
        }
        StampKind::Mesa => {
            let t = 1.0 - (d / stamp.radius);
            // Flat top, everything within inner half of radius at full height
            let plateau_t = (t / 0.1).min(1.0);
            smoothstep(plateau_t) * stamp.strength
        }
        StampKind::Crater => {
            let t = d / stamp.radius;
            let dip = (-4.0 * t * t).exp();
            -dip * stamp.strength
        }
        StampKind::Peak => {
            let t = 1.0 - (d / stamp.radius);
            t * t * stamp.strength
        }
    }
}

pub fn generate_stamps(
    seed: u64,
    planet: &PlanetConfig,
    noise: &Perlin,
    kinds: &[(StampKind, u32)],
) -> Vec<FeatureStamp> {
    let world_size = planet.grid_size as f64 * CHUNK_SIZE as f64 * TERRAIN_RESOLUTION as f64;
    let continent_offset = (seed.wrapping_mul(1234567891) % 100_000) as f64;
    let mut rng = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let mut stamps = Vec::new();

    for (kind, count) in kinds {
        for _ in 0..*count {
            // LCG steps for x, z, radius, and strength
            rng = lcg(rng);
            let wx = (rng as f64 / u64::MAX as f64) * world_size;
            rng = lcg(rng);
            let wz = (rng as f64 / u64::MAX as f64) * world_size;
            rng = lcg(rng);
            let radius_t = rng as f64 / u64::MAX as f64;
            rng = lcg(rng);
            let strength_t = rng as f64 / u64::MAX as f64;

            // Only place on land
            let cx = wx * planet.continent_freq + continent_offset;
            let cz = wz * planet.continent_freq + continent_offset;
            let continent = noise.get([cx, cz]);
            if continent < planet.water_threshold + 0.1 {
                continue;
            }

            let (radius, strength) = stamp_params(kind, radius_t, strength_t, planet);
            stamps.push(FeatureStamp {
                world_x: wx,
                world_z: wz,
                radius,
                strength,
                kind: *kind,
            });
        }
    }

    stamps
}

fn stamp_params(
    kind: &StampKind,
    radius_t: f64,
    strength_t: f64,
    planet: &PlanetConfig,
) -> (f64, f64) {
    // TODO: Move to configs
    let volcano_vals: [f64; 4] = [300.0, 400.0, 0.7, 0.7];
    let mesa_vals: [f64; 4] = [200.0, 300.0, 0.5, 0.5];
    let valley_vals: [f64; 4] = [150.0, 300.0, 0.4, 0.4];
    let peak_vals: [f64; 4] = [150.0, 250.0, 0.5, 0.6];
    let h = planet.height_scale as f64;

    let process = |base_radius: f64, radius_range: f64, base_str: f64, str_range: f64| {
        (
            base_radius + radius_t * radius_range,
            h * (base_str + strength_t * str_range),
        )
    };

    match kind {
        StampKind::Volcano => process(
            volcano_vals[0],
            volcano_vals[1],
            volcano_vals[2],
            volcano_vals[3],
        ),
        StampKind::Mesa => process(mesa_vals[0], mesa_vals[1], mesa_vals[2], mesa_vals[3]),
        StampKind::Crater => process(
            valley_vals[0],
            valley_vals[1],
            valley_vals[2],
            valley_vals[3],
        ),
        StampKind::Peak => process(peak_vals[0], peak_vals[1], peak_vals[2], peak_vals[3]),
    }
}

fn dist(x: f64, z: f64, stamp: &FeatureStamp) -> f64 {
    let dx = x - stamp.world_x;
    let dz = z - stamp.world_z;

    (dx * dx + dz * dz).sqrt()
}

fn lcg(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}
