package com.analogconnect.client;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.view.Gravity;
import android.view.View;
import android.widget.LinearLayout;
import android.widget.Space;
import android.widget.TextView;

public final class WelcomeActivity extends Activity {
    @Override protected void onCreate(Bundle state) {
        Ui.applyTheme(this);
        super.onCreate(state);
        final EnrollmentSettings settings = new EnrollmentSettings(this);
        if (settings.onboardingComplete()) {
            open(ConversationsActivity.class);
            return;
        }
        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setGravity(Gravity.CENTER_HORIZONTAL);
        content.setPadding(Ui.dp(this, 28), Ui.dp(this, 44), Ui.dp(this, 28), Ui.dp(this, 24));
        TextView mark = Ui.avatar(this, "AnalogConnect", Ui.BLUE);
        content.addView(mark, new LinearLayout.LayoutParams(Ui.dp(this, 88), Ui.dp(this, 88)));
        TextView title = Ui.text(this, "Welcome to AnalogConnect", 28);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        title.setGravity(Gravity.CENTER);
        title.setPadding(0, Ui.dp(this, 28), 0, Ui.dp(this, 14));
        content.addView(title);
        TextView description = Ui.text(this,
                "Use your iPhone calls and messages from this phone.", 18);
        description.setGravity(Gravity.CENTER);
        description.setLineSpacing(0, 1.2f);
        content.addView(description);
        content.addView(new Space(this), new LinearLayout.LayoutParams(1, 0, 1f));
        content.addView(Ui.button(this, "Get started", new View.OnClickListener() {
            @Override public void onClick(View view) {
                settings.completeOnboarding();
                open(ConversationsActivity.class);
            }
        }), new LinearLayout.LayoutParams(-1, Ui.dp(this, 56)));
        content.addView(Ui.button(this, "Developer setup", new View.OnClickListener() {
            @Override public void onClick(View view) {
                settings.completeOnboarding();
                open(MainActivity.class);
            }
        }), new LinearLayout.LayoutParams(-1, Ui.dp(this, 52)));
        setContentView(content);
    }

    private void open(Class<?> target) {
        startActivity(new Intent(this, target));
        finish();
    }
}
