package com.analogconnect.client;

final class MessageSendDraft {
    interface IdGenerator {
        String generate();
    }

    private final IdGenerator generator;
    private String recipient;
    private String body;
    private String operationId;

    MessageSendDraft() {
        this(new IdGenerator() {
            @Override public String generate() {
                return MessageOperationId.generate();
            }
        });
    }

    MessageSendDraft(IdGenerator generator) {
        this.generator = generator;
    }

    String operationIdFor(String currentRecipient, String currentBody) {
        if (currentRecipient.equals(recipient) && currentBody.equals(body)
                && MessageOperationId.isValid(operationId)) {
            return operationId;
        }
        String generated = generator.generate();
        if (!MessageOperationId.isValid(generated)) {
            throw new IllegalStateException("Message operation ID generation failed");
        }
        recipient = currentRecipient;
        body = currentBody;
        operationId = generated;
        return operationId;
    }

    void accepted() {
        recipient = null;
        body = null;
        operationId = null;
    }
}
