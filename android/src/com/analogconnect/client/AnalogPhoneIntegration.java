package com.analogconnect.client;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.telecom.PhoneAccount;
import android.telecom.TelecomManager;

final class AnalogPhoneIntegration {
    private final Context context;
    private final TelecomManager telecom;

    AnalogPhoneIntegration(Context context) {
        this.context = context.getApplicationContext();
        telecom = (TelecomManager) context.getSystemService(Context.TELECOM_SERVICE);
        if (telecom == null) {
            throw new IllegalStateException("Android Phone integration is unavailable");
        }
    }

    boolean isRegistered() {
        return telecom.getPhoneAccount(AnalogPhoneAccount.handle(context)) != null;
    }

    void setRegistered(boolean registered) {
        if (registered) {
            PhoneAccount account = AnalogPhoneAccount.descriptor(context);
            telecom.registerPhoneAccount(account);
        } else {
            telecom.unregisterPhoneAccount(AnalogPhoneAccount.handle(context));
        }
    }

    void openCallingAccountSettings(Activity activity) {
        Intent intent = new Intent(TelecomManager.ACTION_CHANGE_PHONE_ACCOUNTS);
        activity.startActivity(intent);
    }
}
