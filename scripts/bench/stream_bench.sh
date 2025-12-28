#!/bin/bash

# ================= 配置区域 =================
MODEL=${MODEL:-"qwen2.5-coder:7b"} 
BASE_URL=${BASE_URL:-"http://localhost:8080"}
ENDPOINT=${ENDPOINT:-"/v1/chat/completions"}
TIMEOUT_SEC=${TIMEOUT_SEC:-10}

# 矩阵测试的并发等级
CONCURRENCY_LEVELS=(1 2 4 8 16 32)
ROUNDS_PER_THREAD=3
# ===========================================

OUT_FILE=$(mktemp)
trap 'rm -f "$OUT_FILE"' EXIT

# --- 核心测试逻辑 (只取第一行 = TTFT) ---
CMD_SCRIPT='
    i="$1"
    base="$2"
    endpoint="$3"
    model="$4"
    timeout_sec="$5"
    out_file="$6"

    start_ts=$(python3 -c "import time; print(time.time())")

    # 重点：head -n 1 保证只测第一个包的时间
    response=$(timeout "$timeout_sec" curl -sS -N "${base}${endpoint}" \
        -H "Content-Type: application/json" \
        -d "{\"model\":\"${model}\",\"messages\":[{\"role\":\"user\",\"content\":\"hi benchmark ${i}\"}],\"stream\":true}" \
        2>/dev/null | head -n 1)

    end_ts=$(python3 -c "import time; print(time.time())")
    ttft_ms=$(python3 -c "print(int(($end_ts - $start_ts) * 1000))")

    if [[ "$response" == *"data:"* ]] || [[ "$response" == *"content"* ]]; then
        echo "$ttft_ms" >> "$out_file"
    else
        : 
    fi
'
export CMD_SCRIPT BASE_URL ENDPOINT MODEL TIMEOUT_SEC OUT_FILE

echo "================================================================================"
echo "🚀 [Matrix Bench] OpenAI LLM Gateway TTFT Test"
echo "   Model: $MODEL"
echo "================================================================================"

# 【修改点】表头明确标注 TTFT
printf "%-8s | %-10s | %-12s | %-12s | %-12s | %-15s\n" \
  "Workers" "Total Reqs" "Avg TTFT" "P50 TTFT" "P99 TTFT" "Stability"
printf "%-8s | %-10s | %-12s | %-12s | %-12s | %-15s\n" \
  "--------" "----------" "------------" "------------" "------------" "---------------"

for p in "${CONCURRENCY_LEVELS[@]}"; do
    total_reqs=$((p * ROUNDS_PER_THREAD))
    > "$OUT_FILE"
    sleep 1

    seq 1 "$total_reqs" | xargs -I{} -P "$p" bash -c "$CMD_SCRIPT" _ {} "$BASE_URL" "$ENDPOINT" "$MODEL" "$TIMEOUT_SEC" "$OUT_FILE"

    if [ -s "$OUT_FILE" ]; then
        stats=$(sort -n "$OUT_FILE" | awk '
        BEGIN { count=0; sum=0; }
        { a[count++] = $1; sum += $1; }
        END {
            if (count > 0) {
                p50 = a[int(count * 0.50)];
                p99 = a[int(count * 0.99)];
                avg = sum / count;
                printf "%.2f %d %d", avg, p50, p99;
            } else { printf "0 0 0"; }
        }')
        read avg p50 p99 <<< "$stats"
        
        status="✅ OK"
        if [ "$p99" -gt 1000 ]; then status="⚠️ High Latency"; fi
        if [ "$p99" -gt 5000 ]; then status="🔥 Overload"; fi
        if [ "$avg" == "0.00" ]; then status="❌ All Failed"; fi
    else
        avg="N/A"
        p50="N/A"
        p99="N/A"
        status="❌ No Data"
    fi

    printf "%-8s | %-10s | %-12s | %-12s | %-12s | %-15s\n" \
      "$p" "$total_reqs" "$avg" "$p50" "$p99" "$status"

done
echo "================================================================================"