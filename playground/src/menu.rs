use crate::{
    config::{GRID_MAX, GRID_MIN, GRID_STEP, WINDOW_WIDTH},
    noise::{BaseSignal, NoiseConfig, NoiseParams},
};
use raylib::prelude::*;

const KEY_REPEAT_DELAY: f32 = 0.2;

pub enum MenuAction {
    None,
    Rebake, // recolor only — heightmap unchanged
    Regen,  // new noise + rebake
}

pub enum MenuLevel {
    Root,
    BaseSignalParams(BaseSignalParam),
    ContinentParams(FilterParam),
    RedistributionParams(FilterParam),
    MoistureParams(MoistureParam),
}

pub enum MoistureParam {
    Frequency,
    Offset,
}

pub enum BaseSignalParam {
    Frequency,
    Octaves,
    Persistence,
    Lacunarity,
}

pub enum FilterParam {
    ContinentFreq,
    ContinentOctaves,
    ContinentBlend,
    ContinentLandBias,
    RedistributionExp,
    RedistributionElevationDep,
    RedistributionSmoothstep,
}

pub enum MenuItem {
    // Base signal section
    Random,
    Perlin,
    Fbm,
    // Filter section
    ContinentMask,
    Redistribution,
    Moisture,
    // Map section
    GridSize,
    // Other options
    DoLerp,
}

impl MenuItem {
    fn next(&self) -> Self {
        match self {
            Self::GridSize => Self::DoLerp,
            Self::DoLerp => Self::Random,
            Self::Random => Self::Perlin,
            Self::Perlin => Self::Fbm,
            Self::Fbm => Self::ContinentMask,
            Self::ContinentMask => Self::Redistribution,
            Self::Redistribution => Self::Moisture,
            Self::Moisture => Self::GridSize,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Self::GridSize => Self::Moisture,
            Self::DoLerp => Self::GridSize,
            Self::Random => Self::DoLerp,
            Self::Perlin => Self::Random,
            Self::Fbm => Self::Perlin,
            Self::ContinentMask => Self::Fbm,
            Self::Redistribution => Self::ContinentMask,
            Self::Moisture => Self::Redistribution,
        }
    }
}

