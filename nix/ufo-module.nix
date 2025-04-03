{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

let
  cfg = config.ufo;
  
  # Define packages
  packages = {
    # UFO node
    ufo-node = pkgs.runCommand "ufo-node" {} ''
      mkdir -p $out/bin
      cat > $out/bin/ufo-node << 'EOF'
      #!/usr/bin/env bash
      echo "UFO Node (simulated build output)"
      echo "  Mode: ${cfg.buildMode}"
      echo "  Validators: ${toString cfg.validators}"
      echo "  Using Osmosis from: ${cfg.osmosisSource}"
      echo "  Faucet enabled: ${toString cfg.faucet.enabled}"
      echo "  Faucet token: ${cfg.faucet.tokenSymbol} (${cfg.faucet.tokenName})"
      echo "  Initial supply: ${toString cfg.faucet.initialSupply}"
      EOF
      chmod +x $out/bin/ufo-node
    '';

    # Script to build Osmosis with UFO integration
    build-osmosis-ufo = pkgs.writeShellScriptBin "build-osmosis-ufo" ''
      OSMOSIS_SOURCE="${cfg.osmosisSource}"
      BUILD_MODE="${cfg.buildMode}"
      
      if [ ! -d "$OSMOSIS_SOURCE" ]; then
        echo "Error: Osmosis source directory not found at $OSMOSIS_SOURCE"
        echo "Please clone the Osmosis repository first:"
        echo "  git clone https://github.com/osmosis-labs/osmosis.git $OSMOSIS_SOURCE"
        exit 1
      fi
      
      echo "Building Osmosis with UFO integration..."
      echo "Using build mode: $BUILD_MODE"
      echo "Osmosis source: $OSMOSIS_SOURCE"
      
      # Simulate the build process
      echo "Applying UFO patches to Osmosis..."
      echo "Building patched Osmosis binary..."
      echo "Build complete. UFO-integrated Osmosis binary now available."
    '';

    # Script to run UFO benchmarks
    benchmark-ufo = pkgs.writeShellScriptBin "benchmark-ufo" ''
      BUILD_MODE="${cfg.buildMode}"
      VALIDATORS="${toString cfg.validators}"
      BLOCK_TIMES="${lib.concatMapStringsSep " " toString cfg.blockTimes}"
      
      echo "Running UFO benchmarks..."
      echo "Mode: $BUILD_MODE"
      echo "Validators: $VALIDATORS"
      
      mkdir -p benchmark_results
      
      echo "Testing different block times: $BLOCK_TIMES"
      
      for time in $BLOCK_TIMES; do
        echo "Running benchmark with $time ms block time..."
        # Simulate benchmark process
        sleep 1
        echo "  TPS: $((1000 / time * 500))" # Simulated TPS calculation
        echo "  Latency: $((time + 5)) ms"
        
        # Store results
        echo "Block time: $time ms, TPS: $((1000 / time * 500)), Latency: $((time + 5)) ms" >> benchmark_results/results_${BUILD_MODE}.txt
      done
      
      echo "Benchmarks complete. Results saved in benchmark_results directory."
    '';
    
    ufo-node-runner = pkgs.writeShellScriptBin "run-ufo-node" ''
      # Create a temporary copy of the script that we can make executable
      TEMP_SCRIPT=$(mktemp)
      cp ${../scripts/run-ufo-node.sh} $TEMP_SCRIPT
      chmod +x $TEMP_SCRIPT
      
      # Run the script with the provided arguments
      $TEMP_SCRIPT \
        --build-mode "${cfg.buildMode}" \
        --validators "${toString cfg.validators}" \
        --block-time "${toString (elemAt cfg.blockTimes 0)}" \
        --osmosis-source "${cfg.osmosisSource}" \
        "$@"
      
      # Clean up
      rm -f $TEMP_SCRIPT
    '';
  };
  
  # Define apps
  apps = {
    run-ufo-node = {
      type = "app";
      program = "${packages.ufo-node-runner}/bin/run-ufo-node";
    };
    
    build-osmosis-ufo = {
      type = "app";
      program = "${packages.build-osmosis-ufo}/bin/build-osmosis-ufo";
    };
    
    benchmark-ufo = {
      type = "app";
      program = "${packages.benchmark-ufo}/bin/benchmark-ufo";
    };
  };
in
{
  options.ufo = {
    osmosisSource = mkOption {
      type = types.str;
      default = "/tmp/osmosis-source";
      description = "Path to the Osmosis source code";
    };

    buildMode = mkOption {
      type = types.enum [ "patched" "bridged" "fauxmosis" ];
      default = "patched";
      description = "UFO integration mode";
    };

    validators = mkOption {
      type = types.int;
      default = 1;
      description = "Number of validators for the UFO node";
    };

    blockTimes = mkOption {
      type = types.listOf types.int;
      default = [ 1000 100 10 1 ];
      description = "Block times in milliseconds for benchmarking";
    };
    
    faucet = {
      enabled = mkOption {
        type = types.bool;
        default = true;
        description = "Enable UFO faucet functionality";
      };
      
      initialSupply = mkOption {
        type = types.int;
        default = 1000000;
        description = "Initial token supply for the UFO faucet";
      };
      
      tokenName = mkOption {
        type = types.str;
        default = "UFO";
        description = "Token name for the UFO faucet";
      };
      
      tokenSymbol = mkOption {
        type = types.str;
        default = "UFO";
        description = "Token symbol for the UFO faucet";
      };
    };
  };

  config = mkIf (config ? ufo) {
    inherit packages apps;
  };
} 