//! Hash-pinned loading of the original grass and foliage climate palettes.

use image::{ImageFormat, RgbaImage};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::io;
use std::path::Path;

const PALETTE_WIDTH: u32 = 256;
const PALETTE_HEIGHT: u32 = 256;
const GRASS_SHA256: &str = "d9b2d9073f06013b9f36b1c61a91678be478760abff31fae942ee52ad898ca1b";
const FOLIAGE_SHA256: &str = "644a527dd7daea5a195bbed056a6a41e295b6043a05621f4c5c263a25fb171cc";

pub struct ColorMaps {
    pub grass: Box<[u32; 65_536]>,
    pub foliage: Box<[u32; 65_536]>,
}

pub fn load_color_maps(grass: &Path, foliage: &Path) -> Result<ColorMaps, Box<dyn Error>> {
    Ok(ColorMaps {
        grass: load_palette(grass, GRASS_SHA256, "grasscolor.png")?,
        foliage: load_palette(foliage, FOLIAGE_SHA256, "foliagecolor.png")?,
    })
}

fn load_palette(
    path: &Path,
    expected_hash: &str,
    label: &str,
) -> Result<Box<[u32; 65_536]>, Box<dyn Error>> {
    let encoded = std::fs::read(path)?;
    let actual_hash = format!("{:x}", Sha256::digest(&encoded));
    if actual_hash != expected_hash {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} SHA-256 mismatch: {actual_hash}"),
        )
        .into());
    }
    let image = image::load_from_memory_with_format(&encoded, ImageFormat::Png)?.to_rgba8();
    palette_from_rgba(&image, label)
}

fn palette_from_rgba(image: &RgbaImage, label: &str) -> Result<Box<[u32; 65_536]>, Box<dyn Error>> {
    if image.dimensions() != (PALETTE_WIDTH, PALETTE_HEIGHT) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{label} must be 256x256, got {}x{}",
                image.width(),
                image.height()
            ),
        )
        .into());
    }
    let pixels: Vec<u32> = image
        .pixels()
        .map(|pixel| {
            let [red, green, blue, alpha] = pixel.0;
            u32::from_be_bytes([alpha, red, green, blue])
        })
        .collect();
    let palette: Box<[u32; 65_536]> = pixels.into_boxed_slice().try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "palette must contain 65536 pixels",
        )
    })?;
    Ok(palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_preserves_row_major_argb_channels_and_requires_exact_dimensions() {
        let mut image = RgbaImage::new(256, 256);
        image.put_pixel(0, 0, image::Rgba([0x12, 0x34, 0x56, 0x78]));
        image.put_pixel(255, 0, image::Rgba([0xaa, 0xbb, 0xcc, 0xdd]));
        image.put_pixel(0, 1, image::Rgba([0x01, 0x23, 0x45, 0x67]));
        let palette = palette_from_rgba(&image, "synthetic").unwrap();
        assert_eq!(palette[0], 0x7812_3456);
        assert_eq!(palette[255], 0xddaa_bbcc);
        assert_eq!(palette[256], 0x6701_2345);

        let wrong_size = RgbaImage::new(255, 256);
        assert!(palette_from_rgba(&wrong_size, "synthetic").is_err());
    }
}
