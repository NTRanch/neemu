# neemu — benchmarks

Measured 2026-11-27 on primo (13th Gen Intel Core i7-13620H, 16 threads,
62 GiB RAM, cargo 1.100.0). Primo was under multi-lane load that night
(load average in the tens), so these are baselines, not peaks.

## Workloads (`benches/hot_paths.rs`, hand-rolled on `std::time`)

Sessions run against a throwaway registry home (`--home <scratch>`)
with `--profile full`, the same launch the repo's wire tests use.

- `cold_start_discover`: spawn the server, send `server/discover`, await
  the response; 10 spawns.
- `tools_list`: `tools/list` over one warm stdio session; 20 warmup +
  200 measured requests.
- `tool_call_neemu_list_records`: `tools/call neemu_list_records
  {"work_id": "work_nope"}` over the same warm session; 20 warmup
  (mixed with tools/list) + 200 measured requests. An unknown work is
  answered with the typed gate refusal (isError=true,
  kind=screen_refusal), so this measures the read-only refusal path —
  the cheapest representative call against an empty registry.

## Results

| Workload | µs/op | ops/s |
|---|---|---|
| cold_start_discover | 8,409.0 | 119 |
| tools_list | 1,642.4 | 609 |
| tool_call_neemu_list_records | 203.7 | 4,909 |

## Example

`examples/stdio_session.rs` drives one modern-era session (discover →
tools/list → `neemu_list_records` for an unknown work): server neemu
1.0.1, 15 tools, and the typed `screen_refusal` gate refusal as the
expected, honest output.

## Gates (2026-11-27)

- `cargo build --examples` clean; example run green.
- `cargo bench --bench hot_paths` ran to the numbers above.
- `cargo test`: 157 passed / 0 failed.
- `cargo clippy` on the added files: clean. rustfmt applied to the
  added files only.
