#!/usr/bin/env python3
"""Forward target-phone physical call keys over its existing trusted ADB link."""

import argparse
import re
import subprocess
import time

ACTION = "com.analogconnect.client.PHYSICAL_CALL_KEY"
COMPONENT = "com.analogconnect.client/.PhysicalCallKeyReceiver"
KEYCODES = {
    "KEY_SEND": 5,
}


def broadcast(adb: str, keycode: int) -> None:
    subprocess.run(
        [adb, "shell", "am", "broadcast", "-n", COMPONENT, "-a", ACTION,
         "--ei", "key_code", str(keycode)], stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL, check=False,
    )


def parse_events(output: str, power_hold_seconds: float) -> list[int]:
    keycodes = []
    power_times = []
    for line in output.splitlines():
        if "EV_KEY" not in line:
            continue
        if "KEY_POWER" in line:
            match = re.search(r"\[\s*([0-9.]+)\]", line)
            if match:
                power_times.append(float(match.group(1)))
            continue
        if "DOWN" not in line:
            continue
        for name, keycode in KEYCODES.items():
            if name in line:
                keycodes.append(keycode)
                break
    if len(power_times) >= 2 and power_times[-1] - power_times[0] < power_hold_seconds:
        keycodes.append(26)
    return keycodes


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--adb", default="adb")
    parser.add_argument("--device", default="/dev/input/event1")
    parser.add_argument("--power-hold-seconds", type=float, default=0.7)
    args = parser.parse_args()
    try:
        while True:
            result = subprocess.run(
                [args.adb, "shell", "getevent", "-lt", "-c", "4", args.device],
                capture_output=True, text=True, check=False,
            )
            if result.returncode != 0 or not result.stdout:
                time.sleep(2.0)
                continue
            for keycode in parse_events(result.stdout, args.power_hold_seconds):
                broadcast(args.adb, keycode)
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
