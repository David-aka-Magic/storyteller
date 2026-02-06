// src-tauri/src/llm_parser.rs
//
// JSON Parser for StoryEngine LLM Output
// ========================================
// Parses the structured JSON that the "storyengine" Ollama model returns each turn.
// Handles malformed output gracefully and provides typed access to all fields.
//
// The LLM outputs a JSON object with:
//   - turn_id:              Sequence number for this story turn
//   - story_json:           Narrative text + summary hint
//   - scene_json:           Location, lighting, mood, etc.
//   - characters_in_scene:  Array of character positions/actions/expressions
//   - generation_flags:     Whether to generate an image, what changed
//
// This module provides:
//   1. Typed structs that mirror the JSON schema
//   2. A robust parse function with fallback for malformed output
//   3. Helper functions for extracting story text, character names, and flags
//   4. Tauri commands to expose parsing to the frontend

use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// ENUMS
// ============================================================================

/// Where a character is positioned in the scene.
/// Covers all 10 region values the LLM can produce.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CharacterRegion {
    Left,
    Center,
    Right,
    LeftSeated,
    CenterSeated,
    RightSeated,
    LeftBackground,
    CenterBackground,
    RightBackground,
    OffScreen,
    /// Fallback for any value the LLM invents that we don't recognize
    Other(String),
}

impl CharacterRegion {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "left" => Self::Left,
            "center" => Self::Center,
            "right" => Self::Right,
            "left-seated" | "left_seated" | "leftseated" => Self::LeftSeated,
            "center-seated" | "center_seated" | "centerseated" => Self::CenterSeated,
            "right-seated" | "right_seated" | "rightseated" => Self::RightSeated,
            "left-background" | "left_background" | "leftbackground" => Self::LeftBackground,
            "center-background" | "center_background" | "centerbackground" => Self::CenterBackground,
            "right-background" | "right_background" | "rightbackground" => Self::RightBackground,
            "off-screen" | "off_screen" | "offscreen" => Self::OffScreen,
            other => Self::Other(other.to_string()),
        }
    }

    /// Returns the canonical kebab-case string for this region.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
            Self::LeftSeated => "left-seated",
            Self::CenterSeated => "center-seated",
            Self::RightSeated => "right-seated",
            Self::LeftBackground => "left-background",
            Self::CenterBackground => "center-background",
            Self::RightBackground => "right-background",
            Self::OffScreen => "off-screen",
            Self::Other(s) => s.as_str(),
        }
    }

    /// True if the character is seated in any position.
    pub fn is_seated(&self) -> bool {
        matches!(self, Self::LeftSeated | Self::CenterSeated | Self::RightSeated)
    }

    /// True if the character is in the background.
    pub fn is_background(&self) -> bool {
        matches!(self, Self::LeftBackground | Self::CenterBackground | Self::RightBackground)
    }

    /// True if the character is off-screen (present in scene data but not visible).
    pub fn is_off_screen(&self) -> bool {
        matches!(self, Self::OffScreen)
    }
}

impl fmt::Display for CharacterRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Camera framing for how a character should be rendered.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CharacterView {
    Portrait,
    UpperBody,
    FullBody,
    None,
    /// Fallback for unrecognized values
    Other(String),
}

impl CharacterView {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_uppercase().trim() {
            "PORTRAIT" => Self::Portrait,
            "UPPER-BODY" | "UPPER_BODY" | "UPPERBODY" | "UPPER BODY" => Self::UpperBody,
            "FULL-BODY" | "FULL_BODY" | "FULLBODY" | "FULL BODY" => Self::FullBody,
            "NONE" => Self::None,
            other => Self::Other(other.to_string()),
        }
    }

    /// Returns the canonical SCREAMING-KEBAB string the LLM expects.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Portrait => "PORTRAIT",
            Self::UpperBody => "UPPER-BODY",
            Self::FullBody => "FULL-BODY",
            Self::None => "NONE",
            Self::Other(s) => s.as_str(),
        }
    }

    /// True if this view requires rendering an image of the character.
    pub fn needs_render(&self) -> bool {
        !matches!(self, Self::None)
    }
}

