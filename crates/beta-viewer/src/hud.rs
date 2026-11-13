//! Fixed, render-only first-person HUD layout in native GUI pixels.
//!
//! Atlas resources/layout constants are pinned by HUD-001-C01; this module keeps
//! screen layout, ZIP resource extraction, and coordinate conversion testable.

use std::{error::Error, fs, io::Read, path::Path};

pub const GUI_SHA256: &str = "e223bea0b5004a3b5497607c6d529daf840396526a46eff75c299364cf9dc795";
pub const ICONS_SHA256: &str = "8b5a8cfd44b933559c36aecaa722165ad048f1e942f1eeae616554124d6dac43";
pub const INVENTORY_SHA256: &str =
    "53dd5bde88588215b1700aee4a7dd70a3049e7e53f914982774ebdc26d6862b0";
pub const ITEMS_SHA256: &str = "545699e0c5f655c1e1876bb9f2fa5d93c4fd514dae3802d1cea43ae0003343f3";
pub const FONT_SHA256: &str = "6ded9b9e06a61beaa61bf8542c46b5d690d45fdaf42e6539afcbc02e4c7a6408";
pub const MENU_BACKGROUND_SHA256: &str =
    "416becee17bd2b22999e439e1560aff906f5fc876f47ba53e1001c548a320185";
pub const MENU_LOGO_SHA256: &str =
    "1ec37872aa65aa3f73b327db488472f6a301269b1a458e0d93108b652a487a82";
pub const MENU_LANG_SHA256: &str =
    "15309f7acb72e9d08b81cf8ef0bfda31e127112cd506eea2e268a4348efeb68a";
pub const CLIENT_JAR_SHA256: &str =
    "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";

pub fn load_atlases(
    client_jar: &Path,
) -> Result<(image::RgbaImage, image::RgbaImage), Box<dyn Error>> {
    let jar = fs::read(client_jar)?;
    verify_hash(&jar, CLIENT_JAR_SHA256, "pinned client JAR")?;
    let gui_bytes = extract_zip_resource(&jar, "gui/gui.png")?;
    let icons_bytes = extract_zip_resource(&jar, "gui/icons.png")?;
    let gui = decode_atlas(&gui_bytes, GUI_SHA256, "gui/gui.png")?;
    let icons = decode_atlas(&icons_bytes, ICONS_SHA256, "gui/icons.png")?;
    Ok((gui, icons))
}

pub fn load_inventory_atlases(
    client_jar: &Path,
) -> Result<(image::RgbaImage, image::RgbaImage), Box<dyn Error>> {
    let jar = fs::read(client_jar)?;
    verify_hash(&jar, CLIENT_JAR_SHA256, "pinned client JAR")?;
    let inventory_bytes = extract_zip_resource(&jar, "gui/inventory.png")?;
    let items_bytes = extract_zip_resource(&jar, "gui/items.png")?;
    let inventory = decode_atlas(&inventory_bytes, INVENTORY_SHA256, "gui/inventory.png")?;
    let items = decode_atlas(&items_bytes, ITEMS_SHA256, "gui/items.png")?;
    Ok((inventory, items))
}

pub fn load_font_atlas(client_jar: &Path) -> Result<image::RgbaImage, Box<dyn Error>> {
    let jar = fs::read(client_jar)?;
    verify_hash(&jar, CLIENT_JAR_SHA256, "pinned client JAR")?;
    let bytes = extract_zip_resource(&jar, "font/default.png")?;
    decode_atlas_sized(&bytes, FONT_SHA256, "font/default.png", (128, 128))
}

/// Load the original menu resources from the pinned JAR without extracting
/// copyrighted resource files to disk.
pub type MenuResourceImages = (
    image::RgbaImage,
    image::RgbaImage,
    image::RgbaImage,
    image::RgbaImage,
    String,
);

