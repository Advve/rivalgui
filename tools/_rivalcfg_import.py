"""Imports rivalcfg from a reference source tree without needing hidapi.

Set RIVALCFG_SRC to point at a rivalcfg checkout. Otherwise the newest
``rivalcfg-X.Y.Z`` folder found next to this project or in ~/Downloads is used.
"""

import glob
import os
import re
import sys
import types

_HERE = os.path.dirname(os.path.abspath(__file__))


def _version_key(path):
    m = re.search(r"rivalcfg-(\d+)\.(\d+)\.(\d+)$", path.rstrip("/\\"))
    return tuple(int(p) for p in m.groups()) if m else (0, 0, 0)


def _find_source():
    if os.environ.get("RIVALCFG_SRC"):
        return os.environ["RIVALCFG_SRC"]
    candidates = []
    for base in (os.path.join(_HERE, "..", ".."), os.path.join(os.path.expanduser("~"), "Downloads")):
        for path in glob.glob(os.path.join(base, "rivalcfg-*")):
            if os.path.isfile(os.path.join(path, "rivalcfg", "__init__.py")):
                candidates.append(os.path.abspath(path))
    if not candidates:
        raise SystemExit("rivalcfg source not found: set RIVALCFG_SRC")
    return max(candidates, key=_version_key)


RIVALCFG_SRC = _find_source()

# Only FakeDevice is used by the tools, so a stub is enough.
if "hid" not in sys.modules:
    hid_stub = types.ModuleType("hid")
    hid_stub.enumerate = lambda *args, **kwargs: []
    sys.modules["hid"] = hid_stub

sys.path.insert(0, RIVALCFG_SRC)
os.environ["RIVALCFG_DEBUG_NO_COMMAND_DELAY"] = "1"

import rivalcfg  # noqa: E402,F401
from rivalcfg import devices, mouse, mouse_settings, usbhid  # noqa: E402,F401
from rivalcfg.handlers.buttons import buttons as buttons_handler  # noqa: E402,F401
from rivalcfg.handlers.buttons import layout_multimedia, layout_qwerty  # noqa: E402,F401
from rivalcfg.version import VERSION  # noqa: E402,F401

print("Using rivalcfg %s from %s" % (VERSION, RIVALCFG_SRC))
