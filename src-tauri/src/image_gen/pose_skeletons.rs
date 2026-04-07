// src-tauri/src/image_gen/pose_skeletons.rs
//
// Generates OpenPose-compatible skeleton PNG images for each CharacterPose variant.
// Called once at startup to pre-generate any missing files.

use image::{Rgb, RgbImage};
use std::path::{Path, PathBuf};

// =============================================================================
// Pose definition
// =============================================================================

pub struct PoseDefinition {
    pub name: &'static str,
    /// 18 keypoints in normalized 0.0-1.0 space.
    /// OpenPose order: nose(0), neck(1), r_shoulder(2), r_elbow(3), r_wrist(4),
    ///   l_shoulder(5), l_elbow(6), l_wrist(7),
    ///   r_hip(8), r_knee(9), r_ankle(10),
    ///   l_hip(11), l_knee(12), l_ankle(13),
    ///   r_eye(14), l_eye(15), r_ear(16), l_ear(17)
    /// (-1.0, -1.0) = keypoint not visible / not present.
    pub keypoints: [(f32, f32); 18],
}

// =============================================================================
// Pose data
// =============================================================================

const STANDING: PoseDefinition = PoseDefinition {
    name: "standing",
    keypoints: [
        (0.50, 0.08), // 0  nose
        (0.50, 0.15), // 1  neck
        (0.42, 0.17), // 2  r_shoulder
        (0.38, 0.30), // 3  r_elbow
        (0.36, 0.42), // 4  r_wrist
        (0.58, 0.17), // 5  l_shoulder
        (0.62, 0.30), // 6  l_elbow
        (0.64, 0.42), // 7  l_wrist
        (0.44, 0.45), // 8  r_hip
        (0.43, 0.62), // 9  r_knee
        (0.43, 0.80), // 10 r_ankle
        (0.56, 0.45), // 11 l_hip
        (0.57, 0.62), // 12 l_knee
        (0.57, 0.80), // 13 l_ankle
        (0.47, 0.06), // 14 r_eye
        (0.53, 0.06), // 15 l_eye
        (0.44, 0.08), // 16 r_ear
        (0.56, 0.08), // 17 l_ear
    ],
};

const SITTING: PoseDefinition = PoseDefinition {
    name: "sitting",
    keypoints: [
        (0.50, 0.12),
        (0.50, 0.20),
        (0.40, 0.22),
        (0.36, 0.35),
        (0.38, 0.45),
        (0.60, 0.22),
        (0.64, 0.35),
        (0.62, 0.45),
        (0.44, 0.48),
        (0.38, 0.60),
        (0.38, 0.78),
        (0.56, 0.48),
        (0.62, 0.60),
        (0.62, 0.78),
        (0.47, 0.10),
        (0.53, 0.10),
        (0.44, 0.12),
        (0.56, 0.12),
    ],
};

const LYING_DOWN: PoseDefinition = PoseDefinition {
    name: "lying_down",
    keypoints: [
        (0.15, 0.35),
        (0.22, 0.38),
        (0.24, 0.32),
        (0.30, 0.28),
        (0.36, 0.25),
        (0.24, 0.44),
        (0.30, 0.48),
        (0.36, 0.50),
        (0.45, 0.35),
        (0.60, 0.32),
        (0.75, 0.30),
        (0.45, 0.41),
        (0.60, 0.44),
        (0.75, 0.46),
        (0.13, 0.33),
        (0.13, 0.37),
        (0.11, 0.32),
        (0.11, 0.40),
    ],
};

const RUNNING: PoseDefinition = PoseDefinition {
    name: "running",
    keypoints: [
        (0.50, 0.08),
        (0.48, 0.16),
        (0.40, 0.18),
        (0.32, 0.22),
        (0.28, 0.14),
        (0.56, 0.18),
        (0.64, 0.28),
        (0.68, 0.38),
        (0.44, 0.42),
        (0.36, 0.58),
        (0.30, 0.75),
        (0.56, 0.42),
        (0.64, 0.55),
        (0.70, 0.70),
        (0.47, 0.06),
        (0.53, 0.06),
        (0.44, 0.08),
        (0.56, 0.08),
    ],
};

const KNEELING: PoseDefinition = PoseDefinition {
    name: "kneeling",
    keypoints: [
        (0.50, 0.15),
        (0.50, 0.23),
        (0.42, 0.25),
        (0.38, 0.38),
        (0.40, 0.48),
        (0.58, 0.25),
        (0.62, 0.38),
        (0.60, 0.48),
        (0.44, 0.50),
        (0.42, 0.70),
        (0.40, 0.82),
        (0.56, 0.50),
        (0.58, 0.70),
        (0.60, 0.82),
        (0.47, 0.13),
        (0.53, 0.13),
        (0.44, 0.15),
        (0.56, 0.15),
    ],
};

