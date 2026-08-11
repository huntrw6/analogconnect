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
import java.util.ArrayList;
import java.util.List;

import javax.net.ssl.HttpsURLConnection;
import javax.net.ssl.HostnameVerifier;

import org.json.JSONException;
import org.json.JSONArray;
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

    String callState(String endpoint, String token) throws IOException {
        if (token == null || token.isEmpty()) {
            throw new IOException("Enrollment token is missing");
        }
        HttpURLConnection connection = open(Endpoint.parse(endpoint, "/api/v1/status"));
        connection.setConnectTimeout(TIMEOUT_MS);
        connection.setReadTimeout(TIMEOUT_MS);
        connection.setRequestMethod("GET");
        connection.setRequestProperty("Accept", "application/json");
        connection.setRequestProperty("Authorization", "Bearer " + token);
        try {
            int status = connection.getResponseCode();
            if (status < 200 || status >= 300) {
                throw new IOException("Daemon returned HTTP " + status);
            }
            JSONObject json = new JSONObject(new String(
                    readBounded(connection.getInputStream(), 4096), StandardCharsets.UTF_8));
            String call = json.getString("call");
            if (!("idle".equals(call) || "dialing".equals(call) || "incoming".equals(call)
                    || "active".equals(call) || "ended".equals(call) || "error".equals(call))) {
                throw new IOException("Daemon call state is invalid");
            }
            return call;
        } catch (JSONException error) {
            throw new IOException("Daemon status response is invalid");
        } finally {
            connection.disconnect();
        }
    }

    int sendMessage(String endpoint, String token, String recipient, String body,
            String operationId)
            throws IOException {
        if (token == null || token.isEmpty()) {
            throw new IOException("Enrollment token is missing");
        }
        try {
            JSONObject request = new JSONObject();
            request.put("recipient", recipient);
            request.put("body", body);
            request.put("operation_id", operationId);
            return post(endpoint, "/api/v1/messages", token, request);
        } catch (JSONException error) {
            throw new IOException("Could not encode message request");
        }
    }

    ConversationPageData<ConversationSummary> conversations(String endpoint, String token)
            throws IOException {
        JSONObject response = getJson(endpoint, "/api/v2/conversations?limit=100", token, 65536);
        try {
            if (response.length() != 2) {
                throw new JSONException("unexpected response fields");
            }
            JSONArray items = response.getJSONArray("items");
            if (items.length() > 100) {
                throw new JSONException("too many conversation items");
            }
            List<ConversationSummary> parsed = new ArrayList<ConversationSummary>(items.length());
            for (int index = 0; index < items.length(); index++) {
                JSONObject item = items.getJSONObject(index);
                if (item.length() != 13) {
                    throw new JSONException("unexpected conversation fields");
                }
                parsed.add(new ConversationSummary(
                        item.getString("conversation_id"), item.getString("display_address"),
                        optionalString(item, "display_name"),
                        item.getBoolean("is_group"), item.getBoolean("reply_supported"),
                        item.getLong("latest_unix_millis"), item.getLong("message_count"),
                        item.getLong("unread_count"), optionalString(item, "latest_outgoing_state"),
                        item.getString("kind"), item.getString("title"),
                        item.getBoolean("can_reply"), item.getBoolean("identity_conflict")));
            }
            return new ConversationPageData<ConversationSummary>(
                    parsed, optionalString(response, "next_cursor"));
        } catch (JSONException error) {
            throw new IOException("Conversation response is invalid");
        } catch (IllegalArgumentException error) {
            throw new IOException("Conversation response is invalid");
        }
    }

    ConversationPageData<ContactListItem> contacts(String endpoint, String token, String query)
            throws IOException {
        return contacts(endpoint, token, query, null);
    }

    ConversationPageData<ContactListItem> contacts(String endpoint, String token, String query,
            String cursor) throws IOException {
        JSONObject request = new JSONObject();
        try {
            request.put("query", query == null ? "" : query);
            request.put("limit", 100);
            if (cursor != null) {
                request.put("cursor", cursor);
            }
        } catch (JSONException error) {
            throw new IOException("Could not encode contact request");
        }
        JSONObject response = postJson(endpoint, "/api/v2/contacts/search", token,
                request, HttpURLConnection.HTTP_OK, 262144);
        try {
            if (response.length() != 2) {
                throw new JSONException("unexpected response fields");
            }
            JSONArray items = response.getJSONArray("items");
            if (items.length() > 100) {
                throw new JSONException("too many contact items");
            }
            List<ContactListItem> parsed = new ArrayList<ContactListItem>(items.length());
            for (int index = 0; index < items.length(); index++) {
                JSONObject item = items.getJSONObject(index);
                if (item.length() != 2) {
                    throw new JSONException("unexpected contact fields");
                }
                JSONArray phoneValues = item.getJSONArray("phone_numbers");
                if (phoneValues.length() == 0 || phoneValues.length() > 32) {
                    throw new JSONException("invalid contact phone count");
                }
                List<String> phones = new ArrayList<String>(phoneValues.length());
                for (int phoneIndex = 0; phoneIndex < phoneValues.length(); phoneIndex++) {
                    phones.add(phoneValues.getString(phoneIndex));
                }
                parsed.add(new ContactListItem(optionalString(item, "display_name"), phones));
            }
            return new ConversationPageData<ContactListItem>(
                    parsed, optionalString(response, "next_cursor"));
        } catch (JSONException error) {
            throw new IOException("Contact response is invalid");
        } catch (IllegalArgumentException error) {
            throw new IOException("Contact response is invalid");
        }
    }

    ConversationPageData<ConversationMessage> conversationMessages(String endpoint, String token,
            String conversationId) throws IOException {
        if (token == null || token.isEmpty()) {
            throw new IOException("Enrollment token is missing");
        }
        JSONObject request = new JSONObject();
        try {
            request.put("conversation_id", conversationId);
            request.put("limit", 100);
        } catch (JSONException error) {
            throw new IOException("Could not encode conversation request");
        }
        JSONObject response = postJson(endpoint, "/api/v2/conversations/messages", token,
                request, HttpURLConnection.HTTP_OK, 262144);
        try {
            if (response.length() != 2) {
                throw new JSONException("unexpected response fields");
            }
            JSONArray items = response.getJSONArray("items");
            if (items.length() > 100) {
                throw new JSONException("too many message items");
            }
            List<ConversationMessage> parsed = new ArrayList<ConversationMessage>(items.length());
            for (int index = 0; index < items.length(); index++) {
                JSONObject item = items.getJSONObject(index);
                if (item.length() != 7) {
                    throw new JSONException("unexpected message fields");
                }
                parsed.add(new ConversationMessage(
                        item.getString("message_id"), item.getLong("timestamp_unix_millis"),
                        item.getString("direction"), item.getString("peer_address"),
                        item.getString("body"),
                        item.getBoolean("read"), optionalString(item, "outgoing_state")));
            }
            return new ConversationPageData<ConversationMessage>(
                    parsed, optionalString(response, "next_cursor"));
        } catch (JSONException error) {
            throw new IOException("Message history response is invalid");
        } catch (IllegalArgumentException error) {
            throw new IOException("Message history response is invalid");
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

    private JSONObject getJson(String endpoint, String path, String token, int maximumBytes)
            throws IOException {
        if (token == null || token.isEmpty()) {
            throw new IOException("Enrollment token is missing");
        }
        HttpURLConnection connection = open(Endpoint.parse(endpoint, path));
        connection.setConnectTimeout(TIMEOUT_MS);
        connection.setReadTimeout(TIMEOUT_MS);
        connection.setRequestMethod("GET");
        connection.setRequestProperty("Accept", "application/json");
        connection.setRequestProperty("Authorization", "Bearer " + token);
        try {
            int status = connection.getResponseCode();
            if (status < 200 || status >= 300) {
                throw new IOException("Daemon returned HTTP " + status);
            }
            return new JSONObject(new String(
                    readBounded(connection.getInputStream(), maximumBytes), StandardCharsets.UTF_8));
        } catch (JSONException error) {
            throw new IOException("Daemon response is invalid");
        } finally {
            connection.disconnect();
        }
    }

    private JSONObject postJson(String endpoint, String path, String token, JSONObject request,
            int expectedStatus, int maximumBytes) throws IOException {
        byte[] payload = request.toString().getBytes(StandardCharsets.UTF_8);
        HttpURLConnection connection = open(Endpoint.parse(endpoint, path));
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
            if (status != expectedStatus) {
                throw new IOException("Daemon returned HTTP " + status);
            }
            return new JSONObject(new String(
                    readBounded(connection.getInputStream(), maximumBytes), StandardCharsets.UTF_8));
        } catch (JSONException error) {
            throw new IOException("Daemon response is invalid");
        } finally {
            connection.disconnect();
        }
    }

    private static String optionalString(JSONObject object, String name) throws JSONException {
        return object.isNull(name) ? null : object.getString(name);
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
