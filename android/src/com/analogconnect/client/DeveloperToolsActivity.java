package com.analogconnect.client;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.view.View;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class DeveloperToolsActivity extends Activity {
    private final ExecutorService executor = Executors.newSingleThreadExecutor();
    private TextView apiState;
    private EnrollmentSettings settings;

    @Override protected void onCreate(Bundle state) {
        Ui.applyTheme(this);
        super.onCreate(state);
        settings = new EnrollmentSettings(this);
        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setPadding(Ui.dp(this, 20), Ui.dp(this, 20), Ui.dp(this, 20), Ui.dp(this, 20));
        content.addView(Ui.button(this, "‹ Settings", new View.OnClickListener() {
            @Override public void onClick(View view) { finish(); }
        }));
        TextView title = Ui.text(this, "Developer Tools", 28);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        content.addView(title);
        section(content, "Connection");
        apiState = Ui.text(this, "API        Checking…\niPhone     Status unavailable", 16);
        content.addView(apiState);
        content.addView(Ui.button(this, "Refresh status", new View.OnClickListener() {
            @Override public void onClick(View view) { refresh(); }
        }));
        section(content, "Messages");
        content.addView(Ui.text(this, "MAP        Production adapter configured\n"
                + "ANCS       Production LE bearer enabled (hardware test pending)\n"
                + "Group reply Disabled", 16));
        section(content, "Contacts");
        content.addView(Ui.text(this, "PBAP       Production adapter configured\n"
                + "Cache      Encrypted last-known-good snapshot", 16));
        section(content, "Calls and audio");
        content.addView(Ui.text(this, "HFP        Backend-authoritative control\n"
                + "Audio      Automatic bounded media session", 16));
        section(content, "Hardware Validation");
        for (String test : new String[] {"○ Production ANCS connection",
                "○ Named group receive", "○ Unnamed group stability",
                "○ Same-name groups", "○ Group rename", "○ Direct send",
                "○ Direct receive", "✓ Incoming call", "✓ Outgoing call",
                "✓ Call audio", "○ ANCS Reply action"}) {
            boolean complete = test.startsWith("✓");
            TextView row = Ui.text(this, test + "\n   "
                    + (complete ? "VERIFIED_HARDWARE" : "Pending user/iPhone test"), 16);
            row.setPadding(0, Ui.dp(this, 7), 0, Ui.dp(this, 7));
            content.addView(row);
        }
        section(content, "Diagnostics");
        content.addView(Ui.button(this, "Open raw diagnostics", new View.OnClickListener() {
            @Override public void onClick(View view) {
                startActivity(new Intent(DeveloperToolsActivity.this, MainActivity.class));
            }
        }));
        TextView evidence = Ui.text(this,
                "Synthetic checks are VERIFIED_AUTOMATED. Real communication tests remain "
                        + "pending and never run automatically.", 14);
        evidence.setTextColor(Ui.mutedColor(this));
        content.addView(evidence);
        ScrollView scroll = new ScrollView(this);
        scroll.addView(content);
        setContentView(scroll);
        refresh();
    }

    private void section(LinearLayout content, String title) {
        TextView heading = Ui.text(this, title, 18);
        heading.setTypeface(null, android.graphics.Typeface.BOLD);
        heading.setPadding(0, Ui.dp(this, 22), 0, Ui.dp(this, 7));
        content.addView(heading);
    }

    private void refresh() {
        apiState.setText("API        Checking…\niPhone     Status unavailable");
        executor.execute(new Runnable() {
            @Override public void run() {
                boolean connected = false;
                try {
                    ApiClient client = new ApiClient(settings.certificatePin(), settings.tlsName());
                    connected = client.health(settings.endpoint()) >= 200;
                } catch (Exception ignored) { }
                final boolean result = connected;
                runOnUiThread(new Runnable() {
                    @Override public void run() {
                        apiState.setText(result ? "API        Connected\niPhone     Query authenticated status below"
                                : "API        Disconnected\niPhone     Trying to reconnect…");
                    }
                });
            }
        });
    }

    @Override protected void onDestroy() {
        executor.shutdownNow();
        super.onDestroy();
    }
}
