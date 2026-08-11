package com.analogconnect.client;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.view.Gravity;
import android.view.View;
import android.widget.LinearLayout;
import android.widget.Space;
import android.widget.TextView;

public final class HomeActivity extends Activity {
    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(Ui.dp(this, 20), Ui.dp(this, 24), Ui.dp(this, 20), 0);

        TextView title = Ui.text(this, "Messages", 30);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        root.addView(title);
        TextView status = Ui.text(this, "Your iPhone conversations, calls, and contacts", 16);
        status.setTextColor(Ui.MUTED);
        status.setPadding(0, Ui.dp(this, 6), 0, Ui.dp(this, 24));
        root.addView(status);

        root.addView(Ui.button(this, "Open conversations", open(ConversationsActivity.class)),
                new LinearLayout.LayoutParams(-1, Ui.dp(this, 56)));
        TextView hint = Ui.text(this,
                "Messages stay available when the connection drops. Group conversations are "
                        + "kept together; group replies remain safely unavailable.", 16);
        hint.setLineSpacing(0, 1.15f);
        hint.setPadding(Ui.dp(this, 4), Ui.dp(this, 18), Ui.dp(this, 4), 0);
        root.addView(hint);
        Space space = new Space(this);
        root.addView(space, new LinearLayout.LayoutParams(1, 0, 1f));
        root.addView(Ui.bottomNavigation(this, "Messages"));
        setContentView(root);
    }

    private View.OnClickListener open(final Class<?> target) {
        return new View.OnClickListener() {
            @Override public void onClick(View view) {
                startActivity(new Intent(HomeActivity.this, target));
            }
        };
    }
}