pub fn load_menu_resources(client_jar: &Path) -> Result<MenuResourceImages, Box<dyn Error>> {
    let jar = fs::read(client_jar)?;
    verify_hash(&jar, CLIENT_JAR_SHA256, "pinned client JAR")?;
    let background = decode_atlas_sized(
        &extract_zip_resource(&jar, "gui/background.png")?,
        MENU_BACKGROUND_SHA256,
        "gui/background.png",
        (16, 16),
    )?;
    let gui = decode_atlas(
        &extract_zip_resource(&jar, "gui/gui.png")?,
        GUI_SHA256,
        "gui/gui.png",
    )?;
    let font = decode_atlas_sized(
        &extract_zip_resource(&jar, "font/default.png")?,
        FONT_SHA256,
        "font/default.png",
        (128, 128),
    )?;
    let logo = decode_atlas_sized(
        &extract_zip_resource(&jar, "title/mclogo.png")?,
        MENU_LOGO_SHA256,
        "title/mclogo.png",
        (256, 256),
    )?;
    let lang = extract_zip_resource(&jar, "lang/en_US.lang")?;
    verify_hash(&lang, MENU_LANG_SHA256, "native menu language")?;
    let lang = String::from_utf8(lang)?;
    Ok((background, gui, font, logo, lang))
}

pub const MENU_SPLASHES_SHA256: &str =
    "47f63121551fb299c836642c5fd79e0e26771fdaba7689698fd703a25dd01c9e";

pub fn load_menu_splashes(client_jar: &Path) -> Result<String, Box<dyn Error>> {
    let jar = fs::read(client_jar)?;
    verify_hash(&jar, CLIENT_JAR_SHA256, "pinned client JAR")?;
    let bytes = extract_zip_resource(&jar, "title/splashes.txt")?;
    verify_hash(&bytes, MENU_SPLASHES_SHA256, "native menu splashes")?;
    Ok(String::from_utf8(bytes)?)
}

fn verify_hash(bytes: &[u8], expected: &str, label: &str) -> Result<(), Box<dyn Error>> {
    use sha2::Digest;
    let actual = format!("{:x}", sha2::Sha256::digest(bytes));
    if actual != expected {
        return Err(format!("{label} SHA-256 mismatch: {actual}").into());
    }
    Ok(())
}

