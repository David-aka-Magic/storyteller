use serde_json::Value;

pub fn extract_json(text: &str) -> Option<String> {
    if let Some(s) = text.find('{') {
        let mut c = 0;
        let mut st = false;
        let mut esc = false;
        for (i, ch) in text[s..].char_indices() {
            match ch {
                '{' if !st => c += 1,
                '}' if !st => {
                    c -= 1;
                    if c == 0 {
                        let e = s + i + 1;
                        return Some(text[s..e].to_string());
                    }
                }
                '"' if !esc => st = !st,
                '\\' => esc = !esc,
                _ => esc = false,
            }
        }
    }
    None
}

pub fn extract_story(v: &Value) -> String {
    if let Some(s) = v.get("story_json") {
        if let Some(r) = s.get("response").and_then(|v| v.as_str()) {
            return r.to_string();
        }
        if let Some(t) = s.as_str() {
            return t.to_string();
        }
    }
    if let Some(t) = v.get("response").and_then(|v| v.as_str()) {
        return t.to_string();
    }
    serde_json::to_string(v).unwrap_or_default()
}