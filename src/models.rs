use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Equalizer {
    pub style: String,
    pub presets: Vec<Preset>,
    pub custom_id: Option<u8>,
    pub treble_hz: u16,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Preset {
    pub id: u8,
    pub label: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct SpatialMode {
    pub key: String,
    pub label: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct GestureProfile {
    pub slots: Vec<GestureSlot>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct GestureSlot {
    pub label: String,
    pub r#type: u8,
    pub actions: Vec<GestureAction>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct GestureAction {
    pub value: serde_json::Value,
    pub label: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Model {
    pub name: String,
    pub capabilities: Capabilities,
}

pub fn all() -> &'static [Model] {
    static MODELS: OnceLock<Vec<Model>> = OnceLock::new();
    MODELS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/models.json")).expect("embedded model data")
    })
}

/// Longest match wins, including when users prefix/suffix the advertised name.
/// Unknown devices deliberately have no writable capability profile.
pub fn find(name: &str) -> Option<Capabilities> {
    let name = name.to_lowercase();
    all()
        .iter()
        .filter(|m| name.contains(&m.name.to_lowercase()))
        .max_by_key(|m| m.name.len())
        .map(|m| m.capabilities.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_names_keep_the_specific_profile() {
        let caps = find("My CMF Buds 2 Plus").unwrap();
        assert_eq!(caps.personal_sound.as_deref(), Some("audiodo"));
        assert_eq!(find("Nothing ear (1)").unwrap().eq.custom_id, None);
        assert!(find("Unrelated headset").is_none());
        assert_eq!(all().len(), 20);
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