impl fmt::Display for CharacterView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// CORE STRUCTS — mirror the LLM JSON schema
// ============================================================================

/// The narrative text and compression hint for a single story turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryJson {
    /// The main narrative text to display to the player.
    #[serde(default)]
    pub response: String,
    /// One-line summary the LLM provides for future context compression.
    #[serde(default)]
    pub summary_hint: String,
}

/// Visual/environmental details about the current scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneJson {
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub location_type: String,
    #[serde(default)]
    pub time_of_day: String,
    #[serde(default)]
    pub weather: String,
    #[serde(default)]
    pub lighting: String,
    #[serde(default)]
    pub mood: String,
}

/// A single character's placement and appearance in the current scene.
/// The raw version uses String fields; call `.typed()` to get parsed enums.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneCharacterRaw {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub view: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub expression: String,
    #[serde(default)]
    pub clothing: String,
    #[serde(default)]
    pub facing: String,
}

impl SceneCharacterRaw {
    /// Convert loose string fields into typed enums.
    pub fn typed(&self) -> SceneCharacterTyped {
        SceneCharacterTyped {
            name: self.name.clone(),
            region: CharacterRegion::from_str_loose(&self.region),
            view: CharacterView::from_str_loose(&self.view),
            action: self.action.clone(),
            expression: self.expression.clone(),
            clothing: self.clothing.clone(),
            facing: self.facing.clone(),
        }
    }

    /// Convert to the existing SceneCharacter model (from models.rs)
    /// for compatibility with the character database lookup system.
    pub fn to_scene_character(&self) -> crate::models::SceneCharacter {
        crate::models::SceneCharacter {
            name: self.name.clone(),
            region: if self.region.is_empty() { None } else { Some(self.region.clone()) },
            view: if self.view.is_empty() { None } else { Some(self.view.clone()) },
            action: if self.action.is_empty() { None } else { Some(self.action.clone()) },
            expression: if self.expression.is_empty() { None } else { Some(self.expression.clone()) },
            clothing: if self.clothing.is_empty() { None } else { Some(self.clothing.clone()) },
            facing: if self.facing.is_empty() { None } else { Some(self.facing.clone()) },
        }
    }
}

/// A character with fully typed region and view enums.
#[derive(Debug, Clone, Serialize)]
pub struct SceneCharacterTyped {
    pub name: String,
    pub region: CharacterRegion,
    pub view: CharacterView,
    pub action: String,
    pub expression: String,
    pub clothing: String,
    pub facing: String,
}

impl SceneCharacterTyped {
    /// True if this character should be rendered in the scene image.
    pub fn needs_render(&self) -> bool {
        !self.region.is_off_screen() && self.view.needs_render()
    }
}

/// Flags that tell us whether to generate a new image for this turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationFlags {
    /// Master flag: should we generate any image at all?
    #[serde(default)]
    pub generate_image: bool,
    /// Did the scene location/environment change since last turn?
    #[serde(default)]
    pub scene_changed: bool,
    /// Did any character enter/leave/change appearance?
    #[serde(default)]
    pub characters_changed: bool,
}

impl Default for GenerationFlags {
    fn default() -> Self {
        Self {
            generate_image: false,
            scene_changed: false,
            characters_changed: false,
        }
    }
}

/// The complete parsed output from one LLM story turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTurnOutput {
    /// Sequence number for this turn (1-based).
    #[serde(default)]
    pub turn_id: u32,
    /// The narrative text and summary hint.
    #[serde(default)]
    pub story_json: Option<StoryJson>,
    /// Scene environment details.
    #[serde(default)]
    pub scene_json: Option<SceneJson>,
    /// Characters present in the scene with their positions and appearance.
    #[serde(default)]
    pub characters_in_scene: Vec<SceneCharacterRaw>,
    /// Flags controlling image generation.
    #[serde(default)]
    pub generation_flags: Option<GenerationFlags>,
}

// ============================================================================
// PARSE RESULT — wraps success or partial/fallback output
// ============================================================================

