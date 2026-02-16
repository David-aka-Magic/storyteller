// src-tauri/src/mask_generator.rs
//
// Mask Generator for ComfyUI Regional IP-Adapter
// ================================================
// Converts character region data (from LLM output) into color mask PNGs.
// Each character gets a unique color region; ComfyUI uses these masks
// to apply different IP-Adapter references to different parts of the image.
//
// Color assignments:
//   Character 0 (color_index 0): Red   #FF0000
//   Character 1 (color_index 1): Blue  #0000FF
//   Character 2 (color_index 2): Green #00FF00
//   Background:                         Black #000000
//
// Zero external dependencies — PNG encoding is done inline using
// uncompressed deflate blocks (valid PNG, just larger than compressed).

use crate::llm_parser::CharacterRegion;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Manager;


// ============================================================================
// CONSTANTS
// ============================================================================

/// Predefined mask colors. Index into this with `color_index`.
const MASK_COLORS: [[u8; 3]; 3] = [
    [255, 0, 0],   // Red   — Character 0
    [0, 0, 255],   // Blue  — Character 1
    [0, 255, 0],   // Green — Character 2
];

// ============================================================================
// TYPES
// ============================================================================

/// A single character's mask request — received from the frontend or scene processor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskCharacter {
    pub name: String,
    pub region: String,
    pub color_index: usize,
}

/// The full request to generate a mask image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskRequest {
    pub characters: Vec<MaskCharacter>,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
}

fn default_width() -> u32 { 512 }
fn default_height() -> u32 { 768 }

/// Result returned after mask generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskResult {
    /// Absolute path to the generated PNG file.
    pub path: String,
    /// Width of the generated image.
    pub width: u32,
    /// Height of the generated image.
    pub height: u32,
    /// How many character regions were drawn (excludes off-screen).
    pub regions_drawn: usize,
}

/// A pixel rectangle: (x, y, width, height) — all in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

// ============================================================================
// REGION → RECTANGLE MAPPING
// ============================================================================

/// Convert a region string to a pixel rectangle for the given image dimensions.
///
/// | Region             | Horizontal              | Vertical                 |
/// |--------------------|-------------------------|--------------------------|
/// | left               | Left third              | Full height              |
/// | center             | Middle third            | Full height              |
/// | right              | Right third             | Full height              |
/// | left-seated        | Left third              | Bottom 60%               |
/// | center-seated      | Middle third            | Bottom 60%               |
/// | right-seated       | Right third             | Bottom 60%               |
/// | left-background    | Left third, inner 60%   | Top 70%, inner 60%       |
/// | center-background  | Mid third, inner 60%    | Top 70%, inner 60%       |
/// | right-background   | Right third, inner 60%  | Top 70%, inner 60%       |
/// | off-screen         | None                    | None                     |
///
/// Background regions are "scaled smaller" — they occupy the inner 60% of their
/// column and vertical zone, centered within it, to visually represent depth.
///
/// Returns `None` for off-screen or unrecognized regions.
pub fn region_to_rect(region: &str, img_w: u32, img_h: u32) -> Option<Rect> {
    let region_enum = CharacterRegion::from_str_loose(region);

    let third_w = img_w / 3;

    // Determine column x-offset and width
    let (col_x, col_w) = match &region_enum {
        CharacterRegion::Left
        | CharacterRegion::LeftSeated
        | CharacterRegion::LeftBackground => (0, third_w),

        CharacterRegion::Center
        | CharacterRegion::CenterSeated
        | CharacterRegion::CenterBackground => (third_w, third_w),

        CharacterRegion::Right
        | CharacterRegion::RightSeated
        | CharacterRegion::RightBackground => (third_w * 2, img_w - third_w * 2), // absorb rounding

        CharacterRegion::OffScreen => return None,
        CharacterRegion::Other(_) => return None,
    };

    match &region_enum {
        // --- Full-height foreground ---
        CharacterRegion::Left | CharacterRegion::Center | CharacterRegion::Right => {
            Some(Rect { x: col_x, y: 0, w: col_w, h: img_h })
        }

        // --- Seated: bottom 60% of the column ---
        CharacterRegion::LeftSeated
        | CharacterRegion::CenterSeated
        | CharacterRegion::RightSeated => {
            let top_offset = (img_h as f32 * 0.40) as u32;
            Some(Rect {
                x: col_x,
                y: top_offset,
                w: col_w,
                h: img_h - top_offset,
            })
        }

        // --- Background: top 70%, inner 60% (centered) ---
        CharacterRegion::LeftBackground
        | CharacterRegion::CenterBackground
        | CharacterRegion::RightBackground => {
            let zone_h = (img_h as f32 * 0.70) as u32;
            let scale = 0.60_f32;

            let inner_w = (col_w as f32 * scale) as u32;
            let inner_h = (zone_h as f32 * scale) as u32;

            let inner_x = col_x + (col_w - inner_w) / 2;
            let inner_y = (zone_h - inner_h) / 2;

            Some(Rect {
                x: inner_x,
                y: inner_y,
                w: inner_w,
                h: inner_h,
            })
        }

        CharacterRegion::OffScreen | CharacterRegion::Other(_) => None,
    }
}

