// src-tauri/src/llm_parser.rs
//
// LLM Output Parser for StoryEngine
// ===================================
// Parses the structured JSON output from the Ollama story model into typed Rust structs.
//
// The parser is resilient to malformed output:
//   1. Direct JSON parse of the full string
//   2. Extract JSON from within preamble/markdown fences
//   3. Strip // comments and retry (model sometimes adds them)
//   4. Try parsing as just StoryJson (partial failure mode)
//   5. Full fallback — treat everything as raw narrative text

use crate::models::SceneCharacter;
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// ENUMS — region and view
// ============================================================================

/// Where in the scene frame a character is positioned.
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
    /// Fallback for unrecognized values
    Other(String),
}

impl CharacterRegion {
    /// Parse from the loose string values the LLM might produce.
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().trim().replace(' ', "-").as_str() {
            "left" => Self::Left,
            "center" => Self::Center,
            "right" => Self::Right,
            "left-seated" | "left_seated" => Self::LeftSeated,
            "center-seated" | "center_seated" => Self::CenterSeated,
            "right-seated" | "right_seated" => Self::RightSeated,
            "left-background" | "left_background" => Self::LeftBackground,
            "center-background" | "center_background" => Self::CenterBackground,
            "right-background" | "right_background" => Self::RightBackground,
            "off-screen" | "off_screen" | "offscreen" => Self::OffScreen,
            other => Self::Other(other.to_string()),
        }
    }

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

    pub fn is_seated(&self) -> bool {
        matches!(self, Self::LeftSeated | Self::CenterSeated | Self::RightSeated)
    }

    pub fn is_background(&self) -> bool {
        matches!(
            self,
            Self::LeftBackground | Self::CenterBackground | Self::RightBackground
        )
    }

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
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub summary_hint: String,
}

/// Scene environment details.
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

