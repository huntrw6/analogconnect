package com.analogconnect.client;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.view.View;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Switch;
import android.widget.CompoundButton;

public final class SettingsActivity extends Activity {
    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setPadding(Ui.dp(this, 20), Ui.dp(this, 24), Ui.dp(this, 20), Ui.dp(this, 12));
        TextView title = Ui.text(this, "Settings", 30);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        content.addView(title);
        section(content, "Connection");
        EnrollmentSettings settings = new EnrollmentSettings(this);
        boolean configured = !settings.endpoint().trim().isEmpty();
        TextView connection = Ui.text(this, configured
                ? "iPhone bridge configured\nMessages, calls, and contacts reconnect automatically"
                : "Set up your iPhone bridge in Developer Tools", 17);
        connection.setLineSpacing(0, 1.2f);
        connection.setContentDescription(configured ? "iPhone bridge configured"
                : "iPhone bridge not configured");
        content.addView(connection);
        section(content, "Appearance");
        content.addView(Ui.text(this, "Uses the phone's readable light appearance", 17));
        section(content, "About");
        content.addView(Ui.text(this, "AnalogConnect\nAndroid companion for your iPhone", 17));
        section(content, "Advanced");
        Switch demo = new Switch(this);
        demo.setText("Use offline demo data");
        demo.setContentDescription("Use isolated offline demo conversations and contacts");
        demo.setChecked(settings.demoMode());
        demo.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
            @Override public void onCheckedChanged(CompoundButton button, boolean checked) {
                new EnrollmentSettings(SettingsActivity.this).setDemoMode(checked);
            }
        });
        content.addView(demo);
        TextView demoNote = Ui.text(this,
                "Demo data stays in memory and never enters your message store.", 14);
        demoNote.setTextColor(Ui.MUTED);
        content.addView(demoNote);
        content.addView(Ui.button(this, "Preview incoming call", demoCall("incoming")));
        content.addView(Ui.button(this, "Preview outgoing call", demoCall("dialing")));
        content.addView(Ui.button(this, "Preview active call", demoCall("active")));
        content.addView(Ui.button(this, "Developer Tools", new View.OnClickListener() {
            @Override public void onClick(View view) {
                startActivity(new Intent(SettingsActivity.this, MainActivity.class));
            }
        }));
        content.addView(Ui.text(this,
                "Connection diagnostics and hardware test controls", 14));
        content.addView(Ui.bottomNavigation(this, "Settings"));
        ScrollView scroll = new ScrollView(this);
        scroll.addView(content);
        setContentView(scroll);
    }

    private void section(LinearLayout content, String label) {
        TextView heading = Ui.text(this, label, 18);
        heading.setTypeface(null, android.graphics.Typeface.BOLD);
        heading.setPadding(0, Ui.dp(this, 26), 0, Ui.dp(this, 8));
        content.addView(heading);
    }

    private View.OnClickListener demoCall(final String state) {
        return new View.OnClickListener() {
            @Override public void onClick(View view) {
                EnrollmentSettings settings = new EnrollmentSettings(SettingsActivity.this);
                settings.setDemoMode(true);
                Intent call = new Intent(SettingsActivity.this, CallsActivity.class);
                call.putExtra(CallsActivity.EXTRA_DEMO_CALL_STATE, state);
                startActivity(call);
            }
        };
    }
}
