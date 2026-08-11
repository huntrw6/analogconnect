#!/usr/bin/env python3
"""Direct-GATT ANCS diagnostic.

Raw GATT output and ANCS values are consumed only in memory. The program never
requests the notification Message attribute. Plaintext Title/Subtitle output
requires an explicit diagnostic flag.
"""

from __future__ import annotations

import os
import pty
import re
import hashlib
import hmac
import select
import secrets
import signal
import subprocess
import sys
import time
import tomllib
import unicodedata
from pathlib import Path

NOTIFICATION_SOURCE_HANDLE = "0x0028"
NOTIFICATION_SOURCE_CCCD = "0x0029"
CONTROL_POINT_HANDLE = "0x0025"
DATA_SOURCE_HANDLE = "0x002b"
DATA_SOURCE_CCCD = "0x002c"
MESSAGES_APP = b"com.apple.MobileSMS"
ATTRIBUTE_IDS = (0, 1, 2, 4, 5, 6, 7)
ADDRESS_RE = re.compile(r"(?i)^(?:[0-9a-f]{2}:){5}[0-9a-f]{2}$")
VALUE_RE = re.compile(
    rb"(?:Notification|Indication) handle = (0x[0-9a-fA-F]+) value: ((?:[0-9a-fA-F]{2} ?)+)"
)


def find_address(value: object) -> str | None:
    if isinstance(value, str) and ADDRESS_RE.fullmatch(value):
        return value
    if isinstance(value, dict):
        for key, child in value.items():
            if "address" in str(key).lower():
                found = find_address(child)
                if found:
                    return found
        for child in value.values():
            found = find_address(child)
            if found:
                return found
    if isinstance(value, list):
        for child in value:
            found = find_address(child)
            if found:
                return found
    return None


def normalize_subtitle(value: bytes) -> str:
    text = value.decode("utf-8", "replace")
    return " ".join(unicodedata.normalize("NFKC", text).casefold().split())


def subtitle_fingerprint(value: bytes, key: bytes) -> tuple[bool, str, int]:
    normalized = normalize_subtitle(value)
    if not normalized:
        return False, "absent", 0
    digest = hmac.new(key, normalized.encode("utf-8"), hashlib.sha256).hexdigest()
    return True, digest[:12], len(normalized.split())


def display_text(value: bytes) -> str:
    return value.decode("utf-8", "replace").replace("\r", "\\r").replace("\n", "\\n")


def attribute_request(uid: bytes) -> bytes:
    request = bytearray((0,))
    request.extend(uid)
    request.append(0)
    for attribute_id in (1, 2):
        request.append(attribute_id)
        request.extend((256).to_bytes(2, "little"))
    request.extend((4, 5, 6, 7))
    return bytes(request)


def parse_attributes(data: bytes) -> tuple[list[bytes], int] | None:
    if len(data) < 5 or data[0] != 0:
        return None
    offset = 5
    values: list[bytes] = []
    for expected in ATTRIBUTE_IDS:
        if len(data) < offset + 3 or data[offset] != expected:
            return None
        length = int.from_bytes(data[offset + 1 : offset + 3], "little")
        start = offset + 3
        end = start + length
        if len(data) < end:
            return None
        values.append(data[start:end])
        offset = end
    return values, offset


class GattSession:
    def __init__(self, address: str) -> None:
        self.master, slave = pty.openpty()
        self.process = subprocess.Popen(
            ["gatttool", "-b", address, "-t", "public", "-I"],
            stdin=slave,
            stdout=slave,
            stderr=slave,
            close_fds=True,
        )
        os.close(slave)
        self.buffer = b""

    def command(self, command: str) -> None:
        os.write(self.master, command.encode("ascii") + b"\n")

    def read(self, timeout: float) -> bytes:
        ready, _, _ = select.select([self.master], [], [], timeout)
        if not ready:
            return b""
        try:
            return os.read(self.master, 4096)
        except OSError:
            return b""

    def wait_for(self, marker: bytes, timeout: float) -> bool:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline and self.process.poll() is None:
            self.buffer += self.read(min(0.5, deadline - time.monotonic()))
            if marker in self.buffer:
                self.buffer = b""
                return True
        return False

    def close(self) -> None:
        if self.process.poll() is None:
            self.command("disconnect")
            self.command("exit")
            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.process.terminate()
                self.process.wait(timeout=2)
        os.close(self.master)


def subscribe(session: GattSession, handle: str) -> bool:
    session.command(f"char-write-req {handle} 0100")
    return session.wait_for(b"Characteristic value was written successfully", 10)


