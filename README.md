# Twitch Plays Paper Mario
Twitch Plays Paper Mario is a pet project and direct spinoff of Twitch Plays Pokemon. You can find it deployed live at [TwitchTV}(http://www.twitch.tv/twitchplayspapermario)

## Building and Running
TPPM can be built with Cargo, Rust's package manager. Due to its dependency on [vJoy](http://vjoystick.sourceforge.net/site/), it will only work in Windows.

From the project root, type `cargo build`. If everything goes right, it'll fail after building but before linking, because it can't find the vJoy DLL. Copy `src/libvn64c/vjoyinterface/vjoyinterface.dll` to `target/debug` and try running `cargo build` again; it should generate a binary. Run it with `cargo run`.

Before TPPM will do anything useful, you'll also need to
* install vJoy,
* configure your vJoy device to have 14 buttons and an X and Y axis,
* configure your emulator of choice to listen to that vJoy device, and
* configure tppm.toml with your Twitch credentials.

## Code Overview
TPPM's virtual N64 controller is composed of, from the bottom up,
* FFI bindings to vJoy's C API, automatically generated by [rust-bindgen](https://github.com/crabtw/rust-bindgen) and cleaned up by hand (vjoyinterface)
* "Rustifying" functions that wrap these FFI bindings, and an abstraction of a virtual N64 controller (libvn64c)
* An abstraction of a democratized (shared control) N64 controller (libdemc)

TPPM's Twitch interface is composed of
* An IRC listener bot (libirc)
* A text-based controller command parser (currently sitting in main, lol)

## Disclaimer
I wrote TPPM in my spare time to learn Rust.
