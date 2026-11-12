"""Exports rivalcfg device profiles and button layouts to JSON for RivalGUI.

Usage::

    python tools/export_profiles.py [output.json]

The output is embedded in the Rust binary (src-tauri/resources/profiles.json).
"""

import json
import os
import sys

import _rivalcfg_import as r

OUTPUT = (
    sys.argv[1]
    if len(sys.argv) > 1
    else os.path.join(
        os.path.dirname(__file__), "..", "src-tauri", "resources", "profiles.json"
    )
)

# Probe data used to identify which battery formula a profile's lambdas implement
_BATTERY_PROBE = [0x10, 0x85, 0x01]
_BATTERY_FORMATS = {
    # ((d[1] & 0x7F) - 1) * 5, charging = d[1] & 0x80
    "aerox": (20, True),
    # d[0], charging = d[2]
    "rival": (16, True),
    # d[2], no charging state (swappable battery)
    "rival_pro": (1, None),
}

# Probe data used to identify which parser a profile uses for the firmware version
_FIRMWARE_PROBE = list(b"\x90\x001.2.3".ljust(32, b"\0"))
_FIRMWARE_FORMATS = {
    # "<cmd> 00 <ASCII version>", e.g. "2.14.2"
    "ascii": [1, 2, 3],
}


def battery_format(battery_level):
    result = (
        battery_level["level"](_BATTERY_PROBE),
        battery_level["is_charging"](_BATTERY_PROBE)
        if "is_charging" in battery_level
        else None,
    )
    for name, expected in _BATTERY_FORMATS.items():
        if result == expected:
            return name
    raise ValueError("Unknown battery formula (probe result: %r)" % (result,))


def firmware_format(firmware_version):
    result = list(firmware_version["parse"](_FIRMWARE_PROBE))
    for name, expected in _FIRMWARE_FORMATS.items():
        if result == expected:
            return name
    raise ValueError("Unknown firmware version parser (probe result: %r)" % (result,))


def export_setting(name, info):
    setting = {"name": name}
    for key, value in info.items():
        if key == "choices":
            # JSON objects only have string keys: keep the original key type and order
            setting["choices"] = [{"key": k, "value": v} for k, v in value.items()]
        elif key == "output_choices":
            # DPI -> device value tables (range_choice handlers). Identity tables
            # (DPI sent as-is, e.g. 65000 entries for the Rival Pro) are only
            # flagged, the app rebuilds them from input_range.
            start, stop, step = info["input_range"]
            if list(value.items()) == [(i, i) for i in range(start, stop + 1, step)]:
                setting["output_choices_identity"] = True
            else:
                setting["output_choices"] = [[k, v] for k, v in value.items()]
        elif key == "buttons":
            setting["buttons"] = [{"name": k, **v} for k, v in value.items()]
        else:
            setting[key] = value
    return setting


def export_profile(profile):
    result = {
        "name": profile["name"],
        "vendor_id": profile["vendor_id"],
        "product_id": profile["product_id"],
        "endpoint": profile["endpoint"],
        "settings": [export_setting(n, i) for n, i in profile["settings"].items()],
    }
    if "save_command" in profile and profile["save_command"]:
        result["save_command"] = profile["save_command"]
    if "firmware_version" in profile and profile["firmware_version"]:
        firmware = {
            k: v for k, v in profile["firmware_version"].items() if k != "parse"
        }
        if "parse" in profile["firmware_version"]:
            firmware["format"] = firmware_format(profile["firmware_version"])
        result["firmware_version"] = firmware
    if "battery_level" in profile:
        battery = {
            k: v
            for k, v in profile["battery_level"].items()
            if k not in ("level", "is_charging")
        }
        battery["format"] = battery_format(profile["battery_level"])
        result["battery_level"] = battery
    return result


def main():
    keyboard = r.buttons_handler.build_layout(r.layout_qwerty)
    multimedia = r.buttons_handler.build_layout(r.layout_multimedia)
    data = {
        "rivalcfg_version": r.VERSION,
        "profiles": [export_profile(p) for p in r.devices.PROFILES.values()],
        "layouts": {
            # Lookup tables (lowercase name or alias -> keycode)
            "qwerty": keyboard,
            "multimedia": multimedia,
            # Canonical key names in source order, for the UI pickers
            "qwerty_keys": list(r.layout_qwerty.layout.keys()),
            "multimedia_keys": list(r.layout_multimedia.layout.keys()),
        },
    }
    os.makedirs(os.path.dirname(os.path.abspath(OUTPUT)), exist_ok=True)
    with open(OUTPUT, "w", encoding="utf-8") as file_:
        json.dump(data, file_, indent=1, ensure_ascii=False)
    print("Exported %i profiles to %s" % (len(data["profiles"]), OUTPUT))

    # udev rules shipped in the Linux packages (same as `rivalcfg --update-udev`)
    from rivalcfg import udev

    rules_path = os.path.join(os.path.dirname(os.path.abspath(OUTPUT)), "99-steelseries-rival.rules")
    with open(rules_path, "w", encoding="utf-8", newline="\n") as file_:
        file_.write(udev.generate_rules())
    print("Exported udev rules to %s" % rules_path)


if __name__ == "__main__":
    main()
