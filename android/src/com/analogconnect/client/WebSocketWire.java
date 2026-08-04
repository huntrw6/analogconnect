package com.analogconnect.client;

import java.io.ByteArrayOutputStream;
import java.io.EOFException;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.Base64;
import java.util.Locale;

final class WebSocketWire {
    static final int MAX_PAYLOAD_BYTES = 264 * AudioPacketCodec.MAX_BATCH_FRAMES;
    static final int OPCODE_BINARY = 2;
    static final int OPCODE_CLOSE = 8;
    static final int OPCODE_PING = 9;
    static final int OPCODE_PONG = 10;
    private static final int MAX_HEADER_BYTES = 8192;
    private static final String ACCEPT_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    interface MaskSource {
        void fill(byte[] mask);
    }

    private WebSocketWire() {}

    static String newHandshakeKey(SecureRandom random) {
        byte[] nonce = new byte[16];
        random.nextBytes(nonce);
        return Base64.getEncoder().encodeToString(nonce);
    }

    static String expectedAccept(String key) throws IOException {
        try {
            MessageDigest sha1 = MessageDigest.getInstance("SHA-1");
            return Base64.getEncoder().encodeToString(
                    sha1.digest((key + ACCEPT_GUID).getBytes(StandardCharsets.US_ASCII)));
        } catch (GeneralSecurityException error) {
            throw new IOException("WebSocket handshake validation is unavailable");
        }
    }

    static void validateHandshake(InputStream input, String key) throws IOException {
        int consumed = 0;
        String status = readLine(input, MAX_HEADER_BYTES);
        consumed += status.length() + 2;
        if (!"HTTP/1.1 101 Switching Protocols".equals(status)) {
            throw new IOException("WebSocket upgrade was rejected");
        }
        boolean upgrade = false;
        boolean connection = false;
        boolean accept = false;
        while (true) {
            String line = readLine(input, MAX_HEADER_BYTES - consumed);
            consumed += line.length() + 2;
            if (line.isEmpty()) {
                break;
            }
            int separator = line.indexOf(':');
            if (separator <= 0) {
                throw new IOException("WebSocket handshake is invalid");
            }
            String name = line.substring(0, separator).trim().toLowerCase(Locale.US);
            String value = line.substring(separator + 1).trim();
            if ("upgrade".equals(name)) {
                upgrade = "websocket".equalsIgnoreCase(value);
            } else if ("connection".equals(name)) {
                connection = containsToken(value, "upgrade");
            } else if ("sec-websocket-accept".equals(name)) {
                accept = MessageDigest.isEqual(
                        expectedAccept(key).getBytes(StandardCharsets.US_ASCII),
                        value.getBytes(StandardCharsets.US_ASCII));
            } else if ("sec-websocket-extensions".equals(name)) {
                throw new IOException("WebSocket extensions are not supported");
            }
        }
        if (!upgrade || !connection || !accept) {
            throw new IOException("WebSocket handshake is invalid");
        }
    }

    static void writeClientFrame(OutputStream output, int opcode, byte[] payload,
            MaskSource masks) throws IOException {
        validateOpcodeAndPayload(opcode, payload);
        output.write(0x80 | opcode);
        int length = payload.length;
        if (length <= 125) {
            output.write(0x80 | length);
        } else {
            output.write(0x80 | 126);
            output.write((length >>> 8) & 0xff);
            output.write(length & 0xff);
        }
        byte[] mask = new byte[4];
        masks.fill(mask);
        output.write(mask);
        byte[] encoded = new byte[length];
        for (int index = 0; index < length; index++) {
            encoded[index] = (byte) (payload[index] ^ mask[index % mask.length]);
        }
        output.write(encoded);
        output.flush();
    }

    static Frame readServerFrame(InputStream input) throws IOException {
        int first = readByte(input);
        int second = readByte(input);
        if ((first & 0x80) == 0 || (first & 0x70) != 0 || (second & 0x80) != 0) {
            throw new IOException("WebSocket frame is invalid");
        }
        int opcode = first & 0x0f;
        int length = second & 0x7f;
        if (length == 126) {
            length = (readByte(input) << 8) | readByte(input);
            if (length <= 125) {
                throw new IOException("WebSocket frame length is invalid");
            }
        } else if (length == 127) {
            throw new IOException("WebSocket frame is too large");
        }
        if (length > MAX_PAYLOAD_BYTES || (opcode >= OPCODE_CLOSE && length > 125)) {
            throw new IOException("WebSocket frame is too large");
        }
        byte[] payload = readExact(input, length);
        validateOpcodeAndPayload(opcode, payload);
        return new Frame(opcode, payload);
    }

    private static void validateOpcodeAndPayload(int opcode, byte[] payload) throws IOException {
        if (payload == null || payload.length > MAX_PAYLOAD_BYTES
                || !(opcode == OPCODE_BINARY || opcode == OPCODE_CLOSE
                || opcode == OPCODE_PING || opcode == OPCODE_PONG)
                || (opcode >= OPCODE_CLOSE && payload.length > 125)
                || (opcode == OPCODE_CLOSE && payload.length == 1)) {
            throw new IOException("WebSocket frame is invalid");
        }
    }

    private static boolean containsToken(String value, String expected) {
        for (String token : value.split(",")) {
            if (expected.equalsIgnoreCase(token.trim())) {
                return true;
            }
        }
        return false;
    }

    private static String readLine(InputStream input, int remaining) throws IOException {
        if (remaining <= 1) {
            throw new IOException("WebSocket handshake is too large");
        }
        ByteArrayOutputStream line = new ByteArrayOutputStream();
        while (line.size() + 2 <= remaining) {
            int value = readByte(input);
            if (value == '\r') {
                if (readByte(input) != '\n') {
                    throw new IOException("WebSocket handshake is invalid");
                }
                return new String(line.toByteArray(), StandardCharsets.US_ASCII);
            }
            if (value < 0x20 || value > 0x7e) {
                throw new IOException("WebSocket handshake is invalid");
            }
            line.write(value);
        }
        throw new IOException("WebSocket handshake is too large");
    }

    private static int readByte(InputStream input) throws IOException {
        int value = input.read();
        if (value < 0) {
            throw new EOFException("WebSocket connection ended");
        }
        return value;
    }

    private static byte[] readExact(InputStream input, int length) throws IOException {
        byte[] bytes = new byte[length];
        int offset = 0;
        while (offset < length) {
            int count = input.read(bytes, offset, length - offset);
            if (count < 0) {
                throw new EOFException("WebSocket connection ended");
            }
            if (count > 0) {
                offset += count;
            }
        }
        return bytes;
    }

    static final class Frame {
        final int opcode;
        final byte[] payload;

        Frame(int opcode, byte[] payload) {
            this.opcode = opcode;
            this.payload = payload;
        }
    }
}
