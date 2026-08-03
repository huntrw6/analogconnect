package com.analogconnect.client;

import java.io.IOException;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;

import org.json.JSONException;
import org.json.JSONObject;

final class ApiClient {
    private static final int TIMEOUT_MS = 5000;

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

    private int post(String endpoint, String path, String token, JSONObject request)
            throws IOException {
        byte[] payload = request.toString().getBytes(StandardCharsets.UTF_8);
        URL url = Endpoint.parse(endpoint, path);
        HttpURLConnection connection = (HttpURLConnection) url.openConnection();
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
        HttpURLConnection connection = (HttpURLConnection) url.openConnection();
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
}
