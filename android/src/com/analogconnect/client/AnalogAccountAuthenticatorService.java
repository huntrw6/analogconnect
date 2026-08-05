package com.analogconnect.client;

import android.accounts.AbstractAccountAuthenticator;
import android.accounts.Account;
import android.accounts.AccountAuthenticatorResponse;
import android.accounts.NetworkErrorException;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.os.Bundle;
import android.os.IBinder;

public final class AnalogAccountAuthenticatorService extends Service {
    private Authenticator authenticator;

    @Override public void onCreate() {
        authenticator = new Authenticator(this);
    }

    @Override public IBinder onBind(Intent intent) {
        return authenticator.getIBinder();
    }

    private static final class Authenticator extends AbstractAccountAuthenticator {
        Authenticator(Context context) {
            super(context);
        }

        @Override public Bundle editProperties(
                AccountAuthenticatorResponse response, String accountType) {
            return unavailable();
        }

        @Override public Bundle addAccount(AccountAuthenticatorResponse response,
                String accountType, String authTokenType, String[] features, Bundle options)
                throws NetworkErrorException {
            return unavailable();
        }

        @Override public Bundle confirmCredentials(
                AccountAuthenticatorResponse response, Account account, Bundle options) {
            return unavailable();
        }

        @Override public Bundle getAuthToken(AccountAuthenticatorResponse response,
                Account account, String authTokenType, Bundle options)
                throws NetworkErrorException {
            return unavailable();
        }

        @Override public String getAuthTokenLabel(String authTokenType) {
            return "";
        }

        @Override public Bundle updateCredentials(AccountAuthenticatorResponse response,
                Account account, String authTokenType, Bundle options)
                throws NetworkErrorException {
            return unavailable();
        }

        @Override public Bundle hasFeatures(AccountAuthenticatorResponse response,
                Account account, String[] features) throws NetworkErrorException {
            Bundle result = new Bundle();
            result.putBoolean(android.accounts.AccountManager.KEY_BOOLEAN_RESULT, false);
            return result;
        }

        private static Bundle unavailable() {
            Bundle result = new Bundle();
            result.putInt(android.accounts.AccountManager.KEY_ERROR_CODE,
                    android.accounts.AccountManager.ERROR_CODE_UNSUPPORTED_OPERATION);
            result.putString(android.accounts.AccountManager.KEY_ERROR_MESSAGE,
                    "AnalogBridge contact setup is not enabled");
            return result;
        }
    }
}
