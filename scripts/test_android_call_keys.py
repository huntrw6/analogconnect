#!/usr/bin/env python3
import importlib.util
import pathlib
import unittest

MODULE = pathlib.Path(__file__).with_name("android-call-keys.py")
SPEC = importlib.util.spec_from_file_location("android_call_keys", MODULE)
call_keys = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(call_keys)


class CallKeyParserTest(unittest.TestCase):
    def test_short_power_becomes_end(self):
        data = "[ 10.000] EV_KEY KEY_POWER DOWN\n[ 10.120] EV_KEY KEY_POWER UP\n"
        self.assertEqual([26], call_keys.parse_events(data, 0.7))

    def test_held_power_is_left_to_android(self):
        data = "[ 10.000] EV_KEY KEY_POWER DOWN\n[ 11.000] EV_KEY KEY_POWER UP\n"
        self.assertEqual([], call_keys.parse_events(data, 0.7))

    def test_only_reserved_call_key_is_forwarded(self):
        data = "[ 1.0] EV_KEY KEY_SEND DOWN\n[ 1.1] EV_KEY KEY_SEND UP\n" \
               "[ 2.0] EV_KEY KEY_5 DOWN\n[ 2.1] EV_KEY KEY_5 UP\n"
        self.assertEqual([5], call_keys.parse_events(data, 0.7))


if __name__ == "__main__":
    unittest.main()
