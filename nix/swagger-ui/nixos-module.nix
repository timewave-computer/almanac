# NixOS module for running Almanac Swagger UI as a system service
{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.almanac-swagger-ui;
in {
  options.services.almanac-swagger-ui = {
    enable = mkEnableOption "Almanac Swagger UI service";
    
    port = mkOption {
      type = types.port;
      default = 8080;
      description = "Port number for the Swagger UI server";
    };
    
    host = mkOption {
      type = types.str;
      default = "0.0.0.0";
      description = "Host address to bind to (0.0.0.0 for all interfaces)";
    };
    
    configFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Path to the Almanac configuration file";
    };
    
    openFirewall = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to open the firewall port";
    };
    
    environmentFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Path to environment file containing additional variables";
    };
  };
  
  config = mkIf cfg.enable {
    # Open firewall port if requested
    networking.firewall.allowedTCPPorts = mkIf cfg.openFirewall [ cfg.port ];
    
    # Create systemd service
    systemd.services.almanac-swagger-ui = {
      description = "Almanac Swagger UI Service";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      
      environment = {
        API_HOST = cfg.host;
        API_PORT = toString cfg.port;
        RUST_LOG = "info";
        # Set HOME to a writable directory for cargo
        HOME = "/var/lib/almanac-swagger-ui";
      };
      
      serviceConfig = {
        Type = "simple";
        ExecStart = let
          almanacFlake = builtins.fetchGit {
            url = "https://github.com/your-org/almanac.git"; # Update with your repo
            ref = "main";
          };
          swaggerScript = "${almanacFlake}#swagger-ui";
          configArg = if cfg.configFile != null then cfg.configFile else "";
        in "${pkgs.nix}/bin/nix run ${swaggerScript} -- ${toString cfg.port} ${cfg.host} ${configArg}";
        
        Restart = "always";
        RestartSec = "10s";
        
        # Security hardening
        DynamicUser = true;
        StateDirectory = "almanac-swagger-ui";
        RuntimeDirectory = "almanac-swagger-ui";
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        NoNewPrivileges = true;
        
        # Load environment file if specified
        EnvironmentFile = mkIf (cfg.environmentFile != null) cfg.environmentFile;
      };
    };
    
    # Create convenience alias
    environment.systemPackages = [ 
      (pkgs.writeShellScriptBin "almanac-swagger-status" ''
        systemctl status almanac-swagger-ui
        echo ""
        echo "Swagger UI URLs:"
        echo "  Local:    http://localhost:${toString cfg.port}/swagger-ui"
        echo "  External: http://$(hostname -I | awk '{print $1}'):${toString cfg.port}/swagger-ui"
      '')
    ];
  };
}