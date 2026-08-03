package com.analogconnect.client;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

public final class WebSocketWireTest {
    private static int tests;

    public static void main(String[] args) throws Exception {
        acceptsStrictHandshake();
        rejectsBadOrExtendedHandshake();
        writesMaskedClientFrames();
        readsBoundedServerFrames();
        rejectsMaskedFragmentedAndUnexpectedFrames();
        System.out.println("ANDROID_WEBSOCKET_TESTS=PASS tests=" + tests);
    }

    private static void acceptsStrictHandshake() throws Exception {
        String key = "dGhlIHNhbXBsZSBub25jZQ==";
        String response = "HTTP/1.1 101 Switching Protocols\r\n"
                + "Upgrade: websocket\r\nConnection: keep-alive, Upgrade\r\n"
                + "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n";
        WebSocketWire.validateHandshake(stream(response), key);
        tests++;
    }

    private static void rejectsBadOrExtendedHandshake() throws Exception {
        String key = "dGhlIHNhbXBsZSBub25jZQ==";
        expectFailure("HTTP/1.1 401 Unauthorized\r\n\r\n", key);
        expectFailure("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n"
                + "Connection: Upgrade\r\nSec-WebSocket-Accept: wrong\r\n\r\n", key);
        expectFailure("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n"
                + "Connection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n"
                + "Sec-WebSocket-Extensions: permessage-deflate\r\n\r\n", key);
        tests++;
    }

    private static void writesMaskedClientFrames() throws Exception {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        WebSocketWire.writeClientFrame(output, WebSocketWire.OPCODE_BINARY,
                new byte[] {1, 2, 3}, new WebSocketWire.MaskSource() {
                    @Override public void fill(byte[] mask) {
                        Arrays.fill(mask, (byte) 0x55);
                    }
                });
        assertArrayEquals(new byte[] {(byte) 0x82, (byte) 0x83, 0x55, 0x55, 0x55, 0x55,
                0x54, 0x57, 0x56}, output.toByteArray());
        tests++;
    }

    private static void readsBoundedServerFrames() throws Exception {
        byte[] payload = AudioPacketCodec.encode(AudioPacketCodec.FORMAT_WIDEBAND,
                0, 0, new short[120]);
        byte[] wire = new byte[payload.length + 4];
        wire[0] = (byte) 0x82;
        wire[1] = 126;
        wire[2] = (byte) (payload.length >>> 8);
        wire[3] = (byte) payload.length;
        System.arraycopy(payload, 0, wire, 4, payload.length);
        WebSocketWire.Frame frame = WebSocketWire.readServerFrame(new ByteArrayInputStream(wire));
        assertEquals(WebSocketWire.OPCODE_BINARY, frame.opcode);
        assertArrayEquals(payload, frame.payload);
        tests++;
    }

    private static void rejectsMaskedFragmentedAndUnexpectedFrames() throws Exception {
        expectFrameFailure(new byte[] {(byte) 0x82, (byte) 0x80, 0, 0, 0, 0});
        expectFrameFailure(new byte[] {0x02, 0});
        expectFrameFailure(new byte[] {(byte) 0x81, 0});
        expectFrameFailure(new byte[] {(byte) 0x82, 127});
        tests++;
    }

    private static void expectFailure(String response, String key) throws Exception {
        try {
            WebSocketWire.validateHandshake(stream(response), key);
            throw new AssertionError("expected handshake failure");
        } catch (IOException expected) {
            assertFalse(expected.getMessage().contains(key));
        }
    }

    private static void expectFrameFailure(byte[] frame) throws Exception {
        try {
            WebSocketWire.readServerFrame(new ByteArrayInputStream(frame));
            throw new AssertionError("expected frame failure");
        } catch (IOException expected) {
            // Expected fixed diagnostic.
        }
    }

    private static ByteArrayInputStream stream(String value) {
        return new ByteArrayInputStream(value.getBytes(StandardCharsets.US_ASCII));
    }

    private static void assertArrayEquals(byte[] expected, byte[] actual) {
        if (!Arrays.equals(expected, actual)) {
            throw new AssertionError("byte arrays differ");
        }
    }

    private static void assertEquals(int expected, int actual) {
        if (expected != actual) {
            throw new AssertionError("expected " + expected + " but got " + actual);
        }
    }

    private static void assertFalse(boolean value) {
        if (value) {
            throw new AssertionError("expected false");
        }
    }
}
