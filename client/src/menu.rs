use crate::chunk_loader::ChunkLoader;
use crate::config::{CHUNK_SIZE, TERRAIN_RESOLUTION};
use crate::planet::{PlanetConfig, PlanetType, height_to_color};

use noise::Perlin;
use raylib::prelude::*;

pub enum AppState {
    Menu,
    Loading,
    Playing,
}

#[derive(PartialEq)]
enum MenuRow {
    Planet,
    Enter,
    Quit,
}

impl MenuRow {
    fn next(&self) -> Self {
        match self {
            MenuRow::Planet => MenuRow::Enter,
            MenuRow::Enter => MenuRow::Quit,
            MenuRow::Quit => MenuRow::Planet,
        }
    }

    fn prev(&self) -> Self {
        match self {
            MenuRow::Planet => MenuRow::Quit,
            MenuRow::Enter => MenuRow::Planet,
            MenuRow::Quit => MenuRow::Enter,
        }
    }
}

pub enum MenuAction {
    None,
    EnterWorld,
    Quit,
}

pub struct MenuState {
    pub selected_planet: PlanetType,
    cursor: MenuRow,
    pub preview_texture: Option<Texture2D>,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            selected_planet: PlanetType::Volcanic,
            cursor: MenuRow::Planet,
            preview_texture: None,
        }
    }

    pub fn handle_input(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        seed: u64,
    ) -> MenuAction {
        if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            self.cursor = self.cursor.next();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            self.cursor = self.cursor.prev();
        }

        match self.cursor {
            MenuRow::Planet => {
                if rl.is_key_pressed(KeyboardKey::KEY_RIGHT) {
                    self.selected_planet = self.selected_planet.next();
                    self.generate_preview(rl, thread, seed);
                }
                if rl.is_key_pressed(KeyboardKey::KEY_LEFT) {
                    self.selected_planet = self.selected_planet.prev();
                    self.generate_preview(rl, thread, seed);
                }
                if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    self.generate_preview(rl, thread, seed);
                }
            }
            MenuRow::Enter => {
                if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    return MenuAction::EnterWorld;
                }
            }
            MenuRow::Quit => {
                if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    return MenuAction::Quit;
                }
            }
        }

        MenuAction::None
    }

    pub fn generate_preview(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread, seed: u64) {
        let planet = PlanetConfig::new_typed(seed, self.selected_planet.clone());
        let noise = Perlin::new(seed as u32);

        const PREVIEW_SIZE: i32 = 256;
        let world_size = planet.grid_size as f32 * CHUNK_SIZE as f32 * TERRAIN_RESOLUTION;
        let step = world_size / PREVIEW_SIZE as f32;

        let mut image = Image::gen_image_color(PREVIEW_SIZE, PREVIEW_SIZE, Color::BLACK);

        for pz in 0..PREVIEW_SIZE {
            for px in 0..PREVIEW_SIZE {
                let wx = px as f32 * step + step * 0.5;
                let wz = pz as f32 * step + step * 0.5;
                let h = ChunkLoader::get_height(wx, wz, &noise, &planet);
                let color = height_to_color(h, &planet.bands);
                image.draw_pixel(px, pz, color);
            }
        }

        self.preview_texture = Some(
            rl.load_texture_from_image(thread, &image)
                .expect("preview texture"),
        );
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle, screen_w: i32, screen_h: i32) {
        let cx = screen_w / 2;
        let cy = screen_h / 2;

        // Preview panel — left half, centered vertically
        let preview_size = 512;
        let preview_x = cx / 2 - preview_size / 2;
        let preview_y = cy - preview_size / 2;

        if let Some(tex) = &self.preview_texture {
            let src = Rectangle::new(0.0, 0.0, 256.0, 256.0);
            let dst = Rectangle::new(
                preview_x as f32,
                preview_y as f32,
                preview_size as f32,
                preview_size as f32,
            );
            d.draw_texture_pro(tex, src, dst, Vector2::zero(), 0.0, Color::WHITE);
        } else {
            d.draw_rectangle(
                preview_x,
                preview_y,
                preview_size,
                preview_size,
                Color::new(30, 30, 30, 255),
            );
            d.draw_text(
                "Press Enter to preview",
                preview_x + 80,
                preview_y + preview_size / 2 - 10,
                20,
                Color::GRAY,
            );
        }

        // Menu panel — right half
        let menu_x = cx + 80;
        let row_h = 50;
        let mut y = cy - row_h;

        d.draw_text("TERRAIN EXPLORER", menu_x, y - 80, 32, Color::WHITE);
        d.draw_line(menu_x, y - 40, menu_x + 400, y - 40, Color::DARKGRAY);

        self.draw_row(
            d,
            menu_x,
            y,
            &format!("< {} >", self.selected_planet.label()),
            self.cursor == MenuRow::Planet,
            Some("Planet:"),
        );
        y += row_h;

        self.draw_row(d, menu_x, y, "[ Enter World ]", self.cursor == MenuRow::Enter, None);
        y += row_h;

        self.draw_row(d, menu_x, y, "[ Quit ]", self.cursor == MenuRow::Quit, None);

        d.draw_text(
            "UP/DOWN: navigate   LEFT/RIGHT: change planet   ENTER: preview / select",
            menu_x,
            cy + row_h * 2 + 20,
            16,
            Color::DARKGRAY,
        );
    }

    fn draw_row(
        &self,
        d: &mut RaylibDrawHandle,
        x: i32,
        y: i32,
        label: &str,
        active: bool,
        prefix: Option<&str>,
    ) {
        if active {
            d.draw_rectangle(x - 10, y - 5, 420, 36, Color::new(50, 50, 80, 255));
        }

        let color = if active { Color::WHITE } else { Color::GRAY };

        if let Some(p) = prefix {
            d.draw_text(p, x, y, 24, Color::DARKGRAY);
            d.draw_text(label, x + 100, y, 24, color);
        } else {
            d.draw_text(label, x, y, 24, color);
        }
    }
}