/// Extract one exact ZIP/JAR member, supporting the stored/deflated methods used
/// by the pinned client. Resource bytes stay in memory and are never copied out.
fn extract_zip_resource(jar: &[u8], member: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    const EOCD: u32 = 0x0605_4b50;
    const CENTRAL: u32 = 0x0201_4b50;
    const LOCAL: u32 = 0x0403_4b50;
    fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, Box<dyn Error>> {
        Ok(u16::from_le_bytes(
            bytes
                .get(offset..offset + 2)
                .ok_or("truncated ZIP u16")?
                .try_into()?,
        ))
    }
    fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, Box<dyn Error>> {
        Ok(u32::from_le_bytes(
            bytes
                .get(offset..offset + 4)
                .ok_or("truncated ZIP u32")?
                .try_into()?,
        ))
    }
    let scan_start = jar.len().saturating_sub(65_557);
    let eocd = (scan_start..jar.len().saturating_sub(3))
        .rev()
        .find(|&offset| u32_at(jar, offset).ok() == Some(EOCD))
        .ok_or("ZIP end-of-central-directory record is missing")?;
    let disk = u16_at(jar, eocd + 4)?;
    let central_disk = u16_at(jar, eocd + 6)?;
    let entry_count = u16_at(jar, eocd + 10)? as usize;
    let disk_entry_count = u16_at(jar, eocd + 8)? as usize;
    let central_size = u32_at(jar, eocd + 12)? as usize;
    let central_start = u32_at(jar, eocd + 16)? as usize;
    let comment_len = u16_at(jar, eocd + 20)? as usize;
    if disk != 0 || central_disk != 0 || entry_count != disk_entry_count {
        return Err("multi-disk ZIP/JAR is unsupported".into());
    }
    if eocd + 22 + comment_len != jar.len() {
        return Err("malformed ZIP end record length".into());
    }
    let central_end = central_start
        .checked_add(central_size)
        .filter(|end| *end <= eocd)
        .ok_or("invalid ZIP central directory bounds")?;
    let mut cursor = central_start;
    let mut selected = None;
    for _ in 0..entry_count {
        if u32_at(jar, cursor)? != CENTRAL {
            return Err("malformed ZIP central directory entry".into());
        }
        let flags = u16_at(jar, cursor + 8)?;
        let method = u16_at(jar, cursor + 10)?;
        let compressed = u32_at(jar, cursor + 20)? as usize;
        let uncompressed = u32_at(jar, cursor + 24)? as usize;
        let name_len = u16_at(jar, cursor + 28)? as usize;
        let extra_len = u16_at(jar, cursor + 30)? as usize;
        let comment_len = u16_at(jar, cursor + 32)? as usize;
        let local_start = u32_at(jar, cursor + 42)? as usize;
        let name_start = cursor.checked_add(46).ok_or("ZIP name offset overflow")?;
        let name_end = name_start
            .checked_add(name_len)
            .ok_or("ZIP name length overflow")?;
        let entry_end = name_end
            .checked_add(extra_len)
            .and_then(|n| n.checked_add(comment_len))
            .filter(|end| *end <= central_end)
            .ok_or("invalid ZIP central entry bounds")?;
        let entry_name = std::str::from_utf8(
            jar.get(name_start..name_end)
                .ok_or("truncated ZIP member name")?,
        )?;
        if entry_name == member {
            if selected.is_some() {
                return Err(format!("duplicate ZIP member {member}").into());
            }
            selected = Some((flags, method, compressed, uncompressed, local_start));
        }
        cursor = entry_end;
    }
    if cursor != central_end {
        return Err("ZIP central directory size mismatch".into());
    }
    let (flags, method, compressed, uncompressed, local_start) =
        selected.ok_or_else(|| format!("ZIP/JAR member missing: {member}"))?;
    if flags & 0x41 != 0 {
        return Err(format!("encrypted ZIP member is unsupported: {member}").into());
    }
    if uncompressed > 1_048_576 {
        return Err(format!("ZIP member exceeds atlas size limit: {member}").into());
    }
    if u32_at(jar, local_start)? != LOCAL {
        return Err(format!("invalid local ZIP header: {member}").into());
    }
    let local_name = u16_at(jar, local_start + 26)? as usize;
    let local_extra = u16_at(jar, local_start + 28)? as usize;
    let data_start = local_start
        .checked_add(30)
        .and_then(|n| n.checked_add(local_name))
        .and_then(|n| n.checked_add(local_extra))
        .ok_or("ZIP data offset overflow")?;
    let data_end = data_start
        .checked_add(compressed)
        .ok_or("ZIP data length overflow")?;
    let compressed_data = jar
        .get(data_start..data_end)
        .ok_or("truncated ZIP member data")?;
    let output = match method {
        0 => compressed_data.to_vec(),
        8 => {
            let mut decoded = Vec::with_capacity(uncompressed);
            flate2::read::DeflateDecoder::new(compressed_data)
                .take(1_048_577)
                .read_to_end(&mut decoded)?;
            decoded
        }
        value => {
            return Err(format!("unsupported ZIP compression method {value} for {member}").into());
        }
    };
    if output.len() != uncompressed {
        return Err(format!("ZIP uncompressed size mismatch: {member}").into());
    }
    Ok(output)
}

fn decode_atlas(
    bytes: &[u8],
    expected_hash: &str,
    source: &str,
) -> Result<image::RgbaImage, Box<dyn Error>> {
    decode_atlas_sized(bytes, expected_hash, source, (256, 256))
}

fn decode_atlas_sized(
    bytes: &[u8],
    expected_hash: &str,
    source: &str,
    dimensions: (u32, u32),
) -> Result<image::RgbaImage, Box<dyn Error>> {
    verify_hash(bytes, expected_hash, &format!("native HUD atlas {source}"))?;
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.to_rgba8();
    if image.dimensions() != dimensions {
        return Err(format!(
            "native HUD atlas must be {}x{}: {source}",
            dimensions.0, dimensions.1
        )
        .into());
    }
    Ok(image)
}

#[cfg(test)]
mod resource_tests {
    use super::*;
    use flate2::{Compression, write::DeflateEncoder};
    use sha2::Digest;
    use std::io::Write;