def main() -> int:
    plaintext = sys.argv[1:] == ["--plaintext-title-subtitle"]
    if sys.argv[1:] and not plaintext:
        print("usage: ancs-gatt-probe.py [--plaintext-title-subtitle]")
        return 2
    config_path = Path.home() / ".config" / "imsg" / "imsg.toml"
    with config_path.open("rb") as config_file:
        address = find_address(tomllib.load(config_file))
    if not address:
        print("ANCS_GATT_PROBE=FAILED reason=device_address_unavailable")
        return 1

    scan = subprocess.Popen(
        ["bluetoothctl", "--timeout", "30", "scan", "le"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    session = GattSession(address)
    try:
        time.sleep(1)
        session.command("connect")
        if not session.wait_for(b"Connection successful", 20):
            print("ANCS_GATT_PROBE=FAILED reason=ble_connection")
            return 1
        if not subscribe(session, DATA_SOURCE_CCCD):
            print("ANCS_GATT_PROBE=FAILED reason=data_source_subscription")
            return 1
        if not subscribe(session, NOTIFICATION_SOURCE_CCCD):
            print("ANCS_GATT_PROBE=FAILED reason=notification_source_subscription")
            return 1

        # ANCS may replay notifications that already exist when subscriptions are
        # established. Drain them before creating the ephemeral key or declaring
        # the controlled test ready.
        warmup_deadline = time.monotonic() + 5
        while time.monotonic() < warmup_deadline:
            session.read(min(0.5, warmup_deadline - time.monotonic()))

        mode = "PLAINTEXT" if plaintext else "HMAC"
        print(f"ANCS_SUBTITLE_PROBE=READY mode={mode} timeout_seconds=1800", flush=True)
        deadline = time.monotonic() + 1800
        response = bytearray()
        request_pending = False
        requested_uid: bytes | None = None
        emitted_uids: set[bytes] = set()
        retained_samples: list[tuple[bytes, bytes]] = []
        test_key = secrets.token_bytes(32)
        messages = 0
        stream = b""
        while time.monotonic() < deadline and messages < 5:
            stream += session.read(min(0.5, deadline - time.monotonic()))
            while b"\n" in stream:
                line, stream = stream.split(b"\n", 1)
                match = VALUE_RE.search(line)
                if not match:
                    continue
                handle = match.group(1).decode("ascii").lower()
                value = bytes.fromhex(match.group(2).decode("ascii"))
                if handle == NOTIFICATION_SOURCE_HANDLE and len(value) == 8:
                    if value[0] <= 1 and not request_pending:
                        if value[4:8] in emitted_uids:
                            continue
                        request = attribute_request(value[4:8]).hex()
                        session.command(f"char-write-req {CONTROL_POINT_HANDLE} {request}")
                        response.clear()
                        request_pending = True
                        requested_uid = value[4:8]
                elif handle == DATA_SOURCE_HANDLE and request_pending:
                    response.extend(value)
                    parsed = parse_attributes(response)
                    if parsed is None:
                        continue
                    values, consumed = parsed
                    del response[:consumed]
                    request_pending = False
                    if values[0] != MESSAGES_APP:
                        continue
                    if requested_uid is not None:
                        emitted_uids.add(requested_uid)
                        retained_samples.append((requested_uid, values[5]))
                    messages += 1
                    if plaintext:
                        print(
                            f"notification={messages} app=messages\n"
                            f"Title: {display_text(values[1])}\n"
                            f"Subtitle: {display_text(values[2])}",
                            flush=True,
                        )
                    else:
                        present, prefix, words = subtitle_fingerprint(values[2], test_key)
                        print(
                            f"notification={messages} app=messages "
                            f"subtitle_present={str(present).lower()} "
                            f"subtitle_hmac_prefix={prefix} word_count={words}",
                            flush=True,
                        )
        if messages == 5:
            print(
                "ANCS_SUBTITLE_PROBE=CAPTURE_COMPLETE "
                "messages_notifications=5 notification_uids_retained_in_memory=5",
                flush=True,
            )
            # Keep the GATT session and sample UIDs alive for the explicitly
            # separate action experiment. Neither UIDs nor labels are printed.
            while time.monotonic() < deadline:
                session.read(min(0.5, deadline - time.monotonic()))
            return 0
        print(f"ANCS_SUBTITLE_PROBE=INCOMPLETE messages_notifications={messages}")
        return 2
    finally:
        session.close()
        if scan.poll() is None:
            scan.send_signal(signal.SIGINT)
            try:
                scan.wait(timeout=2)
            except subprocess.TimeoutExpired:
                scan.terminate()
                scan.wait(timeout=2)


if __name__ == "__main__":
    sys.exit(main())
