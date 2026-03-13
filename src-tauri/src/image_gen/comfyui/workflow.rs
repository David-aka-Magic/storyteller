// src-tauri/src/image_gen/comfyui/workflow.rs
//
// Workflow template operations
// ==============================
// Load, inspect, and modify ComfyUI workflow JSON templates.

use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

use super::client::ComfyError;
use super::pipeline::ImageGenRequest;

// ============================================================================
// TEMPLATE I/O
// ============================================================================

/// Load a workflow JSON template from disk.
pub(super) fn load_workflow_template(path: &Path) -> Result<Value, ComfyError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        ComfyError::WorkflowLoadFailed(format!("Cannot read {}: {}", path.display(), e))
    })?;

    serde_json::from_str(&content).map_err(|e| {
        ComfyError::WorkflowLoadFailed(format!("Invalid JSON in {}: {}", path.display(), e))
    })
}

// ============================================================================
// WORKFLOW MODIFICATION
// ============================================================================

/// Modify a workflow JSON in-place to set node inputs.
///
/// `modifications` maps node_id → (input_key → new_value).
/// Example: `{ "2": { "text": "cozy coffee shop" }, "4": { "width": 1152 } }`
pub(super) fn modify_workflow(
    workflow: &mut Value,
    modifications: &HashMap<String, HashMap<String, Value>>,
) -> Result<(), ComfyError> {
    let obj = workflow
        .as_object_mut()
        .ok_or_else(|| ComfyError::WorkflowLoadFailed("Workflow root is not an object".into()))?;

    for (node_id, inputs) in modifications {
        if let Some(node) = obj.get_mut(node_id) {
            if let Some(node_inputs) = node.get_mut("inputs") {
                if let Some(input_obj) = node_inputs.as_object_mut() {
                    for (key, value) in inputs {
                        input_obj.insert(key.clone(), value.clone());
                    }
                }
            }
        } else {
            println!(
                "[ComfyUI] Warning: node '{}' not found in workflow, skipping modifications",
                node_id
            );
        }
    }

    Ok(())
}

