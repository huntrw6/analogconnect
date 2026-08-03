package com.analogconnect.client;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.SecureRandom;

import javax.net.ssl.HostnameVerifier;
import javax.net.ssl.HttpsURLConnection;
import javax.net.ssl.SSLSession;
import javax.net.ssl.SSLSocket;
import javax.net.ssl.SSLSocketFactory;

final class MediaWebSocket implements CallAudioPump.Transport {
    private static final int TIMEOUT_MS = 5000;
    private final SSLSocket socket;
    private final InputStream input;
    private final OutputStream output;
    private final SecureRandom random;
    private volatile boolean closed;

    private MediaWebSocket(SSLSocket socket, InputStream input, OutputStream output,
            SecureRandom random) {
        this.socket = socket;
        this.input = input;
        this.output = output;
        this.random = random;
    }

    static MediaWebSocket connect(String endpoint, String pin, String tlsName,
            MediaSessionCredentials credentials) throws IOException, GeneralSecurityException {
        if (credentials == null) {
            throw new IOException("Media session credentials are missing");
        }
        URL url = Endpoint.parse(endpoint, "/api/v1/audio/stream");
        if (!"https".equals(url.getProtocol())) {
            throw new IOException("Call audio requires HTTPS");
        }
        CertificatePin certificatePin = CertificatePin.parse(pin);
        String verificationName = tlsName == null || tlsName.trim().isEmpty()
                ? url.getHost() : tlsName.trim();
        int port = url.getPort() >= 0 ? url.getPort() : 443;
        Socket transport = new Socket();
        SSLSocket tls = null;
        try {
            transport.connect(new InetSocketAddress(url.getHost(), port), TIMEOUT_MS);
            transport.setSoTimeout(TIMEOUT_MS);
            SSLSocketFactory factory = certificatePin.socketFactory();
            tls = (SSLSocket) factory.createSocket(transport, verificationName, port, true);
            tls.setEnabledProtocols(new String[] {"TLSv1.2"});
            tls.startHandshake();
            SSLSession session = tls.getSession();
            HostnameVerifier verifier = HttpsURLConnection.getDefaultHostnameVerifier();
            if (!verifier.verify(verificationName, session)) {
                throw new IOException("TLS server name did not match");
            }
            SecureRandom random = new SecureRandom();
            String key = WebSocketWire.newHandshakeKey(random);
            InputStream input = tls.getInputStream();
            OutputStream output = tls.getOutputStream();
            output.write(handshakeRequest(url, credentials, key).getBytes(StandardCharsets.US_ASCII));
            output.flush();
            WebSocketWire.validateHandshake(input, key);
            tls.setSoTimeout(0);
            return new MediaWebSocket(tls, input, output, random);
        } catch (IOException | GeneralSecurityException error) {
            if (tls != null) {
                try {
                    tls.close();
                } catch (IOException ignored) {
                    // Preserve the original fixed diagnostic.
                }
            } else {
                try {
                    transport.close();
                } catch (IOException ignored) {
                    // Preserve the original fixed diagnostic.
                }
            }
            throw error;
        }
    }

    public synchronized void sendBinary(byte[] packet) throws IOException {
        ensureOpen();
        WebSocketWire.writeClientFrame(output, WebSocketWire.OPCODE_BINARY, packet,
                new WebSocketWire.MaskSource() {
                    @Override public void fill(byte[] mask) {
                        random.nextBytes(mask);
                    }
                });
    }

    WebSocketWire.Frame receive() throws IOException {
        ensureOpen();
        WebSocketWire.Frame frame = WebSocketWire.readServerFrame(input);
        if (frame.opcode == WebSocketWire.OPCODE_PING) {
            synchronized (this) {
                WebSocketWire.writeClientFrame(output, WebSocketWire.OPCODE_PONG, frame.payload,
                        new WebSocketWire.MaskSource() {
                            @Override public void fill(byte[] mask) {
                                random.nextBytes(mask);
                            }
                        });
            }
        } else if (frame.opcode == WebSocketWire.OPCODE_CLOSE) {
            close();
        }
        return frame;
    }

    @Override
    public byte[] receiveBinary() throws IOException {
        while (true) {
            WebSocketWire.Frame frame = receive();
            if (frame.opcode == WebSocketWire.OPCODE_BINARY) {
                return frame.payload;
            }
            if (frame.opcode == WebSocketWire.OPCODE_CLOSE) {
                throw new IOException("Media connection ended");
            }
        }
    }

    @Override
    public synchronized void close() {
        if (closed) {
            return;
        }
        closed = true;
        try {
            WebSocketWire.writeClientFrame(output, WebSocketWire.OPCODE_CLOSE, new byte[0],
                    new WebSocketWire.MaskSource() {
                        @Override public void fill(byte[] mask) {
                            random.nextBytes(mask);
                        }
                    });
        } catch (IOException ignored) {
            // Closing is best effort and diagnostics must not expose transport data.
        }
        try {
            socket.close();
        } catch (IOException ignored) {
            // Closing is idempotent.
        }
    }

    private void ensureOpen() throws IOException {
        if (closed) {
            throw new IOException("Media connection is closed");
        }
    }

    private static String handshakeRequest(URL url, MediaSessionCredentials credentials,
            String key) {
        int port = url.getPort() >= 0 ? url.getPort() : 443;
        String host = url.getHost() + (port == 443 ? "" : ":" + port);
        String path = url.getFile().isEmpty() ? "/" : url.getFile();
        return "GET " + path + " HTTP/1.1\r\n"
                + "Host: " + host + "\r\n"
                + "Upgrade: websocket\r\n"
                + "Connection: Upgrade\r\n"
                + "Sec-WebSocket-Version: 13\r\n"
                + "Sec-WebSocket-Key: " + key + "\r\n"
                + "X-AnalogConnect-Session: " + credentials.sessionId() + "\r\n"
                + "Authorization: Bearer " + credentials.token() + "\r\n"
                + "\r\n";
    }

    @Override
    public String toString() {
        return "MediaWebSocket{credentials=[REDACTED]}";
    }
}
