# Cross platform shebang:
shebang := if os() == 'windows' {
    'powershell.exe'
} else if os() == 'macos' {
    '/usr/bin/env zsh'
} else {
    '/usr/bin/env bash'
}

cert-path := if os() == 'windows' {
    'D:\Engineer\web3\rizilab\bismillahdao\root.crt'
} else {
    '/usr/local/share/ca-certificates/root.crt'
}

# Cross platform command selection
install_cmd := if os() == 'windows' {
    'certutil.exe -addstore -f "Root"'
} else if os() == 'macos' {
    'sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain'
} else {
    'sudo update-ca-certificates'
}

# Set shell for non-Windows OSs:
set shell := ["sh", "-c"]

# Set shell for Windows OSs:
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

git *command:
  @git {{command}}

backend:
    just frontend/prod
    just backend/dev

infra-setup:
  docker network create bismillahdao_net

rebuild-dev *container:
  docker compose --env-file docker/.env.dev -f docker/docker-compose.dev.yaml up --build {{container}}

up-dev:
  docker compose --env-file docker/.env.dev -f docker/docker-compose.dev.yaml up --watch

down-dev *container:
  docker compose -f docker/docker-compose.dev.yaml down {{container}}

fix-encoding *file:
  sudo iconv -f us-ascii -t utf-8 {{file}} > {{file}}_fix.key && \
  sudo rm {{file}} && \
  sudo mv {{file}}_fix.key {{file}}
  sudo chmod 400 {{file}}

remove-volumes *volume:
  docker volume rm {{volume}}
caddy-reload:
  docker compose -f docker/docker-compose.dev.yaml exec -w /etc/caddy caddy sh -c "caddy fmt --overwrite && caddy reload"
up-prod:
  docker compose -f docker-compose.prod.yaml up -d

down-prod:
  docker compose -f docker-compose.prod.yaml down

install-cert:
    #!{{shebang}}
    echo "Installing cert to {{cert-path}}"
    docker compose cp "rizilab-bismillahdao-caddy:/data/caddy/pki/authorities/local/root.crt" "{{cert-path}}"
    {{install_cmd}} {{cert-path}}

logs container:
  docker compose -f docker/docker-compose.dev.yaml logs --follow {{container}}

shell container:
  docker compose -f docker/docker-compose.dev.yaml exec {{container}} sh
