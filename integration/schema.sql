CREATE TABLE IF NOT EXISTS listener (
    flow_id text NOT NULL,
    flows_user text NOT NULL,
    handler_fn text,
    channel_id text,
    bot_token text NOT NULL,
    UNIQUE (flow_id, flows_user, handler_fn, channel_id, bot_token)
);

CREATE TABLE IF NOT EXISTS guild_author (
    flows_user text NOT NULL,
    discord_guild_id text NOT NULL,
    discord_guild_name text NOT NULL,
    discord_user_id text NOT NULL,
    discord_username text NOT NULL,
    discord_email text NOT NULL,
    PRIMARY KEY (flows_user, discord_guild_id, discord_user_id)
);