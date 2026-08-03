package com.analogconnect.client;

final class AudioDeviceConfig {
    final int wireFormat;
    final int sampleRate;
    final int samplesPerFrame;

    private AudioDeviceConfig(int wireFormat, int sampleRate, int samplesPerFrame) {
        this.wireFormat = wireFormat;
        this.sampleRate = sampleRate;
        this.samplesPerFrame = samplesPerFrame;
    }

    static AudioDeviceConfig forWireFormat(int wireFormat) {
        if (wireFormat == AudioPacketCodec.FORMAT_NARROWBAND) {
            return new AudioDeviceConfig(wireFormat, 8_000, 60);
        }
        if (wireFormat == AudioPacketCodec.FORMAT_WIDEBAND) {
            return new AudioDeviceConfig(wireFormat, 16_000, 120);
        }
        throw new IllegalArgumentException("Unsupported HFP audio format");
    }

    int minimumFrameBytes() {
        return samplesPerFrame * 2;
    }

    int preferredBufferBytes() {
        return minimumFrameBytes() * 8;
    }
}
