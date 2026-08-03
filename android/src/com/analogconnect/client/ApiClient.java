package com.analogconnect.client;

import android.os.SystemClock;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;

import javax.net.ssl.HttpsURLConnection;
import javax.net.ssl.HostnameVerifier;

import org.json.JSONException;
import org.json.JSONObject;

final class ApiClient {
    private static final int TIMEOUT_MS = 5000;
    private final CertificatePin certificatePin;
    private final String tlsName;

    ApiClient() {
        certificatePin = null;
        tlsName = "";
    }

    ApiClient(String pin) throws GeneralSecurityException {
        this(pin, "");
    }

    ApiClient(String pin, String tlsName) throws GeneralSecurityException {
        certificatePin = pin == null || pin.trim().isEmpty() ? null : CertificatePin.parse(pin);
        this.tlsName = tlsName == null ? "" : tlsName.trim();
    }

    int health(String endpoint) throws IOException {
        return request(Endpoint.parse(endpoint, "/api/v1/health"), "");
    }

    int status(String endpoint, String token) throws IOException {
        if (token == null || token.isEmpty()) {
            throw new IOException("Enrollment token is missing");
        }
        return request(Endpoint.parse(endpoint, "/api/v1/status"), token);
    }

    int sendMessage(String endpoint, String token, String recipient, String body)
            throws IOException {
        if (token == null || token.isEmpty()) {
            throw new IOException("Enrollment token is missing");
        }
        try {
            JSONObject request = new JSONObject();
            request.put("recipient", recipient);
            request.put("body", body);
            return post(endpoint, "/api/v1/messages", token, request);
        } catch (JSONException error) {
            throw new IOException("Could not encode message request");
        }
    }

    int executeCallCommand(String endpoint, String token, String action, String value)
            throws IOException {
        if (token == null || token.isEmpty()) {
            throw new IOException("Enrollment token is missing");
        }
        try {
            JSONObject request = new JSONObject();
            request.put("action", action);
            if ("dial".equals(action)) {
                request.put("target", value);
            } else if ("send_dtmf".equals(action)) {
                request.put("tone", value);
            }
            return post(endpoint, "/api/v1/calls/commands", token, request);
        } catch (JSONException error) {
            throw new IOException("Could not encode call command");
        }
    }

    MediaSessionCredentials createMediaSession(String endpoint, String token) throws IOException {
        if (token == null || token.isEmpty()) {
            throw new IOException("Enrollment token is missing");
        }
        URL url = Endpoint.parse(endpoint, "/api/v1/audio/sessions");
        HttpURLConnection connection = open(url);
        connection.setConnectTimeout(TIMEOUT_MS);
        connection.setReadTimeout(TIMEOUT_MS);
        connection.setRequestMethod("POST");
        connection.setRequestProperty("Accept", "application/json");
        connection.setRequestProperty("Authorization", "Bearer " + token);
        connection.setFixedLengthStreamingMode(0);
        connection.setDoOutput(true);
        try {
            connection.getOutputStream().close();
            int status = connection.getResponseCode();
            if (status != HttpURLConnection.HTTP_CREATED) {
                throw new IOException("Daemon returned HTTP " + status);
            }
            byte[] response = readBounded(connection.getInputStream(), 1024);
            JSONObject json = new JSONObject(new String(response, StandardCharsets.UTF_8));
            if (json.length() != 4 || !json.has("session_id") || !json.has("token")
                    || !json.has("lifetime_seconds") || !json.has("audio_format")) {
                throw new IOException("Media session response is invalid");
            }
            Object lifetime = json.get("lifetime_seconds");
            if (!(lifetime instanceof Integer) && !(lifetime instanceof Long)) {
                throw new IOException("Media session response is invalid");
            }
            return new MediaSessionCredentials(
                    json.getString("session_id"),
                    json.getString("token"),
                    ((Number) lifetime).longValue(),
                    SystemClock.elapsedRealtime(),
                    json.getString("audio_format"));
        } catch (JSONException error) {
            throw new IOException("Media session response is invalid");
        } catch (MediaSessionCredentials.CredentialException error) {
            throw new IOException("Media session response is invalid");
        } finally {
            connection.disconnect();
        }
    }

    private int post(String endpoint, String path, String token, JSONObject request)
            throws IOException {
        byte[] payload = request.toString().getBytes(StandardCharsets.UTF_8);
        URL url = Endpoint.parse(endpoint, path);
        HttpURLConnection connection = open(url);
        connection.setConnectTimeout(TIMEOUT_MS);
        connection.setReadTimeout(TIMEOUT_MS);
        connection.setRequestMethod("POST");
        connection.setRequestProperty("Accept", "application/json");
        connection.setRequestProperty("Content-Type", "application/json");
        connection.setRequestProperty("Authorization", "Bearer " + token);
        connection.setFixedLengthStreamingMode(payload.length);
        connection.setDoOutput(true);
        try {
            OutputStream output = connection.getOutputStream();
            output.write(payload);
            output.close();
            int status = connection.getResponseCode();
            if (status != HttpURLConnection.HTTP_ACCEPTED) {
                throw new IOException("Daemon returned HTTP " + status);
            }
            return status;
        } finally {
            connection.disconnect();
        }
    }

    private int request(URL url, String token) throws IOException {
        HttpURLConnection connection = open(url);
        connection.setConnectTimeout(TIMEOUT_MS);
        connection.setReadTimeout(TIMEOUT_MS);
        connection.setRequestMethod("GET");
        connection.setRequestProperty("Accept", "application/json");
        if (!token.isEmpty()) {
            connection.setRequestProperty("Authorization", "Bearer " + token);
        }
        try {
            int status = connection.getResponseCode();
            if (status < 200 || status >= 300) {
                throw new IOException("Daemon returned HTTP " + status);
            }
            return status;
        } finally {
            connection.disconnect();
        }
    }

    private HttpURLConnection open(URL url) throws IOException {
        HttpURLConnection connection = (HttpURLConnection) url.openConnection();
        connection.setUseCaches(false);
        connection.setRequestProperty("Cache-Control", "no-store");
        if (connection instanceof HttpsURLConnection && certificatePin == null) {
            throw new IOException("HTTPS certificate pin is required");
        }
        if (connection instanceof HttpsURLConnection) {
            try {
                HttpsURLConnection https = (HttpsURLConnection) connection;
                https.setSSLSocketFactory(certificatePin.socketFactory());
                if (!tlsName.isEmpty()) {
                    final HostnameVerifier verifier = HttpsURLConnection.getDefaultHostnameVerifier();
                    https.setHostnameVerifier(new HostnameVerifier() {
                        @Override public boolean verify(String ignored, javax.net.ssl.SSLSession session) {
                            return verifier.verify(tlsName, session);
                        }
                    });
                }
            } catch (GeneralSecurityException error) {
                throw new IOException("Could not configure certificate pinning");
            }
        }
        return connection;
    }

    private static byte[] readBounded(InputStream input, int maximumBytes) throws IOException {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        byte[] buffer = new byte[256];
        int total = 0;
        while (true) {
            int count = input.read(buffer);
            if (count < 0) {
                return output.toByteArray();
            }
            total += count;
            if (total > maximumBytes) {
                throw new IOException("Media session response is too large");
            }
            output.write(buffer, 0, count);
        }
    }
}
