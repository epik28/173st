//! Native Beta 1.7.3 main-menu and world-list presentation primitives.

use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, error::Error, path::Path};

const SHEET_SIZE: u32 = 512;
const BACKGROUND_ORIGIN: (u32, u32) = (0, 0);
const GUI_ORIGIN: (u32, u32) = (0, 32);
const LOGO_ORIGIN: (u32, u32) = (256, 0);
const FONT_ORIGIN: (u32, u32) = (256, 256);
const WHITE_PIXEL: (u32, u32) = (511, 511);
const FONT_GLYPH_DRAW_WIDTH: f32 = 7.99;
const NATIVE_BACKGROUND_TINT: f32 = 64.0 / 255.0;

pub fn parse_splashes(resource: &str) -> Vec<String> {
    resource
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn choose_splash(lines: &[String], choice: usize) -> String {
    if lines.is_empty() {
        return "missingno".to_owned();
    }
    lines[choice % lines.len()].clone()
}

pub fn date_override(month: u32, day: u32) -> Option<&'static str> {
    match (month, day) {
        (11, 9) => Some("Happy birthday, ez!"),
        (6, 1) => Some("Happy birthday, Notch!"),
        (12, 24) => Some("Merry X-mas!"),
        (1, 1) => Some("Happy new year!"),
        _ => None,
    }
}

pub fn splash_for_date(random: String, month: u32, day: u32) -> String {
    date_override(month, day).unwrap_or(&random).to_owned()
}

pub fn splash_scale(text_width: u32, epoch_millis: u128) -> f32 {
    let phase = (epoch_millis % 1000) as f32 / 1000.0 * std::f32::consts::TAU;
    (100.0 / (text_width as f32 + 32.0)) * (1.8 - 0.1 * phase.sin().abs())
}

pub fn splash_transform(width: u32, text_width: u32, epoch_millis: u128) -> ([f32; 2], f32) {
    (
        [width as f32 / 2.0 + 90.0, 70.0],
        splash_scale(text_width, epoch_millis),
    )
}

#[allow(dead_code)]
pub fn system_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[derive(Clone)]
pub struct MenuAssets {
    pub atlas: image::RgbaImage,
    pub labels: HashMap<String, String>,
    glyphs: [Glyph; 256],
}

#[derive(Clone, Copy, Debug, Default)]
struct Glyph {
    width: u8,
}

impl MenuAssets {
    pub fn load(jar: &Path) -> Result<Self, Box<dyn Error>> {
        let (background, gui, font, logo, lang) = super::hud::load_menu_resources(jar)?;
        let mut atlas = image::RgbaImage::new(SHEET_SIZE, SHEET_SIZE);
        blit(&mut atlas, &background, BACKGROUND_ORIGIN);
        blit(&mut atlas, &gui, GUI_ORIGIN);
        blit(&mut atlas, &logo, LOGO_ORIGIN);
        blit(&mut atlas, &font, FONT_ORIGIN);
        atlas.put_pixel(WHITE_PIXEL.0, WHITE_PIXEL.1, image::Rgba([255; 4]));
        let glyphs = std::array::from_fn(|index| glyph(&font, index as u8));
        let labels = lang
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        Ok(Self {
            atlas,
            labels,
            glyphs,
        })
    }

