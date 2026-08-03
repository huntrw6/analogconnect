package com.analogconnect.client;

public final class AudioDeviceConfigTest {
    public static void main(String[] args) {
        validatesNarrowband();
        validatesWideband();
        rejectsUnknownFormat();
        System.out.println("ANDROID_AUDIO_CONFIG_TESTS=PASS tests=3");
    }

    private static void validatesNarrowband() {
        AudioDeviceConfig config = AudioDeviceConfig.forWireFormat(
                AudioPacketCodec.FORMAT_NARROWBAND);
        assertTrue(config.sampleRate == 8_000);
        assertTrue(config.samplesPerFrame == 60);
        assertTrue(config.minimumFrameBytes() == 120);
    }

    private static void validatesWideband() {
        AudioDeviceConfig config = AudioDeviceConfig.forWireFormat(
                AudioPacketCodec.FORMAT_WIDEBAND);
        assertTrue(config.sampleRate == 16_000);
        assertTrue(config.samplesPerFrame == 120);
        assertTrue(config.preferredBufferBytes() == 1_920);
    }

    private static void rejectsUnknownFormat() {
        try {
            AudioDeviceConfig.forWireFormat(99);
            throw new AssertionError("expected format rejection");
        } catch (IllegalArgumentException expected) {
            // Expected.
        }
    }

    private static void assertTrue(boolean condition) {
        if (!condition) {
            throw new AssertionError("audio device configuration assertion failed");
        }
    }
}
