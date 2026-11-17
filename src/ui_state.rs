//! Compatibility adapter for the original, unmodified Nothing X Mac renderer.
use crate::device::{Device, Key, Value};
use serde_json::{Value as Json, json};

fn byte(d: &Device, key: Key) -> Option<u8> {
    match d.value(key) {
        Some(Value::Byte(v)) => Some(*v),
        _ => None,
    }
}
fn flag(d: &Device, key: Key) -> Option<bool> {
    match d.value(key) {
        Some(Value::Flag(v)) => Some(*v),
        _ => None,
    }
}
pub fn mac(address: [u8; 6]) -> String {
    address
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join("-")
}
pub fn snapshot(d: &Device) -> Json {
    let mut caps = match &d.caps {
        Some(c) => {
            let data: Json = serde_json::from_str(include_str!("../assets/models.json"))
                .expect("embedded profiles");
            let name = d.name.to_lowercase();
            let mut caps = data
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| name.contains(&m["name"].as_str().unwrap().to_lowercase()))
                .max_by_key(|m| m["name"].as_str().unwrap().len())
                .map(|m| m["capabilities"].clone())
                .unwrap_or(Json::Null);
            caps["bassSteps"] = json!(c.bass_steps);
            caps["bassLevels"] = if c.bass_steps == 3 {
                json!([1, 2])
            } else if c.bass_steps == 5 {
                json!([1, 2, 3, 4, 5])
            } else {
                json!([])
            };
            caps
        }
        None => {
            json!({"transparency":false,"strengths":[],"adaptive":false,"bassSteps":0,"bassLevels":[],"inEar":false,"lowLatency":false,"ring":"buds","spatialModes":[],"dualConnection":false,"dualRestarts":false,"personalSound":null,"gestures":null,"eq":{"style":"eq","presets":[],"customId":null,"trebleHz":3400}})
        }
    };
    if d.caps.is_none() {
        caps["gestures"] = Json::Null;
    }
    let battery = d
        .battery
        .iter()
        .filter_map(|(part, b)| {
            let name = match part {
                2 => "left",
                3 => "right",
                4 => "case",
                6 => "stereo",
                _ => return None,
            };
            Some((
                name.to_string(),
                json!({"level":b.level,"charging":b.charging}),
            ))
        })
        .collect::<serde_json::Map<_, _>>();
    let mut battery = json!(battery);
    for key in ["left", "right", "case", "stereo"] {
        if battery.get(key).is_none() {
            battery[key] = Json::Null;
        }
    }
    let code = byte(d, Key::Noise);
    let noise = match code {
        Some(1..=4) => Some("anc"),
        Some(5) => Some("off"),
        Some(7) => Some("transparency"),
        _ => None,
    };
    let strength = match code {
        Some(1) => Some("high"),
        Some(2) => Some("mid"),
        Some(3) => Some("low"),
        Some(4) => Some("adaptive"),
        _ => None,
    };
    let bass = match d.value(Key::Bass) {
        Some(Value::Pair(on, wire)) => {
            let steps = d.caps.as_ref().map_or(5, |c| c.bass_steps);
            let level = if steps == 3 {
                if *wire == 10 { 2 } else { 1 }
            } else {
                (*wire / 2).clamp(1, 5)
            };
            json!({"enabled":*on==1,"level":level})
        }
        _ => Json::Null,
    };
    let custom = match d.value(Key::CustomEq) {
        Some(Value::Eq(b)) => json!({"bass":b[0],"mid":b[1],"treble":b[2]}),
        _ => Json::Null,
    };
    let spatial = match d.value(Key::Spatial) {
        Some(Value::Pair(m, h)) => Some(match (*m, *h) {
            (_, 1) => "headTracked",
            (1, _) => "fixed",
            (2, _) => "concert",
            (3, _) => "theatre",
            (4, _) => "game",
            _ => "off",
        }),
        _ => None,
    };
    let mut gestures = json!({"left":{},"right":{},"pending":{}});
    for (side, name) in [(2, "left"), (3, "right")] {
        for (kind, slot) in [
            (2, "doubleTap"),
            (3, "tripleTap"),
            (7, "tapHold"),
            (9, "doubleTapHold"),
        ] {
            if let Some(value) = byte(d, Key::Gesture(side, kind)) {
                gestures[name][slot] = json!(value);
            }
            if d.is_pending(Key::Gesture(side, kind)) {
                gestures["pending"][format!("{name}.{slot}")] = json!(true);
            }
        }
    }
    json!({"connected":d.connected,"name":if d.connected{Some(d.name.as_str())}else{None},"battery":battery,
        "noiseMode":noise,"ancStrength":strength,"noisePending":d.is_pending(Key::Noise),"bass":bass,"bassPending":d.is_pending(Key::Bass),
        "quick":{"inEar":flag(d,Key::InEar),"lowLatency":flag(d,Key::LowLatency),"personalSound":flag(d,Key::Personal),"spatial":spatial,
            "pending":{"inEar":d.is_pending(Key::InEar),"lowLatency":d.is_pending(Key::LowLatency),"personalSound":d.is_pending(Key::Personal),"spatial":d.is_pending(Key::Spatial)},
            "firmware":if d.firmware.is_empty(){None}else{Some(d.firmware.as_str())},"ringing":d.ringing.map(|s|match s{2=>"left",3=>"right",_=>"all"})},
        "eq":{"preset":byte(d,Key::Preset),"pending":d.is_pending(Key::Preset),"custom":custom},
        "dual":{"enabled":flag(d,Key::Dual),"pending":d.is_pending(Key::Dual),"devices":d.devices.iter().map(|dev|json!({"mac":mac(dev.address),"name":dev.name,"isSelf":dev.is_self,"isConnected":dev.connected})).collect::<Vec<_>>(),"busyMac":d.dual_busy.map(|(a,_)|mac(a)),"maxConnected":2},
        "gestures":gestures,"capabilities":caps})
}