/// What happened when we tried to parse the LLM output.
#[derive(Debug, Clone, Serialize)]
pub enum ParseStatus {
    /// Clean parse — all fields present and valid.
    Ok,
    /// Parsed as JSON but some expected fields were missing.
    /// The Vec contains human-readable descriptions of what was missing.
    Partial(Vec<String>),
    /// The raw string wasn't valid JSON at all.
    /// We still try to extract useful text from it.
    Fallback,
}

/// The result of parsing one LLM turn.
#[derive(Debug, Clone, Serialize)]
pub struct ParsedTurn {
    /// Whether parsing fully succeeded, partially succeeded, or fell back.
    pub status: ParseStatus,

    /// The parsed turn data. Always present (may have empty/default fields on fallback).
    pub turn: LlmTurnOutput,

    /// If parsing failed entirely, this holds the raw text for display.
    /// On success this is None.
    pub raw_text: Option<String>,
}

impl ParsedTurn {
    // ── Convenience helpers ─────────────────────────────────────────────

    /// Get the story text for display.
    /// Falls back to raw_text if story_json wasn't parsed.
    pub fn story_text(&self) -> &str {
        if let Some(ref sj) = self.turn.story_json {
            if !sj.response.is_empty() {
                return &sj.response;
            }
        }
        if let Some(ref raw) = self.raw_text {
            return raw;
        }
        ""
    }

    /// Get the summary hint (for future context compression).
    pub fn summary_hint(&self) -> &str {
        self.turn
            .story_json
            .as_ref()
            .map(|sj| sj.summary_hint.as_str())
            .unwrap_or("")
    }

    /// Get the list of character names present in this scene.
    /// These names are used to look up master images from the Character Database.
    pub fn character_names(&self) -> Vec<String> {
        self.turn
            .characters_in_scene
            .iter()
            .filter(|c| !c.name.is_empty())
            .map(|c| c.name.clone())
            .collect()
    }

    /// Get characters as typed structs with parsed enums.
    pub fn characters_typed(&self) -> Vec<SceneCharacterTyped> {
        self.turn
            .characters_in_scene
            .iter()
            .map(|c| c.typed())
            .collect()
    }

    /// Get characters converted to the models::SceneCharacter format
    /// for use with the existing `lookup_scene_characters` command.
    pub fn characters_for_lookup(&self) -> Vec<crate::models::SceneCharacter> {
        self.turn
            .characters_in_scene
            .iter()
            .map(|c| c.to_scene_character())
            .collect()
    }

    /// Get only the characters that need to be rendered (not off-screen, view != NONE).
    pub fn renderable_characters(&self) -> Vec<SceneCharacterTyped> {
        self.characters_typed()
            .into_iter()
            .filter(|c| c.needs_render())
            .collect()
    }

    /// Get the generation flags, or defaults if they weren't in the output.
    pub fn flags(&self) -> GenerationFlags {
        self.turn
            .generation_flags
            .clone()
            .unwrap_or_default()
    }

    /// Should we generate an image for this turn?
    pub fn should_generate_image(&self) -> bool {
        self.flags().generate_image
    }

    /// Did the scene environment change?
    pub fn scene_changed(&self) -> bool {
        self.flags().scene_changed
    }

    /// Did any character enter/leave/change appearance?
    pub fn characters_changed(&self) -> bool {
        self.flags().characters_changed
    }

    /// Get the scene data (if present).
    pub fn scene(&self) -> Option<&SceneJson> {
        self.turn.scene_json.as_ref()
    }

    /// Build a scene description string suitable for an image generation prompt.
    /// Combines location, lighting, mood, etc. into a single comma-separated line.
    pub fn scene_prompt_fragment(&self) -> String {
        match &self.turn.scene_json {
            Some(s) => {
                let mut parts = Vec::new();
                if !s.location.is_empty() { parts.push(s.location.as_str()); }
                if !s.location_type.is_empty() { parts.push(s.location_type.as_str()); }
                if !s.time_of_day.is_empty() { parts.push(s.time_of_day.as_str()); }
                if !s.lighting.is_empty() { parts.push(s.lighting.as_str()); }
                if !s.mood.is_empty() { parts.push(s.mood.as_str()); }
                if !s.weather.is_empty() && s.weather.to_lowercase() != "n/a" {
                    parts.push(s.weather.as_str());
                }
                parts.join(", ")
            }
            None => String::new(),
        }
    }
}