    fn test_zip(member: &str, method: u16, payload: &[u8]) -> Vec<u8> {
        let compressed = match method {
            8 => {
                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(payload).unwrap();
                encoder.finish().unwrap()
            }
            _ => payload.to_vec(),
        };
        let name = member.as_bytes();
        let mut zip = Vec::new();
        let mut local = Vec::new();
        local.extend(0x0403_4b50_u32.to_le_bytes());
        local.extend(20_u16.to_le_bytes());
        local.extend(0_u16.to_le_bytes());
        local.extend(method.to_le_bytes());
        local.extend([0_u8; 4]);
        local.extend(0_u32.to_le_bytes()); // CRC is checked by the resource SHA.
        local.extend((compressed.len() as u32).to_le_bytes());
        local.extend((payload.len() as u32).to_le_bytes());
        local.extend((name.len() as u16).to_le_bytes());
        local.extend(0_u16.to_le_bytes());
        local.extend(name);
        local.extend(&compressed);
        zip.extend(&local);

        let mut central = Vec::new();
        central.extend(0x0201_4b50_u32.to_le_bytes());
        central.extend(20_u16.to_le_bytes());
        central.extend(20_u16.to_le_bytes());
        central.extend(0_u16.to_le_bytes());
        central.extend(method.to_le_bytes());
        central.extend([0_u8; 4]);
        central.extend(0_u32.to_le_bytes());
        central.extend((compressed.len() as u32).to_le_bytes());
        central.extend((payload.len() as u32).to_le_bytes());
        central.extend((name.len() as u16).to_le_bytes());
        central.extend(0_u16.to_le_bytes());
        central.extend(0_u16.to_le_bytes());
        central.extend(0_u16.to_le_bytes());
        central.extend(0_u16.to_le_bytes());
        central.extend(0_u32.to_le_bytes());
        central.extend(0_u32.to_le_bytes());
        central.extend(name);
        let central_offset = zip.len() as u32;
        zip.extend(&central);
        zip.extend(0x0605_4b50_u32.to_le_bytes());
        zip.extend(0_u16.to_le_bytes());
        zip.extend(0_u16.to_le_bytes());
        zip.extend(1_u16.to_le_bytes());
        zip.extend(1_u16.to_le_bytes());
        zip.extend((central.len() as u32).to_le_bytes());
        zip.extend(central_offset.to_le_bytes());
        zip.extend(0_u16.to_le_bytes());
        zip
    }

    #[test]
    fn extracts_stored_and_deflated_members_without_writing_files() {
        let payload = b"native-atlas-fixture";
        for method in [0, 8] {
            let archive = test_zip("gui/test.png", method, payload);
            assert_eq!(
                extract_zip_resource(&archive, "gui/test.png").unwrap(),
                payload
            );
        }
    }

    #[test]
    fn rejects_missing_member_malformed_archive_and_unsupported_method() {
        let archive = test_zip("gui/other.png", 0, b"x");
        assert!(extract_zip_resource(&archive, "gui/icons.png").is_err());
        assert!(extract_zip_resource(&archive[..12], "gui/other.png").is_err());
        let unsupported = test_zip("gui/icons.png", 99, b"x");
        assert!(extract_zip_resource(&unsupported, "gui/icons.png").is_err());
    }

    #[test]
    fn rejects_pinned_archive_and_resource_hash_mismatches() {
        assert!(verify_hash(b"altered archive", CLIENT_JAR_SHA256, "pinned client JAR").is_err());
        assert!(verify_hash(b"altered resource", GUI_SHA256, "native HUD atlas").is_err());
    }

    #[test]
    fn rejects_non_native_atlas_dimensions_after_valid_resource_hash() {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 4]));
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let bytes = encoded.into_inner();
        let digest = format!("{:x}", sha2::Sha256::digest(&bytes));
        assert!(decode_atlas(&bytes, &digest, "fixture.png").is_err());
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudLayout {
    pub scale: u32,
    pub hotbar: Rect,
    pub selected_slot: Rect,
    pub hearts: [Rect; 10],
    pub crosshair: Rect,
}

/// Native GUI uses an integer scale over a 320x240 minimum logical viewport.
pub fn gui_scale(width: u32, height: u32) -> u32 {
    if width == 0 || height == 0 {
        return 1;
    }
    let mut scale = 1;
    while scale < 4 && width / (scale + 1) >= 320 && height / (scale + 1) >= 240 {
        scale += 1;
    }
    scale
}