/// Build node modifications for the workflow template based on the request.
///
/// Creates a mapping of node_id → { input_key: value } that will be injected
/// into the workflow JSON. Node IDs must match the template.
///
/// Node ID conventions (matching workflow JSON):
///   - "2" = positive prompt, "3" = negative prompt, "4" = empty latent
///   - "20","21" = character reference image loaders
///   - "40","41" = per-character mask image loaders
///   - "35" = KSampler (seed, steps, cfg)
pub(super) fn build_workflow_modifications(
    request: &ImageGenRequest,
    uploaded_refs: &[String],
    mask_filenames: &[String],
) -> HashMap<String, HashMap<String, Value>> {
    let mut mods: HashMap<String, HashMap<String, Value>> = HashMap::new();

    // --- Positive prompt ---
    let mut positive_inputs = HashMap::new();
    positive_inputs.insert("text".to_string(), Value::String(request.scene_prompt.clone()));
    mods.insert("2".to_string(), positive_inputs);

    // --- Negative prompt ---
    let neg_text = request.negative_prompt.as_deref().unwrap_or(
        "(worst quality, low quality:1.4), (bad anatomy:1.3), (bad hands:1.4), \
         (missing fingers:1.3), (extra fingers:1.3), (too many fingers:1.4), \
         (fused fingers:1.3), (poorly drawn hands:1.4), (floating limbs:1.3), \
         (disconnected limbs:1.3), (extra limbs:1.3), (missing arms:1.2), \
         (extra arms:1.2), (deformed:1.3), (mutated:1.2), (disfigured:1.2), \
         (malformed:1.2), blurry, lowres, watermark, text, signature, cropped, \
         out of frame, ugly, duplicate, cloned face, poorly drawn face, \
         (floating head:1.4), (detached head:1.4), (severed head:1.3), \
         bad proportions, gross proportions, long neck, (mutation:1.2), \
         (asymmetric eyes:1.4), (crossed eyes:1.4), (lazy eye:1.3), \
         (uneven eyes:1.4), (different sized eyes:1.4), (misaligned eyes:1.4), \
         (bad eyes:1.3), (poorly drawn eyes:1.3), (extra eyes:1.3), \
         (bad iris:1.3), (bad pupils:1.3), (distorted pupils:1.3), \
         (dead eyes:1.2), (empty eyes:1.2), \
         (bad face:1.3), (asymmetric face:1.3), (distorted face:1.3), \
         (cross-eyed:1.4), (wall-eyed:1.3), (strabismus:1.4), \
         (unfocused eyes:1.3), (different colored eyes:1.3), (heterochromia:1.2), \
         (wonky eyes:1.3), (derpy eyes:1.3), \
         close up, closeup, headshot, upper body only, face only, \
         portrait crop, zoomed in, \
         masculine features, strong jawline, cleft chin, square jaw, \
         angular face, manly, \
         standing when should be sitting, standing when should be lying down, \
         stiff pose, t-pose, a-pose, mannequin pose"
    );
    let mut neg_inputs = HashMap::new();
    neg_inputs.insert("text".to_string(), Value::String(neg_text.to_string()));
    mods.insert("3".to_string(), neg_inputs);

    // --- Empty latent dimensions ---
    if request.width.is_some() || request.height.is_some() {
        let mut latent_inputs = HashMap::new();
        if let Some(w) = request.width {
            latent_inputs.insert("width".to_string(), Value::Number(w.into()));
        }
        if let Some(h) = request.height {
            latent_inputs.insert("height".to_string(), Value::Number(h.into()));
        }
        mods.insert("4".to_string(), latent_inputs);
    }

    // --- KSampler ---
    let mut ksampler_inputs = HashMap::new();
    if let Some(seed) = request.seed {
        ksampler_inputs.insert(
            "seed".to_string(),
            Value::Number(serde_json::Number::from(seed)),
        );
    }
    if let Some(steps) = request.steps {
        ksampler_inputs.insert("steps".to_string(), Value::Number(steps.into()));
    }
    if let Some(cfg) = request.cfg {
        if let Some(n) = serde_json::Number::from_f64(cfg) {
            ksampler_inputs.insert("cfg".to_string(), Value::Number(n));
        }
    }
    if !ksampler_inputs.is_empty() {
        mods.insert("35".to_string(), ksampler_inputs);
    }

    // --- Character reference images (nodes 20, 21) ---
    let ref_node_ids = ["20", "21"];
    for (i, ref_filename) in uploaded_refs.iter().enumerate() {
        if i >= ref_node_ids.len() {
            break;
        }
        let mut inputs = HashMap::new();
        inputs.insert("image".to_string(), Value::String(ref_filename.clone()));
        mods.insert(ref_node_ids[i].to_string(), inputs);
    }

    // --- Per-character mask images (nodes 40, 41) ---
    let mask_node_ids = ["40", "41"];
    for (i, mask_name) in mask_filenames.iter().enumerate() {
        if i >= mask_node_ids.len() {
            break;
        }
        let mut inputs = HashMap::new();
        inputs.insert("image".to_string(), Value::String(mask_name.clone()));
        mods.insert(mask_node_ids[i].to_string(), inputs);
    }

    mods
}

// ============================================================================
// POSE LORA INJECTION
// ============================================================================

