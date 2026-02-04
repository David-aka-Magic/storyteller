use tauri::{AppHandle, State};
use crate::state::OllamaState;
use crate::models::{CharacterProfile, StoryPremise};

// --- Characters ---
#[tauri::command]
pub fn save_character(
    character: CharacterProfile,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut chars = state.characters.lock().map_err(|e| format!("{}", e))?;
        if let Some(existing_char) = chars.iter_mut().find(|c| c.id == character.id) {
            *existing_char = character;
        } else {
            chars.push(character);
        }
    } 
    state.save(&app)?;
    Ok(())
}

#[tauri::command]
pub fn delete_character(
    id: String,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut chars = state.characters.lock().map_err(|e| format!("{}", e))?;
        chars.retain(|c| c.id != id);
    } 
    state.save(&app)?;
    Ok(())
}

#[tauri::command]
pub fn get_character_list(state: State<'_, OllamaState>) -> Result<Vec<CharacterProfile>, String> {
    let chars = state.characters.lock().map_err(|e| format!("{}", e))?;
    Ok(chars.clone())
}

// --- Stories ---
#[tauri::command]
pub fn save_story_premise(
    story: StoryPremise,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut stories = state.stories.lock().map_err(|e| format!("{}", e))?;
        if let Some(existing_story) = stories.iter_mut().find(|s| s.id == story.id) {
            *existing_story = story;
        } else {
            stories.push(story);
        }
    } 
    state.save(&app)?;
    Ok(())
}

#[tauri::command]
pub fn delete_stories(
    ids: Vec<String>,
    state: State<'_, OllamaState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut stories = state.stories.lock().map_err(|e| format!("{}", e))?;
        stories.retain(|story| !ids.contains(&story.id));
    } 
    state.save(&app)?;
    Ok(())
}

#[tauri::command]
pub fn get_story_list(state: State<'_, OllamaState>) -> Result<Vec<StoryPremise>, String> {
    let stories = state.stories.lock().map_err(|e| format!("{}", e))?;
    Ok(stories.clone())
}