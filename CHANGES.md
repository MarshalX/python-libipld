# Change Log

## Version 3.5.0

**12.10.2026**

**🐍 Python 3.15 and free-threaded Python 3.14t / 3.15t wheels**

**⚡ Up to 2x faster DAG-CBOR encode, up to 10% faster decode**

| Encode         |  v3.4.1 |  v3.5.0 |           Speedup |
|----------------|--------:|--------:|------------------:|
| `citm_catalog` | 1.04 ms | 0.52 ms | **2.01x** (+101%) |
| `github`       | 0.05 ms | 0.03 ms |  **1.78x** (+78%) |
| `twitter`      | 0.47 ms | 0.27 ms |  **1.74x** (+74%) |
| `canada`       | 0.83 ms | 0.64 ms |  **1.29x** (+29%) |

| Decode             |  v3.4.1 |  v3.5.0 |          Speedup |
|--------------------|--------:|--------:|-----------------:|
| `canada`           | 4.22 ms | 3.82 ms | **1.10x** (+10%) |
| `torture_cids`     | 63.5 ms | 59.0 ms |      1.08x (+8%) |
| `github`           | 0.07 ms | 0.06 ms |      1.06x (+6%) |
| `atproto_repo.car` | 65.0 ms | 61.8 ms |      1.05x (+5%) |
| `twitter`          | 0.77 ms | 0.75 ms |      1.02x (+2%) |
| `citm_catalog`     | 1.71 ms | 1.71 ms |      1.00x (+0%) |

❗ Behavior changes: `-0.0` is now rejected on decode and encoded as `0.0`, as the DAG-CBOR spec requires; encoding deeply nested data raises `RecursionError` instead of crashing; malformed CAR header types raise `ValueError`.

