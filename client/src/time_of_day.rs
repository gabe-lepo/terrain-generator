use crate::config::{
    AMBIENT_DAY, AMBIENT_NIGHT, SKY_COLOR_DAY, SKY_COLOR_NIGHT, SKY_COLOR_SUNRISE, SUNRISE_END,
    SUNRISE_START, SUNSET_END, SUNSET_START,
};
use raylib::prelude::*;

const HOURS_IN_DAY: f32 = 24.0;
const SECONDS_IN_HOUR: f32 = 3600.0;

pub struct TimeOfDay {
    hour: f32,
    paused: bool,
}

impl TimeOfDay {
    pub fn new(hour: f32) -> Self {
        Self {
            hour: hour.rem_euclid(HOURS_IN_DAY),
            paused: false,
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
        // No direction if between night/morning hours
        // if self.hour < SUNRISE_START || self.hour > SUNSET_END {
        //     return Vector3::new(0.0, -1.0, 0.0);
        // }

        let day_duration = SUNSET_END - SUNRISE_START;
        let angle = (self.hour - SUNRISE_START) / day_duration * std::f32::consts::PI;

        let x = angle.cos();
        let y = angle.sin();
        let z = 0.3; // ?

        let len = (x * x + y * y + z * z).sqrt();

        Vector3::new(x / len, y / len, z / len)
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
            SKY_COLOR_NIGHT.lerp(SKY_COLOR_SUNRISE, t)
        } else if self.hour < SUNRISE_END {
            t = (self.hour - sunrise_mid) / (SUNRISE_END - sunrise_mid);
            SKY_COLOR_SUNRISE.lerp(SKY_COLOR_DAY, t)
        } else if self.hour < SUNSET_START {
            SKY_COLOR_DAY
        } else if self.hour < sunset_mid {
            t = (self.hour - SUNSET_START) / (sunset_mid - SUNSET_START);
            SKY_COLOR_DAY.lerp(SKY_COLOR_SUNRISE, t)
        } else {
            t = (self.hour - sunset_mid) / (SUNSET_END - sunset_mid);
            SKY_COLOR_SUNRISE.lerp(SKY_COLOR_NIGHT, t)
        }
    }

    pub fn fog_color(&self) -> Color {
        self.sky_color()
    }
}