// ============================================================================
// MASK IMAGE GENERATION
// ============================================================================

/// Generate a raw RGB pixel buffer for the mask image.
/// Returns (buffer, regions_drawn). Buffer length = width * height * 3.
pub fn generate_mask_buffer(
    characters: &[MaskCharacter],
    width: u32,
    height: u32,
) -> (Vec<u8>, usize) {
    let pixel_count = (width * height) as usize;
    let mut buffer = vec![0u8; pixel_count * 3]; // all-black background

    let mut regions_drawn = 0usize;

    for ch in characters {
        if ch.color_index >= MASK_COLORS.len() {
            eprintln!(
                "[MaskGen] WARNING: color_index {} out of range for '{}', skipping",
                ch.color_index, ch.name
            );
            continue;
        }

        let color = MASK_COLORS[ch.color_index];

        if let Some(rect) = region_to_rect(&ch.region, width, height) {
            fill_rect(&mut buffer, width, &rect, color);
            regions_drawn += 1;
        }
    }

    (buffer, regions_drawn)
}

/// Fill a rectangle in the RGB buffer with a solid color.
fn fill_rect(buffer: &mut [u8], img_width: u32, rect: &Rect, color: [u8; 3]) {
    let stride = (img_width * 3) as usize;

    for row in rect.y..(rect.y + rect.h) {
        let row_start = (row as usize) * stride + (rect.x as usize) * 3;
        for col in 0..rect.w {
            let offset = row_start + (col as usize) * 3;
            if offset + 2 < buffer.len() {
                buffer[offset] = color[0];
                buffer[offset + 1] = color[1];
                buffer[offset + 2] = color[2];
            }
        }
    }
}

// ============================================================================
// PNG ENCODING (zero external dependencies)
// ============================================================================

/// Encode an RGB buffer as a valid PNG file.
/// Uses uncompressed deflate blocks — no compression crate needed.
/// Output is larger but 100% spec-compliant (readable by ComfyUI, Pillow, etc).
pub fn encode_png(buffer: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut png = Vec::new();

    // PNG signature
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8);  // bit depth
    ihdr.push(2);  // color type: RGB
    ihdr.push(0);  // compression: deflate
    ihdr.push(0);  // filter: adaptive
    ihdr.push(0);  // interlace: none
    write_png_chunk(&mut png, b"IHDR", &ihdr);

    // Build raw scanlines: [filter_byte=0][R,G,B...] per row
    let row_bytes = (width as usize) * 3;
    let mut raw_data = Vec::with_capacity((1 + row_bytes) * height as usize);
    for y in 0..height as usize {
        raw_data.push(0); // filter: None
        let start = y * row_bytes;
        raw_data.extend_from_slice(&buffer[start..start + row_bytes]);
    }

    // Zlib wrapper: header + uncompressed deflate blocks + adler32
    let mut zlib = Vec::new();
    zlib.push(0x78); // CMF
    zlib.push(0x01); // FLG

    let max_block = 65535usize;
    let chunks: Vec<&[u8]> = raw_data.chunks(max_block).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_final = i == chunks.len() - 1;
        zlib.push(if is_final { 0x01 } else { 0x00 });
        let len = chunk.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(chunk);
    }

    zlib.extend_from_slice(&adler32(&raw_data).to_be_bytes());
    write_png_chunk(&mut png, b"IDAT", &zlib);

    // IEND
    write_png_chunk(&mut png, b"IEND", &[]);

    png
}