/// Compute pixel-aligned positions in physical pixels while retaining square
/// atlas pixels. `native_gui_scale` is the integer GUI scale selected by client.
/// Compute the HUD layout with the native selection frame at `selected_slot`.
pub fn layout_with_selection(
    width: u32,
    height: u32,
    native_gui_scale: u32,
    selected_slot: u8,
) -> HudLayout {
    let scale = native_gui_scale.max(1);
    let s = scale as f32;
    // qq computes ceil(display / scale), then GuiIngame uses integer halves
    // and integer logical-pixel offsets before the scaler multiplies them.
    let logical_width = width.div_ceil(scale);
    let logical_height = height.div_ceil(scale);
    let center_x = logical_width / 2;
    let center_y = logical_height / 2;
    let hotbar_x = center_x as i32 - 91;
    let hotbar_y = logical_height as i32 - 22;
    let x = hotbar_x as f32 * s;
    let y = hotbar_y as f32 * s;
    let hotbar_width = 182.0 * s;
    let hotbar_height = 22.0 * s;
    let selected = Rect {
        x: (center_x as i32 - 92 + i32::from(selected_slot.min(8)) * 20) as f32 * s,
        y: (logical_height as i32 - 23) as f32 * s,
        width: 24.0 * s,
        height: 22.0 * s,
    };
    let hearts = std::array::from_fn(|i| Rect {
        x: (center_x as i32 - 91 + i as i32 * 8) as f32 * s,
        y: (logical_height as i32 - 32) as f32 * s,
        width: 9.0 * s,
        height: 9.0 * s,
    });
    HudLayout {
        scale: native_gui_scale.max(1),
        hotbar: Rect {
            x,
            y,
            width: hotbar_width,
            height: hotbar_height,
        },
        selected_slot: selected,
        hearts,
        crosshair: Rect {
            x: (center_x as i32 - 7) as f32 * s,
            y: (center_y as i32 - 7) as f32 * s,
            width: 16.0 * s,
            height: 16.0 * s,
        },
    }
}

/// Convert a physical-pixel rectangle and atlas pixel UV rectangle to triangle
/// vertices in clip space. The 256x256 atlas is never rescaled in UV space.
pub fn quad_vertices(rect: Rect, uv: Rect, viewport: (u32, u32)) -> [[f32; 4]; 6] {
    let (width, height) = (viewport.0.max(1) as f32, viewport.1.max(1) as f32);
    let left = rect.x / width * 2.0 - 1.0;
    let right = (rect.x + rect.width) / width * 2.0 - 1.0;
    let top = 1.0 - rect.y / height * 2.0;
    let bottom = 1.0 - (rect.y + rect.height) / height * 2.0;
    let u0 = uv.x / 256.0;
    let v0 = uv.y / 256.0;
    let u1 = (uv.x + uv.width) / 256.0;
    let v1 = (uv.y + uv.height) / 256.0;
    [
        [left, top, u0, v0],
        [right, top, u1, v0],
        [right, bottom, u1, v1],
        [left, top, u0, v0],
        [right, bottom, u1, v1],
        [left, bottom, u0, v1],
    ]
}

