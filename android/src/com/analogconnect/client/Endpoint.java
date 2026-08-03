package com.analogconnect.client;

import java.net.MalformedURLException;
import java.net.URL;

final class Endpoint {
    private Endpoint() {}

    static URL parse(String input, String path) throws MalformedURLException {
        String value = input == null ? "" : input.trim();
        URL base = new URL(value.endsWith("/") ? value : value + "/");
        String protocol = base.getProtocol();
        if (!("http".equals(protocol) || "https".equals(protocol))) {
            throw new MalformedURLException("Only HTTP or HTTPS endpoints are supported");
        }
        if (base.getUserInfo() != null || base.getHost().isEmpty()) {
            throw new MalformedURLException("Endpoint must not contain credentials");
        }
        if ("http".equals(protocol) && !isLoopback(base.getHost())) {
            throw new MalformedURLException("Cleartext HTTP is restricted to loopback");
        }
        return new URL(base, path.startsWith("/") ? path.substring(1) : path);
    }

    private static boolean isLoopback(String host) {
        return "127.0.0.1".equals(host) || "localhost".equalsIgnoreCase(host)
                || "[::1]".equals(host) || "::1".equals(host);
    }
}