// ============================================================================
// CORE PARSE FUNCTION
// ============================================================================

/// Parse raw LLM output into a structured `ParsedTurn`.
///
/// This function is resilient to malformed output:
/// 1. First tries a direct JSON parse of the full string.
/// 2. If that fails, tries to extract a JSON object from within surrounding text
///    (LLMs sometimes wrap JSON in markdown fences or add preamble).
/// 3. If embedded JSON is found but doesn't match LlmTurnOutput, tries parsing
///    it as just a StoryJson (common partial-output failure mode).
/// 4. If no valid JSON is found, returns a Fallback with the raw text preserved
///    so the narrative is never lost.
///
/// Missing fields within valid JSON are handled via serde defaults — they
/// won't cause a parse failure, just empty/default values with warnings.
pub fn parse_llm_output(raw: &str) -> ParsedTurn {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return ParsedTurn {
            status: ParseStatus::Fallback,
            turn: empty_turn(),
            raw_text: Some(String::new()),
        };
    }

    // Attempt 1: direct parse of the full string
    if let Ok(turn) = serde_json::from_str::<LlmTurnOutput>(trimmed) {
        let warnings = validate_turn(&turn);
        let status = if warnings.is_empty() {
            ParseStatus::Ok
        } else {
            ParseStatus::Partial(warnings)
        };
        return ParsedTurn {
            status,
            turn,
            raw_text: None,
        };
    }

    // Attempt 2: extract JSON object from within surrounding text
    // (handles ```json ... ``` fences, preamble text, etc.)
    if let Some(json_str) = extract_json_object(trimmed) {
        // 2a: try as full LlmTurnOutput
        if let Ok(turn) = serde_json::from_str::<LlmTurnOutput>(&json_str) {
            let mut warnings = validate_turn(&turn);
            warnings.insert(0, "JSON was embedded in surrounding text".to_string());
            return ParsedTurn {
                status: ParseStatus::Partial(warnings),
                turn,
                raw_text: None,
            };
        }

        // 2b: try as just the story_json portion (common partial failure)
        if let Ok(story) = serde_json::from_str::<StoryJson>(&json_str) {
            if !story.response.is_empty() {
                let turn = LlmTurnOutput {
                    turn_id: 0,
                    story_json: Some(story),
                    scene_json: None,
                    characters_in_scene: vec![],
                    generation_flags: None,
                };
                return ParsedTurn {
                    status: ParseStatus::Partial(vec![
                        "Only story_json was parseable; scene, characters, and flags are missing"
                            .to_string(),
                    ]),
                    turn,
                    raw_text: Some(trimmed.to_string()),
                };
            }
        }
    }

    // Attempt 3: complete fallback — treat everything as raw narrative text
    let fallback_text = clean_raw_text(trimmed);
    let turn = LlmTurnOutput {
        turn_id: 0,
        story_json: Some(StoryJson {
            response: fallback_text.clone(),
            summary_hint: String::new(),
        }),
        scene_json: None,
        characters_in_scene: vec![],
        generation_flags: None,
    };

    ParsedTurn {
        status: ParseStatus::Fallback,
        turn,
        raw_text: Some(fallback_text),
    }
}

/// Create an empty turn with all default values.
fn empty_turn() -> LlmTurnOutput {
    LlmTurnOutput {
        turn_id: 0,
        story_json: None,
        scene_json: None,
        characters_in_scene: vec![],
        generation_flags: None,
    }
}

// ============================================================================
// VALIDATION
// ============================================================================

