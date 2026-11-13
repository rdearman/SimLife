# Evolutionary stability baseline

Run on the development Linux host with the optimized benchmark build:

```sh
cargo build --release -p simlife-bench
./target/release/simlife-bench stability 20261113 1 small 20 1
./target/release/simlife-bench stability 20261113 1 small 20 2
```

The benchmark started one asexual founder in a small (32 × 64) world, with fertility alleles set to 20. It advanced to days 1,000, 5,000, and 10,000. The engine continued reproducing and mutating autonomously. At every checkpoint the benchmark serialized a snapshot, restored it, and checked that the restored hash matched. The two version runs were performed in separate processes so peak resident memory is comparable.

## Results

| Mechanics | Day | Population | Births | Deaths | Total lineages | Extant | Extinct | Lineage extinction | Speciation events | Events / 1,000 days | Events / 1,000 births | Mean allele heterozygosity | Ticks / second | Peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| v1 | 1,000 | 2,531 | 18,262 | 15,732 | 1 | 1 | 0 | 0% | 0 | 0 | 0 | 0.146 | 2,842 | 15 MiB |
| v1 | 5,000 | 4,489 | 134,546 | 130,058 | 1 | 1 | 0 | 0% | 0 | 0 | 0 | 0.378 | 928 | 92 MiB |
| v1 | 10,000 | 4,539 | 278,700 | 274,162 | 1 | 1 | 0 | 0% | 0 | 0 | 0 | 0.459 | 686 | 180 MiB |
| v2 | 1,000 | 2,531 | 18,262 | 15,732 | 3 | 2 | 1 | 33.3% | 2 | 2.0 | 0.110 | 0.146 | 2,838 | 15 MiB |
| v2 | 5,000 | 4,489 | 134,546 | 130,058 | 174 | 26 | 148 | 85.1% | 173 | 34.6 | 1.286 | 948 | 102 MiB |
| v2 | 10,000 | 4,539 | 278,700 | 274,162 | 515 | 31 | 484 | 94.0% | 514 | 51.4 | 1.844 | 675 | 212 MiB |

Allele heterozygosity is the mean expected heterozygosity across the speed, metabolism, and fertility allele copies (0 means no variation, 1 means maximal even variation). It does not include diet or lifespan. Lineage extinction fraction is extinct historical lineages divided by all lineages founded to date. Performance includes simulation advancement plus snapshot creation and checkpoint restore/hash verification; it excludes HTTP, database writes, and browser rendering. Rates and memory are host-specific and the timings are a single deterministic seed, not capacity guarantees.

Both versions preserved the same population, birth/death, and allele-diversity trajectory for this all-asexual cohort. Version 2 classified 514 divergent lineages by day 10,000; 31 remained extant while 484 had gone extinct. That is substantial lineage turnover and warrants threshold/definition calibration, but the result does not establish runaway population growth or collapse in this seed: population settled near 4,500 after day 1,000 and remained there. This cohort cannot establish behaviour for sexual populations, other fertility levels, larger founder populations, or other seeds.

All six checkpoint restores matched their stored hashes. The PostgreSQL integration test separately replays accepted commands and restores checkpoints under both mechanics versions. It checks database recovery and replay over a shorter 40-day sequence, not a 10,000-day PostgreSQL replay.
