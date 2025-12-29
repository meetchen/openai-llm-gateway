#!/usr/bin/env bash

# ================= 配置区域 =================
BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
ENDPOINT="${ENDPOINT:-/v1/chat/completions}"
MODEL="${MODEL:-qwen2.5-coder:7b}"
MAX_TOKENS="${MAX_TOKENS:-32}"
N="${N:-50}"
C_LIST="${C_LIST:-1 4 8 16}"
OUT_DIR="${OUT_DIR:-bench/out}"
# ===========================================

mkdir -p "$OUT_DIR"
if ! command -v oha >/dev/null 2>&1; then echo "❌ Error: 'oha' missing"; exit 1; fi

TS="$(date +%Y%m%d_%H%M%S)"
REQ_FILE="$OUT_DIR/req_${TS}.json"

# 1. 准备 Request Body
cat > "$REQ_FILE" <<EOF
{"model": "$MODEL", "messages": [{"role": "user", "content": "hi"}], "stream": false, "max_tokens": $MAX_TOKENS}
EOF
# 读取到变量，防止 oha 读取文件失败
PAYLOAD=$(cat "$REQ_FILE")

echo "================================================================================"
echo "🚀 [OHA Matrix] Universal Unit Support (us/ms/secs)"
echo "   Model:  $MODEL"
echo "================================================================================"

printf "%-8s | %-12s | %-12s | %-12s | %-12s | %-15s\n" "Workers" "RPS" "Avg(ms)" "P95(ms)" "P99(ms)" "Status"
printf "%-8s | %-12s | %-12s | %-12s | %-12s | %-15s\n" "--------" "------------" "------------" "------------" "------------" "---------------"

for C in $C_LIST; do
  OUT="$OUT_DIR/oha_${TS}_c${C}.txt"
  
  # 2. 运行 oha (回到文本模式 --no-tui)
  oha -n "$N" -c "$C" --no-tui \
    -m POST -H "Content-Type: application/json" -d "$PAYLOAD" \
    "${BASE_URL}${ENDPOINT}" > "$OUT" 2>&1

  if grep -q "Requests/sec" "$OUT"; then
      # 提取 RPS
      rps=$(grep "Requests/sec:" "$OUT" | awk '{printf "%.2f", $2}')
      
      # === 核心修复：全能单位换算函数 ===
      # 支持 us (微秒), ms (毫秒), secs (秒) 统一转为 ms
        extract_time() {
            key="$1"
            line=$(grep -m1 -E "$key" "$OUT" || true)
            if [[ $line =~ ([0-9.]+)[[:space:]]*(us|ms|sec|secs|s) ]]; then
                val="${BASH_REMATCH[1]}"
                unit="${BASH_REMATCH[2]}"
                if [[ "$unit" == "us" ]]; then
                    printf "%.2f" "$(awk "BEGIN {print $val/1000}")"
                elif [[ "$unit" == "ms" ]]; then
                    printf "%.2f" "$val"
                elif [[ "$unit" == "sec" || "$unit" == "secs" || "$unit" == "s" ]]; then
                    printf "%.2f" "$(awk "BEGIN {print $val*1000}")"
                else
                    printf "0"
                fi
            else
                printf "0"
            fi
        }

      avg_ms=$(extract_time "Average:")
      p95_ms=$(extract_time "95([.]0+)?% in")
      p99_ms=$(extract_time "99([.]0+)?% in")

      # 状态检查
      if grep -q "\[200\]" "$OUT"; then 
        status="✅ OK"; 
      elif grep -q "\[400\]" "$OUT"; then
        status="❌ 400 Bad Req";
      else
        status="❌ Error"; 
      fi
      
      printf "%-8s | %-12s | %-12s | %-12s | %-12s | %-15s\n" "$C" "$rps" "$avg_ms" "$p95_ms" "$p99_ms" "$status"
  else
      # 失败时打印日志路径，方便排查
      printf "%-8s | %-12s | %-12s | %-12s | %-12s | %-15s\n" "$C" "-" "-" "-" "-" "❌ Crash"
      # echo "Debug: cat $OUT"
  fi
  sleep 1
done
echo "================================================================================"
