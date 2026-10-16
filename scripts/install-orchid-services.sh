#!/bin/sh
# Installs independent user services. Provider setup and credentials stay on the host.
set -eu
if [ "$#" -ne 3 ]; then
  echo "Usage: $0 ABSOLUTE_BIN_DIRECTORY SERVER_LISTEN_ADDRESS SERVER_URL" >&2
  exit 2
fi
case "$1" in /*) ;; *) echo 'Binary directory must be absolute' >&2; exit 2;; esac
case "$1$2$3" in *[!a-zA-Z0-9/_+.:\[\]-]*) echo 'Use paths and addresses without whitespace or systemd expansion characters' >&2; exit 2;; esac
config="$HOME/.config/orchid"
units="$HOME/.config/systemd/user"
mkdir -p "$config" "$units" "$HOME/.local/share/orchid/server" "$HOME/.local/share/orchid/host"
if [ ! -f "$config/services.env" ]; then
  echo "Create $config/services.env containing ORCHID_SERVICE_TOKEN=<shared-token> with mode 600 first." >&2
  exit 2
fi
for role in server host; do
  if [ "$role" = server ]; then
    options="--listen $2"
  else
    options="--listen 127.0.0.1:4481 --server $3"
  fi
  cat > "$units/orchid-$role.service" <<EOF
[Unit]
Description=Orchid $role
After=network.target
[Service]
EnvironmentFile=$config/services.env
Environment="PATH=%h/.local/bin:%h/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
ExecStart=$1/orchid-$role --data-dir %h/.local/share/orchid/$role $options
Restart=on-failure
RestartSec=3
TimeoutStopSec=30
[Install]
WantedBy=default.target
EOF
done
printf '\n[Service]\nEnvironmentFile=-%s/host.env\n' "$config" >> "$units/orchid-host.service"
systemctl --user daemon-reload
systemctl --user enable --now orchid-server orchid-host
echo 'Both services started. Enable user lingering if they must survive logout.'
