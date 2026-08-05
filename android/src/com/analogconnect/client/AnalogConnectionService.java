package com.analogconnect.client;

import android.telecom.Connection;
import android.telecom.ConnectionRequest;
import android.telecom.ConnectionService;
import android.telecom.DisconnectCause;
import android.telecom.PhoneAccount;
import android.telecom.PhoneAccountHandle;
import android.telecom.TelecomManager;
import android.net.Uri;

/**
 * Inactive Android Telecom boundary. The account is intentionally not registered
 * until native Phone UI behavior is hardware-validated on the target API-27 device.
 */
public final class AnalogConnectionService extends ConnectionService {
    private static final String NOT_ENABLED = "AnalogBridge calling is not enabled";

    @Override
    public Connection onCreateIncomingConnection(
            PhoneAccountHandle managerPhoneAccount, ConnectionRequest request) {
        return unavailable();
    }

    @Override
    public Connection onCreateOutgoingConnection(
            PhoneAccountHandle managerPhoneAccount, ConnectionRequest request) {
        Uri address = request == null ? null : request.getAddress();
        if (address == null || !PhoneAccount.SCHEME_TEL.equals(address.getScheme())) {
            return unavailable();
        }
        final String target;
        try {
            target = TelecomDialTarget.validate(address.getSchemeSpecificPart());
        } catch (IllegalArgumentException error) {
            return unavailable();
        }
        AnalogTelecomConnection connection = new AnalogTelecomConnection(this);
        connection.setAddress(address, TelecomManager.PRESENTATION_ALLOWED);
        connection.dial(target);
        return connection;
    }

    private static Connection unavailable() {
        return Connection.createFailedConnection(
                new DisconnectCause(DisconnectCause.ERROR, NOT_ENABLED));
    }
}
