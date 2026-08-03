package com.analogconnect.client;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;

final class AudioPacketCodec {
    static final int FORMAT_NARROWBAND = 1;
    static final int FORMAT_WIDEBAND = 2;
    private static final int HEADER_BYTES = 24;
    private static final byte[] MAGIC = new byte[] {'A', 'C', 'A', 'P'};

    private AudioPacketCodec() {}

    static byte[] encode(int format, long sequence, long captureTimeMicros, short[] samples)
            throws PacketException {
        int expectedSamples = expectedSamples(format);
        if (samples == null || samples.length != expectedSamples) {
            throw new PacketException("Invalid audio payload");
        }
        ByteBuffer packet = ByteBuffer.allocate(HEADER_BYTES + expectedSamples * 2);
        packet.order(ByteOrder.BIG_ENDIAN);
        packet.put(MAGIC);
        packet.put((byte) 1);
        packet.put((byte) format);
        packet.putShort((short) 0);
        packet.putLong(sequence);
        packet.putLong(captureTimeMicros);
        packet.order(ByteOrder.LITTLE_ENDIAN);
        for (short sample : samples) {
            packet.putShort(sample);
        }
        return packet.array();
    }

    static Decoded decode(byte[] bytes) throws PacketException {
        if (bytes == null || bytes.length < HEADER_BYTES) {
            throw new PacketException("Invalid audio header");
        }
        ByteBuffer packet = ByteBuffer.wrap(bytes).order(ByteOrder.BIG_ENDIAN);
        for (byte value : MAGIC) {
            if (packet.get() != value) {
                throw new PacketException("Invalid audio header");
            }
        }
        if (packet.get() != 1) {
            throw new PacketException("Unsupported audio version");
        }
        int format = packet.get() & 0xff;
        if (packet.getShort() != 0) {
            throw new PacketException("Invalid audio header");
        }
        int expectedSamples = expectedSamples(format);
        if (packet.remaining() != 16 + expectedSamples * 2) {
            throw new PacketException("Invalid audio payload");
        }
        long sequence = packet.getLong();
        long captureTimeMicros = packet.getLong();
        packet.order(ByteOrder.LITTLE_ENDIAN);
        short[] samples = new short[expectedSamples];
        for (int index = 0; index < samples.length; index++) {
            samples[index] = packet.getShort();
        }
        return new Decoded(format, sequence, captureTimeMicros, samples);
    }

    private static int expectedSamples(int format) throws PacketException {
        if (format == FORMAT_NARROWBAND) {
            return 60;
        }
        if (format == FORMAT_WIDEBAND) {
            return 120;
        }
        throw new PacketException("Unsupported audio format");
    }

    static final class Decoded {
        final int format;
        final long sequence;
        final long captureTimeMicros;
        final short[] samples;

        Decoded(int format, long sequence, long captureTimeMicros, short[] samples) {
            this.format = format;
            this.sequence = sequence;
            this.captureTimeMicros = captureTimeMicros;
            this.samples = samples;
        }

        @Override
        public String toString() {
            return "DecodedAudioPacket{format=" + format + ", sequence=" + sequence
                    + ", captureTimeMicros=" + captureTimeMicros
                    + ", sampleCount=" + samples.length + "}";
        }
    }

    static final class PacketException extends Exception {
        PacketException(String message) {
            super(message);
        }
    }
}
