use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub noise_cancellation: bool,
    pub transparency: bool,
    pub strengths: Vec<String>,
    pub adaptive: bool,
    pub bass_steps: u8,
    pub in_ear: bool,
    pub low_latency: bool,
    pub ring: String,
    pub eq: Equalizer,
    pub spatial_modes: Vec<SpatialMode>,
    pub dual_connection: bool,
    pub dual_restarts: bool,
    pub personal_sound: Option<String>,
    pub gestures: Option<GestureProfile>,
}

impl Capabilities {
    pub fn empty() -> Self {
        Self {
            noise_cancellation: false,
            transparency: false,
            strengths: Vec::new(),
            adaptive: false,
            bass_steps: 0,
            in_ear: false,
            low_latency: false,
            ring: "none".into(),
            eq: Equalizer {
                style: "eq".into(),
                presets: Vec::new(),
                custom_id: None,
                treble_hz: 3400,
            },
            spatial_modes: Vec::new(),
            dual_connection: false,
            dual_restarts: false,
            personal_sound: None,
            gestures: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Equalizer {
    pub style: String,
    pub presets: Vec<Preset>,
    pub custom_id: Option<u8>,
    pub treble_hz: u16,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Preset {
    pub id: u8,
    pub key: String,
    pub label: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpatialMode {
    pub key: String,
    pub label: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GestureProfile {
    pub sides: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise_cycles: Option<serde_json::Value>,
    pub slots: Vec<GestureSlot>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GestureSlot {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub r#type: u8,
    pub actions: Vec<GestureAction>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GestureAction {
    pub value: serde_json::Value,
    pub label: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Model {
    pub name: String,
    #[serde(skip)]
    name_lower: String,
    pub capabilities: Capabilities,
}

pub fn all() -> &'static [Model] {
    static MODELS: OnceLock<Vec<Model>> = OnceLock::new();
    MODELS.get_or_init(|| {
        let mut models: Vec<Model> = serde_json::from_str(include_str!("../assets/models.json"))
            .expect("embedded model data");
        for model in &mut models {
            model.name_lower = model.name.to_lowercase();
        }
        models
    })
}

/// Longest match wins, including when users prefix/suffix the advertised name,
/// without misclassifying newer model generations (e.g. "CMF Buds 3" as "CMF Buds"
/// or "Nothing Ear (4)" as "Nothing Ear").
pub fn find(name: &str) -> Option<Capabilities> {
    let name = name.to_lowercase();
    all()
        .iter()
        .filter(|m| matches_model(&name, &m.name_lower))
        .max_by_key(|m| m.name.len())
        .map(|m| m.capabilities.clone())
}

fn matches_model(name: &str, model: &str) -> bool {
    let mut offset = 0;
    while let Some(pos) = name[offset..].find(model) {
        let start = offset + pos;
        let end = start + model.len();
        let before_ok = name[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        let after_ok = name[end..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        if before_ok && after_ok && !has_model_generation_suffix(&name[end..]) {
            return true;
        }
        offset = start + model.len();
    }
    false
}

fn has_model_generation_suffix(after: &str) -> bool {
    let trimmed = after.trim_start();
    let Some(first) = trimmed.chars().next() else {
        return false;
    };
    if first.is_ascii_digit() || first == '(' {
        return true;
    }
    let word: String = trimmed
        .chars()
        .take_while(|c| c.is_alphanumeric())
        .collect();
    matches!(
        word.as_str(),
        "pro"
            | "plus"
            | "max"
            | "mini"
            | "lite"
            | "air"
            | "neo"
            | "ultra"
            | "open"
            | "stick"
            | "clip"
            | "se"
            | "fe"
            | "a"
            | "s"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_names_keep_the_specific_profile() {
        let caps = find("My CMF Buds 2 Plus").unwrap();
        assert_eq!(caps.personal_sound.as_deref(), Some("audiodo"));
        assert!(find("CMF Buds Pro 2 - Work").is_some());
        assert_eq!(find("Nothing ear (1)").unwrap().eq.custom_id, None);
        assert!(find("Unrelated headset").is_none());
        assert_eq!(all().len(), 20);
    }
    #[test]
    fn newer_unlisted_generations_do_not_misclassify_as_older_models() {
        for future in [
            "CMF Buds 3",
            "CMF Buds 3 Pro",
            "CMF Buds Pro 3",
            "Nothing Ear (4)",
            "Nothing Ear Pro",
        ] {
            assert!(
                find(future).is_none(),
                "{future} should stay unlisted for auto-probing"
            );
        }
    }
    #[test]
    fn anc_support_is_independent_of_adjustable_intensity() {
        for name in ["CMF Buds", "CMF Buds 2a"] {
            let caps = find(name).unwrap();
            assert!(caps.noise_cancellation);
            assert!(caps.strengths.is_empty());
        }
        assert!(!find("Nothing Ear (open)").unwrap().noise_cancellation);
    }
}
