# Benchmark Snapshot

Date: 2026-03-26

Environment:

- CPU: AMD Ryzen 9 7950X 16-Core Processor
- OS: Microsoft Windows NT 10.0.26200.0
- `rustc`: 1.94.0 (4a4ef493e 2026-03-02)
- run mode: `cargo run --release --example runtime_bench`

Methodology:

- single-process local microbenchmark
- release mode
- no network I/O
- deterministic in-memory models and tools only
- benchmark measures orchestration overhead, not real LLM latency

Observed mean range across 3 release runs:

| Scenario | Mean range | Median range | p95 range |
| --- | ---: | ---: | ---: |
| `flow_run` | 368-461 ns | 300-400 ns | 500 ns |
| `crew_kickoff_sequential` | 5.7-6.4 us | 5.4-6.0 us | 6.5-6.9 us |
| `crew_kickoff_hierarchical` | 7.1-8.0 us | 6.9-7.7 us | 7.4-8.4 us |
| `blueprint_parse_and_build` | 20.1-22.4 us | 18.8-21.1 us | 24.1-26.7 us |

Derived signal:

- manager layer overhead versus sequential kickoff: `+23.8%` to `+25.4%`

Raw runs:

| Run | `flow_run` mean | `crew_kickoff_sequential` mean | `crew_kickoff_hierarchical` mean | `blueprint_parse_and_build` mean | Manager overhead |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 390 ns | 6.2 us | 7.7 us | 20.1 us | +23.8% |
| 2 | 461 ns | 6.4 us | 8.0 us | 22.4 us | +25.4% |
| 3 | 368 ns | 5.7 us | 7.1 us | 20.8 us | +24.7% |
