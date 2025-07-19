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
            echo "║                Almanac API Swagger UI Server              ║"
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
            export ALMANAC_HOST="$HOST"
            export ALMANAC_PORT="$PORT"
            
            # Start the API server
            print_info "Starting Almanac API server..."
            print_info "Host: $HOST"
            print_info "Port: $PORT"
            print_info "Config: ''${CONFIG:-default}"
            print_info "Binary: $API_BINARY"
            
            # Start the server in the background
            if [ -n "$CONFIG" ]; then
                $API_BINARY --config "$CONFIG" --host "$HOST" --port "$PORT" &
            else
                $API_BINARY --host "$HOST" --port "$PORT" &
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

        # OpenAPI spec generator
        openApiGenerator = pkgs.writeShellApplication {
          name = "generate-openapi-spec";
          runtimeInputs = with pkgs; [ 
            cargo
            rustc
            jq
          ];
          text = ''
            # Generate OpenAPI specification from Almanac API
            
            set -euo pipefail
            
            OUTPUT="''${1:-openapi.json}"
            
            echo "Generating OpenAPI specification..."
            
            # Build and run the API in spec-generation mode
            cargo run --bin almanac-api -- --generate-openapi > "$OUTPUT"
            
            # Pretty-print the JSON
            jq . "$OUTPUT" > "$OUTPUT.tmp" && mv "$OUTPUT.tmp" "$OUTPUT"
            
            echo "OpenAPI specification generated: $OUTPUT"
            echo ""
            echo "To view with Swagger UI, run:"
            echo "  swagger-static-server 8080 $OUTPUT"
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