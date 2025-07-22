{
  description = "Swagger UI environment for Almanac API documentation";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ { flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devshell.flakeModule
      ];

      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, lib, ... }: let
        # Swagger UI Server Script
        swaggerUiScript = pkgs.writeShellApplication {
          name = "almanac-swagger-ui";
          runtimeInputs = with pkgs; [ 
            lsof 
            coreutils 
            curl
            cargo
            rustc
          ];
          text = ''
            # Swagger UI Server Script for Almanac API
            # This script serves the Swagger UI documentation for the Almanac API
            
            set -euo pipefail
            
            # Configuration
            DEFAULT_PORT=8080
            DEFAULT_HOST="127.0.0.1"
            DEFAULT_CONFIG="config/dev.toml"
            
            # Parse command line arguments
            PORT="''${1:-$DEFAULT_PORT}"
            HOST="''${2:-$DEFAULT_HOST}"
            CONFIG="''${3:-$DEFAULT_CONFIG}"
            
            # Colors for output
            RED='\033[0;31m'
            GREEN='\033[0;32m'
            YELLOW='\033[1;33m'
            BLUE='\033[0;34m'
            NC='\033[0m' # No Color
            
            # Function to print colored output
            print_info() {
                echo -e "''${BLUE}[INFO]''${NC} $1"
            }
            
            print_success() {
                echo -e "''${GREEN}[SUCCESS]''${NC} $1"
            }
            
            print_warning() {
                echo -e "''${YELLOW}[WARNING]''${NC} $1"
            }
            
            print_error() {
                echo -e "''${RED}[ERROR]''${NC} $1"
            }
            
            # Function to check if port is available
            check_port() {
                local port=$1
                if lsof -Pi :"$port" -sTCP:LISTEN -t >/dev/null 2>&1; then
                    return 1
                else
                    return 0
                fi
            }
            
            # Function to find a free port
            find_free_port() {
                local start_port=$1
                local port=$start_port
                
                while ! check_port "$port"; do
                    port=$((port + 1))
                    if [ $port -gt 65535 ]; then
                        print_error "No available ports found"
                        exit 1
                    fi
                done
                
                echo "$port"
            }
            
            # Function to cleanup on exit
            cleanup() {
                if [ -n "''${API_PID:-}" ]; then
                    print_info "Stopping API server (PID: $API_PID)..."
                    kill $API_PID 2>/dev/null || true
                fi
                exit 0
            }
            
            # Set up signal handlers
            trap cleanup SIGTERM SIGINT
            
            # Print banner
            echo -e "''${BLUE}"
            echo "╔════════════════════════════════════════════════════════════╗"
            echo "║                Almanac API Swagger UI Server               ║"
            echo "║         High-Performance Multi-Chain Blockchain Indexer    ║"
            echo "╚════════════════════════════════════════════════════════════╝"
            echo -e "''${NC}"
            
            # Check if we're in the right directory
            if [ ! -f "Cargo.toml" ] || [ ! -d "crates/almanac-api" ]; then
                print_error "Please run this script from the almanac project root directory"
                exit 1
            fi
            
            # Check if config file exists
            if [ ! -f "$CONFIG" ]; then
                print_warning "Config file '$CONFIG' not found. Using default configuration."
                CONFIG=""
            fi
            
            # Find a free port if the default is taken
            if ! check_port "$PORT"; then
                print_warning "Port $PORT is already in use"
                NEW_PORT=$(find_free_port $((PORT + 1)))
                print_info "Using port $NEW_PORT instead"
                PORT=$NEW_PORT
            fi
            
            # Check if FoundationDB is available
            if ! command -v fdbcli &> /dev/null; then
                print_warning "FoundationDB CLI not found. Some features may not work."
                print_info "To install FoundationDB, run: nix develop .#fdb"
            fi
            
            # Build the API
            print_info "Building Almanac API with Swagger UI integration..."
            if ! cargo build --release --bin almanac-api 2>/dev/null; then
                print_warning "Release build failed, trying debug build..."
                if ! cargo build --bin almanac-api; then
                    print_error "Failed to build almanac-api"
                    exit 1
                fi
                API_BINARY="target/debug/almanac-api"
            else
                API_BINARY="target/release/almanac-api"
            fi
            
            print_success "API built successfully: $API_BINARY"
            
            # Prepare environment
            export RUST_LOG="''${RUST_LOG:-info}"
            export API_HOST="$HOST"
            export API_PORT="$PORT"
            
            # Set up FoundationDB environment variables
            PROJECT_ROOT="''${PROJECT_ROOT:-$(pwd)}"
            export FDB_CLUSTER_FILE="$PROJECT_ROOT/data/fdb.cluster"
            export FDB_NETWORK_OPTION_TLS_CERT_PATH=""
            export FDB_NETWORK_OPTION_TLS_KEY_PATH=""
            export FDB_NETWORK_OPTION_TLS_CA_PATH=""
            
            # Set library paths based on platform for FoundationDB
            ${if pkgs.stdenv.isDarwin then ''
              # macOS: Set DYLD_LIBRARY_PATH for FoundationDB
              if command -v nix >/dev/null 2>&1; then
                FDB_LIB_PATH=$(nix eval --raw .#foundationdb.outPath 2>/dev/null)/lib || true
                if [ -n "$FDB_LIB_PATH" ] && [ -d "$FDB_LIB_PATH" ]; then
                  export DYLD_LIBRARY_PATH="$FDB_LIB_PATH:''${DYLD_LIBRARY_PATH:-}"
                  print_info "FoundationDB library path: $FDB_LIB_PATH"
                fi
              fi
            '' else ''
              # Linux: Set LD_LIBRARY_PATH for FoundationDB
              if command -v nix >/dev/null 2>&1; then
                FDB_LIB_PATH=$(nix eval --raw .#foundationdb.outPath 2>/dev/null)/lib || true
                if [ -n "$FDB_LIB_PATH" ] && [ -d "$FDB_LIB_PATH" ]; then
                  export LD_LIBRARY_PATH="$FDB_LIB_PATH:''${LD_LIBRARY_PATH:-}"
                  print_info "FoundationDB library path: $FDB_LIB_PATH"
                fi
              fi
            ''}
            
            # Set include paths for compilation  
            if command -v nix >/dev/null 2>&1; then
              FDB_INCLUDE_PATH=$(nix eval --raw .#foundationdb.outPath 2>/dev/null)/include || true
              if [ -n "$FDB_INCLUDE_PATH" ] && [ -d "$FDB_INCLUDE_PATH" ]; then
                export FDB_INCLUDE_DIR="$FDB_INCLUDE_PATH"
                export C_INCLUDE_PATH="$FDB_INCLUDE_PATH:''${C_INCLUDE_PATH:-}"
                export CPLUS_INCLUDE_PATH="$FDB_INCLUDE_PATH:''${CPLUS_INCLUDE_PATH:-}"
              fi
            fi
            
            # Start the API server
            print_info "Starting Almanac API server..."
            print_info "Host: $HOST"
            print_info "Port: $PORT"
            print_info "Config: ''${CONFIG:-default}"
            print_info "Binary: $API_BINARY"
            
            # Start the server in the background (environment variables are used for config)
            if [ -n "$CONFIG" ]; then
                $API_BINARY --config "$CONFIG" &
            else
                $API_BINARY &
            fi
            
            API_PID=$!
            
            # Wait for server to start
            print_info "Waiting for server to start..."
            sleep 3
            
            # Check if server is running
            if ! kill -0 $API_PID 2>/dev/null; then
                print_error "API server failed to start"
                exit 1
            fi
            
            # Print access information
            print_success "Almanac API server is running!"
            echo ""
            echo -e "''${GREEN}Documentation URLs:''${NC}"
            echo -e "  ''${BLUE}Swagger UI:''${NC}      http://$HOST:$PORT/swagger-ui"
            echo -e "  ''${BLUE}OpenAPI Spec:''${NC}    http://$HOST:$PORT/openapi.json"
            echo -e "  ''${BLUE}ReDoc:''${NC}           http://$HOST:$PORT/redoc"
            echo -e "  ''${BLUE}Health Check:''${NC}    http://$HOST:$PORT/health"
            echo ""
            echo -e "''${GREEN}API Endpoints:''${NC}"
            echo -e "  ''${BLUE}Events:''${NC}          http://$HOST:$PORT/api/v1/events"
            echo -e "  ''${BLUE}Chain Events:''${NC}    http://$HOST:$PORT/api/v1/chains/{chain}/events"
            echo -e "  ''${BLUE}Latest Block:''${NC}    http://$HOST:$PORT/api/v1/chains/{chain}/latest-block"
            echo ""
            echo -e "''${YELLOW}Pro Tips:''${NC}"
            echo -e "  • Use the Swagger UI to test endpoints interactively"
            echo -e "  • Check the OpenAPI spec for complete API documentation"
            echo -e "  • Health check endpoint provides server status"
            echo -e "  • Press Ctrl+C to stop the server"
            echo ""
            
            # Open browser (optional)
            if [[ "$OSTYPE" == "darwin"* ]]; then
                if command -v open &> /dev/null; then
                    print_info "Opening Swagger UI in browser..."
                    open "http://$HOST:$PORT/swagger-ui" &
                fi
            elif command -v xdg-open &> /dev/null; then
                print_info "Opening Swagger UI in browser..."
                xdg-open "http://$HOST:$PORT/swagger-ui" &
            fi
            
            # Keep the script running and wait for server
            print_info "Server is running. Press Ctrl+C to stop."
            wait $API_PID
          '';
        };

        # Static Swagger UI server (for serving pre-generated OpenAPI specs)
        swaggerStaticServer = pkgs.writeShellApplication {
          name = "swagger-static-server";
          runtimeInputs = with pkgs; [ 
            python3
            curl
          ];
          text = ''
            # Static Swagger UI Server
            # Serves pre-generated OpenAPI specifications
            
            set -euo pipefail
            
            PORT="''${1:-8080}"
            SPEC_FILE="''${2:-openapi.json}"
            
            if [ ! -f "$SPEC_FILE" ]; then
                echo "Error: OpenAPI spec file '$SPEC_FILE' not found"
                echo "Usage: swagger-static-server [port] [spec-file]"
                exit 1
            fi
            
            echo "Starting static Swagger UI server..."
            echo "Port: $PORT"
            echo "Spec: $SPEC_FILE"
            echo ""
            echo "Access at: http://localhost:$PORT"
            echo ""
            
            # Create a simple HTML page with embedded Swagger UI
            cat > /tmp/swagger-ui.html << 'EOF'
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <title>Almanac API - Swagger UI</title>
                <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css">
            </head>
            <body>
                <div id="swagger-ui"></div>
                <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
                <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-standalone-preset.js"></script>
                <script>
                    window.onload = function() {
                        window.ui = SwaggerUIBundle({
                            url: '/openapi.json',
                            dom_id: '#swagger-ui',
                            presets: [
                                SwaggerUIBundle.presets.apis,
                                SwaggerUIStandalonePreset
                            ],
                            layout: "StandaloneLayout"
                        });
                    };
                </script>
            </body>
            </html>
            EOF
            
            # Copy spec file to temp directory
            cp "$SPEC_FILE" /tmp/openapi.json
            
            # Start Python HTTP server
            cd /tmp
            python3 -m http.server "$PORT"
          '';
        };

        # OpenAPI spec generator with FoundationDB support
        openApiGenerator = pkgs.writeShellApplication {
          name = "generate-openapi-spec";
          runtimeInputs = with pkgs; [ 
            cargo
            rustc
            jq
            coreutils
            curl
            python3
          ];
          text = ''
            # Generate OpenAPI specification from Almanac API with FoundationDB support
            
            set -euo pipefail
            
            OUTPUT="''${1:-openapi.json}"
            PROJECT_ROOT="''${PROJECT_ROOT:-$(pwd)}"
            
            echo "=== Generating OpenAPI Specification ==="
            echo "Output file: $OUTPUT"
            echo "Platform: ${system}"
            echo ""
            
            # Set up FoundationDB environment variables
            export FDB_CLUSTER_FILE="$PROJECT_ROOT/data/fdb.cluster"
            export FDB_NETWORK_OPTION_TLS_CERT_PATH=""
            export FDB_NETWORK_OPTION_TLS_KEY_PATH=""
            export FDB_NETWORK_OPTION_TLS_CA_PATH=""
            
            # Set library paths based on platform
            ${if pkgs.stdenv.isDarwin then ''
              # macOS: Set DYLD_LIBRARY_PATH for FoundationDB
              if command -v nix >/dev/null 2>&1; then
                FDB_LIB_PATH=$(nix eval --raw .#foundationdb.outPath 2>/dev/null)/lib || true
                if [ -n "$FDB_LIB_PATH" ] && [ -d "$FDB_LIB_PATH" ]; then
                  export DYLD_LIBRARY_PATH="$FDB_LIB_PATH:''${DYLD_LIBRARY_PATH:-}"
                  echo "FoundationDB library path (macOS): $FDB_LIB_PATH"
                else
                  echo "Warning: FoundationDB library path not found. Some features may not work."
                fi
              fi
            '' else ''
              # Linux: Set LD_LIBRARY_PATH for FoundationDB
              if command -v nix >/dev/null 2>&1; then
                FDB_LIB_PATH=$(nix eval --raw .#foundationdb.outPath 2>/dev/null)/lib || true
                if [ -n "$FDB_LIB_PATH" ] && [ -d "$FDB_LIB_PATH" ]; then
                  export LD_LIBRARY_PATH="$FDB_LIB_PATH:''${LD_LIBRARY_PATH:-}"
                  echo "FoundationDB library path (Linux): $FDB_LIB_PATH"
                else
                  echo "Warning: FoundationDB library path not found. Some features may not work."
                fi
              fi
            ''}
            
            # Set include paths for compilation  
            if command -v nix >/dev/null 2>&1; then
              FDB_INCLUDE_PATH=$(nix eval --raw .#foundationdb.outPath 2>/dev/null)/include || true
              if [ -n "$FDB_INCLUDE_PATH" ] && [ -d "$FDB_INCLUDE_PATH" ]; then
                export FDB_INCLUDE_DIR="$FDB_INCLUDE_PATH"
                export C_INCLUDE_PATH="$FDB_INCLUDE_PATH:''${C_INCLUDE_PATH:-}"
                export CPLUS_INCLUDE_PATH="$FDB_INCLUDE_PATH:''${CPLUS_INCLUDE_PATH:-}"
                echo "FoundationDB include path: $FDB_INCLUDE_PATH"
              fi
            fi
            
            # Check if we're in the right directory
            if [ ! -f "Cargo.toml" ] || [ ! -d "crates/almanac-api" ]; then
                echo "Error: Please run this script from the almanac project root directory"
                exit 1
            fi
            
            # Create cluster file if it doesn't exist
            if [ ! -f "$FDB_CLUSTER_FILE" ]; then
                echo "Creating default FoundationDB cluster file..."
                mkdir -p "$(dirname "$FDB_CLUSTER_FILE")"
                echo "almanac_dev:almanac_dev@127.0.0.1:4500" > "$FDB_CLUSTER_FILE"
            fi
            
            echo ""
            echo "Environment variables set:"
            echo "  FDB_CLUSTER_FILE=$FDB_CLUSTER_FILE"
            ${if pkgs.stdenv.isDarwin then ''
              echo "  DYLD_LIBRARY_PATH=''${DYLD_LIBRARY_PATH:-}"
            '' else ''
              echo "  LD_LIBRARY_PATH=''${LD_LIBRARY_PATH:-}"
            ''}
            echo "  FDB_INCLUDE_DIR=''${FDB_INCLUDE_DIR:-}"
            echo ""
            
            echo "Building and running almanac-api in spec-generation mode..."
            
            # Start the API server temporarily and fetch the OpenAPI spec
            echo "Starting almanac-api server to fetch OpenAPI specification..."
            
            # Find an available port
            API_PORT=$(python3 -c "import socket; s=socket.socket(); s.bind(('localhost',0)); print(s.getsockname()[1]); s.close()")
            export API_PORT="$API_PORT"
            echo "Using port: $API_PORT"
            
            # Try memory backend as fallback if FDB fails
            echo "Attempting with memory backend fallback..."
            ALMANAC_BACKEND=memory cargo run --bin almanac-api > api_server.log 2>&1 &
            API_PID=$!
            
            echo "API server started (PID: $API_PID)"
            echo "Waiting for server to be ready..."
            
            # Wait for server to be ready (max 30 seconds)
            for i in {1..30}; do
              if curl -s "http://localhost:$API_PORT/health" > /dev/null 2>&1; then
                echo "Server is ready"
                break
              fi
              if ! kill -0 $API_PID 2>/dev/null; then
                echo "Error: Server process died"
                echo "Server log:"
                cat api_server.log || echo "Could not read log file"
                exit 1
              fi
              echo "Waiting... ($i/30)"
              sleep 1
            done
            
            # Try to fetch the OpenAPI specification from different possible endpoints
            OPENAPI_FETCHED=false
            for endpoint in "/openapi.json" "/api/openapi.json" "/docs/openapi.json"; do
              echo "Trying endpoint: $endpoint"
              if curl -s "http://localhost:$API_PORT$endpoint" -o "$OUTPUT" 2>/dev/null; then
                if jq empty "$OUTPUT" >/dev/null 2>&1; then
                  echo "Successfully fetched OpenAPI spec from $endpoint"
                  OPENAPI_FETCHED=true
                  break
                fi
              fi
            done
            
            # Clean up: kill the API server
            echo "Stopping API server..."
            kill $API_PID 2>/dev/null || true
            wait $API_PID 2>/dev/null || true
            rm -f api_server.log
            
            # Check if we successfully fetched the spec
            if [ "$OPENAPI_FETCHED" != "true" ]; then
              echo "Error: Could not fetch OpenAPI specification from any endpoint"
              echo "Tried endpoints: /openapi.json, /api/openapi.json, /docs/openapi.json"
              echo "Make sure the almanac-api server serves OpenAPI spec at one of these endpoints"
              exit 1
            fi
            
            # Validate the generated JSON
            if ! jq empty "$OUTPUT" >/dev/null 2>&1; then
              echo "Error: Generated file is not valid JSON"
              exit 1
            fi
            
            # Pretty-print the JSON
            echo "Formatting JSON output..."
            jq . "$OUTPUT" > "$OUTPUT.tmp" && mv "$OUTPUT.tmp" "$OUTPUT"
            
            echo ""
            echo "=== OpenAPI Specification Generated Successfully ==="
            echo "File: $OUTPUT"
            echo "Size: $(du -h "$OUTPUT" | cut -f1)"
            echo ""
            echo "Next steps:"
            echo "  View with Swagger UI:  nix run .#swagger-ui"
            echo "  Serve static file:     swagger-static-server 8080 $OUTPUT"
            echo "  Validate spec:         jq . $OUTPUT >/dev/null && echo 'Valid JSON'"
          '';
        };

      in {
        packages = {
          default = swaggerUiScript;
          swagger-ui = swaggerUiScript;
          swagger-static-server = swaggerStaticServer;
          generate-openapi-spec = openApiGenerator;
        };

        devshells.default = {
          name = "swagger-ui-development";
          
          commands = [
            {
              package = self'.packages.swagger-ui;
              help = "Start Almanac API with Swagger UI";
            }
            {
              package = self'.packages.swagger-static-server;
              help = "Serve static OpenAPI specs with Swagger UI";
            }
            {
              package = self'.packages.generate-openapi-spec;
              help = "Generate OpenAPI specification from API";
            }
            {
              package = pkgs.httpie;
              help = "HTTP client for testing API endpoints";
            }
            {
              package = pkgs.jq;
              help = "JSON processor for API responses";
            }
          ];
          
          env = [
            {
              name = "RUST_LOG";
              value = "info";
            }
          ];
          
          motd = ''
            {14}{bold}Swagger UI development environment{reset}
            {10}API Documentation Tools for Almanac{reset}
            
            {13}Quick start:{reset}
              swagger-ui              # Start API with Swagger UI (default port 8080)
              swagger-ui 3000         # Start on custom port
              generate-openapi-spec   # Generate OpenAPI spec file
              swagger-static-server   # Serve static OpenAPI spec
              
            {13}URLs:{reset}
              Swagger UI:   http://localhost:8080/swagger-ui
              OpenAPI Spec: http://localhost:8080/openapi.json
              Health Check: http://localhost:8080/health
          '';
        };
      };
    };
}