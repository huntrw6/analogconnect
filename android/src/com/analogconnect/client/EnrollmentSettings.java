package com.analogconnect.client;

import android.content.Context;
import android.content.SharedPreferences;

final class EnrollmentSettings {
    private static final String PREFERENCES = "analogbridge_connection";
    private static final String ENDPOINT = "endpoint";
    private static final String CERTIFICATE_PIN = "certificate_pin";
    private static final String TLS_NAME = "tls_name";
    private static final String DEFAULT_ENDPOINT = "http://127.0.0.1:8787";
    private static final String DEMO_MODE = "demo_mode";

    private final SharedPreferences preferences;

    EnrollmentSettings(Context context) {
        preferences = context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE);
    }

    void migrateLegacy(SharedPreferences legacy) {
        if (preferences.contains(ENDPOINT) || legacy == null || !legacy.contains(ENDPOINT)) {
            return;
        }
        save(legacy.getString(ENDPOINT, DEFAULT_ENDPOINT),
                legacy.getString(CERTIFICATE_PIN, ""), legacy.getString(TLS_NAME, ""));
    }

    String endpoint() {
        return preferences.getString(ENDPOINT, DEFAULT_ENDPOINT);
    }

    String certificatePin() {
        return preferences.getString(CERTIFICATE_PIN, "");
    }

    String tlsName() {
        return preferences.getString(TLS_NAME, "");
    }

    boolean demoMode() { return preferences.getBoolean(DEMO_MODE, false); }

    void setDemoMode(boolean enabled) {
        preferences.edit().putBoolean(DEMO_MODE, enabled).apply();
    }

    void save(String endpoint, String certificatePin, String tlsName) {
        preferences.edit()
                .putString(ENDPOINT, endpoint)
                .putString(CERTIFICATE_PIN, certificatePin)
                .putString(TLS_NAME, tlsName)
                .apply();
    }
}
