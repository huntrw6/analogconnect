package com.analogconnect.client;

import java.util.Arrays;

public final class AudioPacketCodecTest {
    public static void main(String[] args) throws Exception {
        roundTrips(AudioPacketCodec.FORMAT_NARROWBAND, 60);
        roundTrips(AudioPacketCodec.FORMAT_WIDEBAND, 120);
        rejectsMalformedPackets();
        matchesGoldenHeader();
        rejectsNegativeSequence();
        System.out.println("ANDROID_AUDIO_TESTS=PASS tests=5");
    }

    private static void roundTrips(int format, int count) throws Exception {
        short[] samples = new short[count];
        for (int index = 0; index < count; index++) {
            samples[index] = (short) (index - count / 2);
        }
        byte[] encoded = AudioPacketCodec.encode(format, 42, 99_000, samples);
        AudioPacketCodec.Decoded decoded = AudioPacketCodec.decode(encoded);
        assertTrue(decoded.format == format);
        assertTrue(decoded.sequence == 42);
        assertTrue(decoded.captureTimeMicros == 99_000);
        assertTrue(Arrays.equals(decoded.samples, samples));
        assertTrue(!decoded.toString().contains("-60"));
    }

    private static void rejectsMalformedPackets() throws Exception {
        assertRejected(new byte[3]);
        short[] samples = new short[60];
        byte[] packet = AudioPacketCodec.encode(
                AudioPacketCodec.FORMAT_NARROWBAND, 1, 2, samples);
        packet[0] = 'X';
        assertRejected(packet);
        packet = AudioPacketCodec.encode(AudioPacketCodec.FORMAT_NARROWBAND, 1, 2, samples);
        assertRejected(Arrays.copyOf(packet, packet.length - 1));
    }

    private static void matchesGoldenHeader() throws Exception {
        byte[] encoded = AudioPacketCodec.encode(AudioPacketCodec.FORMAT_NARROWBAND,
                0x0102030405060708L, 0x1112131415161718L, new short[60]);
        byte[] expected = new byte[] {
                0x41, 0x43, 0x41, 0x50, 0x01, 0x01, 0x00, 0x00,
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18
        };
        assertTrue(Arrays.equals(Arrays.copyOf(encoded, 24), expected));
    }

    private static void rejectsNegativeSequence() throws Exception {
        try {
            AudioPacketCodec.encode(
                    AudioPacketCodec.FORMAT_NARROWBAND, -1, 0, new short[60]);
            throw new AssertionError("expected sequence rejection");
        } catch (AudioPacketCodec.PacketException expected) {
            // Expected.
        }
    }

    private static void assertRejected(byte[] packet) throws Exception {
        try {
            AudioPacketCodec.decode(packet);
            throw new AssertionError("expected packet rejection");
        } catch (AudioPacketCodec.PacketException expected) {
            // Expected. Packet bytes are deliberately not included in output.
        }
    }

    private static void assertTrue(boolean condition) {
        if (!condition) {
            throw new AssertionError("audio packet assertion failed");
        }
    }
}
