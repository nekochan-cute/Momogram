# Momogram Rust core

This crate is the migration target for Momogram logic that does not need direct Android UI access.

The first migrated component is URL cleanup. Kotlin calls the Rust `cdylib` through JNI and falls back to the previous Kotlin implementation only when the native library cannot be loaded. The pure URL logic lives in `src/url.rs` so it can be tested independently from JNI.

## Android build prerequisites

Install Rust and the Android targets:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

The existing Mozilla Rust Android Gradle plugin builds `momogram_core` for all four Android ABIs and packages the generated shared libraries into the APK.

## Host tests

```sh
cd TMessagesProj/rust/momogram-core
cargo test
```

The JNI wrapper is deliberately thin. New migrations should keep Android framework interaction in Kotlin/Java and move deterministic parsing, filtering, serialization, cryptographic glue, and similar platform-independent logic into this crate.
