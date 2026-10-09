//! Replays every case recorded from rivalcfg (tools/gen_golden.py) and checks
//! that the Rust port produces the exact same HID packets (or also fails).

use rivalgui_lib::core::encode::{save_packet, setting_packet};
use rivalgui_lib::core::profile::catalog;
use serde_json::Value;

#[test]
fn golden_packets_match_rivalcfg() {
    let golden: Value = serde_json::from_str(include_str!("golden.json")).unwrap();
    let cases = golden["cases"].as_array().unwrap();
    let mut failures = Vec::new();

    for case in cases {
        let vid = case["vid"].as_u64().unwrap() as u16;
        let pid = case["pid"].as_u64().unwrap() as u16;
        let setting_name = case["setting"].as_str().unwrap();
        let value = &case["value"];
        let profile = catalog().get(vid, pid).expect("profile exported");

        let result = if setting_name == "__save__" {
            save_packet(profile)
        } else {
            let setting = profile.setting(setting_name).expect("setting exported");
            setting_packet(setting, value, case["selected_preset"].as_i64())
        };
        let actual = result.map(|p| {
            let mut bytes = vec![p.report_type];
            bytes.extend(p.data);
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        });

        let ok = match (case.get("bytes"), &actual) {
            (Some(expected), Ok(actual)) => expected.as_str() == Some(actual.as_str()),
            (None, Err(_)) => true,
            _ => false,
        };
        if !ok {
            failures.push(format!(
                "{} / {} = {}\n    expected: {}\n    actual:   {:?}",
                profile.name,
                setting_name,
                value,
                case.get("bytes").or(case.get("error")).unwrap(),
                actual
            ));
        }
    }

    if !failures.is_empty() {
        let shown: Vec<_> = failures.iter().take(25).collect();
        panic!(
            "{} / {} golden cases failed:\n{}",
            failures.len(),
            cases.len(),
            shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
        );
    }
    println!("{} golden cases OK", cases.len());
}