pub fn command(d: &Device, name: &str, args: &[Json]) -> Option<crate::device::Action> {
    use crate::device::{Action, spatial};
    let first = args.first()?;
    let (key, value) = match name {
        "setNoiseMode" => (
            Key::Noise,
            Value::Byte(match first.as_str()? {
                "anc" => byte(d, Key::Noise)
                    .filter(|n| (1..=4).contains(n))
                    .unwrap_or(1),
                "transparency" => 7,
                "off" => 5,
                _ => return None,
            }),
        ),
        "setAncStrength" => (
            Key::Noise,
            Value::Byte(match first.as_str()? {
                "high" => 1,
                "mid" => 2,
                "low" => 3,
                "adaptive" => 4,
                _ => return None,
            }),
        ),
        "setBass" => {
            let current = match d.value(Key::Bass) {
                Some(Value::Pair(on, wire)) => (*on, *wire),
                _ => (0, 2),
            };
            let steps = d.caps.as_ref()?.bass_steps;
            let level = first.get("level").and_then(Json::as_u64).map(|n| n as u8);
            let on = first
                .get("enabled")
                .and_then(Json::as_bool)
                .map(u8::from)
                .unwrap_or(current.0);
            let wire = level
                .map(|n| {
                    if steps == 3 {
                        n.saturating_mul(5)
                    } else {
                        n.saturating_mul(2)
                    }
                })
                .unwrap_or(current.1);
            (
                Key::Bass,
                Value::Pair(on, if steps == 3 && on == 0 { 0 } else { wire }),
            )
        }
        "setQuickToggle" => (
            match first.as_str()? {
                "inEar" => Key::InEar,
                "lowLatency" => Key::LowLatency,
                "personalSound" => Key::Personal,
                _ => return None,
            },
            Value::Flag(args.get(1)?.as_bool()?),
        ),
        "setSpatialAudio" => {
            let (a, b) = spatial(first.as_str()?);
            (Key::Spatial, Value::Pair(a, b))
        }
        "setDualConnection" => (Key::Dual, Value::Flag(first.as_bool()?)),
        "setEqPreset" => (
            Key::Preset,
            Value::Byte(u8::try_from(first.as_u64()?).ok()?),
        ),
        "setCustomEq" => {
            let mut bands = match d.value(Key::CustomEq) {
                Some(Value::Eq(b)) => *b,
                _ => [0; 3],
            };
            for (i, key) in ["bass", "mid", "treble"].iter().enumerate() {
                if let Some(n) = first.get(key).and_then(Json::as_i64) {
                    bands[i] = n.clamp(-6, 6) as i8;
                }
            }
            (Key::CustomEq, Value::Eq(bands))
        }
        "ring" => {
            return Some(if args.get(1)?.as_bool()? {
                Action::Ring(match first.as_str()? {
                    "left" => 2,
                    "right" => 3,
                    "all" => 6,
                    _ => return None,
                })
            } else {
                Action::StopRing
            });
        }
        "setDualDevice" => {
            let bytes = first
                .as_str()?
                .split(['-', ':'])
                .map(|s| u8::from_str_radix(s, 16))
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            return Some(Action::DualConnect(
                bytes.try_into().ok()?,
                args.get(1)?.as_bool()?,
            ));
        }
        "setGesture" | "setGestureNoiseCycle" => {
            let side = match first.as_str()? {
                "left" => 2,
                "right" => 3,
                _ => return None,
            };
            let slot = match args.get(1)?.as_str()? {
                "doubleTap" => 2,
                "tripleTap" => 3,
                "tapHold" => 7,
                "doubleTapHold" => 9,
                _ => return None,
            };
            let v = args.get(2)?;
            let code = if name == "setGestureNoiseCycle" {
                match (
                    v["anc"].as_bool()?,
                    v["transparency"].as_bool()?,
                    v["off"].as_bool()?,
                ) {
                    (true, true, true) => 10,
                    (false, true, true) => 20,
                    (true, false, true) => 21,
                    (true, true, false) => 22,
                    _ => return None,
                }
            } else if v.as_str() == Some("noise") {
                22
            } else {
                u8::try_from(v.as_u64()?).ok()?
            };
            (Key::Gesture(side, slot), Value::Byte(code))
        }
        _ => return None,
    };
    Some(Action::Set(key, value))
}

pub fn appearance() -> Json {
    serde_json::from_str(include_str!("../assets/appearance.json")).expect("embedded appearance")
}
pub fn init(d: &Device) -> Json {
    let mut a = appearance();
    a["appName"] = json!("NothingX Desktop");
    a["device"] = snapshot(d);
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_renderer_receives_complete_model_data() {
        let d = Device::simulate("CMF Buds 2 Plus");
        let s = snapshot(&d);
        assert_eq!(s["capabilities"]["gestures"]["sides"]["left"], 2);
        assert_eq!(s["capabilities"]["eq"]["presets"][0]["key"], "pop");
        assert_eq!(s["battery"]["left"]["level"], 87);
        assert!(s["quick"]["pending"]["spatial"].is_boolean());
    }
    #[test]
    fn disconnected_state_has_every_renderer_branch() {
        let s = snapshot(&Device::default());
        assert_eq!(s["battery"]["left"], Json::Null);
        assert_eq!(s["dual"]["devices"], json!([]));
        assert_eq!(s["capabilities"]["spatialModes"], json!([]));
        assert_eq!(crate::models::all().len(), 20);
    }
}