const LEANING: PoseDefinition = PoseDefinition {
    name: "leaning",
    keypoints: [
        (0.45, 0.10),
        (0.44, 0.18),
        (0.36, 0.20),
        (0.30, 0.32),
        (0.28, 0.42),
        (0.52, 0.20),
        (0.58, 0.28),
        (0.62, 0.22),
        (0.42, 0.46),
        (0.38, 0.64),
        (0.36, 0.80),
        (0.54, 0.46),
        (0.58, 0.62),
        (0.60, 0.78),
        (0.43, 0.08),
        (0.48, 0.08),
        (0.40, 0.10),
        (0.50, 0.10),
    ],
};

const DRIVING: PoseDefinition = PoseDefinition {
    name: "driving",
    keypoints: [
        (0.50, 0.12),
        (0.50, 0.20),
        (0.40, 0.22),
        (0.34, 0.30),
        (0.30, 0.22),
        (0.60, 0.22),
        (0.66, 0.30),
        (0.70, 0.22),
        (0.44, 0.48),
        (0.38, 0.62),
        (0.34, 0.75),
        (0.56, 0.48),
        (0.62, 0.62),
        (0.66, 0.75),
        (0.47, 0.10),
        (0.53, 0.10),
        (0.44, 0.12),
        (0.56, 0.12),
    ],
};

const COOKING: PoseDefinition = PoseDefinition {
    name: "cooking",
    keypoints: [
        (0.50, 0.08),
        (0.50, 0.15),
        (0.42, 0.17),
        (0.36, 0.24),
        (0.32, 0.18),
        (0.58, 0.17),
        (0.64, 0.24),
        (0.68, 0.18),
        (0.44, 0.45),
        (0.43, 0.62),
        (0.43, 0.80),
        (0.56, 0.45),
        (0.57, 0.62),
        (0.57, 0.80),
        (0.47, 0.06),
        (0.53, 0.06),
        (0.44, 0.08),
        (0.56, 0.08),
    ],
};

const FIGHTING: PoseDefinition = PoseDefinition {
    name: "fighting",
    keypoints: [
        (0.48, 0.10),
        (0.47, 0.18),
        (0.38, 0.20),
        (0.32, 0.16),
        (0.30, 0.10),
        (0.56, 0.20),
        (0.62, 0.14),
        (0.64, 0.08),
        (0.42, 0.45),
        (0.36, 0.60),
        (0.32, 0.78),
        (0.54, 0.45),
        (0.60, 0.58),
        (0.64, 0.74),
        (0.46, 0.08),
        (0.51, 0.08),
        (0.43, 0.10),
        (0.53, 0.10),
    ],
};

const WALKING: PoseDefinition = PoseDefinition {
    name: "walking",
    keypoints: [
        (0.50, 0.08),
        (0.50, 0.16),
        (0.42, 0.18),
        (0.38, 0.28),
        (0.40, 0.38),
        (0.58, 0.18),
        (0.62, 0.26),
        (0.58, 0.35),
        (0.44, 0.44),
        (0.40, 0.60),
        (0.38, 0.78),
        (0.56, 0.44),
        (0.60, 0.58),
        (0.62, 0.74),
        (0.47, 0.06),
        (0.53, 0.06),
        (0.44, 0.08),
        (0.56, 0.08),
    ],
};

fn get_all_poses() -> Vec<&'static PoseDefinition> {
    vec![
        &STANDING,
        &SITTING,
        &LYING_DOWN,
        &RUNNING,
        &KNEELING,
        &LEANING,
        &DRIVING,
        &COOKING,
        &FIGHTING,
        &WALKING,
    ]
}

// =============================================================================
// Rendering
// =============================================================================

/// Standard OpenPose bone connections as (from_kp_idx, to_kp_idx, color).
const BONES: &[(usize, usize, [u8; 3])] = &[
    (0, 1, [255, 0, 0]),     // nose → neck
    (1, 2, [255, 85, 0]),    // neck → r_shoulder
    (2, 3, [255, 170, 0]),   // r_shoulder → r_elbow
    (3, 4, [255, 255, 0]),   // r_elbow → r_wrist
    (1, 5, [170, 255, 0]),   // neck → l_shoulder
    (5, 6, [85, 255, 0]),    // l_shoulder → l_elbow
    (6, 7, [0, 255, 0]),     // l_elbow → l_wrist
    (1, 8, [0, 255, 170]),   // neck → r_hip
    (8, 9, [0, 255, 255]),   // r_hip → r_knee
    (9, 10, [0, 170, 255]),  // r_knee → r_ankle
    (1, 11, [0, 85, 255]),   // neck → l_hip
    (11, 12, [0, 0, 255]),   // l_hip → l_knee
    (12, 13, [85, 0, 255]),  // l_knee → l_ankle
    (0, 14, [170, 0, 255]),  // nose → r_eye
    (0, 15, [255, 0, 255]),  // nose → l_eye
    (14, 16, [255, 0, 170]), // r_eye → r_ear
    (15, 17, [255, 0, 85]),  // l_eye → l_ear
];

