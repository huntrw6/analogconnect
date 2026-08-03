package com.analogconnect.client;

import java.net.Inet6Address;
import java.net.InetAddress;

final class DiscoveryTarget {
    final String endpoint;
    final String tlsName;

    private DiscoveryTarget(String endpoint, String tlsName) {
        this.endpoint = endpoint;
        this.tlsName = tlsName;
    }

    static DiscoveryTarget from(InetAddress host, int port) {
        return from(host, port, host == null ? null : host.getHostName());
    }

    static DiscoveryTarget from(InetAddress host, int port, String tlsName) {
        if (host == null || port < 1 || port > 65535) {
            throw new IllegalArgumentException("Discovery result is invalid");
        }
        if (tlsName == null || !tlsName.toLowerCase().endsWith(".local")) {
            throw new IllegalArgumentException("Discovery TLS identity is unavailable");
        }
        String address = host.getHostAddress();
        if (host instanceof Inet6Address) {
            address = "[" + address.replace("%", "%25") + "]";
        }
        return new DiscoveryTarget("https://" + address + ":" + port, tlsName);
    }
}
