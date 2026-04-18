use crate::config::*;
use raylib::prelude::*;

const HOURS_IN_DAY: f32 = 24.0;
const SECONDS_IN_HOUR: f32 = 3600.0;

pub struct TimeOfDay {
    hour: f32,
    paused: bool,
    sky_color: Color,
}

impl TimeOfDay {
    pub fn new(sky_color: Color) -> Self {
        Self {
            hour: TIME_STARTING_HOUR.rem_euclid(HOURS_IN_DAY),
            paused: false,
            sky_color,
        }
    }

    pub fn hour(&self) -> f32 {
        self.hour
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn set_hour(&mut self, hour: f32) {
        self.hour = hour.rem_euclid(HOURS_IN_DAY);
    }

    pub fn advance(&mut self, dt: f32, speed: f32) {
        if self.paused {
            println!("WARNING: Tried to advance time of day while paused");
            return;
        }

        self.hour = (self.hour + dt * speed / SECONDS_IN_HOUR).rem_euclid(HOURS_IN_DAY);
    }

    pub fn sun_direction(&self) -> Vector3 {
        let day_duration = SUNSET_END - SUNRISE_START;

        // Extend arc by sun radius so it clears the horizon properly at night/day start times
        let margin = (SUN_RADIUS / SUN_PLAYER_DISTANCE).asin();
        let t = (self.hour - SUNRISE_START) / day_duration;
        let angle = t * (std::f32::consts::PI + 2.0 * margin) - margin;

        Vector3::new(angle.cos(), angle.sin(), 0.0)
    }

    pub fn sun_intensity(&self) -> f32 {
        if self.hour < SUNRISE_START || self.hour > SUNSET_END {
            0.0
        } else if self.hour < SUNRISE_END {
            (self.hour - SUNRISE_START) / (SUNRISE_END - SUNRISE_START)
        } else if self.hour > SUNSET_START {
            (SUNSET_END - self.hour) / (SUNSET_END - SUNSET_START)
        } else {
            1.0
        }
    }

    pub fn ambient_strength(&self) -> f32 {
        let t = self.sun_intensity();
        AMBIENT_NIGHT + (AMBIENT_DAY - AMBIENT_NIGHT) * t
    }

    pub fn sky_color(&self) -> Color {
        let sunrise_mid = (SUNRISE_START + SUNRISE_END) / 2.0;
        let sunset_mid = (SUNSET_START + SUNSET_END) / 2.0;
        let t: f32;

        if self.hour < SUNRISE_START || self.hour > SUNSET_END {
            SKY_COLOR_NIGHT
        } else if self.hour < sunrise_mid {
            t = (self.hour - SUNRISE_START) / (sunrise_mid - SUNRISE_START);
            SKY_COLOR_NIGHT.lerp(self.sunrise_color(), t)
        } else if self.hour < SUNRISE_END {
            t = (self.hour - sunrise_mid) / (SUNRISE_END - sunrise_mid);
            self.sunrise_color().lerp(self.sky_color, t)
        } else if self.hour < SUNSET_START {
            self.sky_color
        } else if self.hour < sunset_mid {
            t = (self.hour - SUNSET_START) / (sunset_mid - SUNSET_START);
            self.sky_color.lerp(self.sunrise_color(), t)
        } else {
            t = (self.hour - sunset_mid) / (SUNSET_END - sunset_mid);
            self.sunrise_color().lerp(SKY_COLOR_NIGHT, t)
        }
    }

    pub fn fog_color(&self) -> Color {
        self.sky_color()
    }

    // Private

    fn sunrise_color(&self) -> Color {
        self.sky_color
            .lerp(SKY_SUNRISE_TINT, SKY_SUNRISE_TINT_STRENGTH)
    }
}