/// Bresenham line draw onto an RgbImage.
fn draw_line(img: &mut RgbImage, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgb<u8>, thickness: i32) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut cx = x0;
    let mut cy = y0;
    let w = img.width() as i32;
    let h = img.height() as i32;

    loop {
        // Paint a square of `thickness` centered on (cx, cy)
        let half = thickness / 2;
        for ty in (cy - half)..=(cy + half) {
            for tx in (cx - half)..=(cx + half) {
                if tx >= 0 && ty >= 0 && tx < w && ty < h {
                    img.put_pixel(tx as u32, ty as u32, color);
                }
            }
        }

        if cx == x1 && cy == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            cx += sx;
        }
        if e2 < dx {
            err += dx;
            cy += sy;
        }
    }
}

/// Draw a filled circle.
fn draw_circle(img: &mut RgbImage, cx: i32, cy: i32, radius: i32, color: Rgb<u8>) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= radius * radius {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 && px < w && py < h {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

/// Render a single pose as an OpenPose-compatible skeleton image.
pub fn render_skeleton_image(pose: &PoseDefinition, width: u32, height: u32) -> RgbImage {
    let mut img = RgbImage::new(width, height);
    // Black background (default zero-initialized, which is black for RGB)

    let kp_px: Vec<Option<(i32, i32)>> = pose.keypoints.iter().map(|&(nx, ny)| {
        if nx < 0.0 || ny < 0.0 {
            None
        } else {
            Some(((nx * width as f32) as i32, (ny * height as f32) as i32))
        }
    }).collect();

    // Draw bones (lines)
    for &(from, to, color) in BONES {
        if let (Some((x0, y0)), Some((x1, y1))) = (kp_px[from], kp_px[to]) {
            draw_line(&mut img, x0, y0, x1, y1, Rgb(color), 4);
        }
    }

    // Draw keypoint circles on top
    let kp_color = Rgb([255u8, 255, 255]);
    for kp in &kp_px {
        if let Some((x, y)) = *kp {
            draw_circle(&mut img, x, y, 6, kp_color);
        }
    }

    img
}

// =============================================================================
// Public API
// =============================================================================

/// Ensure all pose skeleton PNGs exist under `<app_data_dir>/pose_skeletons/`.
/// Generates only missing files, so subsequent launches are instant.
/// Returns the directory path.
pub fn ensure_pose_skeletons(app_data_dir: &Path) -> Result<PathBuf, String> {
    let skeletons_dir = app_data_dir.join("pose_skeletons");
    std::fs::create_dir_all(&skeletons_dir)
        .map_err(|e| format!("Cannot create pose_skeletons dir: {}", e))?;

    for pose in get_all_poses() {
        let filename = format!("{}.png", pose.name);
        let path = skeletons_dir.join(&filename);
        if !path.exists() {
            let img = render_skeleton_image(pose, 1024, 1024);
            img.save(&path)
                .map_err(|e| format!("Cannot save {}: {}", filename, e))?;
            println!("[PoseSkeletons] Generated {}", filename);
        }
    }

    Ok(skeletons_dir)
}

/// Get the skeleton image path for a given pose name string.
/// Falls back to `standing.png` for unrecognized values.
pub fn get_skeleton_path_for_pose(pose_name: &str, skeletons_dir: &Path) -> PathBuf {
    // Normalize to lowercase, replace spaces/hyphens with underscores
    let normalized = pose_name.to_lowercase()
        .replace(' ', "_")
        .replace('-', "_");

    // Check built-in poses first
    let built_in = match normalized.as_str() {
        "standing" => Some("standing.png"),
        "sitting" | "seated" => Some("sitting.png"),
        "lying_down" | "lying" | "lyingdown" => Some("lying_down.png"),
        "running" => Some("running.png"),
        "kneeling" => Some("kneeling.png"),
        "leaning" => Some("leaning.png"),
        "driving" => Some("driving.png"),
        "cooking" => Some("cooking.png"),
        "fighting" => Some("fighting.png"),
        "walking" => Some("walking.png"),
        _ => None,
    };

    if let Some(filename) = built_in {
        return skeletons_dir.join(filename);
    }

    // Check for a custom pose file matching the normalized name
    let custom_filename = format!("{}.png", normalized);
    let custom_path = skeletons_dir.join(&custom_filename);
    if custom_path.exists() {
        println!("[PoseSkeletons] Using custom pose: {}", custom_filename);
        return custom_path;
    }

    // Fallback to standing
    println!("[PoseSkeletons] Unknown pose '{}', falling back to standing", pose_name);
    skeletons_dir.join("standing.png")
}
