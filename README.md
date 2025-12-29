# openai-llm-gateway

一个用 Rust 编写的 **LLM 推理网关（Inference Gateway）**：对外提供 OpenAI-compatible HTTP API（后续支持 SSE 流式），对内负责请求治理与调度（限流/并发控制/队列/路由等），将请求转发/分发到后端推理引擎（例如 vLLM/TGI 或自研后端）。

当前阶段目标：先完成最小可运行闭环（HTTP server + health check + 结构化日志 + 错误上下文），再逐步叠加网关能力。

## 实现概览

- 请求处理与上游转发由 `InferenceWorker` 负责（统一处理流式/非流式）
- HTTP 路由通过 channel 将请求发送给 worker，等待结果后返回

## 目标与范围（Scope）

网关关注点（计划逐步实现）：
- **统一接入**：OpenAI-compatible API（`/v1/chat/completions`）
- **治理**：Auth/Quota、Rate Limit、Concurrency Control、Timeout/Circuit Breaker
- **调度**：队列（backpressure）、continuous batching（进阶）
- **路由**：按 model/version/tenant 将请求分发到不同后端
- **可观测性**：结构化日志（tracing），后续加入 metrics/tracing

非目标（暂不做）：
- 不实现底层推理引擎（矩阵乘法/算子/kernel 优化等）
- 不绑定特定反向代理组件（本项目自身承担网关职责）

## 快速开始

### 环境要求

- Rust（建议使用 rustup 安装）
- `rustfmt`（用于格式化）

安装 `rustfmt`：
```bash
rustup component add rustfmt
```

启动：
```bash
cargo run
```

默认监听：`0.0.0.0:8080`
默认上游：`OLLAMA_BASE=http://localhost:11434`

验证（health check）：
```bash
curl -s http://127.0.0.1:8080/healthz; echo
```

期望输出：
```text
OK
```

## 已实现接口

- `GET /healthz`：健康检查，返回 `OK`（并记录一条 tracing 日志）
- `POST /v1/chat/completions`：OpenAI-compatible 请求体，转发到上游（由 `OLLAMA_BASE` 指定）

示例（非流式）：
```bash
curl -s http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"llama3.2","messages":[{"role":"user","content":"hi"}],"stream":false,"max_tokens":32}'
```

## 配置

- `OLLAMA_BASE`：上游推理服务地址，默认 `http://localhost:11434`

## Smoke 脚本

```bash
bash scripts/test/smoke.sh
```

可选环境变量：
- `BASE_URL`：网关地址（默认 `http://127.0.0.1:8080`）
- `OLLAMA_BASE`：上游地址（默认 `http://127.0.0.1:11434`）
- `MODEL`：请求模型名（默认 `qwen2.5-coder:7b`）

## 项目结构

```text
src/
  appstate.rs        # AppState（worker channel）
  inference/
    worker.rs        # 推理 worker：转发上游 + 处理 SSE/非流式
  main.rs            # 程序入口：初始化日志、启动 HTTP server、绑定端口
  routes/
    mod.rs           # routes 模块入口
    healthz.rs       # /healthz handler
    v1/
      mod.rs         # /v1 路由聚合
      chat/
        mod.rs       # /v1/chat 路由聚合
        completions.rs
  types.rs           # 请求/错误类型
scripts/
  bench/
    no_stream_bench.sh  # 非流式压测
    stream_bench.sh     # 流式压测
  test/
    smoke.sh            # 冒烟测试脚本
```


## Roadmap

- [x] `POST /v1/chat/completions`：转发到上游（最小可用）
- [x] SSE streaming：支持 token 流式返回
- [ ] Queue + Backpressure：排队/拒绝策略（429/503）
- [ ] Concurrency Control：并发上限（Semaphore）
- [ ] Backend Adapter：接入 vLLM/TGI（HTTP 或 gRPC）
- [ ] Observability：metrics + trace（Prometheus-compatible / OpenTelemetry-compatible）
- [ ] Scheduler：continuous batching（进阶）

## Benchmarks

环境与配置：
- 环境：4070 TiS / i5-14600KF / 32GB / WSL
- 模型：`qwen2.5-coder:7b`
- Prompt：`"hi"`
- `max_tokens`：`32`
- Endpoint：`POST /v1/chat/completions`（non-stream）

结果（non-stream，`oha`）：

```text
Workers  | RPS          | Avg(ms)      | P95(ms)      | P99(ms)      | Status
-------- | ------------ | ------------ | ------------ | ------------ | ---------------
1        | 4.43         | 225.48       | 360.98       | 404.06       | ✅ OK
4        | 5.33         | 717.00       | 970.30       | 1180.70      | ✅ OK
8        | 4.75         | 1544.70      | 1964.30      | 2057.20      | ✅ OK
16       | 4.97         | 2677.40      | 3398.80      | 3544.50      | ✅ OK
```

结果（stream，TTFT）：

```text
Workers  | Total Reqs | Avg TTFT     | P50 TTFT     | P99 TTFT     | Stability
-------- | ---------- | ------------ | ------------ | ------------ | ---------------
1        | 3          | 124.00       | 78           | 218          | ✅ OK
2        | 6          | 102.00       | 103          | 133          | ✅ OK
4        | 12         | 211.25       | 234          | 268          | ✅ OK
8        | 24         | 432.29       | 477          | 550          | ✅ OK
16       | 48         | 361.00       | 642          | 853          | ✅ OK
32       | 96         | 1515.02      | 1780         | 1841         | ⚠️ High Latency
```

压测脚本：
```bash
bash scripts/bench/no_stream_bench.sh
bash scripts/bench/stream_bench.sh
```

## 开发约定（Conventions）

- 启动阶段错误使用 `anyhow` + `Context/with_context` 提供可诊断错误信息
- 业务请求链路避免 `unwrap()`，后续统一映射为 HTTP status（4xx/5xx）
- 使用 `cargo fmt` 保持统一格式