* Add Python 3.15 support; bump PyO3 0.29 and deps, adopt Ruff 0.17 default rules by [@MarshalX](https://github.com/MarshalX) in [#129](https://github.com/MarshalX/python-libipld/pull/129)
* Add free-threaded wheels for Python 3.14t and 3.15t by [@MarshalX](https://github.com/MarshalX) in [#130](https://github.com/MarshalX/python-libipld/pull/130)
* Ship `py.typed` so type checkers pick up the bundled stubs by [@MarshalX](https://github.com/MarshalX)
* Optimize DAG-CBOR encode by reusing a thread-local buffer and validating CIDs without `Multihash` construction by [@MarshalX](https://github.com/MarshalX) in [#116](https://github.com/MarshalX/python-libipld/pull/116)
* Optimize DAG-CBOR decode by reading each CBOR header once instead of re-decoding it per value by [@MarshalX](https://github.com/MarshalX) in [#118](https://github.com/MarshalX/python-libipld/pull/118)
* Optimize DAG-CBOR decode by reading multi-byte header arguments at a fixed width by [@MarshalX](https://github.com/MarshalX) in [#121](https://github.com/MarshalX/python-libipld/pull/121)
* Optimize DAG-CBOR encode by sorting map entries on a pooled stack and borrowing container items by [@MarshalX](https://github.com/MarshalX) in [#122](https://github.com/MarshalX/python-libipld/pull/122)
* Optimize DAG-CBOR encode/decode by dispatching leaf values inline and recursing only for containers by [@MarshalX](https://github.com/MarshalX) in [#123](https://github.com/MarshalX/python-libipld/pull/123)
* Forbid negative zero in DAG-CBOR: reject `-0.0` on decode and encode it as `0.0` by [@MarshalX](https://github.com/MarshalX) in [#126](https://github.com/MarshalX/python-libipld/pull/126)
* Raise `RecursionError` instead of overflowing the stack when encoding deeply nested DAG-CBOR by [@MarshalX](https://github.com/MarshalX) in [#128](https://github.com/MarshalX/python-libipld/pull/128)
* Document raised exceptions and normalize CAR header type errors to `ValueError` by [@MarshalX](https://github.com/MarshalX) in [#127](https://github.com/MarshalX/python-libipld/pull/127)
* Restructure single-file `lib.rs` into per-domain modules by [@MarshalX](https://github.com/MarshalX) in [#114](https://github.com/MarshalX/python-libipld/pull/114)
* Replace `_PyDict_SetItem_KnownHash` with public `PyDict_SetItem` by [@MarshalX](https://github.com/MarshalX) in [#124](https://github.com/MarshalX/python-libipld/pull/124)
* Fix decode error handling: `PyList` leak, unbounded preallocation, and swallowed errors in `decode_dag_cbor_multi` by [@MarshalX](https://github.com/MarshalX) in [#117](https://github.com/MarshalX/python-libipld/pull/117)
* Fix list and `bytearray` races under free-threading by [@MarshalX](https://github.com/MarshalX) in [#130](https://github.com/MarshalX/python-libipld/pull/130)
* Fix Windows build after PyO3 0.29 by [@MarshalX](https://github.com/MarshalX) in [#131](https://github.com/MarshalX/python-libipld/pull/131)
* Fix `encode_multibase` docs: `'u'` is base64url, not base58btc by [@MarshalX](https://github.com/MarshalX)

## Version 3.4.1

**27.05.2026**

This is a focused decode-side optimization. The numbers below come from `pytest-benchmark` on Apple M1 / CPython 3.14 (PGO-built wheels), comparing v3.4.0 → v3.4.1 head-to-head.

### ⚡ Decode (DAG-CBOR)

| Fixture        |  v3.4.0 |  v3.4.1 |   Speedup |
|----------------|--------:|--------:|----------:|
| `canada`       | 4.75 ms | 4.43 ms | **1.07x** (+7%) |
| `github`       |   69 µs |   66 µs | **1.05x** (+5%) |
| `twitter`      |  790 µs |  766 µs | **1.03x** (+3%) |
| `citm_catalog` | 1.79 ms | 1.77 ms |  1.01x (+1%) |

The decoder used to fetch the Python recursion limit on every recursive step. We now read it once per top-level decode call and pass it through, closing the last decode gap with `cbrrr` (we're within 0.7% of it on `canada` and 30–40% ahead on the others).

### 🔧 Optimizations

* Optimize DAG-CBOR decode by reading `Py_GetRecursionLimit` once per call by [@MarshalX](https://github.com/MarshalX) in [#112](https://github.com/MarshalX/python-libipld/pull/112)

#### Deserialization v3.4.0 → v3.4.1

<img width="2493" height="866" alt="image" src="https://github.com/user-attachments/assets/03778ab9-fb8d-4b38-9cdb-9c721a524ea2" />

#### Serialization v3.4.0 → v3.4.1

<img width="2492" height="861" alt="image" src="https://github.com/user-attachments/assets/f399ff20-fb64-48fe-bb5a-57fc3cf21659" />

## Version 3.4.0

**26.05.2026**

❗ Up to **~2x faster** DAG-CBOR encode and decode!

This release is all about throughput. The numbers below come from `pytest-benchmark` on an Apple M1 / CPython 3.14, comparing v3.3.2 → v3.4.0 head-to-head.

### ⚡ Decode (DAG-CBOR)

| Fixture        |  v3.3.2 |  v3.4.0 |          Speedup |
|----------------|--------:|--------:|-----------------:|
| `github`       |  138 µs |   70 µs | **1.96x** (+96%) |
| `twitter`      | 1.44 ms | 0.80 ms | **1.81x** (+81%) |
| `citm_catalog` | 2.87 ms | 1.81 ms | **1.59x** (+59%) |
| `canada`       | 5.49 ms | 4.77 ms |       1.15x (+15%) |

### ⚡ Encode (DAG-CBOR)

| Fixture        |  v3.3.2 |  v3.4.0 |          Speedup |
|----------------|--------:|--------:|-----------------:|
| `canada`       | 2.31 ms | 1.17 ms | **1.98x** (+98%) |
| `citm_catalog` | 2.25 ms | 1.26 ms | **1.79x** (+79%) |
| `twitter`      |  902 µs |  635 µs | **1.42x** (+42%) |
| `github`       | 75.4 µs | 69.7 µs |        1.08x (+8%) |

### 🔧 Optimizations

* Optimize DAG-CBOR encode/decode and CAR block parsing by [@MarshalX](https://github.com/MarshalX) in [#104](https://github.com/MarshalX/python-libipld/pull/104)
* Optimize DAG-CBOR string decode with an ASCII fast path by [@MarshalX](https://github.com/MarshalX) in [#105](https://github.com/MarshalX/python-libipld/pull/105)
* Optimize DAG-CBOR map decode with a key-intern cache and `KnownHash` insert by [@MarshalX](https://github.com/MarshalX) in [#106](https://github.com/MarshalX/python-libipld/pull/106)
* Optimize int encode with a direct `PyLong` digit read by [@MarshalX](https://github.com/MarshalX) in [#107](https://github.com/MarshalX/python-libipld/pull/107)

### 🧰 Toolchain & Build

* Bump to Rust 1.95 and PyO3 0.28 by [@MarshalX](https://github.com/MarshalX) in [#102](https://github.com/MarshalX/python-libipld/pull/102)
* Drop the macos-13 PGO runner; cross-compile x86_64 macOS wheels from arm64 by [@MarshalX](https://github.com/MarshalX) in [#108](https://github.com/MarshalX/python-libipld/pull/108)
* Bump CI actions (Node 24) and `pytest-codspeed` v5 by [@MarshalX](https://github.com/MarshalX) in [#103](https://github.com/MarshalX/python-libipld/pull/103)

### 📊 Benchmarks & Docs

* Add a head-to-head benchmark against other Python DAG-CBOR libs by [@MarshalX](https://github.com/MarshalX) in [#109](https://github.com/MarshalX/python-libipld/pull/109)
* Update README and copyright years by [@MarshalX](https://github.com/MarshalX) in [#110](https://github.com/MarshalX/python-libipld/pull/110)

#### Deserialization v3.3.2 → v3.4.0

<img width="1935" height="663" alt="image" src="https://github.com/user-attachments/assets/75afc608-5e68-43ec-b583-0d6445b3fe48" />

#### Serialization v3.3.2 → v3.4.0

<img width="1944" height="671" alt="image" src="https://github.com/user-attachments/assets/abd8a50a-2fe3-4f25-9274-0049d6d401e9" />

<details>
<summary>🏎️ Old-style performance comparison. Before (v3.3.2) VS After (v3.4.0):</summary>

<img width="1264" height="620" alt="image" src="https://github.com/user-attachments/assets/28534c2a-613f-4f89-b437-9bf3f8a8eb11" />

</details>

## Version 3.3.2

**05.12.2025**

* Fix DASL testing CI by [@MarshalX](https://github.com/MarshalX) in [#99](https://github.com/MarshalX/python-libipld/pull/99)
* Fix leading zero handling of CIDs in DAG-CBOR by [@MarshalX](https://github.com/MarshalX) in [#98](https://github.com/MarshalX/python-libipld/pull/98)

## Version 3.3.1

**03.12.2025**

* Add DASL testing harnesses to CI workflow by [@MarshalX](https://github.com/MarshalX) in [#97](https://github.com/MarshalX/python-libipld/pull/97)
* Fix error handling in array and map length checks by [@MarshalX](https://github.com/MarshalX) in [#95](https://github.com/MarshalX/python-libipld/pull/95)
* Fix error handling in CID and ULEB128 processing functions by [@MarshalX](https://github.com/MarshalX) in [#96](https://github.com/MarshalX/python-libipld/pull/96)

## Version 3.3.0

**01.12.2025**

❗ Almost **x2 speed for CAR decoding** (Bluesky firehose), **~25-35% faster** DAG-CBOR decoding, and **~17% faster** CID decoding!

I do have more ideas on what to try to speed it up even further! For example, applying BOLT on top of the existing PGO.

**⚡ Optimizations**

* Huge performance gain for DAG-CBOR decoding and CID by replacing deprecated Rust crate `libipld` with `cbor4ii` by [@vmx](https://github.com/vmx) in [#80](https://github.com/MarshalX/python-libipld/pull/80) and even contributed upstream!
* Optimize DAG-CBOR encoding for arrays and maps by [@MarshalX](https://github.com/MarshalX) in [#88](https://github.com/MarshalX/python-libipld/pull/88)
* Optimize CAR decoding by reducing allocations by [@MarshalX](https://github.com/MarshalX) in [#90](https://github.com/MarshalX/python-libipld/pull/90)
* Optimize executable size (`panic = "abort"`) by [@MarshalX](https://github.com/MarshalX) in [#86](https://github.com/MarshalX/python-libipld/pull/86)

**Other**
* Add `pytest-benchmark` GitHub Actions workflow by [@MarshalX](https://github.com/MarshalX) in [#82](https://github.com/MarshalX/python-libipld/pull/82)
* Add lint and format check GHA workflow by [@MarshalX](https://github.com/MarshalX) in [#83](https://github.com/MarshalX/python-libipld/pull/83)
* Add CONTRIBUTING.md by [@MarshalX](https://github.com/MarshalX) in [#85](https://github.com/MarshalX/python-libipld/pull/85)
* Update CodSpeed by [@MarshalX](https://github.com/MarshalX) in [#81](https://github.com/MarshalX/python-libipld/pull/81)
* Fix formatting in profiling by [@MarshalX](https://github.com/MarshalX) in [#84](https://github.com/MarshalX/python-libipld/pull/84)

**🏎️ Performance comparison.** Before (v3.2.0) VS After (v3.3.0):

<img width="1319" height="618" alt="image" src="https://github.com/user-attachments/assets/2bc9aa38-f1cf-4eb7-b5d9-b955af3885ee" />

You can find more benchmark results here: [#92](https://github.com/MarshalX/python-libipld/pull/92)

## Version 3.2.0

**22.10.2025**

❗ Possible breaking change for you: dropped support for PyPy 3.9 and PyPy 3.10 (both no longer supported upstream) in [#77](https://github.com/MarshalX/python-libipld/pull/77). **CPython is still supported from 3.8 to 3.14.**

* Add Python 3.14 support by [@moreati](https://github.com/moreati) in [#74](https://github.com/MarshalX/python-libipld/pull/74)
* Update Rust to 1.90 by [@MarshalX](https://github.com/MarshalX) in [#75](https://github.com/MarshalX/python-libipld/pull/75)
* Update PyO3 to 0.27.1 by [@MarshalX](https://github.com/MarshalX) in [#75](https://github.com/MarshalX/python-libipld/pull/75)
* Fix repository URL specification in `pyproject.toml` file by [@moreati](https://github.com/moreati) in [#73](https://github.com/MarshalX/python-libipld/pull/73)
* Fix clippy warnings by [@MarshalX](https://github.com/MarshalX) in [#76](https://github.com/MarshalX/python-libipld/pull/76)
* Fix CodSpeed by [@MarshalX](https://github.com/MarshalX) in [#78](https://github.com/MarshalX/python-libipld/pull/78)

## Version 3.1.1

**24.06.2025**

* Bump rust to 1.87.0 by [@MarshalX](https://github.com/MarshalX) in [#67](https://github.com/MarshalX/python-libipld/pull/67)
* Specify Rust version using env variable in workflows by [@MarshalX](https://github.com/MarshalX) in [#67](https://github.com/MarshalX/python-libipld/pull/67)
* Fix bignum encoding by [@MarshalX](https://github.com/MarshalX) in [#66](https://github.com/MarshalX/python-libipld/pull/66)

## Version 3.1.0

**20.06.2025**

* Add type stubs by [@zzstoatzz](https://github.com/zzstoatzz) in [#56](https://github.com/MarshalX/python-libipld/pull/56)
* Add PyPy 3.11 support by [@MarshalX](https://github.com/MarshalX) in [#63](https://github.com/MarshalX/python-libipld/pull/63)
* Update PyO3 to 0.25.1 with performance optimizations by [@MarshalX](https://github.com/MarshalX) in [#63](https://github.com/MarshalX/python-libipld/pull/63)
* Update README by [@MarshalX](https://github.com/MarshalX) in [#64](https://github.com/MarshalX/python-libipld/pull/64)
* Fix panic on encoding map with integer keys by [@MarshalX](https://github.com/MarshalX) in [#60](https://github.com/MarshalX/python-libipld/pull/60)
* Fix panic on non UTF-8 string by [@MarshalX](https://github.com/MarshalX) in [#59](https://github.com/MarshalX/python-libipld/pull/59)
* Fix smallest and largest CBOR integer roundtrip by [@MarshalX](https://github.com/MarshalX) in [#58](https://github.com/MarshalX/python-libipld/pull/58)
* Fix decoding of infinities and NaNs according to specs by [@MarshalX](https://github.com/MarshalX) in [#62](https://github.com/MarshalX/python-libipld/pull/62)
* Fix handling of concatenated objects / CBOR sequence by [@MarshalX](https://github.com/MarshalX) in [#65](https://github.com/MarshalX/python-libipld/pull/65)

## Version 3.0.1

**18.02.2025**

* Update PyO3 to 0.23.4 by [@MarshalX](https://github.com/MarshalX) in [#54](https://github.com/MarshalX/python-libipld/pull/54)
* Specify the dynamic version in `pyproject.toml` by [@fxcoudert](https://github.com/fxcoudert) in [#53](https://github.com/MarshalX/python-libipld/pull/53)
* Fix stack overflow when decoding deeply nested structures like arrays and maps by [@MarshalX](https://github.com/MarshalX) in [#51](https://github.com/MarshalX/python-libipld/pull/51)

Possible breaking change for you: dropped support for PyPy 3.8

## Version 3.0.0

**24.10.2024**

**❗ Breaking changes**

* Drop Python 3.7 support by [@MarshalX](https://github.com/MarshalX) in [#50](https://github.com/MarshalX/python-libipld/pull/50)

**🆕 Features**

* Add Python 3.13 support by [@MarshalX](https://github.com/MarshalX) in [#50](https://github.com/MarshalX/python-libipld/pull/50)

**Other**

* Update PyO3 to 0.22.5 by [@MarshalX](https://github.com/MarshalX) in [#50](https://github.com/MarshalX/python-libipld/pull/50)
* Update `decode_car` signature, `bytes` CIDs instead of `str` in README by [@snarfed](https://github.com/snarfed) in [#49](https://github.com/MarshalX/python-libipld/pull/49)

## Version 2.0.0

**07.09.2024**

**❗ Breaking changes**

CID in CAR block keys and CID DAG-CBOR are now in raw byte form instead of stringified one!
* Encode and decode CID as bytes (major 2) instead of string (major 3) by [@MarshalX](https://github.com/MarshalX) in [#36](https://github.com/MarshalX/python-libipld/pull/36)

**⚡ Bunch of new optimizations**
* Optimize duplicate check by [@MarshalX](https://github.com/MarshalX) in [#34](https://github.com/MarshalX/python-libipld/pull/34)
* Optimize type checking by [@MarshalX](https://github.com/MarshalX) in [#37](https://github.com/MarshalX/python-libipld/pull/37)
* Optimize keys duplication check by [@MarshalX](https://github.com/MarshalX) in [#39](https://github.com/MarshalX/python-libipld/pull/39)
* Optimize array creation by [@MarshalX](https://github.com/MarshalX) in [#40](https://github.com/MarshalX/python-libipld/pull/40)
* Optimize string encoding by [@MarshalX](https://github.com/MarshalX) in [#41](https://github.com/MarshalX/python-libipld/pull/41)

**🆕 Features**
* Add CID encoding (`encode_cid(bytes) -> str`) by [@MarshalX](https://github.com/MarshalX) in [#38](https://github.com/MarshalX/python-libipld/pull/38)
* Add DAG-CBOR profiling by [@MarshalX](https://github.com/MarshalX) in [#35](https://github.com/MarshalX/python-libipld/pull/35)

**Other**
* Fix upload to PyPI by [@MarshalX](https://github.com/MarshalX) in [#33](https://github.com/MarshalX/python-libipld/pull/33)
* Update CodSpeed to v3 by [@MarshalX](https://github.com/MarshalX) in [#47](https://github.com/MarshalX/python-libipld/pull/47)
* Update PyO3 to 0.22.2 by [@MarshalX](https://github.com/MarshalX) in [#48](https://github.com/MarshalX/python-libipld/pull/48)
* Lock rust toolchain on v1.76.0 because higher versions segfaults during PGO gather by [@MarshalX](https://github.com/MarshalX) in [#46](https://github.com/MarshalX/python-libipld/pull/46)

**🏎️ Performance comparison**. Before (v1.2.3) VS After (v2.0.0):

<img width="1263" alt="performance diff" src="https://github.com/user-attachments/assets/aa2e72ea-a0ed-449d-836d-ad25c9fdef78">

<details>
  <summary>Raw bench results</summary>

**Before (v1.2.3):**
```
Hello World Decode:
===================
libipld  : 144 ns

Hello World Encode:
===================
libipld  : 150 ns

Realistic Decode Tests:
=======================
canada.json.dagcbor            libipld  : 5.11 ms (197.31 MB/s)
citm_catalog.json.dagcbor      libipld  : 3.43 ms (95.31 MB/s)
github.json.dagcbor            libipld  : 0.17 ms (270.36 MB/s)
twitter.json.dagcbor           libipld  : 2.15 ms (178.96 MB/s)

Realistic Decode Car Tests:
===========================
atproto_repo.car               libipld  : 119.97 ms (154.22 MB/s)

Realistic Encode Tests:
=======================
canada.json.dagcbor            libipld  : 1.19 ms (848.65 MB/s)
citm_catalog.json.dagcbor      libipld  : 1.66 ms (196.42 MB/s)
github.json.dagcbor            libipld  : 0.11 ms (429.15 MB/s)
twitter.json.dagcbor           libipld  : 0.87 ms (443.77 MB/s)
```

**After (v2.0.0):**
```
Hello World Decode:
===================
libipld  : 124 ns

Hello World Encode:
===================
libipld  : 116 ns

Realistic Decode Tests:
=======================
canada.json.dagcbor            libipld  : 4.65 ms (216.57 MB/s)
citm_catalog.json.dagcbor      libipld  : 2.96 ms (110.37 MB/s)
github.json.dagcbor            libipld  : 0.15 ms (311.00 MB/s)
twitter.json.dagcbor           libipld  : 1.75 ms (219.74 MB/s)

Realistic Decode Car Tests:
===========================
atproto_repo.car               libipld  : 92.44 ms (200.13 MB/s)

Realistic Encode Tests:
=======================
canada.json.dagcbor            libipld  : 1.00 ms (1011.47 MB/s)
citm_catalog.json.dagcbor      libipld  : 1.60 ms (203.57 MB/s)
github.json.dagcbor            libipld  : 0.06 ms (733.96 MB/s)
twitter.json.dagcbor           libipld  : 0.56 ms (686.31 MB/s)
```

</details>

Test stand: M1 Pro, Python 3.12.

I highly recommend using Python 3.12. The lower version will be slower.

P.S. Breaking change not only gives a performance boost but also aligns implementations with other languages.

## Version 1.2.3

**11.03.2024**

**⚡ Thanks to PGO and new Bound API in pyo3 v0.21 I achieved huge performance boost ones again! The pyo3 team did an awesome work! Many thanks to all contributors! Also, I implemented own CAR reader that gives +11%**

* Migrate to pyo3 v0.21 Bound API by [@MarshalX](https://github.com/MarshalX) in [#23](https://github.com/MarshalX/python-libipld/pull/23)
* Add PGO-optimized wheels by [@MarshalX](https://github.com/MarshalX) in [#30](https://github.com/MarshalX/python-libipld/pull/30), [#21](https://github.com/MarshalX/python-libipld/pull/21)
* Add own implementation of CAR reader by [@MarshalX](https://github.com/MarshalX) in [#28](https://github.com/MarshalX/python-libipld/pull/28)
* Add IPLD Codec Fixtures by [@MarshalX](https://github.com/MarshalX) in [#19](https://github.com/MarshalX/python-libipld/pull/19)
* Add CAR and CID benchmarks by [@MarshalX](https://github.com/MarshalX) in [#20](https://github.com/MarshalX/python-libipld/pull/20)
* Improve work with numbers by [@MarshalX](https://github.com/MarshalX) in [#25](https://github.com/MarshalX/python-libipld/pull/25)
* Use refs in args and Bound in return type where possible by [@MarshalX](https://github.com/MarshalX) in [#31](https://github.com/MarshalX/python-libipld/pull/31)

**🏎️ Performance comparison**. Before (v1.2.2) VS After (v1.2.3):

<img width="1273" alt="performance diff" src="https://github.com/user-attachments/assets/0fd34b05-10e8-4be7-a109-ce5ce018ef3b">

<details>
  <summary>Raw bench results</summary>

**Before (v1.2.2):**
```
Hello World Decode:
===================
libipld  : 275 ns

Hello World Encode:
===================
libipld  : 237 ns

Realistic Decode Tests:
=======================
canada.json.dagcbor            libipld  : 7.51 ms (134.15 MB/s)
citm_catalog.json.dagcbor      libipld  : 4.97 ms (65.71 MB/s)
github.json.dagcbor            libipld  : 0.23 ms (201.43 MB/s)
twitter.json.dagcbor           libipld  : 2.70 ms (142.51 MB/s)

Realistic Decode Car Tests:
===========================
atproto_repo.car               libipld  : 170.09 ms (108.77 MB/s)

Realistic Encode Tests:
=======================
canada.json.dagcbor            libipld  : 1.89 ms (533.01 MB/s)
citm_catalog.json.dagcbor      libipld  : 2.86 ms (114.02 MB/s)
github.json.dagcbor            libipld  : 0.14 ms (335.64 MB/s)
twitter.json.dagcbor           libipld  : 1.07 ms (358.30 MB/s)
```

**After (v1.2.3):**
```
Hello World Decode:
===================
libipld  : 130 ns

Hello World Encode:
===================
libipld  : 143 ns

Realistic Decode Tests:
=======================
canada.json.dagcbor            libipld  : 5.24 ms (192.11 MB/s)
citm_catalog.json.dagcbor      libipld  : 3.49 ms (93.51 MB/s)
github.json.dagcbor            libipld  : 0.18 ms (257.51 MB/s)
twitter.json.dagcbor           libipld  : 2.11 ms (182.31 MB/s)

Realistic Decode Car Tests:
===========================
atproto_repo.car               libipld  : 123.72 ms (149.55 MB/s)

Realistic Encode Tests:
=======================
canada.json.dagcbor            libipld  : 1.22 ms (823.65 MB/s)
citm_catalog.json.dagcbor      libipld  : 1.69 ms (192.80 MB/s)
github.json.dagcbor            libipld  : 0.11 ms (410.94 MB/s)
twitter.json.dagcbor           libipld  : 0.80 ms (481.72 MB/s)
```

</details>

Test stand: M1 Pro, Python 3.12.

I highly recommend using Python 3.12. The lower version will be slower.

## Version 1.2.2

**02.03.2024**

* Add unit tests and benchmarks by [@MarshalX](https://github.com/MarshalX) in [#11](https://github.com/MarshalX/python-libipld/pull/11)
* Add CodSpeed by [@MarshalX](https://github.com/MarshalX) in [#14](https://github.com/MarshalX/python-libipld/pull/14)
* Fix encode_multibase input data type  by [@MarshalX](https://github.com/MarshalX) in [#12](https://github.com/MarshalX/python-libipld/pull/12)
* Fix DAG CBOR string encoding by [@MarshalX](https://github.com/MarshalX) in [#13](https://github.com/MarshalX/python-libipld/pull/13)
* Fix duplicated map keys check in decode_dag_cbor by [@MarshalX](https://github.com/MarshalX) in [#16](https://github.com/MarshalX/python-libipld/pull/16)
* Fix non-string map keys check in decode_dag_cbor by [@MarshalX](https://github.com/MarshalX) in [#17](https://github.com/MarshalX/python-libipld/pull/17)
* Fix DAG-CBOR map keys order for encoding and decoding by [@MarshalX](https://github.com/MarshalX) in [#18](https://github.com/MarshalX/python-libipld/pull/18)

**🏎️ Performance comparison**

Before (v1.2.1):
```txt
canada.json.dagcbor            libipld  : 1.98 ms (508.99 MB/s)
citm_catalog.json.dagcbor      libipld  : 3.96 ms (82.46 MB/s)
twitter.json.dagcbor           libipld  : 1.92 ms (200.10 MB/s)
```

After (v1.2.2):
```txt
canada.json.dagcbor            libipld  : 2.04 ms (493.43 MB/s)
citm_catalog.json.dagcbor      libipld  : 2.76 ms (118.31 MB/s)
twitter.json.dagcbor           libipld  : 1.03 ms (371.93 MB/s)
```

CodSpeed Performance Report: [#18](https://github.com/MarshalX/python-libipld/pull/18)#issuecomment-1974061926

## Version 1.2.1

**27.02.2024**

* Improve performance of DAG-CBOR encoding by [@MarshalX](https://github.com/MarshalX) in [#10](https://github.com/MarshalX/python-libipld/pull/10)

Encoding results compared to v1.2.0:

- canada.json.dagcbor **x20.6**
- citm_catalog.json.dagcbor  **x4.7**
- twitter.json.dagcbor **x4.1**

## Version 1.2.0

**26.02.2024**

* Add `encode_dag_cbor` by [@MarshalX](https://github.com/MarshalX) in [#5](https://github.com/MarshalX/python-libipld/pull/5)
* Add profiling by [@MarshalX](https://github.com/MarshalX) in [#6](https://github.com/MarshalX/python-libipld/pull/6)
* Improve performance of DAG-CBOR decoding (remove a lot of unnecessary allocs and drops) by [@MarshalX](https://github.com/MarshalX) in [#8](https://github.com/MarshalX/python-libipld/pull/8)

Decoding results compared to v1.1.1:

- citm_catalog.json.dagcbor **x2**
- canada.json.dagcbor **x1.64**
- twitter.json.dagcbor **x1.29**

Decoding results compared to v1.1.0:

- citm_catalog.json.dagcbor **x4.9**
- canada.json.dagcbor **x4.6**
- twitter.json.dagcbor **x4.37**

## Version 1.1.1

**24.02.2024**

* Add error handling by [@MarshalX](https://github.com/MarshalX) in [#4](https://github.com/MarshalX/python-libipld/pull/4)
* Improve performance (at least x2 boost for CAR and DAG-CBOR decoding)  by [@MarshalX](https://github.com/MarshalX) in [#4](https://github.com/MarshalX/python-libipld/pull/4)
* Fix CID decoding in DAG-CBOR by [@MarshalX](https://github.com/MarshalX) in [#4](https://github.com/MarshalX/python-libipld/pull/4)

## Version 1.1.0

**10.01.2024**

* Add multibase encode and decode by [@MarshalX](https://github.com/MarshalX) in [#3](https://github.com/MarshalX/python-libipld/pull/3)

## Version 1.0.1

**27.10.2023**

* Add Python 3.12 by [@MarshalX](https://github.com/MarshalX) in [#2](https://github.com/MarshalX/python-libipld/pull/2)

## Version 1.0.0

**13.09.2023**

**The first stable release**

### Features

* Decode CID (`decode_cid(str) -> dict`)
* Decode DAG-CBOR (`decode_dag_cbor(bytes) -> dict`, `decode_dag_cbor_multi(bytes) -> list[dict]`)
* Decode CAR (`decode_car(bytes) -> tuple[dict, dict[str, dict]]`). Returns a header and blocks mapped by CID.

by [@MarshalX](https://github.com/MarshalX)