    #[cfg(test)]
    fn synthetic() -> Self {
        let mut atlas = image::RgbaImage::new(SHEET_SIZE, SHEET_SIZE);
        // Give glyphs a deterministic opaque pixel so tests exercise atlas UVs
        // and font metric-derived advances without requiring a local JAR.
        for index in 0..256_u32 {
            atlas.put_pixel(
                FONT_ORIGIN.0 + (index % 16) * 8,
                FONT_ORIGIN.1 + (index / 16) * 8,
                image::Rgba([255; 4]),
            );
        }
        atlas.put_pixel(WHITE_PIXEL.0, WHITE_PIXEL.1, image::Rgba([255; 4]));
        let mut labels = HashMap::new();
        for (key, value) in [
            ("menu.singleplayer", "Singleplayer"),
            ("menu.multiplayer", "Multiplayer"),
            ("menu.mods", "Mods and Texture Packs"),
            ("menu.options", "Options..."),
            ("menu.quit", "Quit Game"),
            ("selectWorld.title", "Select World"),
            ("selectWorld.select", "Play Selected World"),
            ("selectWorld.rename", "Rename"),
            ("selectWorld.delete", "Delete"),
            ("selectWorld.create", "Create New World"),
            ("gui.cancel", "Cancel"),
        ] {
            labels.insert(key.to_owned(), value.to_owned());
        }
        Self {
            atlas,
            labels,
            glyphs: [Glyph { width: 8 }; 256],
        }
    }

    fn label<'a>(&'a self, key: &'a str) -> &'a str {
        self.labels.get(key).map(String::as_str).unwrap_or(key)
    }
}

fn blit(destination: &mut image::RgbaImage, source: &image::RgbaImage, at: (u32, u32)) {
    for y in 0..source.height() {
        for x in 0..source.width() {
            destination.put_pixel(at.0 + x, at.1 + y, *source.get_pixel(x, y));
        }
    }
}

