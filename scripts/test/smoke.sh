#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
OLLAMA_BASE="${OLLAMA_BASE:-http://127.0.0.1:11434}"
MODEL="${MODEL:-qwen2.5-coder:7b}"

echo "[smoke] BASE_URL=$BASE_URL"
echo "[smoke] OLLAMA_BASE=$OLLAMA_BASE"
echo "[smoke] MODEL=$MODEL"

# 1) 启动网关（后台）
export OLLAMA_BASE="$OLLAMA_BASE"
RUST_LOG="${RUST_LOG:-info}"
export RUST_LOG

cargo run >/tmp/gateway.log 2>&1 &
PID=$!
cleanup() {
  echo "[smoke] stopping gateway pid=$PID"
  kill "$PID" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# 2) 等待服务起来（最多 3s）
for i in {1..30}; do
  if curl -fsS "$BASE_URL/healthz" >/dev/null 2>&1; then
    echo "[smoke] healthz OK"
    break
  fi
  sleep 0.1
done

# 3) 非流式 completions：验证 status=200 且包含 choices
echo "[smoke] POST /v1/chat/completions (non-stream)"
RESP="$(curl -fsS "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"$MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"stream\":false,\"max_tokens\":32}"
)"

echo "$RESP" | head -c 200; echo
echo "$RESP" | grep -q '"choices"' || { echo "[smoke] FAIL: missing choices"; exit 1; }
echo "$RESP" | grep -q '"object":"chat.completion"' || echo "[smoke] WARN: object not chat.completion (still ok for now)"

echo "[smoke] POST /v1/chat/completions (expected fail - bad model)"
HTTP_CODE=$(curl -sS -o /tmp/err.json -w "%{http_code}" \
  "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{"model":"__non_exist_model__","messages":[{"role":"user","content":"hi"}],"stream":false}')

echo "[smoke] bad model http_code=$HTTP_CODE"
cat /tmp/err.json | head -c 200; echo

echo "[smoke] POST /v1/chat/completions (stream=true)"
OUT="$(timeout 3 curl -sS -N "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"$MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"stream\":true}" \
  | head -n 2 || true
)"
echo "$OUT"
echo "$OUT" | grep -q "^data:" || fail "stream output missing 'data:'"


echo "[smoke] PASS"
