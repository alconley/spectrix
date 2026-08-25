# Performance report

Generated 2026-08-24 with the pinned lmfit 1.3.4, NumPy 2.5.2, and SciPy 1.18.1 parity environment. Times are warmed end-to-end microseconds per fit in release mode. Each case includes preprocessing, frozen-linear-background prefit, solve, scaled covariance, component curves, and complete total/component confidence-band payload construction on the 50× evaluation grid.

| Peaks | Bins | Native Rust (µs) | lmfit (µs) | Speedup |
|---:|---:|---:|---:|---:|
| 1 | 512 | 7,081 | 26,165 | 3.70× |
| 1 | 2,048 | 26,939 | 65,592 | 2.43× |
| 1 | 8,192 | 96,897 | 317,142 | 3.27× |
| 3 | 512 | 22,081 | 121,589 | 5.51× |
| 3 | 2,048 | 92,192 | 336,478 | 3.65× |
| 3 | 8,192 | 341,206 | 2,237,809 | 6.56× |
| 8 | 512 | 106,628 | 992,946 | 9.31× |
| 8 | 2,048 | 407,036 | 2,473,371 | 6.08× |
| 8 | 8,192 | 1,420,238 | 19,856,029 | 13.98× |

Geometric-mean speedup: **5.26×**. No representative case was slower than lmfit, so both release gates passed.

The matrix uses the supported fixed-center workflow so all nine cases have usable covariance in both backends. Free-center behavior, active bounds, and unavailable-covariance classification are compatibility-test concerns rather than performance-case variables. Re-run `crates/spectrix-fitting/benches/compare.ps1`; the script fails unless every native case is faster and the geometric mean is at least 5×.
