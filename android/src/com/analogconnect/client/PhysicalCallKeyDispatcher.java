package com.analogconnect.client;

final class PhysicalCallKeyDispatcher {
    static final int KEY_CALL = 5;
    static final int KEY_ENDCALL = 6;
    static final int KEY_0 = 7;
    static final int KEY_9 = 16;
    static final int KEY_STAR = 17;
    static final int KEY_POUND = 18;
    static final int KEY_POWER = 26;

    enum Action { NONE, OPEN_DIALER, ANSWER, REJECT, HANG_UP, DTMF }

    static final class Decision {
        final Action action;
        final String value;

        Decision(Action action, String value) {
            this.action = action;
            this.value = value;
        }
    }

    private PhysicalCallKeyDispatcher() {}

    static Decision dispatch(String state, int keyCode, boolean down, int repeatCount) {
        if (!down || repeatCount != 0) return none();
        if (keyCode == KEY_CALL) {
            if ("incoming".equals(state)) return new Decision(Action.ANSWER, "");
            if ("idle".equals(state)) {
                return new Decision(Action.OPEN_DIALER, "");
            }
            return none();
        }
        if (keyCode == KEY_ENDCALL || keyCode == KEY_POWER) {
            if ("incoming".equals(state)) return new Decision(Action.REJECT, "");
            if ("dialing".equals(state) || "ringing".equals(state)
                    || "active".equals(state)) return new Decision(Action.HANG_UP, "");
            return none();
        }
        if ("active".equals(state)) {
            if (keyCode >= KEY_0 && keyCode <= KEY_9) {
                return new Decision(Action.DTMF, Integer.toString(keyCode - KEY_0));
            }
            if (keyCode == KEY_STAR) return new Decision(Action.DTMF, "*");
            if (keyCode == KEY_POUND) return new Decision(Action.DTMF, "#");
        }
        return none();
    }

    static boolean isCallKey(int keyCode) {
        return keyCode == KEY_CALL || keyCode == KEY_ENDCALL || keyCode == KEY_POWER
                || keyCode >= KEY_0 && keyCode <= KEY_9
                || keyCode == KEY_STAR || keyCode == KEY_POUND;
    }

    static boolean isEndKey(int keyCode) {
        return keyCode == KEY_ENDCALL || keyCode == KEY_POWER;
    }

    private static Decision none() { return new Decision(Action.NONE, ""); }
}