/// Inject a LoraLoader node into the workflow for pose LoRAs.
/// This rewires the model/CLIP chain: Checkpoint → LoraLoader → KSampler/CLIP.
/// Uses node ID "50" for the LoraLoader to avoid conflicts with existing nodes.
pub(super) fn inject_pose_lora(
    workflow: &mut Value,
    lora_filename: &str,
    strength: f64,
) -> Result<(), ComfyError> {
    let obj = workflow
        .as_object_mut()
        .ok_or_else(|| ComfyError::WorkflowLoadFailed("Workflow root is not an object".into()))?;

    // 1. Insert the LoraLoader node
    let lora_node = serde_json::json!({
        "class_type": "LoraLoader",
        "inputs": {
            "model": ["1", 0],
            "clip": ["1", 1],
            "lora_name": lora_filename,
            "strength_model": strength,
            "strength_clip": strength
        }
    });
    obj.insert("50".to_string(), lora_node);

    // 2. Rewire KSampler (node "35") to take model from LoraLoader instead of Checkpoint
    if let Some(node_35) = obj.get_mut("35") {
        if let Some(inputs) = node_35.get_mut("inputs") {
            if let Some(model_input) = inputs.get_mut("model") {
                if model_input
                    .as_array()
                    .map(|a| a.first().and_then(|v| v.as_str()) == Some("1"))
                    .unwrap_or(false)
                {
                    *model_input = serde_json::json!(["50", 0]);
                }
            }
        }
    }

    // 3. Rewire CLIP text encode nodes ("2" positive, "3" negative) to use LoRA's CLIP
    for clip_node_id in &["2", "3"] {
        if let Some(node) = obj.get_mut(*clip_node_id) {
            if let Some(inputs) = node.get_mut("inputs") {
                if let Some(clip_input) = inputs.get_mut("clip") {
                    if clip_input
                        .as_array()
                        .map(|a| a.first().and_then(|v| v.as_str()) == Some("1"))
                        .unwrap_or(false)
                    {
                        *clip_input = serde_json::json!(["50", 1]);
                    }
                }
            }
        }
    }

    println!(
        "[ComfyUI] Injected LoraLoader node 50: lora={}, strength={}",
        lora_filename, strength
    );
    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_gen::comfyui::pipeline::{CharacterInput, ImageGenRequest};
    use serde_json::json;

    #[test]
    fn test_modify_workflow_sets_inputs() {
        let mut workflow = json!({
            "2": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "clip": ["1", 1],
                    "text": "placeholder"
                }
            },
            "35": {
                "class_type": "KSampler",
                "inputs": {
                    "seed": 0,
                    "steps": 20
                }
            }
        });

        let mut mods = HashMap::new();
        let mut prompt_mod = HashMap::new();
        prompt_mod.insert("text".to_string(), json!("cozy coffee shop, warm lighting"));
        mods.insert("2".to_string(), prompt_mod);

        let mut sampler_mod = HashMap::new();
        sampler_mod.insert("seed".to_string(), json!(42));
        sampler_mod.insert("steps".to_string(), json!(30));
        mods.insert("35".to_string(), sampler_mod);

        modify_workflow(&mut workflow, &mods).unwrap();

        assert_eq!(workflow["2"]["inputs"]["text"], "cozy coffee shop, warm lighting");
        assert_eq!(workflow["35"]["inputs"]["seed"], 42);
        assert_eq!(workflow["35"]["inputs"]["steps"], 30);
        // Untouched inputs preserved
        assert_eq!(workflow["2"]["inputs"]["clip"], json!(["1", 1]));
    }

    #[test]
    fn test_modify_workflow_missing_node_is_warning_not_error() {
        let mut workflow = json!({
            "2": {
                "class_type": "CLIPTextEncode",
                "inputs": { "text": "hello" }
            }
        });

        let mut mods = HashMap::new();
        let mut missing_mod = HashMap::new();
        missing_mod.insert("text".to_string(), json!("world"));
        mods.insert("999".to_string(), missing_mod);

        // Should not error, just warn
        let result = modify_workflow(&mut workflow, &mods);
        assert!(result.is_ok());
        // Original untouched
        assert_eq!(workflow["2"]["inputs"]["text"], "hello");
    }

    #[test]
    fn test_build_workflow_modifications_basic() {
        let request = ImageGenRequest {
            scene_prompt: "test scene".to_string(),
            characters: vec![CharacterInput {
                name: "Alice".to_string(),
                reference_image_path: "/path/to/alice.png".to_string(),
                region: "left".to_string(),
                prompt: "smiling".to_string(),
            }],
            mask_paths: vec!["/path/to/mask.png".to_string()],
            workflow_template: "template.json".to_string(),
            comfyui_url: None,
            seed: Some(123),
            steps: Some(25),
            cfg: Some(5.5),
            width: Some(1152),
            height: Some(896),
            negative_prompt: None,
            timeout_secs: None,
            pose_lora_filename: None,
            pose_lora_strength: None,
        };

        let uploaded_refs = vec!["ref_alice_0.png".to_string()];
        let mask_filenames = vec!["scene_mask.png".to_string()];

        let mods = build_workflow_modifications(&request, &uploaded_refs, &mask_filenames);

        // Positive prompt set
        assert_eq!(mods["2"]["text"], json!("test scene"));
        // KSampler
        assert_eq!(mods["35"]["seed"], json!(123));
        assert_eq!(mods["35"]["steps"], json!(25));
        // Latent
        assert_eq!(mods["4"]["width"], json!(1152));
        assert_eq!(mods["4"]["height"], json!(896));
        // Ref image
        assert_eq!(mods["20"]["image"], json!("ref_alice_0.png"));
        assert_eq!(mods["40"]["image"], json!("scene_mask.png"));
    }
}