/// Return GUI-atlas quads (frame, selected slot) and icons-atlas quads
/// (background/full heart pairs followed by crosshair), in native draw order.
pub fn native_vertices_with_selection(
    width: u32,
    height: u32,
    scale: u32,
    selected_slot: u8,
    health: i16,
) -> (Vec<[f32; 4]>, Vec<[f32; 4]>) {
    let layout = layout_with_selection(width, height, scale, selected_slot);
    let mut gui = Vec::with_capacity(12);
    gui.extend(quad_vertices(
        layout.hotbar,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 182.0,
            height: 22.0,
        },
        (width, height),
    ));
    gui.extend(quad_vertices(
        layout.selected_slot,
        Rect {
            x: 0.0,
            y: 22.0,
            width: 24.0,
            height: 22.0,
        },
        (width, height),
    ));
    let mut icons = Vec::with_capacity(126);
    for (i, heart) in layout.hearts.into_iter().enumerate() {
        let bg = Rect {
            x: 16.0,
            y: 0.0,
            width: 9.0,
            height: 9.0,
        };
        let heart_uv = if health >= 2 * i as i16 + 2 {
            Rect {
                x: 52.0,
                y: 0.0,
                width: 9.0,
                height: 9.0,
            }
        } else if health == 2 * i as i16 + 1 {
            Rect {
                x: 61.0,
                y: 0.0,
                width: 9.0,
                height: 9.0,
            }
        } else {
            bg
        };
        let full = Rect {
            x: heart_uv.x,
            y: 0.0,
            width: 9.0,
            height: 9.0,
        };
        icons.extend(quad_vertices(heart, bg, (width, height)));
        if health > 2 * i as i16 {
            icons.extend(quad_vertices(heart, full, (width, height)));
        }
    }
    icons.extend(quad_vertices(
        layout.crosshair,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 16.0,
            height: 16.0,
        },
        (width, height),
    ));
    (gui, icons)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_scale_is_integer_and_respects_minimum_logical_viewport() {
        assert_eq!(gui_scale(319, 900), 1);
        assert_eq!(gui_scale(640, 480), 2);
        assert_eq!(gui_scale(1280, 720), 3);
        assert_eq!(gui_scale(1920, 1080), 4);
        assert_eq!(gui_scale(0, 0), 1);
    }

    #[test]
    fn hud_is_centered_and_bottom_anchored_at_multiple_aspects() {
        for (w, h, scale) in [(1280, 720, 3), (1920, 1080, 4), (640, 480, 2)] {
            let hud = layout_with_selection(w, h, scale, 0);
            assert_eq!(
                hud.hotbar.x,
                ((w.div_ceil(scale) / 2) as i32 - 91) as f32 * scale as f32
            );
            assert_eq!(hud.hotbar.y + hud.hotbar.height, h as f32);
            assert_eq!(hud.hearts.len(), 10);
            assert_eq!(
                hud.crosshair.x + hud.crosshair.width / 2.0,
                ((w.div_ceil(scale) / 2) as f32 + 1.0) * scale as f32
            );
            assert_eq!(
                hud.crosshair.y + hud.crosshair.height / 2.0,
                ((h.div_ceil(scale) / 2) as f32 + 1.0) * scale as f32
            );
            assert_eq!(hud.hotbar.width / hud.hotbar.height, 182.0 / 22.0);
        }
    }

    #[test]
    fn odd_logical_resolution_uses_native_ceil_then_integer_center() {
        let hud = layout_with_selection(1280, 720, 3, 0);
        assert_eq!(hud.hotbar.x, 366.0); // ceil(1280 / 3) / 2 - 91, then *3
        assert_eq!(hud.hotbar.y, 654.0);
        assert_eq!(hud.selected_slot.x, 363.0);
        assert_eq!(hud.hearts[0].x, 366.0);
        assert_eq!(hud.hearts[0].y, 624.0);
        assert_eq!(hud.crosshair.x, 618.0);
        assert_eq!(hud.crosshair.y, 339.0);
    }

    #[test]
    fn selection_frame_moves_in_twenty_pixel_native_slot_steps() {
        let zero = layout_with_selection(1280, 720, 3, 0).selected_slot;
        for selected in 0..9 {
            let slot = layout_with_selection(1280, 720, 3, selected).selected_slot;
            assert_eq!(slot.x, zero.x + f32::from(selected) * 20.0 * 3.0);
            assert_eq!(slot.y, zero.y);
            assert_eq!(slot.width, zero.width);
            assert_eq!(slot.height, zero.height);
        }
        assert_eq!(
            layout_with_selection(1280, 720, 3, 255).selected_slot,
            layout_with_selection(1280, 720, 3, 8).selected_slot
        );
    }

    #[test]
    fn atlas_uv_conversion_is_bounded_and_uses_full_256_atlas() {
        let vertices = quad_vertices(
            Rect {
                x: 1.0,
                y: 2.0,
                width: 16.0,
                height: 16.0,
            },
            Rect {
                x: 240.0,
                y: 240.0,
                width: 16.0,
                height: 16.0,
            },
            (320, 240),
        );
        assert_eq!(vertices[0], [-0.99375, 0.98333335, 0.9375, 0.9375]);
        assert_eq!(vertices[2][2..], [1.0, 1.0]);
        assert!(
            vertices
                .iter()
                .all(|v| v[2] >= 0.0 && v[2] <= 1.0 && v[3] >= 0.0 && v[3] <= 1.0)
        );
    }

    #[test]
    fn native_sprite_batches_use_pinned_atlas_bounds_and_empty_hotbar() {
        let (gui, icons) = native_vertices_with_selection(1280, 720, 2, 0, 20);
        assert_eq!(gui.len(), 12);
        assert_eq!(icons.len(), 126);
        assert!(
            gui.iter()
                .chain(&icons)
                .all(|v| { (0.0..=1.0).contains(&v[2]) && (0.0..=1.0).contains(&v[3]) })
        );
        // Each slot's background and full heart use the audited native 9x9 UVs.
        assert_eq!(&icons[0][2..], &[16.0 / 256.0, 0.0]);
        assert_eq!(&icons[2][2..], &[25.0 / 256.0, 9.0 / 256.0]);
        assert_eq!(&icons[6][2..], &[52.0 / 256.0, 0.0]);
        // The final quad is the centered 16x16 crosshair.
        assert_eq!(&icons[120][2..], &[0.0, 0.0]);
        assert_eq!(&icons[122][2..], &[16.0 / 256.0, 16.0 / 256.0]);
    }

    #[test]
    fn heart_states_keep_native_order_and_uvs_at_each_gui_scale() {
        for scale in [1, 2, 3, 4] {
            let (_, icons) = native_vertices_with_selection(1280, 720, scale, 0, 3);
            // Background is emitted for every slot; full, half, or no overlay
            // follows it, then the crosshair remains the final quad.
            assert_eq!(icons.len(), 6 * (10 + 2) + 6);
            assert_eq!(&icons[0][2..], &[16.0 / 256.0, 0.0]);
            assert_eq!(&icons[6][2..], &[52.0 / 256.0, 0.0]);
            assert_eq!(&icons[12][2..], &[16.0 / 256.0, 0.0]);
            assert_eq!(&icons[18][2..], &[61.0 / 256.0, 0.0]);
            assert_eq!(&icons[24][2..], &[16.0 / 256.0, 0.0]);
            assert_eq!(&icons[30][2..], &[16.0 / 256.0, 0.0]);
            assert_eq!(&icons[icons.len() - 6][2..], &[0.0, 0.0]);
        }
    }
}

