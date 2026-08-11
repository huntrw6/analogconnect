package com.analogconnect.client;

import android.content.Context;
import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;
import android.view.Gravity;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.app.Activity;
import android.content.res.Configuration;

final class Ui {
    static final int NAVY = Color.rgb(24, 52, 77);
    static final int BLUE = Color.rgb(39, 98, 154);
    static final int GREEN = Color.rgb(31, 122, 74);
    static final int RED = Color.rgb(177, 48, 48);
    static final int MUTED = Color.rgb(91, 103, 112);

    private Ui() {}

    static int dp(Context context, int value) {
        return Math.round(value * context.getResources().getDisplayMetrics().density);
    }

    static TextView text(Context context, String value, float size) {
        TextView view = new TextView(context);
        view.setText(value);
        view.setTextSize(size);
        return view;
    }

    static int mutedColor(Context context) {
        return isDark(context) ? Color.rgb(184, 193, 201) : MUTED;
    }

    static void applyTheme(Activity activity) {
        String appearance = new EnrollmentSettings(activity).appearance();
        boolean dark = "dark".equals(appearance) || "device".equals(appearance)
                && (activity.getResources().getConfiguration().uiMode
                & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
        activity.setTheme(dark ? R.style.AppThemeDark : R.style.AppTheme);
    }

    static boolean isDark(Context context) {
        String appearance = new EnrollmentSettings(context).appearance();
        return "dark".equals(appearance) || "device".equals(appearance)
                && (context.getResources().getConfiguration().uiMode
                & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
    }

    static Button button(Context context, String label, View.OnClickListener listener) {
        Button button = new Button(context);
        button.setText(label);
        button.setMinHeight(dp(context, 48));
        button.setAllCaps(false);
        button.setOnClickListener(listener);
        return button;
    }

    static TextView avatar(Context context, String label, int color) {
        TextView avatar = text(context, initials(label), 18);
        avatar.setTextColor(Color.WHITE);
        avatar.setGravity(Gravity.CENTER);
        GradientDrawable background = new GradientDrawable();
        background.setShape(GradientDrawable.OVAL);
        background.setColor(color);
        avatar.setBackground(background);
        avatar.setContentDescription(label + " avatar");
        avatar.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        return avatar;
    }

    static String initials(String value) {
        if (value == null || value.trim().isEmpty()) return "?";
        String[] words = value.trim().split("\\s+");
        String first = words[0].substring(0, 1).toUpperCase();
        return words.length == 1 ? first
                : first + words[words.length - 1].substring(0, 1).toUpperCase();
    }

    static LinearLayout bottomNavigation(final Context context, String selected) {
        LinearLayout navigation = new LinearLayout(context);
        navigation.setOrientation(LinearLayout.HORIZONTAL);
        navigation.setPadding(0, dp(context, 4), 0, dp(context, 4));
        addNav(navigation, context, "Messages", selected, ConversationsActivity.class);
        addNav(navigation, context, "Calls", selected, CallsActivity.class);
        addNav(navigation, context, "Contacts", selected, ContactsActivity.class);
        addNav(navigation, context, "Settings", selected, SettingsActivity.class);
        return navigation;
    }

    private static void addNav(LinearLayout row, final Context context, String label,
            String selected, final Class<?> target) {
        Button button = button(context, label, new View.OnClickListener() {
            @Override public void onClick(View view) {
                context.startActivity(new android.content.Intent(context, target));
            }
        });
        button.setTextSize(11);
        button.setMinWidth(0);
        button.setPadding(0, 0, 0, 0);
        button.setEnabled(!label.equals(selected));
        button.setContentDescription(label + (label.equals(selected) ? ", selected" : ""));
        row.addView(button, new LinearLayout.LayoutParams(0, dp(context, 56), 1f));
    }
}