fn write_png_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);

    let mut crc_buf = Vec::with_capacity(4 + data.len());
    crc_buf.extend_from_slice(chunk_type);
    crc_buf.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_buf).to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for n in 0..256u32 {
        let mut c = n;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
        table[n as usize] = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

// ============================================================================
// HIGH-LEVEL API
// ============================================================================

/// Generate a color mask PNG and write it to disk.
pub fn generate_mask(
    characters: &[MaskCharacter],
    width: u32,
    height: u32,
    output_dir: &Path,
    filename: Option<&str>,
) -> Result<MaskResult, String> {
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create mask output directory: {}", e))?;

    let (buffer, regions_drawn) = generate_mask_buffer(characters, width, height);
    let png_data = encode_png(&buffer, width, height);

    let fname = filename.map(String::from).unwrap_or_else(|| {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("mask_{}.png", ts)
    });
    let output_path = output_dir.join(&fname);

    std::fs::write(&output_path, &png_data)
        .map_err(|e| format!("Failed to write mask PNG: {}", e))?;

    Ok(MaskResult {
        path: output_path.to_string_lossy().to_string(),
        width,
        height,
        regions_drawn,
    })
}

// ============================================================================
// TAURI COMMAND
// ============================================================================

/// Generate a color mask PNG for ComfyUI Regional IP-Adapter.
///
/// Frontend usage:
/// ```typescript
/// const result = await invoke('generate_color_mask', {
///   characters: [
///     { name: "Marcus", region: "left", color_index: 0 },
///     { name: "Elena", region: "right-seated", color_index: 1 }
///   ],
///   width: 512,
///   height: 768
/// });
/// ```
#[tauri::command]
pub fn generate_color_mask(
    characters: Vec<MaskCharacter>,
    width: Option<u32>,
    height: Option<u32>,
    app: tauri::AppHandle,
) -> Result<MaskResult, String> {
    let w = width.unwrap_or(512);
    let h = height.unwrap_or(768);

    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        return Err(format!("Invalid dimensions: {}x{} (must be 1-4096)", w, h));
    }

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let masks_dir = app_data.join("masks");

    generate_mask(&characters, w, h, &masks_dir, None)
}

