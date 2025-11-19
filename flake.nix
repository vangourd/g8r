{
  description = "G8R - Infrastructure automation platform";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        dbSetupScript = pkgs.writeShellScriptBin "g8r-db-setup" ''
          set -e
          
          CONTAINER_NAME="g8r-postgres"
          DB_USER="g8r"
          DB_PASSWORD="g8r_dev_password"
          DB_NAME="g8r_state"
          
          if ${pkgs.podman}/bin/podman ps -a --format '{{.Names}}' | grep -q "^$CONTAINER_NAME$"; then
            echo "PostgreSQL container already exists"
            if ! ${pkgs.podman}/bin/podman ps --format '{{.Names}}' | grep -q "^$CONTAINER_NAME$"; then
              echo "Starting existing container..."
              ${pkgs.podman}/bin/podman start $CONTAINER_NAME
            else
              echo "Container already running"
            fi
          else
            echo "Creating PostgreSQL container..."
            ${pkgs.podman}/bin/podman run -d \
              --name $CONTAINER_NAME \
              -e POSTGRES_USER=$DB_USER \
              -e POSTGRES_PASSWORD=$DB_PASSWORD \
              -e POSTGRES_DB=$DB_NAME \
              -p 5432:5432 \
              -v g8r-postgres-data:/var/lib/postgresql/data \
              postgres:16-alpine
            
            echo "Waiting for PostgreSQL to be ready..."
            sleep 3
          fi
          
          echo "PostgreSQL is ready at postgresql://$DB_USER:$DB_PASSWORD@localhost:5432/$DB_NAME"
        '';
        
        dbStopScript = pkgs.writeShellScriptBin "g8r-db-stop" ''
          ${pkgs.podman}/bin/podman stop g8r-postgres || echo "Container not running"
        '';
        
        serverScript = pkgs.writeShellScriptBin "g8r-server" ''
          set -e
          
          if [ -f .env ]; then
            export $(cat .env | grep -v '^#' | xargs)
          fi
          
          ${pkgs.cargo}/bin/cargo run -- serve --host 0.0.0.0 --port 8080
        '';
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer

            # Build dependencies
            pkg-config
            openssl

            # Container tools
            podman

            # Database tools
            postgresql

            # AWS CLI (for testing)
            awscli2

            # Additional tools
            git
            jq
            nushell
            
            # Nickel configuration language
            nickel
            
            # G8R scripts
            dbSetupScript
            dbStopScript
            serverScript
          ];

          shellHook = ''
            echo "G8R development environment loaded"
            echo ""
            echo "Available commands:"
            echo "  g8r-db-setup  - Start PostgreSQL container"
            echo "  g8r-db-stop   - Stop PostgreSQL container"
            echo "  g8r-server    - Run G8R API server (loads .env)"
            echo ""
            echo "Versions:"
            echo "  Podman: $(podman --version)"
            echo "  Rust: $(rustc --version)"
            echo ""
            
            if [ -f .env ]; then
              export $(cat .env | grep -v '^#' | xargs)
              echo "Loaded environment from .env"
            else
              export DATABASE_URL="postgresql://g8r:g8r_dev_password@localhost:5432/g8r_state"
              export RUST_LOG=debug
              echo "Warning: .env file not found, using default DATABASE_URL"
            fi
          '';
        };
      }
    );
}
