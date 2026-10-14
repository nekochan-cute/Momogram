# Momogram Rust core

This crate hosts Momogram logic that does not need direct Android UI access. Android code calls the `momogram_core` cdylib through a thin JNI bridge.

Migrated components currently include:

- URL cleanup and tracking-query filtering
- proxy URI parsing and WEB proxy validation
- OpenCC conversion, with dictionaries embedded into the Rust library at compile time
- string helpers used by Momogram
- anti-spam/filter decision state
- secret-chat backup binary/JSON serialization and AES-256-GCM/PBKDF2 crypto

Android framework and database access remain in Kotlin/Java. Deterministic parsing, conversion, serialization, crypto, and state-machine logic live in Rust so they can be tested independently. OpenCC has no runtime JVM dictionary bootstrap; its dictionaries are compiled directly into the Rust library.

## Android build prerequisites

Install Rust and the Android targets:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

The Mozilla Rust Android Gradle plugin builds `momogram_core` for all four Android ABIs. `:TMessagesProj:preBuild` depends on `:TMessagesProj:cargoBuild`, so normal Android builds package the Rust shared library automatically.

The project uses NDK `27.2.12479018` and Android API 21 as the Rust minimum API level.

## Host tests

```sh
cargo test --manifest-path TMessagesProj/rust/momogram-core/Cargo.toml --all-targets
```

The CI workflow also cross-compiles the JNI library for all four Android ABIs and assembles the Android release variant.
