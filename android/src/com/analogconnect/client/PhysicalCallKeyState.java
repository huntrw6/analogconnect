package com.analogconnect.client;

final class PhysicalCallKeyState {
    private static volatile String state = "idle";

    private PhysicalCallKeyState() {}

    static void update(String value) { state = value; }
    static String current() { return state; }
}
