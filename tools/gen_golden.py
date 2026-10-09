"""Generates golden HID packets from the rivalcfg reference implementation.

For every device profile and every setting, a set of representative values is
sent to a rivalcfg ``Mouse`` backed by a ``FakeDevice``; the exact bytes it
writes are recorded. The Rust test suite (src-tauri/tests/golden.rs) replays
every case and compares byte for byte.

Usage::

    python tools/gen_golden.py [output.json]
"""

import json
import os
import sys

import _rivalcfg_import as r

OUTPUT = (
    sys.argv[1]
    if len(sys.argv) > 1
    else os.path.join(
        os.path.dirname(__file__), "..", "src-tauri", "tests", "golden.json"
    )
)

COLORS = ["#ABCDEF", "red", "f00", "Lime", [1, 2, 3], "#12345", "nope"]

GRADIENTS = [
    "red",
    "#00FF7F",
    [10, 20, 30],
    "rgbgradient(duration=5000; colors=0%: red, 50%: #00ff00)",
    "rgbgradient(colors=0: blue, 20: red, 100: lime)",
    "rgbgradient(duration=1000; colors=0%: #ff0000, 33%: #00ff00, 66%: #0000ff)",
    "rgbgradient(duration=2000; colors=0%: #ff0000, 10%: #fff, 20%: 0f0, 30%: navy, 40%: teal, 50%: purple, 60%: olive, 70%: silver)",
    {"duration": 3000, "colors": [{"pos": 0, "color": "red"}, {"pos": 40, "color": [0, 0, 255]}]},
    "rgbgradient(duration=100; colors=0%: red, 50%: blue)",
    "rgbgradient(duration=31000; colors=0%: red, 50%: blue)",
    "rgbgradient(colors=0%: red, 50%: blue, 30%: lime)",
    "rgbgradient(foo=bar)",
    "notacolor",
]


def range_values(info):
    start, stop, step = info["input_range"]
    mid = start + ((stop - start) // step // 2) * step
    return [start, stop, mid, start + step, mid + step // 2 + 1, start - 1, stop + 1]


def multidpi_values(info):
    start, stop, step = info["input_range"]
    count = info["max_preset_count"]
    many = [start + i * step * 3 for i in range(count)]
    values = [start, stop, ", ".join(str(v) for v in many), many[:2], str(start + step)]
    values.append([start] * (count + 1))  # too many presets -> error
    return values


def range_choice_values(info):
    start, stop, step = info["input_range"]
    mid = start + ((stop - start) // step // 2) * step
    return [start, stop, mid, mid + step // 2, mid + step // 2 + 1, start - 1, stop + 1, "1250", "abc"]


def multidpi_xy_values(info):
    start, stop, step = info["input_range"]
    count = info["max_preset_count"]
    a, b = start + step, start + 7 * step
    return [
        start,
        stop,
        "%i:%i, %i:%i" % (a, b, b, a),
        "%i, %i:%i" % (a, b, a),
        [[a, b], [b, a]],
        [a, [b, a]],
        [[a]],
        ", ".join("%i:%i" % (start + i * step, stop - i * step) for i in range(count)),
        [start] * (count + 1),  # too many presets -> error
        "%i:%i:%i" % (a, a, a),  # invalid -> error
    ]


def choice_values(info):
    return [c for c in info["choices"].keys()] + ["__invalid__"]


def buttons_values(info):
    names = list(info["buttons"].keys())
    values = ["default", info["default"]]
    if len(names) >= 2:
        values.append(
            "buttons(%s=%s; %s=%s)" % (names[0], names[1], names[1], names[0])
        )
    last = names[-1]
    values.append("buttons(%s=disabled)" % last)
    if info["button_dpi_switch"] is not None:
        values.append("buttons(%s=dpi)" % names[0])
    if info["button_keyboard"] is not None:
        for key in ["A", "z", "F5", "Enter", "semicolon", "Space", "LeftCtrl"]:
            values.append("buttons(layout=qwerty; %s=%s)" % (last, key))
    if info["button_multimedia"] is not None:
        for key in ["PlayPause", "VolumeUp", "Mute"]:
            values.append("buttons(%s=%s)" % (last, key))
    if info["button_scroll_up"] is not None:
        values.append("buttons(%s=ScrollUp; %s=ScrollDown)" % (names[0], last))
    values.append({"buttons": {last.lower(): "button1"}})
    values.append("buttons(nosuchbutton=button1)")
    values.append("buttons(%s=nosuchkey)" % last)
    return values


def values_for(info):
    vt = info.get("value_type")
    vals = []
    if "default" in info and info["default"] is not None:
        vals.append(info["default"])
    if vt == "choice":
        vals += choice_values(info)
    elif vt == "range":
        vals += range_values(info)
    elif vt == "range_choice":
        vals += range_choice_values(info)
    elif vt in ("multidpi_range", "multidpi_range_choice"):
        vals += multidpi_values(info)
    elif vt == "multidpi_range_choice_xy":
        vals += multidpi_xy_values(info)
    elif vt == "rgbcolor":
        vals += COLORS
    elif vt == "reactive_rgbcolor":
        vals += ["off", "disable", None] + COLORS
    elif vt in ("rgbgradient", "rgbgradientv2"):
        vals += GRADIENTS
    elif vt == "buttons":
        vals += buttons_values(info)
    return vals


def new_mouse(profile):
    settings = r.mouse_settings.FakeMouseSettings(
        profile["vendor_id"], profile["product_id"], profile
    )
    return r.mouse.Mouse(r.usbhid.FakeDevice(), profile, settings)


def run(profile, fn):
    m = new_mouse(profile)
    try:
        fn(m)
    except Exception as e:  # noqa: BLE001
        return {"error": "%s: %s" % (type(e).__name__, e)}
    m._hid_device.bytes.seek(0)
    return {"bytes": m._hid_device.bytes.read().hex()}


def main():
    cases = []
    for (vid, pid), profile in r.devices.PROFILES.items():
        for name, info in profile["settings"].items():
            if info.get("value_type") == "none":
                cases.append(
                    {"vid": vid, "pid": pid, "setting": name, "value": None,
                     **run(profile, lambda m, n=name: getattr(m, "set_" + n)())}
                )
                continue
            seen = []
            for value in values_for(info):
                if value in seen:
                    continue
                seen.append(value)
                result = run(profile, lambda m, n=name, v=value: getattr(m, "set_" + n)(v))
                cases.append({"vid": vid, "pid": pid, "setting": name, "value": value, **result})
            if info.get("value_type", "").startswith("multidpi_range"):
                start, _, step = info["input_range"]
                value = [start, start + step, start + 2 * step][: info["max_preset_count"]]
                for preset in range(len(value) + 1):
                    result = run(
                        profile,
                        lambda m, n=name, v=value, p=preset: getattr(m, "set_" + n)(v, p),
                    )
                    cases.append({"vid": vid, "pid": pid, "setting": name, "value": value,
                                  "selected_preset": preset, **result})
        if "save_command" in profile and profile["save_command"]:
            cases.append({"vid": vid, "pid": pid, "setting": "__save__", "value": None,
                          **run(profile, lambda m: m.save())})

    os.makedirs(os.path.dirname(os.path.abspath(OUTPUT)), exist_ok=True)
    with open(OUTPUT, "w", encoding="utf-8") as file_:
        json.dump({"rivalcfg_version": r.VERSION, "cases": cases}, file_, indent=0)
    errors = sum(1 for c in cases if "error" in c)
    print("Generated %i cases (%i expected errors) to %s" % (len(cases), errors, OUTPUT))


if __name__ == "__main__":
    main()
