package com.analogconnect.client;

final class CallController {
    static final class State {
        final String call;
        final boolean canDial;
        final boolean canAnswer;
        final boolean canReject;
        final boolean canHangUp;
        final boolean canSendDtmf;
        final boolean needsAudio;
        final String title;

        private State(String call, boolean canDial, boolean canAnswer, boolean canReject,
                boolean canHangUp, boolean canSendDtmf, boolean needsAudio, String title) {
            this.call = call;
            this.canDial = canDial;
            this.canAnswer = canAnswer;
            this.canReject = canReject;
            this.canHangUp = canHangUp;
            this.canSendDtmf = canSendDtmf;
            this.needsAudio = needsAudio;
            this.title = title;
        }
    }

    private CallController() {}

    static State reduce(String call) {
        if ("idle".equals(call)) {
            return new State(call, true, false, false, false, false, false, "Ready to call");
        }
        if ("dialing".equals(call) || "ringing".equals(call)) {
            return new State(call, false, false, false, true, false, false, "Calling…");
        }
        if ("incoming".equals(call)) {
            return new State(call, false, true, true, false, false, false, "Incoming call");
        }
        if ("active".equals(call)) {
            return new State(call, false, false, false, true, true, true, "Call in progress");
        }
        if ("ending".equals(call)) {
            return new State(call, false, false, false, false, false, false, "Ending call…");
        }
        if ("ended".equals(call)) {
            return new State(call, true, false, false, false, false, false, "Call ended");
        }
        if ("error".equals(call) || "failed".equals(call)
                || "connection_lost".equals(call)) {
            return new State(call, true, false, false, false, false, false,
                    "Call connection needs attention");
        }
        throw new IllegalArgumentException("Unknown call state");
    }

    static boolean validDtmf(String tone) {
        if (tone == null || tone.length() != 1) {
            return false;
        }
        char value = tone.charAt(0);
        return (value >= '0' && value <= '9') || value == '*' || value == '#'
                || (value >= 'A' && value <= 'D');
    }
}
