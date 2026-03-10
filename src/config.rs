use std::{
    fs::File,
    io::Read,
    path::Path,
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;

const DEFAULT_TRAIL_COLOR: [u8; 4] = [255, 255, 255, 204];
const DEFAULT_TRAIL_ALPHA: u8 = 204;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Configuration related to pattern recognition
    #[serde(default)]
    pub recognizer: RecognizerConfig,
    /// Patterns that associated with commands (Pattern => Command)
    #[serde(default)]
    pub commands: Vec<GestureCommand>,
    /// Visual trail rendering configuration
    #[serde(default)]
    pub trail: TrailConfig,
}

#[serde_inline_default]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognizerConfig {
    /// The percentage of similarity between the original pattern
    /// and the user input requiret to trigger the command
    #[serde_inline_default(0.8)]
    pub command_execute_treshold: f64,
    /// Point count required to trigger command or save new pattern
    #[serde_inline_default(10)]
    pub point_count_treshold: u64,
    /// Acceptable range for pattern rotation (degrees)
    #[serde_inline_default(10.0)]
    pub rotation_angle_range: f64,
    /// Acceptable accuracy in pattern rotation (degrees)
    #[serde_inline_default(2.0)]
    pub rotation_angle_treshold: f64,
    /// The number of points to which the pattern is reduced fo recognition
    #[serde_inline_default(64)]
    pub resample_num_points: u32,
    /// Width used for recognition (may not match screen size)
    #[serde_inline_default(100.0)]
    pub width: f64,
    /// Height used for recognition (may not match screen size)
    #[serde_inline_default(100.0)]
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureCommand {
    pub pattern: String,
    pub command: String,
}

#[serde_inline_default]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailConfig {
    #[serde_inline_default(false)]
    pub enabled: bool,
    /// Trail color as "#rrggbb" or "#rrggbbaa" hex string.
    /// When omitted, auto-detected from the XDG desktop portal accent color.
    #[serde(default)]
    pub color: Option<String>,
    #[serde_inline_default(4.0)]
    pub width: f64,
}

impl TrailConfig {
    pub fn resolve_color(&self) -> [u8; 4] {
        if let Some(ref hex) = self.color {
            if let Some(c) = parse_hex_color(hex) {
                return c;
            }
            eprintln!(
                "WARNING: invalid trail color '{}', trying portal auto-detect",
                hex
            );
        }
        query_portal_accent_color().unwrap_or(DEFAULT_TRAIL_COLOR)
    }
}

fn parse_hex_color(s: &str) -> Option<[u8; 4]> {
    let s = s.strip_prefix('#')?;
    match s.len() {
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some([r, g, b, 255])
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some([r, g, b, a])
        }
        _ => None,
    }
}

/// Query the XDG desktop portal for the user's accent color via busctl.
/// Returns RGBA with a default alpha for overlay visibility.
fn query_portal_accent_color() -> Option<[u8; 4]> {
    let output = Command::new("busctl")
        .args([
            "--user",
            "call",
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Settings",
            "ReadOne",
            "ss",
            "org.freedesktop.appearance",
            "accent-color",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // Output format: "v (ddd) 0.207843 0.517647 0.894118\n"
    let text = String::from_utf8(output.stdout).ok()?;
    let floats: Vec<f64> = text
        .split_whitespace()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();

    if floats.len() == 3 {
        let r = (floats[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (floats[1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (floats[2].clamp(0.0, 1.0) * 255.0) as u8;
        eprintln!(
            "trail: using portal accent color #{:02x}{:02x}{:02x}",
            r, g, b
        );
        Some([r, g, b, DEFAULT_TRAIL_ALPHA])
    } else {
        None
    }
}

impl Default for TrailConfig {
    fn default() -> Self {
        serde_yml::from_str("").unwrap()
    }
}

impl AppConfig {
    pub fn load(config_path: &Path) -> Result<Self, ()> {
        let mut file = File::open(config_path).map_err(|err| {
            eprintln!("ERROR: failed to open config {}, {}", config_path.display(), err);
        })?;

        let mut raw = String::new();
        file.read_to_string(&mut raw).map_err(|err| {
            eprintln!("ERROR: failed to read from config {}, {}", config_path.display(), err);
        })?;

        let config: AppConfig = serde_yml::from_str(&raw).map_err(|err| {
            eprintln!("ERROR: failed to parse config {}, {}", config_path.display(), err);
        })?;

        let exec_treshold = config.recognizer.command_execute_treshold;
        if exec_treshold < 0.0 || exec_treshold > 1.0 {
            eprintln!("ERROR: recognizer.command_execute_treshold should be in range [0,1]");
            return Err(());
        }

        if config.recognizer.width <= 0.0 {
            eprintln!("ERROR: recognizer.width should be positive number");
            return Err(());
        }

        if config.recognizer.height <= 0.0 {
            eprintln!("ERROR: recognizer.height should be positive number");
            return Err(());
        }

        Ok(config)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        serde_yml::from_str("").unwrap()
    }
}

impl Default for RecognizerConfig {
    fn default() -> Self {
        serde_yml::from_str("").unwrap()
    }
}

