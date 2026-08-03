package com.analogconnect.client;

import android.app.Activity;
import android.os.Bundle;
import android.text.InputType;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class MainActivity extends Activity {
    private static final String DEFAULT_ENDPOINT = "http://127.0.0.1:8787";
    private static final String ENDPOINT_KEY = "endpoint";

    private final ExecutorService executor = Executors.newSingleThreadExecutor();
    private EditText endpoint;
    private EditText token;
    private TextView result;
    private TokenVault vault;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        vault = new TokenVault(this);

        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        int padding = dp(24);
        layout.setPadding(padding, padding, padding, padding);

        TextView title = new TextView(this);
        title.setText("AnalogConnect");
        title.setTextSize(28);
        layout.addView(title);

        TextView subtitle = new TextView(this);
        subtitle.setText("Raspberry Pi companion client\nAndroid 8.1 hardware check");
        subtitle.setPadding(0, dp(8), 0, dp(20));
        layout.addView(subtitle);

        endpoint = new EditText(this);
        endpoint.setHint("Daemon endpoint");
        endpoint.setSingleLine(true);
        endpoint.setText(getPreferences(MODE_PRIVATE).getString(ENDPOINT_KEY, DEFAULT_ENDPOINT));
        layout.addView(endpoint);

        token = new EditText(this);
        token.setHint("Enrollment token");
        token.setSingleLine(true);
        token.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
        layout.addView(token);

        Button save = new Button(this);
        save.setText("Save enrollment");
        save.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                saveEnrollment();
            }
        });
        layout.addView(save);

        Button check = new Button(this);
        check.setText("Check daemon");
        check.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                checkDaemon();
            }
        });
        layout.addView(check);

        result = new TextView(this);
        result.setText("Not checked");
        result.setPadding(0, dp(20), 0, 0);
        layout.addView(result);

        setContentView(layout);
    }

    private void saveEnrollment() {
        try {
            Endpoint.parse(endpoint.getText().toString(), "/api/v1/health");
            vault.store(token.getText().toString());
            getPreferences(MODE_PRIVATE).edit()
                    .putString(ENDPOINT_KEY, endpoint.getText().toString().trim()).apply();
            token.setText("");
            result.setText("Enrollment saved securely");
        } catch (Exception error) {
            result.setText("Could not save enrollment: " + safeMessage(error));
        }
    }

    private void checkDaemon() {
        result.setText("Checking…");
        String endpointValue = endpoint.getText().toString();
        executor.execute(new Runnable() {
            @Override
            public void run() {
                String message;
                try {
                    ApiClient client = new ApiClient();
                    client.health(endpointValue);
                    String savedToken = vault.load();
                    if (savedToken.isEmpty()) {
                        message = "Daemon healthy; enrollment token not saved";
                    } else {
                        client.status(endpointValue, savedToken);
                        message = "Daemon healthy and client authenticated";
                    }
                } catch (Exception error) {
                    message = "Check failed: " + safeMessage(error);
                }
                final String finalMessage = message;
                runOnUiThread(new Runnable() {
                    @Override
                    public void run() {
                        result.setText(finalMessage);
                    }
                });
            }
        });
    }

    private static String safeMessage(Exception error) {
        String message = error.getMessage();
        return message == null || message.trim().isEmpty() ? "unexpected error" : message;
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    @Override
    protected void onDestroy() {
        executor.shutdownNow();
        super.onDestroy();
    }
}