/// Check a successfully-parsed turn for missing or suspicious fields.
/// Returns a list of human-readable warnings (empty = everything looks good).
fn validate_turn(turn: &LlmTurnOutput) -> Vec<String> {
    let mut warnings = Vec::new();

    if turn.story_json.is_none() {
        warnings.push("story_json is missing".to_string());
    } else if let Some(ref sj) = turn.story_json {
        if sj.response.is_empty() {
            warnings.push("story_json.response is empty".to_string());
        }
    }

    if turn.scene_json.is_none() {
        warnings.push("scene_json is missing".to_string());
    }

    if turn.characters_in_scene.is_empty() {
        warnings.push("characters_in_scene is empty".to_string());
    } else {
        for (i, c) in turn.characters_in_scene.iter().enumerate() {
            if c.name.is_empty() {
                warnings.push(format!("characters_in_scene[{}] has no name", i));
            }
        }
    }

    if turn.generation_flags.is_none() {
        warnings.push("generation_flags is missing".to_string());
    }

    if turn.turn_id == 0 {
        warnings.push("turn_id is 0 (possibly missing)".to_string());
    }

    warnings
}

// ============================================================================
// TEXT EXTRACTION HELPERS
// ============================================================================

/// Try to find and extract the outermost JSON object from a string.
/// Handles markdown code fences, preamble text before `{`, trailing text after `}`.
fn extract_json_object(text: &str) -> Option<String> {
    // Strip markdown code fences if present
    let stripped = if text.contains("```") {
        let re_start = text.find("```json").or_else(|| text.find("```"));
        let re_end = text.rfind("```");
        match (re_start, re_end) {
            (Some(start), Some(end)) if start < end => {
                let inner_start = text[start..]
                    .find('\n')
                    .map(|i| start + i + 1)
                    .unwrap_or(start + 3);
                &text[inner_start..end]
            }
            _ => text,
        }
    } else {
        text
    };

    // Find the first '{' and match braces to find the closing '}'
    let bytes = stripped.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')?;

    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;

    for (i, &b) in bytes[start..].iter().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        match b {
            b'\\' if in_string => escape = true,
            b'"' => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(stripped[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }

    None
}

/// Clean up raw text that we're treating as a narrative fallback.
/// Strips markdown fences, leading/trailing whitespace.
fn clean_raw_text(text: &str) -> String {
    text.replace("```json", "")
        .replace("```", "")
        .trim()
        .to_string()
}

// ============================================================================
// STANDALONE HELPER FUNCTIONS (for use without full ParsedTurn)
// ============================================================================

/// Quick extraction of just the story text from a raw LLM string.
/// Useful when you only need the narrative and don't care about the rest.
pub fn extract_story_text(raw: &str) -> String {
    let parsed = parse_llm_output(raw);
    parsed.story_text().to_string()
}

/// Quick extraction of character names from a raw LLM string.
/// Returns names in the order they appear in characters_in_scene.
pub fn extract_character_names(raw: &str) -> Vec<String> {
    let parsed = parse_llm_output(raw);
    parsed.character_names()
}

/// Quick check of whether the LLM wants us to generate an image.
pub fn should_generate_image(raw: &str) -> bool {
    let parsed = parse_llm_output(raw);
    parsed.should_generate_image()
}

// ============================================================================
// SERIALIZABLE RESULT FOR FRONTEND
// ============================================================================

/// A frontend-friendly version of ParsedTurn that serializes cleanly over Tauri IPC.
/// All enums are converted to strings and the status is a simple string + warnings array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTurnForFrontend {
    pub status: String,         // "ok", "partial", or "fallback"
    pub warnings: Vec<String>,  // Empty on "ok", populated on "partial"
    pub turn_id: u32,
    pub story_text: String,
    pub summary_hint: String,
    pub scene: Option<SceneJson>,
    pub characters: Vec<FrontendCharacter>,
    pub flags: GenerationFlags,
    pub scene_prompt_fragment: String,
    pub raw_text: Option<String>,
}

/// Frontend-friendly character representation (all string fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendCharacter {
    pub name: String,
    pub region: String,
    pub view: String,
    pub action: String,
    pub expression: String,
    pub clothing: String,
    pub facing: String,
    pub needs_render: bool,
}

