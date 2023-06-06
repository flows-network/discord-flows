CREATE TABLE IF NOT EXISTS listener (
    flow_id text NOT NULL,
    flows_user text NOT NULL,
    channel_id text,
    bot_token text NOT NULL,
    PRIMARY KEY (flow_id, flows_user)
);
