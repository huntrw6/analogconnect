package com.analogconnect.client;

import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.security.cert.CertificateException;
import java.security.cert.X509Certificate;

import javax.net.ssl.SSLContext;
import javax.net.ssl.SSLSocketFactory;
import javax.net.ssl.TrustManager;
import javax.net.ssl.X509TrustManager;

final class CertificatePin {
    private final byte[] expected;

    private CertificatePin(byte[] expected) {
        this.expected = expected;
    }

    static CertificatePin parse(String input) throws GeneralSecurityException {
        String normalized = input == null ? "" : input.trim().replace(":", "");
        if (normalized.length() != 64) {
            throw new GeneralSecurityException("Certificate pin must contain 64 hex characters");
        }
        byte[] expected = new byte[32];
        for (int index = 0; index < expected.length; index++) {
            int high = Character.digit(normalized.charAt(index * 2), 16);
            int low = Character.digit(normalized.charAt(index * 2 + 1), 16);
            if (high < 0 || low < 0) {
                throw new GeneralSecurityException("Certificate pin must be hexadecimal");
            }
            expected[index] = (byte) ((high << 4) | low);
        }
        return new CertificatePin(expected);
    }

    boolean matchesEncoded(byte[] encodedCertificate) throws GeneralSecurityException {
        byte[] actual = MessageDigest.getInstance("SHA-256").digest(encodedCertificate);
        return MessageDigest.isEqual(expected, actual);
    }

    SSLSocketFactory socketFactory() throws GeneralSecurityException {
        X509TrustManager trustManager = new X509TrustManager() {
            @Override
            public void checkClientTrusted(X509Certificate[] chain, String authType)
                    throws CertificateException {
                throw new CertificateException("Client certificates are not accepted");
            }

            @Override
            public void checkServerTrusted(X509Certificate[] chain, String authType)
                    throws CertificateException {
                if (chain == null || chain.length == 0) {
                    throw new CertificateException("Server certificate is missing");
                }
                try {
                    chain[0].checkValidity();
                    if (!matchesEncoded(chain[0].getEncoded())) {
                        throw new CertificateException("Server certificate pin did not match");
                    }
                } catch (GeneralSecurityException error) {
                    throw new CertificateException("Could not verify server certificate", error);
                }
            }

            @Override
            public X509Certificate[] getAcceptedIssuers() {
                return new X509Certificate[0];
            }
        };
        SSLContext context = SSLContext.getInstance("TLS");
        context.init(null, new TrustManager[] {trustManager}, null);
        return context.getSocketFactory();
    }

    @Override
    public String toString() {
        return "CertificatePin([redacted])";
    }
}
