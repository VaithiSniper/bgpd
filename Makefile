SERVER_CONFIG_PATH="examples/configs/server.toml"
CLIENT_CONFIG_PATH="examples/configs/client.toml"

build:
	cargo build

server: build
	cargo run -- $(SERVER_CONFIG_PATH)

client: build
	cargo run -- $(CLIENT_CONFIG_PATH)