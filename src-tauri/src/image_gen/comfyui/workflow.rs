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
        "(cropped head:1.5), (head out of frame:1.5), (cut off head:1.5), (headless:1.5), decapitated, \
         (worst quality, low quality:1.4), (bad anatomy:1.3), (bad hands:1.4), \
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
         stiff pose, t-pose, a-pose, mannequin pose, \
         (blurry eyes:1.4), (unfocused eyes:1.3), (glossy eyes:1.2), \
         (plastic skin:1.3), (waxy skin:1.2), (airbrushed:1.3), (smooth skin:1.2)"
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
    // Always set sampler and scheduler for consistent quality.
    // dpmpp_2m_sde + karras produces the sharpest facial detail with JuggernautXL.
    ksampler_inputs.insert("sampler_name".to_string(), Value::String("dpmpp_2m_sde".to_string()));
    ksampler_inputs.insert("scheduler".to_string(), Value::String("karras".to_string()));
    if !ksampler_inputs.is_empty() {
        mods.insert("35".to_string(), ksampler_inputs);
    }

    // --- InsightFace model name (node 12: IPAdapterInsightFaceLoader) ---
    {
        let mut inputs = HashMap::new();
        inputs.insert("model_name".to_string(), Value::String("buffalo_l".to_string()));
        mods.insert("12".to_string(), inputs);
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
// CONTROLNET INJECTION
// ============================================================================

/// Inject ControlNet nodes into the workflow for pose guidance.
///
/// Adds 3 nodes:
///   Node "60" — ControlNetLoader: loads the OpenPose SDXL model
///   Node "61" — LoadImage: loads the skeleton image (already uploaded to ComfyUI)
///   Node "62" — ControlNetApplyAdvanced: applies conditioning
///
/// Rewires KSampler (node "35") so positive/negative come from the ControlNet
/// apply node instead of directly from the CLIP encoders.
pub(super) fn inject_controlnet(
    workflow: &mut Value,
    skeleton_filename: &str,
    strength: f64,
) -> Result<(), ComfyError> {
    let obj = workflow
        .as_object_mut()
        .ok_or_else(|| ComfyError::WorkflowLoadFailed("Workflow root is not an object".into()))?;

    // Node 60: ControlNetLoader
    obj.insert("60".to_string(), serde_json::json!({
        "class_type": "ControlNetLoader",
        "inputs": {
            "control_net_name": "OpenPoseXL2.safetensors"
        }
    }));

    // Node 61: LoadImage — the uploaded skeleton PNG
    obj.insert("61".to_string(), serde_json::json!({
        "class_type": "LoadImage",
        "inputs": {
            "image": skeleton_filename,
            "upload": "image"
        }
    }));

    // Node 62: ControlNetApplyAdvanced
    obj.insert("62".to_string(), serde_json::json!({
        "class_type": "ControlNetApplyAdvanced",
        "inputs": {
            "positive":    ["2",  0],
            "negative":    ["3",  0],
            "control_net": ["60", 0],
            "image":       ["61", 0],
            "strength":    strength,
            "start_percent": 0.0,
            "end_percent":   0.8
        }
    }));

    // Rewire KSampler (node "35"): positive → ["62", 0], negative → ["62", 1]
    if let Some(node_35) = obj.get_mut("35") {
        if let Some(inputs) = node_35.get_mut("inputs") {
            if let Some(pos) = inputs.get_mut("positive") {
                if pos.as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    == Some("2")
                {
                    *pos = serde_json::json!(["62", 0]);
                }
            }
            if let Some(neg) = inputs.get_mut("negative") {
                if neg.as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    == Some("3")
                {
                    *neg = serde_json::json!(["62", 1]);
                }
            }
        }
    }

    println!(
        "[ComfyUI] Injected ControlNet nodes (60,61,62): skeleton={}, strength={}",
        skeleton_filename, strength
    );
    Ok(())
}

// ============================================================================
// HIRES FIX INJECTION
// ============================================================================

/// Inject a hi-res fix (latent upscale + second sampling pass) into the workflow.
///
/// This mimics A1111's "Hires. fix" feature:
///   Node "70" — LatentUpscaleBy: upscales the first-pass latent by `upscale_factor`
///   Node "71" — KSampler: re-denoises the upscaled latent at low denoise strength
///
/// Rewires the VAEDecode (node "6") to read from the second KSampler instead of the first.
/// The second pass inherits the same model and conditioning sources as the first KSampler.
pub(super) fn inject_hires_fix(
    workflow: &mut Value,
    upscale_factor: f64,
    denoise_strength: f64,
    hires_steps: u32,
) -> Result<(), ComfyError> {
    let obj = workflow
        .as_object_mut()
        .ok_or_else(|| ComfyError::WorkflowLoadFailed("Workflow root is not an object".into()))?;

    // Determine what the first KSampler's positive/negative sources are.
    // If ControlNet was injected, KSampler "35" reads from ["62", 0/1].
    // Otherwise it reads from ["2", 0] and ["3", 0].
    let (pos_source, neg_source) = if let Some(node_35) = obj.get("35") {
        let pos = node_35.pointer("/inputs/positive").cloned().unwrap_or(serde_json::json!(["2", 0]));
        let neg = node_35.pointer("/inputs/negative").cloned().unwrap_or(serde_json::json!(["3", 0]));
        (pos, neg)
    } else {
        (serde_json::json!(["2", 0]), serde_json::json!(["3", 0]))
    };

    // Node 70: LatentUpscaleBy — upscale the first-pass latent output
    obj.insert("70".to_string(), serde_json::json!({
        "class_type": "LatentUpscaleBy",
        "inputs": {
            "samples": ["35", 0],
            "upscale_method": "bilinear",
            "scale_by": upscale_factor
        }
    }));

    // Node 71: Second KSampler — re-denoise the upscaled latent
    obj.insert("71".to_string(), serde_json::json!({
        "class_type": "KSampler",
        "inputs": {
            "model": ["1", 0],
            "positive": pos_source,
            "negative": neg_source,
            "latent_image": ["70", 0],
            "seed": -1,
            "steps": hires_steps,
            "cfg": 5.5,
            "sampler_name": "dpmpp_2m_sde",
            "scheduler": "karras",
            "denoise": denoise_strength
        }
    }));

    // Rewire VAEDecode (node "6") to read from the second KSampler
    if let Some(node_6) = obj.get_mut("6") {
        if let Some(inputs) = node_6.get_mut("inputs") {
            if let Some(samples) = inputs.get_mut("samples") {
                *samples = serde_json::json!(["71", 0]);
            }
        }
    }

    println!(
        "[ComfyUI] Injected hires fix: scale={}x, denoise={}, steps={}",
        upscale_factor, denoise_strength, hires_steps
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
            controlnet_image_path: None,
            controlnet_strength: None,
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