/// Bubble quads for the native underwater HUD (`uq.a(FZII)V`, pinned Beta
/// client). Only ordinary air values 0..=300 are shown; negative drowning air
/// yields no bubbles. The caller decides whether the eyes are submerged.
///
/// Returned vertices use the icons atlas and the same clip coordinates as
/// [`native_vertices_with_selection`]. They can be appended to that function's
/// icons vector, provided the GPU buffer and draw count include them.
pub fn air_bubble_vertices(
    width: u32,
    height: u32,
    native_gui_scale: u32,
    air: i32,
    submerged: bool,
) -> Vec<[f32; 4]> {
    if !submerged {
        return Vec::new();
    }
    let air = air.clamp(0, 300);
    let full = (f64::from(air - 2) * 10.0 / 300.0).ceil() as i32;
    let total = (f64::from(air) * 10.0 / 300.0).ceil() as i32;
    let scale = native_gui_scale.max(1);
    let logical_width = width.div_ceil(scale);
    let logical_height = height.div_ceil(scale);
    let s = scale as f32;
    let mut result = Vec::with_capacity(total as usize * 6);
    for i in 0..total {
        let rect = Rect {
            x: (logical_width as i32 / 2 - 91 + i * 8) as f32 * s,
            y: (logical_height as i32 - 41) as f32 * s,
            width: 9.0 * s,
            height: 9.0 * s,
        };
        let uv = Rect {
            x: if i < full { 16.0 } else { 25.0 },
            y: 18.0,
            width: 9.0,
            height: 9.0,
        };
        result.extend(quad_vertices(rect, uv, (width, height)));
    }
    result
}

/// Render heart flash and low-health motion using the native GUI tick state.
#[derive(Clone, Copy, Debug)]
pub struct HeartAnimation {
    pub tick: i32,
    pub health: i32,
    pub previous_health: i32,
    pub hurt_timer: i32,
}

