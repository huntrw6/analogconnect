package com.analogconnect.client;

import java.io.IOException;
import java.net.HttpURLConnection;
import java.net.URL;

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
