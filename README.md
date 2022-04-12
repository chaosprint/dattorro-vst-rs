# egui_baseview_test_vst2

Based on [baseview_test_vst2](https://github.com/greatest-ape/baseview_test_vst2)

Barebones [baseview](https://github.com/RustAudio/baseview)/[egui_baseview](https://github.com/BillyDM/egui-baseview)
[vst2](https://github.com/RustAudio/vst-rs) plugin.

It implements an [egui](https://github.com/emilk/egui) ui for the [vst gain effect example](https://github.com/RustAudio/vst-rs/blob/master/examples/gain_effect.rs)

The plugin logs events to `~/tmp/EGUIBaseviewTest.log`.

## Usage: macOS (Tested on M1; need to test on previous models)

- Run `sudo zsh scripts/macos-build-and-install.sh`
- Start your DAW, test the plugin

> For M1 users, run `sudo zsh scripts/macos-build-and-install-m1.sh`

## Usage: Windows

- Run `cargo build`
- Copy `target/debug/imgui_baseview_test_vst2.dll` to your VST plugin folder
- Start your DAW, test the plugin

## Usage: Linux (Untested)

- Run `cargo build`
- Copy `target/debug/imgui_baseview_test_vst2.so` to your VST plugin folder
- Start your DAW, test the plugin

![Demo](demo.png)
