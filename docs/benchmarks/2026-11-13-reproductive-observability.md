# Reproductive and lineage tracking benchmark — 2026-11-13

## Setup

- Release `simlife-bench`, Linux x86_64, one process per run.
- Seed `20261113`, medium 64×128 map, 1,000 asexual herbivore founders, 100 simulated days.
- Reproductive mode uses fertility alleles `[100, 100]`; sterile mode uses `[0, 0]`.
- Throughput includes deterministic state hashing, updates, per-organism ancestry records, and sampled allele-frequency history. It excludes PostgreSQL, HTTP, and browser transport.
- One run per mode; timings are a development baseline, not a capacity claim.

## Results

| Mode | Final population | Births | Deaths | Ticks/second | Peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reproductive | 13,379 | 22,812 | 10,433 | 62.01 | 24,288 KiB |
| Sterile | 1,000 | 0 | 0 | 361.04 | 4,124 KiB |

The reproductive population grew by 1,238% net over this run, with substantial turnover. A fertile run costs more because each birth adds permanent lineage state and expands the state serialized for hashing. This short result demonstrates active reproduction and mutation costs; it does not establish a stable carrying capacity. Repeat across seeds, founder mixes, durations, and larger populations before setting operating limits.

## Reproduction

```sh
cargo build -p simlife-bench --release
./target/release/simlife-bench 1000 100 20261113 medium reproductive
./target/release/simlife-bench 1000 100 20261113 medium sterile
```
