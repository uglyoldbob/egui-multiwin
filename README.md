Crate for multiple windows in egui

[![Rust Windows](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/windows_build.yml/badge.svg)](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/windows_build.yml)
[![Rust MacOS](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/macos_build.yml/badge.svg)](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/macos_build.yml)
[![Rust Linux](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/linux_build.yml/badge.svg)](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/linux_build.yml)

This crate is based on the work by vivlim (https://github.com/vivlim) and repository located (https://github.com/vivlim/egui-glow-multiwin). Vivlim's example repository combines the work at https://github.com/shivshank/mini_gl_fb/blob/master/examples/multi_window.rs and egui to form a nice package. This crate makes some modifications to make it useful as an external crate by defining a few traits for users to implement on their custom structs.

There is an example that shows how to use this crate in your project. It is named multiwin-demo and is in the examples folder.
