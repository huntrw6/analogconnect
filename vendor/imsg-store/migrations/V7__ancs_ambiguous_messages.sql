-- Fail-closed marker for received messages whose ANCS evidence has competing matches.
CREATE TABLE ancs_ambiguous_messages (
    map_handle  TEXT PRIMARY KEY REFERENCES messages(map_handle) ON UPDATE CASCADE ON DELETE CASCADE,
    observed_at INTEGER NOT NULL
);
