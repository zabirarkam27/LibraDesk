CREATE TABLE settings (
    scope TEXT NOT NULL CHECK (scope IN ('app', 'reader')),
    key TEXT NOT NULL CHECK (length(key) > 0),
    value_json TEXT NOT NULL CHECK (json_valid(value_json)),
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (scope, key)
) STRICT;