/// A character in the scene as returned by the LLM (raw string fields).
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
    /// Parse string fields into typed enums.
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

    /// Convert to the models::SceneCharacter format for DB lookup compatibility.
    pub fn to_scene_character(&self) -> SceneCharacter {
        SceneCharacter {
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
    #[serde(default)]
    pub generate_image: bool,
    #[serde(default)]
    pub scene_changed: bool,
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
    #[serde(default)]
    pub turn_id: u32,
    #[serde(default)]
    pub story_json: Option<StoryJson>,
    #[serde(default)]
    pub scene_json: Option<SceneJson>,
    #[serde(default)]
    pub characters_in_scene: Vec<SceneCharacterRaw>,
    #[serde(default)]
    pub generation_flags: Option<GenerationFlags>,
}

// ============================================================================
// PARSE RESULT
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub enum ParseStatus {
    Ok,
    Partial(Vec<String>),
    Fallback,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedTurn {
    pub status: ParseStatus,
    pub turn: LlmTurnOutput,
    pub raw_text: Option<String>,
}

impl ParsedTurn {
    /// Get the story text for display. Falls back to raw_text if story_json wasn't parsed.
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

    /// Get the summary hint (for context compression).
    pub fn summary_hint(&self) -> &str {
        self.turn
            .story_json
            .as_ref()
            .map(|sj| sj.summary_hint.as_str())
            .unwrap_or("")
    }

    /// Get character names present in this scene.
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
        self.turn.characters_in_scene.iter().map(|c| c.typed()).collect()
    }

    /// Get characters converted to models::SceneCharacter format for DB lookup.
    pub fn characters_for_lookup(&self) -> Vec<SceneCharacter> {
        self.turn.characters_in_scene.iter().map(|c| c.to_scene_character()).collect()
    }

    /// Get only the characters that need to be rendered (not off-screen, view != NONE).
    pub fn renderable_characters(&self) -> Vec<SceneCharacterTyped> {
        self.characters_typed().into_iter().filter(|c| c.needs_render()).collect()
    }

    /// Get the generation flags, or defaults if missing.
    pub fn flags(&self) -> GenerationFlags {
        self.turn.generation_flags.clone().unwrap_or_default()
    }

    pub fn should_generate_image(&self) -> bool {
        self.flags().generate_image
    }

    pub fn scene_changed(&self) -> bool {
        self.flags().scene_changed
    }

    pub fn characters_changed(&self) -> bool {
        self.flags().characters_changed
    }

    pub fn scene(&self) -> Option<&SceneJson> {
        self.turn.scene_json.as_ref()
    }

    /// Build a scene description string for image generation prompts.
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

    /// Convert to frontend-friendly struct for Tauri IPC.
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
// FRONTEND TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTurnForFrontend {
    pub status: String,
    pub warnings: Vec<String>,
    pub turn_id: u32,
    pub story_text: String,
    pub summary_hint: String,
    pub scene: Option<SceneJson>,
    pub characters: Vec<FrontendCharacter>,
    pub flags: GenerationFlags,
    pub scene_prompt_fragment: String,
    pub raw_text: Option<String>,
}

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

// ============================================================================
// CORE PARSE FUNCTION
// ============================================================================

/// Parse raw LLM output into a structured ParsedTurn.
///
/// Attempts in order:
/// 1. Direct JSON parse
/// 2. Strip // comments, retry
/// 3. Extract JSON from preamble/markdown fences, strip comments, retry
/// 4. Try parsing extracted JSON as just StoryJson
/// 5. Full fallback — preserve raw text as narrative
pub fn parse_llm_output(raw: &str) -> ParsedTurn {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return ParsedTurn {
            status: ParseStatus::Fallback,
            turn: empty_turn(),
            raw_text: Some(String::new()),
        };
    }

    // Attempt 1: direct parse
    if let Ok(turn) = serde_json::from_str::<LlmTurnOutput>(trimmed) {
        let warnings = validate_turn(&turn);
        let status = if warnings.is_empty() { ParseStatus::Ok } else { ParseStatus::Partial(warnings) };
        return ParsedTurn { status, turn, raw_text: None };
    }

    // Attempt 2: strip // comments and retry direct parse
    let comment_stripped = strip_json_comments(trimmed);
    if let Ok(turn) = serde_json::from_str::<LlmTurnOutput>(&comment_stripped) {
        let mut warnings = validate_turn(&turn);
        warnings.insert(0, "JSON contained comments that were stripped".to_string());
        return ParsedTurn { status: ParseStatus::Partial(warnings), turn, raw_text: None };
    }

    // Attempt 3: extract JSON object from surrounding text (preamble, markdown fences)
    if let Some(json_str) = extract_json_object(trimmed) {
        // 3a: try directly
        if let Ok(turn) = serde_json::from_str::<LlmTurnOutput>(&json_str) {
            let mut warnings = validate_turn(&turn);
            warnings.insert(0, "JSON was embedded in surrounding text".to_string());
            return ParsedTurn { status: ParseStatus::Partial(warnings), turn, raw_text: None };
        }

        // 3b: strip comments from extracted JSON and retry
        let clean = strip_json_comments(&json_str);
        if let Ok(turn) = serde_json::from_str::<LlmTurnOutput>(&clean) {
            let mut warnings = validate_turn(&turn);
            warnings.insert(0, "JSON embedded in text with comments stripped".to_string());
            return ParsedTurn { status: ParseStatus::Partial(warnings), turn, raw_text: None };
        }

        // 3c: try as just StoryJson (common partial failure — model only output story text)
        if let Ok(story) = serde_json::from_str::<StoryJson>(&clean) {
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

    // Attempt 4: complete fallback
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
// TEXT HELPERS
// ============================================================================

/// Remove JavaScript-style // comments from JSON-like text.
/// Skips characters inside strings to avoid mangling string content.
fn strip_json_comments(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escape = false;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if escape {
            escape = false;
            result.push(c);
            i += 1;
            continue;
        }

        if c == '\\' && in_string {
            escape = true;
            result.push(c);
            i += 1;
            continue;
        }

        if c == '"' {
            in_string = !in_string;
            result.push(c);
            i += 1;
            continue;
        }

        // Strip // line comments when not inside a string
        if !in_string && c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        result.push(c);
        i += 1;
    }

    result
}

/// Find and extract the outermost JSON object from a string.
/// Handles markdown code fences and preamble text before `{`.
fn extract_json_object(text: &str) -> Option<String> {
    // Strip markdown code fences if present
    let stripped: &str = if text.contains("```") {
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

    // Find first '{' and match braces to find the closing '}'
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

/// Clean up raw text being used as a narrative fallback.
fn clean_raw_text(text: &str) -> String {
    text.replace("```json", "")
        .replace("```", "")
        .trim()
        .to_string()
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub fn parse_story_turn(raw_output: String) -> Result<ParsedTurnForFrontend, String> {
    let parsed = parse_llm_output(&raw_output);
    Ok(parsed.to_frontend())
}

#[tauri::command]
pub fn get_story_text(raw_output: String) -> String {
    let parsed = parse_llm_output(&raw_output);
    parsed.story_text().to_string()
}

#[tauri::command]
pub fn get_character_names(raw_output: String) -> Vec<String> {
    let parsed = parse_llm_output(&raw_output);
    parsed.character_names()
}

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
        assert_eq!(result.character_names(), vec!["Marcus", "Elena"]);
    }

    #[test]
    fn test_character_regions_and_views() {
        let result = parse_llm_output(EXAMPLE_JSON);
        let typed = result.characters_typed();
        assert_eq!(typed[0].region, CharacterRegion::Left);
        assert_eq!(typed[0].view, CharacterView::FullBody);
        assert!(typed[0].needs_render());
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
    fn test_json_with_comments() {
        let with_comments = r#"{
            "turn_id": 2,
            "story_json": { "response": "She walked in.", "summary_hint": "Entrance." },
            "scene_json": { "location": "barn", "location_type": "interior", "time_of_day": "morning", "weather": "clear", "lighting": "sunlight", "mood": "peaceful" },
            "characters_in_scene": [
                {
                    "name": "Lisa",
                    "region": "center",
                    "view": "PORTRAIT",
                    "action": "walking",
                    "expression": "smiling",
                    "clothing": "default clothing", // Not specified
                    "facing": "forward"
                }
            ],
            "generation_flags": { "generate_image": true, "scene_changed": false, "characters_changed": false }
        }"#;
        let result = parse_llm_output(with_comments);
        assert!(!matches!(result.status, ParseStatus::Fallback));
        assert_eq!(result.story_text(), "She walked in.");
        assert_eq!(result.character_names(), vec!["Lisa"]);
        assert!(result.should_generate_image());
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
            "story_json": {"response": "A voice calls from outside.", "summary_hint": ""},
            "scene_json": {"location": "room", "location_type": "interior", "time_of_day": "day", "weather": "n/a", "lighting": "dim", "mood": "tense"},
            "characters_in_scene": [
                {"name": "Ghost", "region": "off-screen", "view": "NONE", "action": "", "expression": "", "clothing": "", "facing": ""}
            ],
            "generation_flags": {"generate_image": true, "scene_changed": false, "characters_changed": false}
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
            "story_json": {"response": "Crowds mill about.", "summary_hint": ""},
            "scene_json": {"location": "market", "location_type": "exterior", "time_of_day": "noon", "weather": "sunny", "lighting": "bright", "mood": "busy"},
            "characters_in_scene": [
                {"name": "Vendor", "region": "left-background", "view": "FULL-BODY", "action": "", "expression": "", "clothing": "", "facing": ""},
                {"name": "Hero", "region": "center", "view": "FULL-BODY", "action": "", "expression": "", "clothing": "", "facing": ""}
            ],
            "generation_flags": {"generate_image": true, "scene_changed": false, "characters_changed": false}
        }"#;
        let result = parse_llm_output(json);
        let typed = result.characters_typed();
        assert!(typed[0].region.is_background());
        assert!(!typed[1].region.is_background());
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
        assert!(matches!(CharacterRegion::from_str_loose("floating"), CharacterRegion::Other(_)));
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
    fn test_generation_flags_default() {
        let json = r#"{"turn_id": 1, "story_json": {"response": "Hello.", "summary_hint": ""}}"#;
        let result = parse_llm_output(json);
        let flags = result.flags();
        assert!(!flags.generate_image);
        assert!(!flags.scene_changed);
        assert!(!flags.characters_changed);
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
    fn test_only_story_json_parseable() {
        let json = r#"{"response": "The hero walked forward.", "summary_hint": "Hero moves."}"#;
        let result = parse_llm_output(json);
        assert!(matches!(result.status, ParseStatus::Partial(_)));
        assert_eq!(result.story_text(), "The hero walked forward.");
    }

    #[test]
    fn test_strip_json_comments() {
        let input = r#"{"key": "value", // a comment
"other": "data"}"#;
        let stripped = strip_json_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["other"], "data");
    }

    #[test]
    fn test_strip_comments_preserves_url_strings() {
        // URLs contain // but should not be stripped
        let input = r#"{"url": "https://example.com/path"}"#;
        let stripped = strip_json_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(parsed["url"], "https://example.com/path");
    }
}