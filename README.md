My first VR program written in Rust.
I use bevy, bevy_mod_xr, bevy_mod_openxr and openxr.

Here the steps to reproduce from scratch (or simply clone this repo).

Go to a folder where you like to position the project.

`cargo new my_bevy_start`

I open this folder with VS code and opened a terminal in there.

```
cargo add bevy -F jpeg
cargo add bevy_mod_xr
cargo add bevy_mod_openxr
cargo add openxr
```

Then I create a assets folder in the same directory where the toml file is. Here assets are loaded from. Absolute paths are not supported afaik.
I have a simple glb file (GLTF content) in this folder and the generating blender file.