pub fn native_vertices_with_heart_animation(
    width: u32,
    height: u32,
    scale: u32,
    selected_slot: u8,
    animation: HeartAnimation,
) -> (Vec<[f32; 4]>, Vec<[f32; 4]>) {
    let (gui, _) = native_vertices_with_selection(width, height, scale, selected_slot, 20);
    let layout = layout_with_selection(width, height, scale, selected_slot);
    let sprites = beta_viewer::heart_animation::heart_sprites(
        animation.tick,
        animation.health,
        animation.previous_health,
        animation.hurt_timer,
    );
    let mut icons = Vec::with_capacity(186);
    for (mut heart, sprite) in layout.hearts.into_iter().zip(sprites) {
        heart.y += f32::from(sprite.y_offset) * layout.scale as f32;
        for u in [
            Some(sprite.background_u),
            sprite.previous_u,
            sprite.current_u,
        ]
        .into_iter()
        .flatten()
        {
            icons.extend(quad_vertices(
                heart,
                Rect {
                    x: f32::from(u),
                    y: 0.0,
                    width: 9.0,
                    height: 9.0,
                },
                (width, height),
            ));
        }
    }
    icons.extend(quad_vertices(
        layout.crosshair,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 16.0,
            height: 16.0,
        },
        (width, height),
    ));
    (gui, icons)
}

#[cfg(test)]
mod air_tests {
    use super::*;

    #[test]
    fn animated_hearts_show_flash_overlay_and_jitter() {
        let params = HeartAnimation {
            tick: 1,
            health: 4,
            previous_health: 20,
            hurt_timer: 15,
        };
        let (_, flash) = native_vertices_with_heart_animation(1280, 720, 3, 0, params);
        // Ten backgrounds, ten previous-health overlays, two current hearts, crosshair.
        assert_eq!(flash.len(), 23 * 6);
        assert_eq!(flash[0][2], 25.0 / 256.0);
        assert_eq!(flash[6][2], 70.0 / 256.0);
        let (_, normal) = native_vertices_with_heart_animation(
            1280,
            720,
            3,
            0,
            HeartAnimation {
                health: 20,
                previous_health: 20,
                hurt_timer: 0,
                ..params
            },
        );
        let (_, legacy) = native_vertices_with_selection(1280, 720, 3, 0, 20);
        assert_eq!(normal, legacy);
    }

    #[test]
    fn bubbles_follow_native_ceiling_boundaries() {
        for (air, full, partial) in [
            (-19, 0, 0),
            (0, 0, 0),
            (1, 0, 1),
            (2, 0, 1),
            (3, 1, 0),
            (30, 1, 0),
            (31, 1, 1),
            (32, 1, 1),
            (33, 2, 0),
            (298, 10, 0),
            (299, 10, 0),
            (300, 10, 0),
        ] {
            let vertices = air_bubble_vertices(1280, 720, 3, air, true);
            assert_eq!(vertices.len(), (full + partial) * 6, "air={air}");
            assert_eq!(
                vertices
                    .chunks_exact(6)
                    .filter(|q| q[0][2] == 16.0 / 256.0)
                    .count(),
                full,
                "air={air}"
            );
            assert_eq!(
                vertices
                    .chunks_exact(6)
                    .filter(|q| q[0][2] == 25.0 / 256.0)
                    .count(),
                partial,
                "air={air}"
            );
        }
        assert!(air_bubble_vertices(1280, 720, 3, 300, false).is_empty());
    }

    #[test]
    fn bubbles_align_above_hearts_at_odd_viewport_sizes() {
        let viewport = (1281, 721);
        let scale = 3;
        let layout = layout_with_selection(viewport.0, viewport.1, scale, 0);
        let first = air_bubble_vertices(viewport.0, viewport.1, scale, 300, true);
        let expected = quad_vertices(
            Rect {
                x: layout.hearts[0].x,
                y: layout.hearts[0].y - 9.0 * scale as f32,
                width: 9.0 * scale as f32,
                height: 9.0 * scale as f32,
            },
            Rect {
                x: 16.0,
                y: 18.0,
                width: 9.0,
                height: 9.0,
            },
            viewport,
        );
        assert_eq!(&first[..6], expected);
    }
}