pub struct MenuState {
    pub cursor: MenuItem,
    pub level: MenuLevel,
    key_repeat_timer: f32,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            cursor: MenuItem::Random,
            level: MenuLevel::Root,
            key_repeat_timer: 0.0,
        }
    }

    pub fn handle_input(
        &mut self,
        handle: &RaylibHandle,
        config: &mut NoiseConfig,
        grid_size: &mut usize,
        dt: f32,
    ) -> MenuAction {
        match self.level {
            MenuLevel::Root => self.handle_root_input(handle, config, grid_size),
            MenuLevel::BaseSignalParams(_) => self.handle_param_input(handle, config, dt),
            MenuLevel::ContinentParams(_) | MenuLevel::RedistributionParams(_) => self.handle_filter_input(handle, config, dt),
            MenuLevel::MoistureParams(_) => self.handle_moisture_input(handle, config, dt),
        }
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle, config: &NoiseConfig, grid_size: usize) {
        match self.level {
            MenuLevel::Root => self.draw_root(d, config, grid_size),
            MenuLevel::BaseSignalParams(ref param) => self.draw_params(d, config, param),
            MenuLevel::ContinentParams(ref param) | MenuLevel::RedistributionParams(ref param) => self.draw_filter_params(d, config, param),
            MenuLevel::MoistureParams(ref param) => self.draw_moisture_params(d, config, param),
        }
    }

    fn handle_root_input(
        &mut self,
        handle: &RaylibHandle,
        config: &mut NoiseConfig,
        grid_size: &mut usize,
    ) -> MenuAction {
        if handle.is_key_pressed(KeyboardKey::KEY_DOWN) {
            self.cursor = self.cursor.next();
        }
        if handle.is_key_pressed(KeyboardKey::KEY_UP) {
            self.cursor = self.cursor.prev();
        }
        if handle.is_key_pressed(KeyboardKey::KEY_ENTER) {
            match self.cursor {
                MenuItem::Random => {
                    if !matches!(config.base_signal, BaseSignal::Random) {
                        config.base_signal = BaseSignal::Random;
                        config.noise_params = NoiseParams::new();
                        return MenuAction::Regen;
                    }
                }
                MenuItem::Perlin => {
                    if !matches!(config.base_signal, BaseSignal::Perlin) {
                        config.base_signal = BaseSignal::Perlin;
                        config.noise_params = NoiseParams::new();
                        return MenuAction::Regen;
                    }
                }
                MenuItem::Fbm => {
                    if !matches!(config.base_signal, BaseSignal::Fbm) {
                        config.base_signal = BaseSignal::Fbm;
                        config.noise_params = NoiseParams::new();
                        return MenuAction::Regen;
                    }
                }
                MenuItem::ContinentMask => {
                    config.use_continent_mask = !config.use_continent_mask;
                    return MenuAction::Regen;
                }
                MenuItem::Redistribution => {
                    config.use_redistribution = !config.use_redistribution;
                    return MenuAction::Regen;
                }
                MenuItem::Moisture => {
                    config.use_moisture = !config.use_moisture;
                    return MenuAction::Rebake;
                }
                MenuItem::GridSize => {}
                MenuItem::DoLerp => {
                    config.do_lerp = !config.do_lerp;
                    return MenuAction::Rebake;
                }
            }
        }
        if handle.is_key_pressed(KeyboardKey::KEY_RIGHT) {
            match self.cursor {
                MenuItem::Perlin | MenuItem::Fbm => {
                    self.level = MenuLevel::BaseSignalParams(BaseSignalParam::Frequency);
                }
                MenuItem::ContinentMask => {
                    self.level = MenuLevel::ContinentParams(FilterParam::ContinentFreq);
                }
                MenuItem::Redistribution => {
                    self.level = MenuLevel::RedistributionParams(FilterParam::RedistributionExp);
                }
                MenuItem::Moisture => {
                    self.level = MenuLevel::MoistureParams(MoistureParam::Frequency);
                }
                MenuItem::GridSize => {
                    *grid_size = (*grid_size + GRID_STEP).min(GRID_MAX);
                    return MenuAction::Regen;
                }
                _ => {}
            }
        }
        if handle.is_key_pressed(KeyboardKey::KEY_LEFT) {
            if let MenuItem::GridSize = self.cursor {
                *grid_size = grid_size.saturating_sub(GRID_STEP).max(GRID_MIN);
                return MenuAction::Regen;
            }
        }
        MenuAction::None
    }

    fn handle_param_input(
        &mut self,
        handle: &RaylibHandle,
        config: &mut NoiseConfig,
        dt: f32,
    ) -> MenuAction {
        let MenuLevel::BaseSignalParams(ref mut param) = self.level else {
            return MenuAction::None;
        };

        if handle.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.level = MenuLevel::Root;
            return MenuAction::None;
        }
        if handle.is_key_pressed(KeyboardKey::KEY_DOWN) {
            *param = match param {
                BaseSignalParam::Frequency => BaseSignalParam::Octaves,
                BaseSignalParam::Octaves => BaseSignalParam::Persistence,
                BaseSignalParam::Persistence => BaseSignalParam::Lacunarity,
                BaseSignalParam::Lacunarity => BaseSignalParam::Frequency,
            };
            // Skip inapplicable params for Perlin
            if matches!(config.base_signal, BaseSignal::Perlin) {
                while !is_param_applicable(param, &config.base_signal) {
                    *param = match param {
                        BaseSignalParam::Frequency => BaseSignalParam::Octaves,
                        BaseSignalParam::Octaves => BaseSignalParam::Persistence,
                        BaseSignalParam::Persistence => BaseSignalParam::Lacunarity,
                        BaseSignalParam::Lacunarity => BaseSignalParam::Frequency,
                    };
                }
            }
        }
        if handle.is_key_pressed(KeyboardKey::KEY_UP) {
            *param = match param {
                BaseSignalParam::Frequency => BaseSignalParam::Lacunarity,
                BaseSignalParam::Octaves => BaseSignalParam::Frequency,
                BaseSignalParam::Persistence => BaseSignalParam::Octaves,
                BaseSignalParam::Lacunarity => BaseSignalParam::Persistence,
            };
            if matches!(config.base_signal, BaseSignal::Perlin) {
                while !is_param_applicable(param, &config.base_signal) {
                    *param = match param {
                        BaseSignalParam::Frequency => BaseSignalParam::Lacunarity,
                        BaseSignalParam::Octaves => BaseSignalParam::Frequency,
                        BaseSignalParam::Persistence => BaseSignalParam::Octaves,
                        BaseSignalParam::Lacunarity => BaseSignalParam::Persistence,
                    };
                }
            }
        }

        // Hold-to-repeat for value adjustment
        let left = handle.is_key_down(KeyboardKey::KEY_LEFT);
        let right = handle.is_key_down(KeyboardKey::KEY_RIGHT);

        if left || right {
            self.key_repeat_timer += dt;
            if self.key_repeat_timer >= KEY_REPEAT_DELAY {
                self.key_repeat_timer = 0.0;
                if right {
                    match param {
                        BaseSignalParam::Frequency => {
                            config.noise_params.frequency =
                                (config.noise_params.frequency + 0.005).min(1.0)
                        }
                        BaseSignalParam::Octaves => {
                            config.noise_params.octaves =
                                config.noise_params.octaves.saturating_add(1).min(8)
                        }
                        BaseSignalParam::Persistence => {
                            config.noise_params.persistence =
                                (config.noise_params.persistence + 0.05).min(1.0)
                        }
                        BaseSignalParam::Lacunarity => {
                            config.noise_params.lacunarity =
                                (config.noise_params.lacunarity + 0.1).min(4.0)
                        }
                    }
                } else {
                    match param {
                        BaseSignalParam::Frequency => {
                            config.noise_params.frequency =
                                (config.noise_params.frequency - 0.005).max(0.001)
                        }
                        BaseSignalParam::Octaves => {
                            config.noise_params.octaves =
                                config.noise_params.octaves.saturating_sub(1).max(1)
                        }
                        BaseSignalParam::Persistence => {
                            config.noise_params.persistence =
                                (config.noise_params.persistence - 0.05).max(0.0)
                        }
                        BaseSignalParam::Lacunarity => {
                            config.noise_params.lacunarity =
                                (config.noise_params.lacunarity - 0.1).max(1.0)
                        }
                    }
                }
                return MenuAction::Regen;
            }
        } else {
            self.key_repeat_timer = KEY_REPEAT_DELAY; // fire immediately on next press
        }

        MenuAction::None
    }

    fn draw_root(&self, d: &mut RaylibDrawHandle, config: &NoiseConfig, grid_size: usize) {
        const PANEL_X: i32 = WINDOW_WIDTH - 270;
        const PANEL_Y: i32 = 10;
        const PANEL_W: i32 = 260;
        const ROW_H: i32 = 24;
        const FONT_SIZE: i32 = 20;
        const PAD: i32 = 8;

        let grid_label = format!("  Grid size: {}", grid_size);
        let lerp_label = format!("  Lerp Colors");
        // (label, is_active, is_header)
        let items: &[(&str, bool, bool)] = &[
            ("MAP", false, true),
            (&grid_label, false, false),
            (&lerp_label, config.do_lerp, false),
            ("BASE SIGNAL", false, true),
            (
                "  Random",
                matches!(config.base_signal, BaseSignal::Random),
                false,
            ),
            (
                "  Perlin",
                matches!(config.base_signal, BaseSignal::Perlin),
                false,
            ),
            (
                "  FBM",
                matches!(config.base_signal, BaseSignal::Fbm),
                false,
            ),
            ("FILTERS", false, true),
            ("  Continent mask", config.use_continent_mask, false),
            ("  Redistribution", config.use_redistribution, false),
            ("  Moisture", config.use_moisture, false),
        ];

        // Maps cursor to item index, skipping headers at indices 0, 2, 6
        let cursor_index = match self.cursor {
            MenuItem::GridSize => 1,
            MenuItem::DoLerp => 2,
            MenuItem::Random => 4,
            MenuItem::Perlin => 5,
            MenuItem::Fbm => 6,
            MenuItem::ContinentMask => 8,
            MenuItem::Redistribution => 9,
            MenuItem::Moisture => 10,
        };

        let panel_h = items.len() as i32 * ROW_H + PAD * 2;
        d.draw_rectangle(PANEL_X, PANEL_Y, PANEL_W, panel_h, Color::new(255, 255, 255, 200));

        for (i, (label, active, is_header)) in items.iter().enumerate() {
            let row_y = PANEL_Y + PAD + i as i32 * ROW_H;

            if *is_header {
                d.draw_line(
                    PANEL_X + PAD,
                    row_y,
                    PANEL_X + PANEL_W - PAD,
                    row_y,
                    Color::new(100, 100, 100, 255),
                );
            }

            if i == cursor_index {
                d.draw_rectangle(
                    PANEL_X,
                    row_y,
                    PANEL_W,
                    ROW_H,
                    Color::new(0, 0, 0, 40),
                );
            }

            let text = if *is_header {
                label.to_string()
            } else {
                let prefix = if *active { "> " } else { "  " };
                format!("{}{}", prefix, label.trim_start())
            };
            let color = if *is_header {
                Color::new(80, 80, 80, 255)
            } else if i == cursor_index {
                Color::BLACK
            } else {
                Color::new(60, 60, 60, 255)
            };
            d.draw_text(&text, PANEL_X + PAD, row_y + 4, FONT_SIZE, color);
        }
    }

    fn handle_filter_input(
        &mut self,
        handle: &RaylibHandle,
        config: &mut NoiseConfig,
        dt: f32,
    ) -> MenuAction {
        let (param, is_redistribution) = match self.level {
            MenuLevel::ContinentParams(ref mut p) => (p, false),
            MenuLevel::RedistributionParams(ref mut p) => (p, true),
            _ => return MenuAction::None,
        };

        if handle.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.level = MenuLevel::Root;
            return MenuAction::None;
        }
        if handle.is_key_pressed(KeyboardKey::KEY_DOWN) {
            *param = if is_redistribution {
                match param {
                    FilterParam::RedistributionExp => FilterParam::RedistributionElevationDep,
                    FilterParam::RedistributionElevationDep => FilterParam::RedistributionSmoothstep,
                    FilterParam::RedistributionSmoothstep => FilterParam::RedistributionExp,
                    _ => FilterParam::RedistributionExp,
                }
            } else {
                match param {
                    FilterParam::ContinentFreq => FilterParam::ContinentOctaves,
                    FilterParam::ContinentOctaves => FilterParam::ContinentBlend,
                    FilterParam::ContinentBlend => FilterParam::ContinentLandBias,
                    FilterParam::ContinentLandBias => FilterParam::ContinentFreq,
                    _ => FilterParam::ContinentFreq,
                }
            };
        }
        if handle.is_key_pressed(KeyboardKey::KEY_UP) {
            *param = if is_redistribution {
                match param {
                    FilterParam::RedistributionExp => FilterParam::RedistributionSmoothstep,
                    FilterParam::RedistributionElevationDep => FilterParam::RedistributionExp,
                    FilterParam::RedistributionSmoothstep => FilterParam::RedistributionElevationDep,
                    _ => FilterParam::RedistributionExp,
                }
            } else {
                match param {
                    FilterParam::ContinentFreq => FilterParam::ContinentLandBias,
                    FilterParam::ContinentOctaves => FilterParam::ContinentFreq,
                    FilterParam::ContinentBlend => FilterParam::ContinentOctaves,
                    FilterParam::ContinentLandBias => FilterParam::ContinentBlend,
                    _ => FilterParam::ContinentFreq,
                }
            };
        }

        if handle.is_key_pressed(KeyboardKey::KEY_ENTER) {
            match param {
                FilterParam::RedistributionElevationDep => {
                    config.redistribution_params.elevation_dependent =
                        !config.redistribution_params.elevation_dependent;
                    return MenuAction::Regen;
                }
                FilterParam::RedistributionSmoothstep => {
                    config.redistribution_params.use_smoothstep =
                        !config.redistribution_params.use_smoothstep;
                    return MenuAction::Regen;
                }
                _ => {}
            }
        }

        let left = handle.is_key_down(KeyboardKey::KEY_LEFT);
        let right = handle.is_key_down(KeyboardKey::KEY_RIGHT);

        if left || right {
            self.key_repeat_timer += dt;
            if self.key_repeat_timer >= KEY_REPEAT_DELAY {
                self.key_repeat_timer = 0.0;
                let cp = &mut config.continent_params;
                if right {
                    match param {
                        FilterParam::ContinentFreq => {
                            cp.frequency = (cp.frequency + 0.001).min(0.05)
                        }
                        FilterParam::ContinentOctaves => {
                            cp.octaves = cp.octaves.saturating_add(1).min(6)
                        }
                        FilterParam::ContinentBlend => {
                            cp.blend_strength = (cp.blend_strength + 0.05).min(1.0)
                        }
                        FilterParam::ContinentLandBias => {
                            cp.land_bias = (cp.land_bias + 0.05).min(0.5)
                        }
                        FilterParam::RedistributionExp => {
                            config.redistribution_params.exponent =
                                (config.redistribution_params.exponent + 0.1).min(4.0)
                        }
                        _ => {}
                    }
                } else {
                    match param {
                        FilterParam::ContinentFreq => {
                            cp.frequency = (cp.frequency - 0.001).max(0.001)
                        }
                        FilterParam::ContinentOctaves => {
                            cp.octaves = cp.octaves.saturating_sub(1).max(1)
                        }
                        FilterParam::ContinentBlend => {
                            cp.blend_strength = (cp.blend_strength - 0.05).max(0.0)
                        }
                        FilterParam::ContinentLandBias => {
                            cp.land_bias = (cp.land_bias - 0.05).max(0.0)
                        }
                        FilterParam::RedistributionExp => {
                            config.redistribution_params.exponent =
                                (config.redistribution_params.exponent - 0.1).max(0.1)
                        }
                        _ => {}
                    }
                }
                return MenuAction::Regen;
            }
        } else {
            self.key_repeat_timer = KEY_REPEAT_DELAY;
        }

        MenuAction::None
    }

    fn draw_filter_params(
        &self,
        d: &mut RaylibDrawHandle,
        config: &NoiseConfig,
        param: &FilterParam,
    ) {
        const PANEL_X: i32 = WINDOW_WIDTH - 270;
        const PANEL_Y: i32 = 10;
        const PANEL_W: i32 = 260;
        const ROW_H: i32 = 24;
        const FONT_SIZE: i32 = 20;
        const PAD: i32 = 8;

        let cp = &config.continent_params;
        let continent_items: &[(&str, String, bool)] = &[
            (
                "freq",
                format!("{:.3}", cp.frequency),
                matches!(param, FilterParam::ContinentFreq),
            ),
            (
                "octaves",
                format!("{}", cp.octaves),
                matches!(param, FilterParam::ContinentOctaves),
            ),
            (
                "blend",
                format!("{:.2}", cp.blend_strength),
                matches!(param, FilterParam::ContinentBlend),
            ),
            (
                "land bias",
                format!("{:.2}", cp.land_bias),
                matches!(param, FilterParam::ContinentLandBias),
            ),
        ];
        let rp = &config.redistribution_params;
        let redistribution_items: &[(&str, String, bool)] = &[
            (
                "exponent",
                format!("{:.1}", rp.exponent),
                matches!(param, FilterParam::RedistributionExp),
            ),
            (
                "elev dependent",
                if rp.elevation_dependent { "on".into() } else { "off".into() },
                matches!(param, FilterParam::RedistributionElevationDep),
            ),
            (
                "smoothstep",
                if rp.use_smoothstep { "on".into() } else { "off".into() },
                matches!(param, FilterParam::RedistributionSmoothstep),
            ),
        ];
        let (header, items): (&str, &[(&str, String, bool)]) = match param {
            FilterParam::RedistributionExp
            | FilterParam::RedistributionElevationDep
            | FilterParam::RedistributionSmoothstep => ("REDISTRIBUTION", redistribution_items),
            _ => ("CONTINENT MASK", continent_items),
        };

        let panel_h = (items.len() as i32 + 2) * ROW_H + PAD * 2;
        d.draw_rectangle(PANEL_X, PANEL_Y, PANEL_W, panel_h, Color::new(255, 255, 255, 200));
        d.draw_text(
            "ESC to go back",
            PANEL_X + PAD,
            PANEL_Y + PAD,
            FONT_SIZE,
            Color::new(80, 80, 80, 255),
        );
        d.draw_text(
            header,
            PANEL_X + PAD,
            PANEL_Y + PAD + ROW_H,
            FONT_SIZE,
            Color::new(80, 80, 80, 255),
        );

        for (i, (label, value, focused)) in items.iter().enumerate() {
            let row_y = PANEL_Y + PAD + (i as i32 + 2) * ROW_H;
            if *focused {
                d.draw_rectangle(PANEL_X, row_y, PANEL_W, ROW_H, Color::new(0, 0, 0, 40));
            }
            let prefix = if *focused { "> " } else { "  " };
            let color = if *focused {
                Color::BLACK
            } else {
                Color::new(60, 60, 60, 255)
            };
            d.draw_text(
                &format!("{}{}: {}", prefix, label, value),
                PANEL_X + PAD,
                row_y + 4,
                FONT_SIZE,
                color,
            );
        }
    }

    fn draw_params(&self, d: &mut RaylibDrawHandle, config: &NoiseConfig, param: &BaseSignalParam) {
        const PANEL_X: i32 = WINDOW_WIDTH - 270;
        const PANEL_Y: i32 = 10;
        const PANEL_W: i32 = 260;
        const ROW_H: i32 = 24;
        const FONT_SIZE: i32 = 20;
        const PAD: i32 = 8;

        let p = &config.noise_params;
        let items: &[(&str, String, bool, bool)] = &[
            (
                "frequency",
                format!("{:.3}", p.frequency),
                matches!(param, BaseSignalParam::Frequency),
                true,
            ),
            (
                "octaves",
                format!("{}", p.octaves),
                matches!(param, BaseSignalParam::Octaves),
                matches!(config.base_signal, BaseSignal::Fbm),
            ),
            (
                "persistence",
                format!("{:.3}", p.persistence),
                matches!(param, BaseSignalParam::Persistence),
                matches!(config.base_signal, BaseSignal::Fbm),
            ),
            (
                "lacunarity",
                format!("{:.3}", p.lacunarity),
                matches!(param, BaseSignalParam::Lacunarity),
                matches!(config.base_signal, BaseSignal::Fbm),
            ),
        ];

        let panel_h = (items.len() as i32 + 1) * ROW_H + PAD * 2;
        d.draw_rectangle(PANEL_X, PANEL_Y, PANEL_W, panel_h, Color::new(255, 255, 255, 200));
        d.draw_text(
            "ESC to go back",
            PANEL_X + PAD,
            PANEL_Y + PAD,
            FONT_SIZE,
            Color::new(80, 80, 80, 255),
        );

        for (i, (label, value, focused, applicable)) in items.iter().enumerate() {
            let row_y = PANEL_Y + PAD + (i as i32 + 1) * ROW_H;
            if *focused && *applicable {
                d.draw_rectangle(
                    PANEL_X,
                    row_y,
                    PANEL_W,
                    ROW_H,
                    Color::new(0, 0, 0, 40),
                );
            }
            let prefix = if *focused && *applicable { "> " } else { "  " };
            let color = if !applicable {
                Color::new(180, 180, 180, 255) // dimmed
            } else if *focused {
                Color::BLACK
            } else {
                Color::new(60, 60, 60, 255)
            };
            d.draw_text(
                &format!("{}{}: {}", prefix, label, value),
                PANEL_X + PAD,
                row_y + 4,
                FONT_SIZE,
                color,
            );
        }
    }

    fn handle_moisture_input(
        &mut self,
        handle: &RaylibHandle,
        config: &mut NoiseConfig,
        dt: f32,
    ) -> MenuAction {
        let MenuLevel::MoistureParams(ref mut param) = self.level else {
            return MenuAction::None;
        };

        if handle.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.level = MenuLevel::Root;
            return MenuAction::None;
        }
        if handle.is_key_pressed(KeyboardKey::KEY_DOWN) {
            *param = match param {
                MoistureParam::Frequency => MoistureParam::Offset,
                MoistureParam::Offset => MoistureParam::Frequency,
            };
        }
        if handle.is_key_pressed(KeyboardKey::KEY_UP) {
            *param = match param {
                MoistureParam::Frequency => MoistureParam::Offset,
                MoistureParam::Offset => MoistureParam::Frequency,
            };
        }

        let left = handle.is_key_down(KeyboardKey::KEY_LEFT);
        let right = handle.is_key_down(KeyboardKey::KEY_RIGHT);

        if left || right {
            self.key_repeat_timer += dt;
            if self.key_repeat_timer >= KEY_REPEAT_DELAY {
                self.key_repeat_timer = 0.0;
                let mp = &mut config.moisture_params;
                if right {
                    match param {
                        MoistureParam::Frequency => mp.frequency = (mp.frequency + 0.001).min(0.05),
                        MoistureParam::Offset => mp.offset = (mp.offset + 0.05).min(1.0),
                    }
                } else {
                    match param {
                        MoistureParam::Frequency => mp.frequency = (mp.frequency - 0.001).max(0.001),
                        MoistureParam::Offset => mp.offset = (mp.offset - 0.05).max(-1.0),
                    }
                }
                return MenuAction::Regen;
            }
        } else {
            self.key_repeat_timer = KEY_REPEAT_DELAY;
        }

        MenuAction::None
    }

    fn draw_moisture_params(
        &self,
        d: &mut RaylibDrawHandle,
        config: &NoiseConfig,
        param: &MoistureParam,
    ) {
        const PANEL_X: i32 = WINDOW_WIDTH - 270;
        const PANEL_Y: i32 = 10;
        const PANEL_W: i32 = 260;
        const ROW_H: i32 = 24;
        const FONT_SIZE: i32 = 20;
        const PAD: i32 = 8;

        let mp = &config.moisture_params;
        let items: &[(&str, String, bool)] = &[
            (
                "frequency",
                format!("{:.3}", mp.frequency),
                matches!(param, MoistureParam::Frequency),
            ),
            (
                "offset",
                format!("{:.2}", mp.offset),
                matches!(param, MoistureParam::Offset),
            ),
        ];

        let panel_h = (items.len() as i32 + 2) * ROW_H + PAD * 2;
        d.draw_rectangle(PANEL_X, PANEL_Y, PANEL_W, panel_h, Color::new(255, 255, 255, 200));
        d.draw_text(
            "ESC to go back",
            PANEL_X + PAD,
            PANEL_Y + PAD,
            FONT_SIZE,
            Color::new(80, 80, 80, 255),
        );
        d.draw_text(
            "MOISTURE",
            PANEL_X + PAD,
            PANEL_Y + PAD + ROW_H,
            FONT_SIZE,
            Color::new(80, 80, 80, 255),
        );

        for (i, (label, value, focused)) in items.iter().enumerate() {
            let row_y = PANEL_Y + PAD + (i as i32 + 2) * ROW_H;
            if *focused {
                d.draw_rectangle(PANEL_X, row_y, PANEL_W, ROW_H, Color::new(0, 0, 0, 40));
            }
            let prefix = if *focused { "> " } else { "  " };
            let color = if *focused { Color::BLACK } else { Color::new(60, 60, 60, 255) };
            d.draw_text(
                &format!("{}{}: {}", prefix, label, value),
                PANEL_X + PAD,
                row_y + 4,
                FONT_SIZE,
                color,
            );
        }
    }
}

fn is_param_applicable(param: &BaseSignalParam, signal: &BaseSignal) -> bool {
    match signal {
        BaseSignal::Perlin => matches!(param, BaseSignalParam::Frequency),
        BaseSignal::Fbm => true,
        BaseSignal::Random => false,
    }
}
