SERVER_CONFIG_PATH="examples/configs/server.toml"
CLIENT_CONFIG_PATH="examples/configs/client.toml"
DEFAULT_LOG_LEVEL = 3

build:
	cargo build

server: build
	cargo run -- $(SERVER_CONFIG_PATH) $(DEFAULT_LOG_LEVEL)

client: build
	cargo run -- $(CLIENT_CONFIG_PATH) $(DEFAULT_LOG_LEVEL)