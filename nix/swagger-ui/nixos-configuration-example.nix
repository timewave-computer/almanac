# Example NixOS configuration for running Almanac Swagger UI
# Add this to your /etc/nixos/configuration.nix or create a separate module

{ config, pkgs, ... }:

{
  imports = [
    # Import the Almanac Swagger UI module
    ./almanac-swagger-ui-module.nix
  ];
  
  # Enable and configure Almanac Swagger UI service
  services.almanac-swagger-ui = {
    enable = true;
    port = 8080;                    # Default port
    host = "0.0.0.0";              # Bind to all interfaces for external access
    openFirewall = true;           # Automatically open firewall port
    
    # Optional: specify a custom config file
    # configFile = "/etc/almanac/config.toml";
    
    # Optional: specify an environment file for secrets
    # environmentFile = "/etc/almanac/swagger-ui.env";
  };
  
  # Optional: Configure nginx as a reverse proxy for HTTPS access
  services.nginx = {
    enable = true;
    virtualHosts."swagger.your-domain.com" = {
      forceSSL = true;
      enableACME = true;
      locations."/" = {
        proxyPass = "http://127.0.0.1:8080";
        proxyWebsockets = true;
        extraConfig = ''
          proxy_set_header Host $host;
          proxy_set_header X-Real-IP $remote_addr;
          proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
          proxy_set_header X-Forwarded-Proto $scheme;
        '';
      };
    };
  };
  
  # Optional: Basic authentication for nginx (if using reverse proxy)
  # services.nginx.virtualHosts."swagger.your-domain.com".basicAuth = {
  #   user = "password_hash_here";  # Use htpasswd to generate
  # };
  
  # Ensure the Almanac project is available
  environment.systemPackages = with pkgs; [
    git
    cargo
    rustc
  ];
}