impl ParsedTurn {
    /// Convert to a frontend-friendly struct for Tauri IPC.
    pub fn to_frontend(&self) -> ParsedTurnForFrontend {
        let (status_str, warnings) = match &self.status {
            ParseStatus::Ok => ("ok".to_string(), vec![]),
            ParseStatus::Partial(w) => ("partial".to_string(), w.clone()),
            ParseStatus::Fallback => (
                "fallback".to_string(),
                vec!["Could not parse JSON".to_string()],
            ),
        };

        let characters: Vec<FrontendCharacter> = self
            .turn
            .characters_in_scene
            .iter()
            .map(|c| {
                let typed = c.typed();
                FrontendCharacter {
                    name: c.name.clone(),
                    region: c.region.clone(),
                    view: c.view.clone(),
                    action: c.action.clone(),
                    expression: c.expression.clone(),
                    clothing: c.clothing.clone(),
                    facing: c.facing.clone(),
                    needs_render: typed.needs_render(),
                }
            })
            .collect();

        ParsedTurnForFrontend {
            status: status_str,
            warnings,
            turn_id: self.turn.turn_id,
            story_text: self.story_text().to_string(),
            summary_hint: self.summary_hint().to_string(),
            scene: self.turn.scene_json.clone(),
            characters,
            flags: self.flags(),
            scene_prompt_fragment: self.scene_prompt_fragment(),
            raw_text: self.raw_text.clone(),
        }
    }
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

/// Parse raw LLM output and return a frontend-friendly result.
/// Call this from the frontend after receiving an Ollama response.
#[tauri::command]
pub fn parse_story_turn(raw_output: String) -> Result<ParsedTurnForFrontend, String> {
    let parsed = parse_llm_output(&raw_output);
    Ok(parsed.to_frontend())
}

/// Extract just the story text from raw LLM output.
/// Lighter-weight than full parsing when you only need the narrative.
#[tauri::command]
pub fn get_story_text(raw_output: String) -> String {
    extract_story_text(&raw_output)
}

/// Extract character names from raw LLM output.
/// Use these names with `lookup_scene_characters` from the character database.
#[tauri::command]
pub fn get_character_names(raw_output: String) -> Vec<String> {
    extract_character_names(&raw_output)
}

/// Check generation flags from raw LLM output.
/// Returns the flags struct with generate_image, scene_changed, characters_changed.
#[tauri::command]
pub fn check_generation_flags(raw_output: String) -> Result<GenerationFlags, String> {
    let parsed = parse_llm_output(&raw_output);
    Ok(parsed.flags())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// The example JSON from the task spec.
    const EXAMPLE_JSON: &str = r#"{
        "turn_id": 1,
        "story_json": {
            "response": "The narrative text...",
            "summary_hint": "One-line summary for compression."
        },
        "scene_json": {
            "location": "cozy coffee shop",
            "location_type": "interior",
            "time_of_day": "afternoon",
            "weather": "n/a",
            "lighting": "warm ambient lighting",
            "mood": "casual, friendly"
        },
        "characters_in_scene": [
            {
                "name": "Marcus",
                "region": "left",
                "view": "FULL-BODY",
                "action": "walking toward table",
                "expression": "friendly smile",
                "clothing": "blue jacket, white t-shirt",
                "facing": "Elena"
            },
            {
                "name": "Elena",
                "region": "right-seated",
                "view": "UPPER-BODY",
                "action": "looking up from phone",
                "expression": "surprised, happy",
                "clothing": "red sweater, glasses",
                "facing": "Marcus"
            }
        ],
        "generation_flags": {
            "generate_image": true,
            "scene_changed": false,
            "characters_changed": true
        }
    }"#;

    #[test]
    fn test_parse_valid_json() {
        let result = parse_llm_output(EXAMPLE_JSON);
        assert!(matches!(result.status, ParseStatus::Ok));
        assert_eq!(result.story_text(), "The narrative text...");
        assert_eq!(result.summary_hint(), "One-line summary for compression.");
        assert_eq!(result.turn.turn_id, 1);
        assert!(result.should_generate_image());
        assert!(!result.scene_changed());
        assert!(result.characters_changed());
    }

    #[test]
    fn test_character_names() {
        let result = parse_llm_output(EXAMPLE_JSON);
        let names = result.character_names();
        assert_eq!(names, vec!["Marcus", "Elena"]);
    }