#[tauri::command]
pub fn save_mask_image(
    base64_data: String,
    filename: String,
    output_dir: String,
) -> Result<String, String> {
    let dir = Path::new(&output_dir);
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_data)
        .map_err(|e| format!("Invalid base64: {}", e))?;

    let path = dir.join(&filename);
    std::fs::write(&path, &bytes)
        .map_err(|e| format!("Failed to write mask file: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // --- region_to_rect tests ---

    #[test]
    fn test_left_full() {
        let r = region_to_rect("left", 512, 768).unwrap();
        assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 170, 768));
    }

    #[test]
    fn test_center_full() {
        let r = region_to_rect("center", 512, 768).unwrap();
        assert_eq!((r.x, r.y, r.w, r.h), (170, 0, 170, 768));
    }

    #[test]
    fn test_right_full_absorbs_rounding() {
        let r = region_to_rect("right", 512, 768).unwrap();
        assert_eq!(r.x, 340);
        assert_eq!(r.w, 172); // 512 - 340
        assert_eq!(r.h, 768);
    }

    #[test]
    fn test_seated_bottom_60() {
        let r = region_to_rect("right-seated", 512, 768).unwrap();
        let top = (768.0_f32 * 0.40) as u32;
        assert_eq!(r.y, top);
        assert_eq!(r.h, 768 - top);
    }

    #[test]
    fn test_background_smaller_centered() {
        let r = region_to_rect("center-background", 512, 768).unwrap();
        assert!(r.x > 170);
        assert!(r.w < 170);
        assert!(r.h < (768.0 * 0.70) as u32);
    }

    #[test]
    fn test_off_screen_returns_none() {
        assert!(region_to_rect("off-screen", 512, 768).is_none());
    }

    #[test]
    fn test_unknown_region_returns_none() {
        assert!(region_to_rect("floating", 512, 768).is_none());
    }

    #[test]
    fn test_case_and_separator_insensitive() {
        assert!(region_to_rect("LEFT", 512, 768).is_some());
        assert!(region_to_rect("Left-Seated", 512, 768).is_some());
        assert!(region_to_rect("center_background", 512, 768).is_some());
    }

    // --- Buffer generation tests ---

    #[test]
    fn test_buffer_single_red() {
        let chars = vec![
            MaskCharacter { name: "A".into(), region: "left".into(), color_index: 0 },
        ];
        let (buf, drawn) = generate_mask_buffer(&chars, 512, 768);
        assert_eq!(buf.len(), 512 * 768 * 3);
        assert_eq!(drawn, 1);
        assert_eq!(&buf[0..3], &[255, 0, 0]); // top-left = red
        assert_eq!(&buf[200 * 3..200 * 3 + 3], &[0, 0, 0]); // outside left third
    }

    #[test]
    fn test_buffer_two_characters() {
        let chars = vec![
            MaskCharacter { name: "A".into(), region: "left".into(), color_index: 0 },
            MaskCharacter { name: "B".into(), region: "right".into(), color_index: 1 },
        ];
        let (buf, drawn) = generate_mask_buffer(&chars, 512, 768);
        assert_eq!(drawn, 2);
        assert_eq!(&buf[0..3], &[255, 0, 0]);
        assert_eq!(&buf[511 * 3..511 * 3 + 3], &[0, 0, 255]);
    }

    #[test]
    fn test_buffer_off_screen_all_black() {
        let chars = vec![
            MaskCharacter { name: "Ghost".into(), region: "off-screen".into(), color_index: 0 },
        ];
        let (buf, drawn) = generate_mask_buffer(&chars, 512, 768);
        assert_eq!(drawn, 0);
        assert!(buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_buffer_invalid_color_index_skipped() {
        let chars = vec![
            MaskCharacter { name: "X".into(), region: "left".into(), color_index: 99 },
        ];
        let (_, drawn) = generate_mask_buffer(&chars, 512, 768);
        assert_eq!(drawn, 0);
    }

    #[test]
    fn test_three_characters_all_colors() {
        let chars = vec![
            MaskCharacter { name: "A".into(), region: "left".into(), color_index: 0 },
            MaskCharacter { name: "B".into(), region: "center-seated".into(), color_index: 1 },
            MaskCharacter { name: "C".into(), region: "right-background".into(), color_index: 2 },
        ];
        let (buf, drawn) = generate_mask_buffer(&chars, 512, 768);
        assert_eq!(drawn, 3);
        assert!(buf.chunks(3).any(|p| p == [255, 0, 0]));
        assert!(buf.chunks(3).any(|p| p == [0, 0, 255]));
        assert!(buf.chunks(3).any(|p| p == [0, 255, 0]));
    }

    #[test]
    fn test_landscape_768x512() {
        let chars = vec![
            MaskCharacter { name: "A".into(), region: "left".into(), color_index: 0 },
            MaskCharacter { name: "B".into(), region: "center".into(), color_index: 1 },
            MaskCharacter { name: "C".into(), region: "right".into(), color_index: 2 },
        ];
        let (buf, drawn) = generate_mask_buffer(&chars, 768, 512);
        assert_eq!(drawn, 3);
        assert_eq!(buf.len(), 768 * 512 * 3);
    }

    // --- PNG encoding tests ---

    #[test]
    fn test_png_signature() {
        let buf = vec![255, 0, 0, 0, 0, 255, 0, 255, 0, 0, 0, 0];
        let png = encode_png(&buf, 2, 2);
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }

    // --- Checksum tests ---

    #[test]
    fn test_crc32_known() {
        assert_eq!(crc32(b"IEND"), 0xAE426082);
    }

    #[test]
    fn test_adler32_known() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E60398);
    }

    // --- End-to-end file write tests ---

    #[test]
    fn test_generate_mask_writes_valid_png() {
        let dir = std::env::temp_dir().join("storyengine_mask_test");
        let _ = std::fs::remove_dir_all(&dir);

        let chars = vec![
            MaskCharacter { name: "Marcus".into(), region: "left".into(), color_index: 0 },
            MaskCharacter { name: "Elena".into(), region: "right-seated".into(), color_index: 1 },
        ];

        let result = generate_mask(&chars, 512, 768, &dir, Some("test.png")).unwrap();
        assert_eq!(result.regions_drawn, 2);
        assert!(PathBuf::from(&result.path).exists());

        let bytes = std::fs::read(&result.path).unwrap();
        assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_mask_creates_nested_dirs() {
        let dir = std::env::temp_dir().join("storyengine_mask_nested");
        let _ = std::fs::remove_dir_all(&dir);
        let deep = dir.join("a").join("b");

        let chars = vec![
            MaskCharacter { name: "A".into(), region: "center".into(), color_index: 2 },
        ];
        let result = generate_mask(&chars, 256, 256, &deep, Some("d.png")).unwrap();
        assert!(PathBuf::from(&result.path).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}