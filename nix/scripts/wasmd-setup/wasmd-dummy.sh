#!/usr/bin/env bash

COMMAND="$1"
shift

case "$COMMAND" in
  version)
    echo "Version: v0.31.0-dummy"
    echo "Git Commit: 0000000000000000000000000000000000000000"
    echo "Build Tags: dummy,testing"
    echo "Go Version: go version go1.18 darwin/arm64"
    ;;
  init)
    CHAIN_ID="testing"
    NAME="testing"
    HOME="$HOME/.wasmd-test"
    
    for arg in "$@"; do
      case "$arg" in
        --chain-id=*)
          CHAIN_ID="${arg#*=}"
          ;;
        --home=*)
          HOME="${arg#*=}"
          ;;
        *)
          if [[ "$NAME" == "testing" ]]; then
            NAME="$arg"
          fi
          ;;
      esac
    done
    
    echo "Initializing wasmd node with chain-id: $CHAIN_ID, name: $NAME, home: $HOME"
    mkdir -p "$HOME/config"
    echo "{\"chain_id\": \"$CHAIN_ID\", \"name\": \"$NAME\"}" > "$HOME/config/genesis.json"
    echo "Genesis created at $HOME/config/genesis.json"
    ;;
  config)
    echo "Setting config: $@"
    ;;
  keys)
    SUBCOMMAND="$1"
    shift
    
    case "$SUBCOMMAND" in
      add)
        KEY_NAME="$1"
        echo "Created key: $KEY_NAME"
        echo "cosmos1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5lzm3h4"
        ;;
      show)
        KEY_NAME="$1"
        for arg in "$@"; do
          case "$arg" in
            -a)
              echo "cosmos1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5lzm3h4"
              exit 0
              ;;
          esac
        done
        echo "Key: $KEY_NAME"
        echo "Address: cosmos1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5lzm3h4"
        ;;
    esac
    ;;
  add-genesis-account)
    ADDR="$1"
    AMOUNT="$2"
    echo "Added genesis account $ADDR with $AMOUNT"
    ;;
  gentx)
    VALIDATOR="$1"
    AMOUNT="$2"
    echo "Generated tx for validator $VALIDATOR with $AMOUNT"
    mkdir -p "$HOME/.wasmd-test/config/gentx"
    echo "{\"validator\": \"$VALIDATOR\", \"amount\": \"$AMOUNT\"}" > "$HOME/.wasmd-test/config/gentx/gentx.json"
    ;;
  collect-gentxs)
    echo "Collected genesis transactions"
    ;;
  start)
    echo "Starting wasmd node... (simulated)"
    # In a real scenario this would actually start the node
    # Here we just pretend to start it and sleep
    sleep infinity
    ;;
  status)
    echo "{\"node_info\":{\"network\":\"testing\"},\"sync_info\":{\"latest_block_height\":\"100\"}}"
    ;;
  *)
    echo "Unknown command: $COMMAND"
    exit 1
    ;;
esac 