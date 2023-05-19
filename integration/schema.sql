CREATE TABLE IF NOT EXISTS listener (
    flow_id text NOT NULL,
    flows_user text NOT NULL,
    bot_token text NOT NULL,
    uuid text NOT NULL,
    PRIMARY KEY (flow_id, flows_user)
);
