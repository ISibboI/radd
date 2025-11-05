{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      perSystem =
        {
          config,
          self',
          pkgs,
          lib,
          system,
          ...
        }:
        let
          runtimeDeps = with pkgs; [ ];
          buildDeps = with pkgs; [
            pkg-config
            rustPlatform.bindgenHook
            cmake
          ];
          devDeps = with pkgs; [ ];

          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          msrv = cargoToml.package.rust-version;

          rustPackage =
            features:
            (pkgs.makeRustPlatform {
              cargo = pkgs.rust-bin.stable.latest.minimal;
              rustc = pkgs.rust-bin.stable.latest.minimal;
            }).buildRustPackage
              {
                inherit (cargoToml.package) name version;
                src = ./.;
                cargoLock.lockFile = ./Cargo.lock;
                buildFeatures = features;
                buildInputs = runtimeDeps;
                nativeBuildInputs = buildDeps;
                # Uncomment if your cargo tests require networking or otherwise
                # don't play nicely with the Nix build sandbox:
                # doCheck = false;
              };

          mkDevShell =
            rustc:
            pkgs.mkShell {
              shellHook = ''
                export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
              '';
              buildInputs = runtimeDeps;
              nativeBuildInputs = buildDeps ++ devDeps ++ [ rustc ];
            };
        in
        {
          formatter = pkgs.nixfmt-tree;
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ (import inputs.rust-overlay) ];
          };

          packages.default = self'.packages.radd;
          devShells.default = self'.devShells.stable;

          packages.radd = (rustPackage "");

          devShells.stable = (mkDevShell pkgs.rust-bin.stable.latest.default);
          devShells.msrv = (mkDevShell pkgs.rust-bin.stable.${msrv}.default);
        };

      flake.nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.services.radd;
          types = lib.types;
        in
        {
          options.services.radd = {
            enable = lib.mkEnableOption "Enable the RuuviTag Additions service.";

            package = lib.mkOption {
              type = types.package;
              default = inputs.self.packages.${pkgs.system}.radd;
              description = "The radd package to use.";
            };

            logLevel = lib.mkOption {
              type = types.str;
              default = "Info";
              description = "The log level, one of: Trace, Debug, Info, Warn, Error.";
            };

            mqttBrokerUrl = lib.mkOption {
              type = types.str;
              default = "tcp://localhost:1883";
              description = "The url pointing to the MQTT broker, as required by paho-mqtt.";
            };

            mqttUsername = lib.mkOption {
              type = types.str;
              default = "radd";
              description = "The username to log into the MQTT broker.";
            };

            mqttPasswordFile = lib.mkOption {
              type = types.str;
              description = "The file containing the password to log into the MQTT broker. Whitespace around the password in the file is removed.";
            };

            mqttListenTopic = lib.mkOption {
              type = types.str;
              default = "home/TheengsGateway/BTtoMQTT/#";
              description = "The topic that radd should listen for RuuviTag messages in.";
            };

            mqttHomeAssistantDiscoveryTopic = lib.mkOption {
              type = types.str;
              default = "homeassistant/";
              description = "The topic that radd should announce additional RuuviTag measures in.";
            };
          };

          config = lib.mkIf cfg.enable {
            systemd.services.radd = {
              description = "The RuuviTag Additions service.";
              wantedBy = [ "multi-user.target" ];

              serviceConfig = {
                ExecStart = ''
                  "bash -c '
                    LOG_LEVEL=\"${cfg.logLevel}\"
                    MQTT_BROKER_URL=\"${cfg.mqttBrokerUrl}\"
                    MQTT_USERNAME=\"${cfg.mqttUsername}\"
                    MQTT_PASSWORD=`cat \"${cfg.mqttPasswordFile}\" | xargs echo` # Remove whitespace before and after password
                    MQTT_LISTEN_TOPIC=\"${cfg.mqttListenTopic}\"
                    MQTT_HASS_DISCOVERY_TOPIC=\"${cfg.mqttHomeAssistantDiscoveryTopic}\"
                    ${cfg.package}/bin/radd
                  '"
                '';
                Restart = "on-failure";
                Type = "exec";
              };
            };
          };
        };
    };
}