    #[test]
    fn test_character_regions_and_views() {
        let result = parse_llm_output(EXAMPLE_JSON);
        let typed = result.characters_typed();

        assert_eq!(typed[0].name, "Marcus");
        assert_eq!(typed[0].region, CharacterRegion::Left);
        assert_eq!(typed[0].view, CharacterView::FullBody);
        assert!(typed[0].needs_render());

        assert_eq!(typed[1].name, "Elena");
        assert_eq!(typed[1].region, CharacterRegion::RightSeated);
        assert!(typed[1].region.is_seated());
        assert_eq!(typed[1].view, CharacterView::UpperBody);
    }

    #[test]
    fn test_scene_data() {
        let result = parse_llm_output(EXAMPLE_JSON);
        let scene = result.scene().unwrap();
        assert_eq!(scene.location, "cozy coffee shop");
        assert_eq!(scene.location_type, "interior");
        assert_eq!(scene.time_of_day, "afternoon");
    }

    #[test]
    fn test_scene_prompt_fragment() {
        let result = parse_llm_output(EXAMPLE_JSON);
        let fragment = result.scene_prompt_fragment();
        assert!(fragment.contains("cozy coffee shop"));
        assert!(fragment.contains("warm ambient lighting"));
        // "n/a" weather should be excluded
        assert!(!fragment.contains("n/a"));
    }

    #[test]
    fn test_json_in_markdown_fence() {
        let fenced = format!("```json\n{}\n```", EXAMPLE_JSON);
        let result = parse_llm_output(&fenced);
        assert!(matches!(result.status, ParseStatus::Partial(_)));
        assert_eq!(result.story_text(), "The narrative text...");
        assert_eq!(result.character_names(), vec!["Marcus", "Elena"]);
    }

    #[test]
    fn test_json_with_preamble() {
        let with_preamble = format!("Here is the story output:\n{}", EXAMPLE_JSON);
        let result = parse_llm_output(&with_preamble);
        assert!(matches!(result.status, ParseStatus::Partial(_)));
        assert_eq!(result.story_text(), "The narrative text...");
    }

    #[test]
    fn test_malformed_json_fallback() {
        let broken = "This is just plain text with no JSON at all.";
        let result = parse_llm_output(broken);
        assert!(matches!(result.status, ParseStatus::Fallback));
        assert_eq!(result.story_text(), broken);
        assert!(result.character_names().is_empty());
        assert!(!result.should_generate_image());
    }

    #[test]
    fn test_partial_json_missing_fields() {
        let partial = r#"{"turn_id": 5, "story_json": {"response": "Something happened."}}"#;
        let result = parse_llm_output(partial);
        // Should parse but with warnings about missing fields
        assert!(matches!(result.status, ParseStatus::Partial(_)));
        assert_eq!(result.story_text(), "Something happened.");
        assert_eq!(result.turn.turn_id, 5);
        assert!(result.character_names().is_empty());
        assert!(!result.should_generate_image());
    }

    #[test]
    fn test_empty_string() {
        let result = parse_llm_output("");
        assert!(matches!(result.status, ParseStatus::Fallback));
    }

