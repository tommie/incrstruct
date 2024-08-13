# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed

 * Fixes head fields memory leak on initialization error in `new_box` and `new_rc`.

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fixes head fields memory leak on initialization error in new_box and new_rc. ([`ece3afa`](https://github.com/tommie/incrstruct/commit/ece3afa0c2660183443532c111a5f89cd55800c7))
</details>

## 0.1.1 (2024-08-13)

### Added

 - Adds support for `#[init_err(E)]` and failable tail field initialization.

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Adds support for #[init_err(E)] and failable tail field initialization. ([`2023c23`](https://github.com/tommie/incrstruct/commit/2023c23c320f8bd70860740606a16d09ed4d2295))
    - Adds a note about Vec. ([`faed96f`](https://github.com/tommie/incrstruct/commit/faed96f52feccafe9f241fac212abf7a0ff35573))
</details>

## 0.1.0 (2024-08-12)

### Added

 - Implemented the `IterStruct` derive macro and support library.

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Implements generics. ([`669f097`](https://github.com/tommie/incrstruct/commit/669f0977d6ada003d7aee14100f0e044adfb0042))
    - Small docs fixes. ([`c0c3838`](https://github.com/tommie/incrstruct/commit/c0c38380d42441c48ce2a4c7eebbb470dc43b79b))
    - Re-exports the macro. ([`792b0b5`](https://github.com/tommie/incrstruct/commit/792b0b5f6be290811b3c81f5df84f84bd042c91f))
    - Implements the macro, adds examples and tests. ([`46fe4e8`](https://github.com/tommie/incrstruct/commit/46fe4e8b64771008ff9c314666678b453bb8c5d9))
    - Initial test. ([`777daf3`](https://github.com/tommie/incrstruct/commit/777daf3de5fe75d744533ba3304960018657df14))
</details>
