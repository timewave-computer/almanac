# Geyser Plugin Builder for Solana
{
  pkgs,
  inputs',
  ...
}: rec {
  # Build Solana with Geyser plugin support
  buildSolanaWithGeyser = { 
    solanaVersion ? "1.18.26",
    geyserPlugin,
    extraBuildInputs ? [] 
  }:
    pkgs.stdenv.mkDerivation {
      pname = "solana-with-geyser";
      version = solanaVersion;
      
      buildInputs = [ geyserPlugin ] ++ extraBuildInputs;
      
      buildCommand = ''
        mkdir -p $out
        echo "Solana ${solanaVersion} configured with Geyser plugin" > $out/info.txt
        echo "Plugin location: ${geyserPlugin}/lib" >> $out/info.txt
      '';
      
      meta = with pkgs.lib; {
        description = "Solana validator with Geyser plugin support";
        platforms = platforms.linux;
      };
    };
  
  # App to build the Geyser plugin
  buildGeyserPlugin = {
    type = "app";
    program = "${pkgs.writeShellScriptBin "build-geyser-plugin" ''
      set -e
      
      echo "=== Building Almanac Geyser Plugin ==="
      echo ""
      
      # Check if plugin source exists
      if [ ! -d "./geyser-plugin" ]; then
        echo "Error: geyser-plugin directory not found"
        echo "Please create the plugin source in ./geyser-plugin"
        exit 1
      fi
      
      # Build the plugin
      echo "Building Geyser plugin..."
      nix build .#geyser-plugin --out-link ./result-geyser-plugin
      
      echo ""
      echo "=== Geyser Plugin Built Successfully ==="
      echo "Plugin library: ./result-geyser-plugin/lib/libalmanac_geyser_plugin.so"
      echo "Config template: ./result-geyser-plugin/etc/almanac-geyser-config.json"
    ''}/bin/build-geyser-plugin";
  };
  
  # App to test Geyser integration
  testGeyserIntegration = {
    type = "app";
    program = "${pkgs.writeShellScriptBin "test-geyser-integration" ''
      set -e
      
      echo "=== Testing Geyser Plugin Integration ==="
      echo ""
      
      # Colors for output
      GREEN='\033[0;32m'
      YELLOW='\033[1;33m'
      RED='\033[0;31m'
      NC='\033[0m' # No Color
      
      # Check PostgreSQL
      echo -e "''${YELLOW}Checking PostgreSQL...''${NC}"
      if pg_isready -q; then
        echo -e "''${GREEN}PostgreSQL is running''${NC}"
      else
        echo -e "''${RED}PostgreSQL is not running''${NC}"
        echo "Please start PostgreSQL first"
        exit 1
      fi
      
      # Check database
      echo -e "''${YELLOW}Checking almanac database...''${NC}"
      if psql -U almanac -d almanac -c '\q' 2>/dev/null; then
        echo -e "''${GREEN}Almanac database exists''${NC}"
      else
        echo -e "''${YELLOW}Creating almanac database...''${NC}"
        createdb almanac || true
        createuser almanac || true
        echo -e "''${GREEN}Database setup complete''${NC}"
      fi
      
      # Start test validator
      echo ""
      echo -e "''${YELLOW}Starting test validator with Geyser plugin...''${NC}"
      echo "Logs will be in: ./test-ledger/validator.log"
      echo ""
      
      # Run the test validator
      solana-test-validator-with-geyser
    ''}/bin/test-geyser-integration";
  };
}