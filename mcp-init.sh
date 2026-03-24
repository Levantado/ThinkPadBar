#!/bin/bash

# mcp-init: Автоматическая настройка MCP Memory для текущего проекта
# Использование: mcp-init [agent_id]

PROJECT_ID=$(basename "$PWD")
AGENT_ID=${1:-$USER}
SERVER_URL="http://127.0.0.1:3000"

# Цвета для вывода
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}Initializing MCP Memory for project: ${GREEN}$PROJECT_ID${NC}"
echo -e "${BLUE}Agent ID: ${GREEN}$AGENT_ID${NC}"

# Проверка API ключа
AUTH_HEADER=""
if [ ! -z "$MCP_API_KEY" ]; then
    echo -e "${GREEN}✔ Found MCP_API_KEY in environment${NC}"
    AUTH_HEADER="\"headers\": { \"Authorization\": \"Bearer $MCP_API_KEY\" },"
else
    echo -e "${YELLOW}⚠ No MCP_API_KEY found. Using unprotected connection.${NC}"
fi

# 1. Настройка Gemini CLI (.gemini/settings.json)
mkdir -p .gemini
cat <<EOF > .gemini/settings.json
{
  "mcpServers": {
    "mcp-memory-shared": {
      "url": "$SERVER_URL/mcp/projects/$PROJECT_ID/shared",
      $AUTH_HEADER
      "trust": true
    },
    "mcp-memory-private": {
      "url": "$SERVER_URL/mcp/projects/$PROJECT_ID/agents/$AGENT_ID",
      $AUTH_HEADER
      "trust": true
    }
  }
}
EOF
echo -e "${GREEN}✔ Created .gemini/settings.json${NC}"

# 2. Настройка Codex (.codex/config.toml)
mkdir -p .codex
cat <<EOF > .codex/config.toml
[mcp_servers.mcp-memory-project]
url = "$SERVER_URL/mcp/projects/$PROJECT_ID/shared"
EOF

if [ ! -z "$MCP_API_KEY" ]; then
cat <<EOF >> .codex/config.toml
http_headers = {Authorization = "Bearer $MCP_API_KEY"}
EOF
fi

cat <<EOF >> .codex/config.toml

[mcp_servers.mcp-memory-agent]
url = "$SERVER_URL/mcp/projects/$PROJECT_ID/agents/$AGENT_ID"
EOF

if [ ! -z "$MCP_API_KEY" ]; then
cat <<EOF >> .codex/config.toml
http_headers = {Authorization = "Bearer $MCP_API_KEY"}
EOF
fi
echo -e "${GREEN}✔ Created .codex/config.toml${NC}"

# 3. Создание инструкций для агента
mkdir -p agent
cat <<EOF > agent/GEMINI.md
# Agent Context for $PROJECT_ID

You are working on the project **$PROJECT_ID**.
Shared memory: $SERVER_URL/mcp/projects/$PROJECT_ID/shared
Private memory: $SERVER_URL/mcp/projects/$PROJECT_ID/agents/$AGENT_ID

Please follow the guidelines in docs/AGENT_GUIDELINES.md if available.
EOF
echo -e "${GREEN}✔ Created agent/GEMINI.md${NC}"

echo -e "\n${GREEN}🚀 Project '$PROJECT_ID' is ready for AI agents!${NC}"
echo -e "Restart your agent or run '/mcp reload' to see the changes."
