SERVER_ADDRESS="0.0.0.0"
SERVER_PORT=8000
FULL_SERVER_ADDRESS="$(SERVER_ADDRESS):$(SERVER_PORT)"
BINARY_PATH=target/debug/bgpd

build:
	cargo build

server: build
	cargo run -- server $(FULL_SERVER_ADDRESS)

client: build
	cargo run -- client $(FULL_SERVER_ADDRESS)

run_netcat_server:
	nc -l -k 0.0.0.0 8000 &

kill_netcat_server:
	ps -eafjx | grep -i "nc -l -k 0.0.0.0" | awk