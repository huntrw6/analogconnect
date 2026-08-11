-- Preserve a stable participant-set key so group messages are not split by sender address.
ALTER TABLE messages ADD COLUMN conversation_key TEXT NOT NULL DEFAULT '';
ALTER TABLE messages ADD COLUMN participants TEXT NOT NULL DEFAULT '';

-- Existing rows remain usable until the forced full refresh replaces this fallback metadata.
UPDATE messages
SET conversation_key = address,
    participants = address
WHERE conversation_key = '';

CREATE INDEX idx_messages_conversation_time
ON messages (conversation_key, timestamp_ms DESC);

-- The old cursor would skip existing handles. A one-time full refresh lets UPSERT attach
-- participant metadata without deleting bodies, read state, or outgoing state.
DELETE FROM folder_cursors;
