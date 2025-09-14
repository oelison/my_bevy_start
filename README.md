# Linux VR program with bevy, bevy_openxr and openxr

My first VR program written in Rust.
I use bevy, bevy_mod_xr, bevy_mod_openxr and openxr.

Here the steps to reproduce from scratch (or simply clone this repo).

Go to a folder where you like to position the project.

`cargo new my_bevy_start` or `cargo init` when you have already a folder

I open this folder with VS code and opened a terminal in there.

```
cargo add bevy -F jpeg
cargo add bevy_mod_xr
cargo add bevy_mod_openxr
cargo add openxr
cargo add schminput -F xr
```

Then I create a assets folder in the same directory where the toml file is. Here assets are loaded from. Absolute paths are not supported afaik.
I have a simple glb file (GLTF content) in this folder and the generating blender file.

To build on a PC, remove the # on the last three lines in cargo.toml and then execute the following command.

```
cargo run
```

To build for e.g. meta quest 3 (android based vr headset) you have to execute the following commands. Maybe you need to adjust your path to keystore.jks and the password.

```
# release with key:
cargo apk build --release --target aarch64-linux-android
adb install -r target/release/apk/my_bevy_start.apk
# debug without key:
cargo apk build --target aarch64-linux-android
adb install -r target/debug/apk/my_bevy_start.apk
# get the error messages if exist
adb logcat | grep my_bevy_start
```

