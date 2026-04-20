use crate::planet::PlanetConfig;
use crate::utils::smoothstep;
use noise::{NoiseFn, Perlin};

pub struct ShapingContext<'a> {
    pub noise: &'a Perlin,
    pub planet: &'a PlanetConfig,
    pub seed_offset: f64,
    pub continent_offset: f64,
}

impl<'a> ShapingContext<'a> {
    pub fn new(
        noise: &'a Perlin,
        planet: &'a PlanetConfig,
        seed_offset: f64,
        continent_offset: f64,
    ) -> Self {
        Self {
            noise,
            planet,
            seed_offset,
            continent_offset,
        }
    }

    // Basic land mass mask, to get noise for heights > 0
    pub fn continent_mask(x: f64, z: f64, ctx: &ShapingContext) -> f64 {
        let cx = x * ctx.planet.continent_freq + ctx.continent_offset;
        let cz = z * ctx.planet.continent_freq + ctx.continent_offset;
        ctx.noise.get([cx, cz])
    }

    // Basic fractional brownian motion, with option for ridged variants
    pub fn fbm(x: f64, z: f64, ctx: &ShapingContext, ridged: bool) -> f64 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = ctx.planet.freq_scale;
        let mut max_value = 0.0;

        for _ in 0..ctx.planet.octaves {
            let nx = x * frequency + ctx.seed_offset;
            let nz = z * frequency + ctx.seed_offset;

            if ridged {
                let sample = 1.0 - ctx.noise.get([nx, nz]).abs();
                total += sample * amplitude;
            } else {
                total += ctx.noise.get([nx, nz]) * amplitude;
            }

            max_value += amplitude;
            amplitude *= ctx.planet.persistence as f64;
            frequency *= ctx.planet.lacunarity;
        }

        total / max_value
    }

    pub fn erosion_mask(x: f64, z: f64, ctx: &ShapingContext) -> f64 {
        let freq = ctx.planet.continent_freq * 3.0;
        let nx = x * freq + ctx.seed_offset + 99_999.0;
        let nz = z * freq + ctx.seed_offset + 99_999.0;

        (ctx.noise.get([nx, nz]) + 1.0) / 2.0
    }

    pub fn domain_warp(x: f64, z: f64, ctx: &ShapingContext) -> (f64, f64) {
        let warp_freq = ctx.planet.freq_scale * 0.25;

        let wx = ctx.noise.get([
            x * warp_freq + ctx.seed_offset + 111_111.0,
            z * warp_freq + ctx.seed_offset + 111_111.0,
        ]);
        let wz = ctx.noise.get([
            x * warp_freq + ctx.seed_offset + 222_222.0,
            z * warp_freq + ctx.seed_offset + 222_222.0,
        ]);

        (
            x + wx * ctx.planet.warp_strength,
            z + wz * ctx.planet.warp_strength,
        )
    }

    pub fn apply_plateau_curve(normalized: f64, plateau_strength: f64) -> f64 {
        if plateau_strength == 0.0 {
            return normalized;
        }

        let s = smoothstep(normalized);

        normalized * (1.0 - plateau_strength) + s * plateau_strength
    }
    pub fn apply_continent_factor(land_height: f64, continent: f64, ctx: &ShapingContext) -> f64 {
        let factor = ((continent - ctx.planet.water_threshold)
            / (1.0 - ctx.planet.water_threshold))
            .clamp(0.0, 1.0)
            .powf(ctx.planet.continent_slope);

        land_height * factor
    }
}
