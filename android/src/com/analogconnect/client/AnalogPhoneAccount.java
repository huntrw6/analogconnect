package com.analogconnect.client;

import android.content.ComponentName;
import android.content.Context;
import android.net.Uri;
import android.telecom.PhoneAccount;
import android.telecom.PhoneAccountHandle;

final class AnalogPhoneAccount {
    private static final String ACCOUNT_ID = "analogbridge_iphone";

    private AnalogPhoneAccount() {}

    static PhoneAccountHandle handle(Context context) {
        return new PhoneAccountHandle(
                new ComponentName(context, AnalogConnectionService.class), ACCOUNT_ID);
    }

    static PhoneAccount descriptor(Context context) {
        return PhoneAccount.builder(handle(context), "AnalogBridge iPhone")
                .setCapabilities(PhoneAccount.CAPABILITY_CALL_PROVIDER)
                .setSupportedUriSchemes(java.util.Collections.singletonList(
                        PhoneAccount.SCHEME_TEL))
                .setAddress(Uri.fromParts(PhoneAccount.SCHEME_TEL, "", null))
                .build();
    }
}
