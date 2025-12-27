# openai-llm-gateway

一个用 Rust 编写的 **LLM 推理网关（Inference Gateway）**：对外提供 OpenAI-compatible HTTP API（后续支持 SSE 流式），对内负责请求治理与调度（限流/并发控制/队列/路由等），将请求转发/分发到后端推理引擎（例如 vLLM/TGI 或自研后端）。

当前阶段目标：先完成最小可运行闭环（HTTP server + health check + 结构化日志 + 错误上下文），再逐步叠加网关能力。

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

## 项目结构

```text
src/
  main.rs            # 程序入口：初始化日志、启动 HTTP server、绑定端口
  routes/
    mod.rs           # routes 模块入口
    healthz.rs       # /healthz handler
```


## Roadmap

- [ ] `POST /v1/chat/completions`：返回 mock JSON（先跑通协议形态）
- [ ] SSE streaming：支持 token 流式返回
- [ ] Queue + Backpressure：排队/拒绝策略（429/503）
- [ ] Concurrency Control：并发上限（Semaphore）
- [ ] Backend Adapter：接入 vLLM/TGI（HTTP 或 gRPC）
- [ ] Observability：metrics + trace（Prometheus-compatible / OpenTelemetry-compatible）
- [ ] Scheduler：continuous batching（进阶）

## 开发约定（Conventions）

- 启动阶段错误使用 `anyhow` + `Context/with_context` 提供可诊断错误信息
- 业务请求链路避免 `unwrap()`，后续统一映射为 HTTP status（4xx/5xx）
- 使用 `cargo fmt` 保持统一格式