    #[test]
    fn test_off_screen_character() {
        let json = r#"{
            "turn_id": 2,
            "story_json": {"response": "A voice calls from outside."},
            "characters_in_scene": [
                {"name": "Ghost", "region": "off-screen", "view": "NONE"}
            ],
            "generation_flags": {"generate_image": true}
        }"#;
        let result = parse_llm_output(json);
        let typed = result.characters_typed();
        assert_eq!(typed[0].region, CharacterRegion::OffScreen);
        assert!(typed[0].region.is_off_screen());
        assert!(!typed[0].needs_render());
        assert!(result.renderable_characters().is_empty());
    }

    #[test]
    fn test_background_characters() {
        let json = r#"{
            "turn_id": 3,
            "story_json": {"response": "Crowds mill about."},
            "characters_in_scene": [
                {"name": "Vendor", "region": "left-background", "view": "FULL-BODY"},
                {"name": "Hero", "region": "center", "view": "FULL-BODY"}
            ],
            "generation_flags": {"generate_image": true}
        }"#;
        let result = parse_llm_output(json);
        let typed = result.characters_typed();
        assert!(typed[0].region.is_background());
        assert!(!typed[1].region.is_background());
        // Both should still be renderable
        assert_eq!(result.renderable_characters().len(), 2);
    }

    #[test]
    fn test_region_variants() {
        assert_eq!(CharacterRegion::from_str_loose("left"), CharacterRegion::Left);
        assert_eq!(CharacterRegion::from_str_loose("LEFT"), CharacterRegion::Left);
        assert_eq!(CharacterRegion::from_str_loose("center-seated"), CharacterRegion::CenterSeated);
        assert_eq!(CharacterRegion::from_str_loose("center_seated"), CharacterRegion::CenterSeated);
        assert_eq!(CharacterRegion::from_str_loose("right-background"), CharacterRegion::RightBackground);
        assert_eq!(CharacterRegion::from_str_loose("off-screen"), CharacterRegion::OffScreen);
        assert!(matches!(
            CharacterRegion::from_str_loose("floating"),
            CharacterRegion::Other(_)
        ));
    }

    #[test]
    fn test_view_variants() {
        assert_eq!(CharacterView::from_str_loose("PORTRAIT"), CharacterView::Portrait);
        assert_eq!(CharacterView::from_str_loose("UPPER-BODY"), CharacterView::UpperBody);
        assert_eq!(CharacterView::from_str_loose("FULL-BODY"), CharacterView::FullBody);
        assert_eq!(CharacterView::from_str_loose("full_body"), CharacterView::FullBody);
        assert_eq!(CharacterView::from_str_loose("NONE"), CharacterView::None);
        assert!(!CharacterView::None.needs_render());
        assert!(CharacterView::FullBody.needs_render());
    }

    #[test]
    fn test_frontend_conversion() {
        let result = parse_llm_output(EXAMPLE_JSON);
        let fe = result.to_frontend();
        assert_eq!(fe.status, "ok");
        assert!(fe.warnings.is_empty());
        assert_eq!(fe.turn_id, 1);
        assert_eq!(fe.story_text, "The narrative text...");
        assert_eq!(fe.characters.len(), 2);
        assert_eq!(fe.characters[0].name, "Marcus");
        assert!(fe.characters[0].needs_render);
        assert!(fe.flags.generate_image);
    }

    #[test]
    fn test_characters_for_lookup_compat() {
        // Verify we produce models::SceneCharacter compatible with
        // the existing lookup_scene_characters command
        let result = parse_llm_output(EXAMPLE_JSON);
        let scene_chars = result.characters_for_lookup();
        assert_eq!(scene_chars.len(), 2);
        assert_eq!(scene_chars[0].name, "Marcus");
        assert_eq!(scene_chars[0].region, Some("left".to_string()));
        assert_eq!(scene_chars[0].view, Some("FULL-BODY".to_string()));
        assert_eq!(scene_chars[1].name, "Elena");
        assert_eq!(
            scene_chars[1].clothing,
            Some("red sweater, glasses".to_string())
        );
    }

    #[test]
    fn test_only_story_json_parseable() {
        let json = r#"{"response": "The hero walked forward.", "summary_hint": "Hero moves."}"#;
        let result = parse_llm_output(json);
        assert!(matches!(result.status, ParseStatus::Partial(_)));
        assert_eq!(result.story_text(), "The hero walked forward.");
        assert_eq!(result.summary_hint(), "Hero moves.");
    }

    #[test]
    fn test_truncated_json() {
        let truncated = r#"{"turn_id": 1, "story_json": {"response": "The door opened slowly..."#;
        let result = parse_llm_output(truncated);
        // Can't extract valid JSON, falls back to raw text
        assert!(matches!(result.status, ParseStatus::Fallback));
        assert!(!result.story_text().is_empty());
    }

    #[test]
    fn test_generation_flags_default() {
        let json = r#"{"turn_id": 1, "story_json": {"response": "Hello."}}"#;
        let result = parse_llm_output(json);
        let flags = result.flags();
        assert!(!flags.generate_image);
        assert!(!flags.scene_changed);
        assert!(!flags.characters_changed);
    }
}