fn glyph(font: &image::RgbaImage, character: u8) -> Glyph {
    if character == b' ' {
        return Glyph { width: 4 };
    }
    let x0 = u32::from(character % 16) * 8;
    let y0 = u32::from(character / 16) * 8;
    let right = (0..8)
        .rev()
        .find(|x| (0..8).any(|y| font.get_pixel(x0 + x, y0 + y).0[3] != 0));
    // The native sj constructor leaves i at -1 for a fully transparent cell,
    // then stores i + 2. Such a glyph therefore advances by one pixel.
    let Some(right) = right else {
        return Glyph { width: 1 };
    };
    Glyph {
        width: (right + 2) as u8,
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub scale: u32,
}

impl Layout {
    /// `setting` follows the client option: zero selects automatic scale;
    /// values one through three select that exact GUI scale.
    pub fn with_gui_scale(width: u32, height: u32, setting: u32) -> Self {
        let scale = if setting == 0 {
            super::hud::gui_scale(width, height).max(1)
        } else {
            setting.clamp(1, 3)
        };
        Self {
            width: width.div_ceil(scale),
            height: height.div_ceil(scale),
            scale,
        }
    }

    pub fn point(self, physical: (f32, f32)) -> (f32, f32) {
        (
            physical.0 / self.scale as f32,
            physical.1 / self.scale as f32,
        )
    }

    pub fn row_at(self, x: f32, y: f32, count: usize) -> Option<usize> {
        let top = 32.0;
        let bottom = self.height as f32 - 64.0;
        if !(32.0..self.width as f32 - 32.0).contains(&x) || y < top || y >= bottom {
            return None;
        }
        let row = ((y - top) / 36.0).floor() as usize;
        (row < count).then_some(row)
    }

    pub fn singleplayer(self, x: f32, y: f32) -> bool {
        self.button_hit(
            x,
            y,
            self.width as f32 / 2.0 - 100.0,
            self.height as f32 / 4.0 + 48.0,
            200.0,
            20.0,
        )
    }

    pub fn quit(self, x: f32, y: f32) -> bool {
        self.button_hit(
            x,
            y,
            self.width as f32 / 2.0 + 2.0,
            self.height as f32 / 4.0 + 132.0,
            98.0,
            20.0,
        )
    }

    pub fn select_world(self, x: f32, y: f32) -> bool {
        self.button_hit(
            x,
            y,
            self.width as f32 / 2.0 - 154.0,
            self.height as f32 - 52.0,
            150.0,
            20.0,
        )
    }

    pub fn cancel(self, x: f32, y: f32) -> bool {
        self.button_hit(
            x,
            y,
            self.width as f32 / 2.0 + 4.0,
            self.height as f32 - 28.0,
            150.0,
            20.0,
        )
    }

    fn button_hit(self, x: f32, y: f32, bx: f32, by: f32, width: f32, height: f32) -> bool {
        (bx..bx + width).contains(&x) && (by..by + height).contains(&y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    MainMenu,
    WorldList,
}

#[allow(clippy::too_many_arguments)]
pub fn vertices(
    size: (u32, u32),
    layout: Layout,
    assets: &MenuAssets,
    entries: &[beta_viewer::world_select::WorldEntry],
    selected: Option<usize>,
    first_visible: usize,
    page: Page,
    cursor: (f32, f32),
) -> Vec<Vertex> {
    let mut out = Vec::new();
    let scale = layout.scale as f32;
    // Native menu background is a 32-pixel quad sampled from the 16x16 tile.
    for y in (0..layout.height).step_by(32) {
        for x in (0..layout.width).step_by(32) {
            let tile_width = (layout.width - x).min(32) as f32;
            let tile_height = (layout.height - y).min(32) as f32;
            quad(
                &mut out,
                size,
                scale,
                [x as f32, y as f32, tile_width, tile_height],
                [
                    BACKGROUND_ORIGIN.0 as f32,
                    BACKGROUND_ORIGIN.1 as f32,
                    tile_width / 2.0,
                    tile_height / 2.0,
                ],
                [
                    NATIVE_BACKGROUND_TINT,
                    NATIVE_BACKGROUND_TINT,
                    NATIVE_BACKGROUND_TINT,
                    1.0,
                ],
            );
        }
    }
    match page {
        Page::MainMenu => draw_main_menu(&mut out, size, layout, assets, cursor),
        Page::WorldList => draw_world_list(
            &mut out,
            size,
            layout,
            assets,
            entries,
            selected,
            first_visible,
            cursor,
        ),
    }
    out
}

fn draw_main_menu(
    out: &mut Vec<Vertex>,
    size: (u32, u32),
    layout: Layout,
    assets: &MenuAssets,
    cursor: (f32, f32),
) {
    let w = layout.width as f32;
    let h = layout.height as f32;
    let center = w / 2.0;
    quad(
        out,
        size,
        layout.scale as f32,
        [center - 137.0, 30.0, 155.0, 44.0],
        [256.0, 0.0, 155.0, 44.0],
        [1.0; 4],
    );
    quad(
        out,
        size,
        layout.scale as f32,
        [center + 18.0, 30.0, 155.0, 44.0],
        [256.0, 45.0, 155.0, 44.0],
        [1.0; 4],
    );
    let y = h / 4.0 + 48.0;
    for (index, key) in ["menu.singleplayer", "menu.multiplayer", "menu.mods"]
        .into_iter()
        .enumerate()
    {
        let button_y = y + index as f32 * 24.0;
        let hover = inside(cursor, center - 100.0, button_y, 200.0, 20.0);
        draw_button(
            out,
            size,
            layout.scale as f32,
            [center - 100.0, button_y, 200.0, 20.0],
            if hover { 2 } else { 1 },
        );
        draw_centered_text(
            out,
            size,
            layout,
            assets,
            center,
            button_y + 6.0,
            assets.label(key),
            if hover {
                [1.0, 1.0, 160.0 / 255.0, 1.0]
            } else {
                [224.0 / 255.0, 224.0 / 255.0, 224.0 / 255.0, 1.0]
            },
            true,
        );
    }
    let bottom_y = h / 4.0 + 132.0;
    let options = [center - 100.0, bottom_y, 98.0, 20.0];
    let quit = [center + 2.0, bottom_y, 98.0, 20.0];
    for (rect, key) in [(options, "menu.options"), (quit, "menu.quit")] {
        let hover = inside(cursor, rect[0], rect[1], rect[2], rect[3]);
        draw_button(
            out,
            size,
            layout.scale as f32,
            rect,
            if hover { 2 } else { 1 },
        );
        draw_centered_text(
            out,
            size,
            layout,
            assets,
            rect[0] + rect[2] / 2.0,
            rect[1] + 6.0,
            assets.label(key),
            if hover {
                [1.0, 1.0, 160.0 / 255.0, 1.0]
            } else {
                [224.0 / 255.0, 224.0 / 255.0, 224.0 / 255.0, 1.0]
            },
            true,
        );
    }

    // Pinned fu.a draws these two shadowed strings after the logo/buttons.
    draw_shadowed_text_at(
        out,
        size,
        layout,
        assets,
        2.0,
        2.0,
        "Minecraft Beta 1.7.3",
        [80.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0, 1.0],
    );
    let copyright = "Copyright Mojang AB. Do not distribute.";
    draw_shadowed_text_at(
        out,
        size,
        layout,
        assets,
        w - text_width(assets, copyright) as f32 - 2.0,
        h - 10.0,
        copyright,
        [1.0; 4],
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_world_list(
    out: &mut Vec<Vertex>,
    size: (u32, u32),
    layout: Layout,
    assets: &MenuAssets,
    entries: &[beta_viewer::world_select::WorldEntry],
    selected: Option<usize>,
    first_visible: usize,
    cursor: (f32, f32),
) {
    let w = layout.width as f32;
    let h = layout.height as f32;
    draw_centered_text(
        out,
        size,
        layout,
        assets,
        w / 2.0,
        20.0,
        assets.label("selectWorld.title"),
        [1.0; 4],
        true,
    );
    let bottom = h - 64.0;
    let visible = ((bottom - 32.0) / 36.0).floor().max(0.0) as usize;
    for local in 0..visible {
        let Some(entry) = entries.get(first_visible + local) else {
            break;
        };
        let index = first_visible + local;
        let y = 32.0 + local as f32 * 36.0;
        if selected == Some(index) {
            solid_quad(
                out,
                size,
                layout.scale as f32,
                [w / 2.0 - 110.0, y - 2.0, 220.0, 36.0],
                [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0],
            );
            solid_quad(
                out,
                size,
                layout.scale as f32,
                [w / 2.0 - 109.0, y - 1.0, 218.0, 34.0],
                [1.0; 4],
            );
        }
        let disabled = entry.launch_reason.is_some();
        let primary = if disabled {
            [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0]
        } else {
            [1.0; 4]
        };
        draw_centered_text(
            out,
            size,
            layout,
            assets,
            w / 2.0,
            y + 1.0,
            &entry.display_name,
            primary,
            true,
        );
        let secondary = if let Some(reason) = &entry.launch_reason {
            format!("{} — {}", entry.folder, reason)
        } else {
            entry.folder.clone()
        };
        draw_centered_text(
            out,
            size,
            layout,
            assets,
            w / 2.0,
            y + 12.0,
            &secondary,
            [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0],
            true,
        );
    }
    let select = [w / 2.0 - 154.0, h - 52.0, 150.0, 20.0];
    let enable_select = selected.is_some_and(|index| {
        entries
            .get(index)
            .is_some_and(|entry| entry.launch_reason.is_none())
    });
    let cancel = [w / 2.0 + 4.0, h - 28.0, 150.0, 20.0];
    for (rect, key, enabled) in [
        (select, "selectWorld.select", enable_select),
        (
            [w / 2.0 - 154.0, h - 28.0, 70.0, 20.0],
            "selectWorld.rename",
            false,
        ),
        (
            [w / 2.0 - 74.0, h - 28.0, 70.0, 20.0],
            "selectWorld.delete",
            false,
        ),
        (
            [w / 2.0 + 4.0, h - 52.0, 150.0, 20.0],
            "selectWorld.create",
            false,
        ),
        (cancel, "gui.cancel", true),
    ] {
        let hover = enabled && inside(cursor, rect[0], rect[1], rect[2], rect[3]);
        draw_button(
            out,
            size,
            layout.scale as f32,
            rect,
            if hover {
                2
            } else if enabled {
                1
            } else {
                0
            },
        );
        draw_centered_text(
            out,
            size,
            layout,
            assets,
            rect[0] + rect[2] / 2.0,
            rect[1] + 6.0,
            assets.label(key),
            if !enabled {
                [160.0 / 255.0, 160.0 / 255.0, 160.0 / 255.0, 1.0]
            } else if hover {
                [1.0, 1.0, 160.0 / 255.0, 1.0]
            } else {
                [224.0 / 255.0, 224.0 / 255.0, 224.0 / 255.0, 1.0]
            },
            true,
        );
    }
}

fn inside(point: (f32, f32), x: f32, y: f32, width: f32, height: f32) -> bool {
    (x..x + width).contains(&point.0) && (y..y + height).contains(&point.1)
}

fn solid_quad(
    out: &mut Vec<Vertex>,
    size: (u32, u32),
    scale: f32,
    rect: [f32; 4],
    color: [f32; 4],
) {
    quad(
        out,
        size,
        scale,
        rect,
        [WHITE_PIXEL.0 as f32, WHITE_PIXEL.1 as f32, 1.0, 1.0],
        color,
    );
}

fn draw_button(out: &mut Vec<Vertex>, size: (u32, u32), scale: f32, rect: [f32; 4], state: u32) {
    let half = rect[2] / 2.0;
    let v = 46.0 + state as f32 * 20.0;
    quad(
        out,
        size,
        scale,
        [rect[0], rect[1], half, rect[3]],
        [GUI_ORIGIN.0 as f32, GUI_ORIGIN.1 as f32 + v, 100.0, 20.0],
        [1.0; 4],
    );
    quad(
        out,
        size,
        scale,
        [rect[0] + half, rect[1], half, rect[3]],
        [
            GUI_ORIGIN.0 as f32 + 100.0,
            GUI_ORIGIN.1 as f32 + v,
            100.0,
            20.0,
        ],
        [1.0; 4],
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_centered_text(
    out: &mut Vec<Vertex>,
    size: (u32, u32),
    layout: Layout,
    assets: &MenuAssets,
    center_x: f32,
    y: f32,
    text: &str,
    color: [f32; 4],
    shadow: bool,
) {
    // Beta's `Gui.drawCenteredString` subtracts integer `stringWidth / 2`.
    // Flooring the floating-point half-width moves odd-width labels one pixel
    // left of the native position.
    let x = center_x - (text_width(assets, text) / 2) as f32;
    if shadow {
        draw_shadowed_text_at(out, size, layout, assets, x, y, text, color);
    } else {
        draw_text(out, size, layout, assets, x, y, text, color);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_shadowed_text_at(
    out: &mut Vec<Vertex>,
    size: (u32, u32),
    layout: Layout,
    assets: &MenuAssets,
    x: f32,
    y: f32,
    text: &str,
    color: [f32; 4],
) {
    let channel = |component: f32| {
        let byte = (component * 255.0).round() as u8;
        f32::from((byte & 0xfc) >> 2) / 255.0
    };
    draw_text(
        out,
        size,
        layout,
        assets,
        x + 1.0,
        y + 1.0,
        text,
        [
            channel(color[0]),
            channel(color[1]),
            channel(color[2]),
            color[3],
        ],
    );
    draw_text(out, size, layout, assets, x, y, text, color);
}

fn text_width(assets: &MenuAssets, text: &str) -> u32 {
    let mut width = 0_u32;
    let mut skip = false;
    for character in text.chars() {
        if skip {
            skip = false;
            continue;
        }
        if character == '\u{a7}' {
            skip = true;
            continue;
        }
        let Some(byte) = char_to_font_byte(character) else {
            continue;
        };
        width += u32::from(assets.glyphs[byte as usize].width);
    }
    width
}

fn char_to_font_byte(character: char) -> Option<u8> {
    let value = u32::from(character);
    (value < 256).then_some(value as u8)
}

#[allow(clippy::too_many_arguments)]
fn draw_text(
    out: &mut Vec<Vertex>,
    size: (u32, u32),
    layout: Layout,
    assets: &MenuAssets,
    mut x: f32,
    y: f32,
    text: &str,
    color: [f32; 4],
) {
    let mut skip = false;
    for character in text.chars() {
        if skip {
            skip = false;
            continue;
        }
        if character == '\u{a7}' {
            skip = true;
            continue;
        }
        let Some(byte) = char_to_font_byte(character) else {
            continue;
        };
        let glyph = assets.glyphs[byte as usize];
        let width = f32::from(glyph.width);
        let uv_x = FONT_ORIGIN.0 as f32 + f32::from(byte % 16) * 8.0;
        let uv_y = FONT_ORIGIN.1 as f32 + f32::from(byte / 16) * 8.0;
        // Native sj display lists draw a 7.99×7.99 quad for every codepoint;
        // the independently scanned glyph metric controls only the next x.
        quad(
            out,
            size,
            layout.scale as f32,
            [x, y, FONT_GLYPH_DRAW_WIDTH, FONT_GLYPH_DRAW_WIDTH],
            [uv_x, uv_y, FONT_GLYPH_DRAW_WIDTH, FONT_GLYPH_DRAW_WIDTH],
            color,
        );
        x += width;
    }
}

fn quad(
    out: &mut Vec<Vertex>,
    size: (u32, u32),
    scale: f32,
    rect: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
) {
    let [x, y, width, height] = rect;
    if width <= 0.0 || height <= 0.0 || size.0 == 0 || size.1 == 0 {
        return;
    }
    let x0 = (2.0 * x * scale / size.0 as f32 - 1.0).clamp(-1.0, 1.0);
    let x1 = (2.0 * (x + width) * scale / size.0 as f32 - 1.0).clamp(-1.0, 1.0);
    let y0 = (1.0 - 2.0 * y * scale / size.1 as f32).clamp(-1.0, 1.0);
    let y1 = (1.0 - 2.0 * (y + height) * scale / size.1 as f32).clamp(-1.0, 1.0);
    let [u, v, uw, vh] = uv;
    let uv0 = [u / SHEET_SIZE as f32, v / SHEET_SIZE as f32];
    let uv1 = [(u + uw) / SHEET_SIZE as f32, (v + vh) / SHEET_SIZE as f32];
    let a = Vertex {
        position: [x0, y0],
        uv: uv0,
        color,
    };
    let b = Vertex {
        position: [x1, y0],
        uv: [uv1[0], uv0[1]],
        color,
    };
    let c = Vertex {
        position: [x1, y1],
        uv: uv1,
        color,
    };
    let d = Vertex {
        position: [x0, y1],
        uv: [uv0[0], uv1[1]],
        color,
    };
    out.extend_from_slice(&[a, b, c, a, c, d]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_resource_matches_java_trim_filter_and_order() {
        let parsed = parse_splashes("  first\r\n\t\r\nsecond  \n third\n");
        assert_eq!(parsed, ["first", "second", "third"]);
        assert_eq!(choose_splash(&parsed, 0), "first");
        assert_eq!(choose_splash(&parsed, 4), "second");
        assert_eq!(choose_splash(&[], 0), "missingno");
    }

    #[test]
    fn splash_date_overrides_use_native_month_day_contract() {
        assert_eq!(date_override(11, 9), Some("Happy birthday, ez!"));
        assert_eq!(date_override(6, 1), Some("Happy birthday, Notch!"));
        assert_eq!(date_override(12, 24), Some("Merry X-mas!"));
        assert_eq!(date_override(1, 1), Some("Happy new year!"));
        assert_eq!(splash_for_date("random".into(), 2, 29), "random");
    }

    #[test]
    fn splash_transform_has_native_anchor_and_bounded_wave() {
        let (anchor, scale) = splash_transform(427, 100, 0);
        assert_eq!(anchor, [303.5, 70.0]);
        assert!((scale - 1.363_636_4).abs() < 1e-6);
        let (_, peak) = splash_transform(427, 100, 250);
        assert!(peak < scale);
        assert!((splash_scale(100, 750) - peak).abs() < 1e-6);
    }

    #[test]
    fn auto_scaled_resolution_matches_pinned_native_profiles_and_resize() {
        for (width, height, scale, logical) in [
            (854, 480, 2, (427, 240)),
            (1280, 720, 3, (427, 240)),
            (1280, 800, 3, (427, 267)),
        ] {
            let layout = Layout::with_gui_scale(width, height, 0);
            assert_eq!(layout.scale, scale);
            assert_eq!((layout.width, layout.height), logical);
        }
        assert_eq!(
            Layout::with_gui_scale(1280, 720, 0).point((640.0, 360.0)),
            (640.0 / 3.0, 120.0)
        );
        for (width, height, setting, scale) in [(854, 480, 1, 1), (1280, 720, 3, 3)] {
            assert_eq!(Layout::with_gui_scale(width, height, setting).scale, scale);
        }
    }

    #[test]
    fn native_menu_buttons_use_scaled_geometry_and_hover_state() {
        let assets = MenuAssets::synthetic();
        let layout = Layout::with_gui_scale(1280, 720, 0);
        assert!(layout.singleplayer(640.0 / 3.0, 120.0));
        assert!(!layout.singleplayer(10.0, 10.0));
        assert!(layout.quit(
            layout.width as f32 / 2.0 + 50.0,
            layout.height as f32 / 4.0 + 140.0
        ));
        let vertices = vertices(
            (1280, 720),
            layout,
            &assets,
            &[],
            None,
            0,
            Page::MainMenu,
            (layout.width as f32 / 2.0, 110.0),
        );
        assert!(vertices.len() > 1000);
        assert!(vertices.iter().all(|v| {
            v.position
                .iter()
                .all(|p| p.is_finite() && (-1.0..=1.0).contains(p))
        }));
        // The native GUI button texture is sampled in its atlas region.
        assert!(
            vertices
                .iter()
                .any(|v| v.uv[0] >= GUI_ORIGIN.0 as f32 / 512.0
                    && v.uv[1] >= (GUI_ORIGIN.1 + 46) as f32 / 512.0)
        );
    }

    #[test]
    fn native_tiled_menu_background_uses_the_original_tessellator_tint() {
        let assets = MenuAssets::synthetic();
        let layout = Layout::with_gui_scale(854, 480, 1);
        let vertices = vertices(
            (854, 480),
            layout,
            &assets,
            &[],
            None,
            0,
            Page::MainMenu,
            (0.0, 0.0),
        );
        let expected = [64.0 / 255.0, 64.0 / 255.0, 64.0 / 255.0, 1.0];
        assert!(vertices[..6].iter().all(|vertex| vertex.color == expected));
    }

    #[test]
    fn native_font_draws_full_cell_while_advancing_by_scanned_metrics() {
        let mut assets = MenuAssets::synthetic();
        assets.glyphs[b'A' as usize].width = 5;
        assets.glyphs[b'B' as usize].width = 3;
        let layout = Layout {
            width: 100,
            height: 100,
            scale: 1,
        };
        let mut vertices = Vec::new();
        draw_text(
            &mut vertices,
            (100, 100),
            layout,
            &assets,
            10.0,
            20.0,
            "AB",
            [1.0; 4],
        );
        assert_eq!(vertices.len(), 12);

        let ndc_x = |x: f32| 2.0 * x / 100.0 - 1.0;
        let ndc_y = |y: f32| 1.0 - 2.0 * y / 100.0;
        assert_eq!(vertices[0].position, [ndc_x(10.0), ndc_y(20.0)]);
        assert_eq!(vertices[1].position[0], ndc_x(10.0 + FONT_GLYPH_DRAW_WIDTH));
        assert_eq!(vertices[6].position, [ndc_x(15.0), ndc_y(20.0)]);
        assert_eq!(vertices[7].position[0], ndc_x(15.0 + FONT_GLYPH_DRAW_WIDTH));
        assert_eq!(vertices[0].uv, [264.0 / 512.0, 288.0 / 512.0]);
        assert_eq!(vertices[6].uv, [272.0 / 512.0, 288.0 / 512.0]);
    }

    #[test]
    fn native_centered_text_uses_integer_half_width_for_odd_labels() {
        let mut assets = MenuAssets::synthetic();
        // Reproduce the pinned font metric width that makes the Mods label odd.
        let text = "Mods and Texture Packs";
        assets.glyphs.fill(Glyph { width: 4 });
        assert_eq!(text_width(&assets, text), 88);
        assets.glyphs[b'M' as usize].width = 5;
        assert_eq!(text_width(&assets, text), 89);

        let mut vertices = Vec::new();
        let layout = Layout {
            width: 854,
            height: 480,
            scale: 1,
        };
        draw_centered_text(
            &mut vertices,
            (854, 480),
            layout,
            &assets,
            427.0,
            100.0,
            text,
            [1.0; 4],
            false,
        );
        let expected_x = 427.0 - (89 / 2) as f32;
        let actual_x = (vertices[0].position[0] + 1.0) * 854.0 / 2.0;
        assert_eq!(actual_x, expected_x);
        assert_eq!(expected_x, 383.0);
    }

    #[test]
    fn blank_font_cells_keep_the_native_one_pixel_advance() {
        let blank = image::RgbaImage::new(128, 128);
        assert_eq!(glyph(&blank, b'A').width, 1);
        assert_eq!(glyph(&blank, b' ').width, 4);
    }

    #[test]
    fn main_menu_includes_pinned_version_and_copyright_decorations() {
        let assets = MenuAssets::synthetic();
        let layout = Layout::with_gui_scale(854, 480, 1);
        let vertices = vertices(
            (854, 480),
            layout,
            &assets,
            &[],
            None,
            0,
            Page::MainMenu,
            (0.0, 0.0),
        );
        let at = |x: f32, y: f32, color: [f32; 4]| {
            let position = [2.0 * x / 854.0 - 1.0, 1.0 - 2.0 * y / 480.0];
            vertices
                .iter()
                .any(|vertex| vertex.position == position && vertex.color == color)
        };
        assert!(at(
            2.0,
            2.0,
            [80.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0, 1.0]
        ));
        assert!(at(
            3.0,
            3.0,
            [20.0 / 255.0, 20.0 / 255.0, 20.0 / 255.0, 1.0]
        ));
        let footer_x =
            854.0 - text_width(&assets, "Copyright Mojang AB. Do not distribute.") as f32 - 2.0;
        assert!(at(footer_x, 470.0, [1.0; 4]));
        assert!(at(
            footer_x + 1.0,
            471.0,
            [63.0 / 255.0, 63.0 / 255.0, 63.0 / 255.0, 1.0]
        ));
    }

    #[test]
    fn world_list_rows_and_native_button_grid_fit_resized_scaled_viewport() {
        let assets = MenuAssets::synthetic();
        let layout = Layout::with_gui_scale(854, 480, 0);
        let entry = beta_viewer::world_select::WorldEntry {
            folder: "world-a".into(),
            display_name: "World A".into(),
            last_played: 4,
            spawn: [0, 64, 0],
            regions: vec![],
            launch_reason: None,
            pins: vec![],
        };
        assert_eq!(layout.row_at(40.0, 36.0, 1), Some(0));
        assert_eq!(layout.row_at(31.0, 36.0, 1), None);
        assert!(layout.select_world(
            layout.width as f32 / 2.0 - 100.0,
            layout.height as f32 - 48.0
        ));
        assert!(layout.cancel(
            layout.width as f32 / 2.0 + 20.0,
            layout.height as f32 - 20.0
        ));
        let vertices = vertices(
            (854, 480),
            layout,
            &assets,
            &[entry],
            Some(0),
            0,
            Page::WorldList,
            (0.0, 0.0),
        );
        assert!(vertices.len() > 200);
        assert!(vertices.iter().all(|v| {
            v.position
                .iter()
                .all(|p| p.is_finite() && (-1.0..=1.0).contains(p))
        }));
    }
}
