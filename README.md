# bevy_simple_text_input

[![crates.io](https://img.shields.io/crates/v/bevy_simple_text_input.svg)](https://crates.io/crates/bevy_simple_text_input)
[![docs](https://docs.rs/bevy_simple_text_input/badge.svg)](https://docs.rs/bevy_simple_text_input)
[![Following released Bevy versions](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://bevyengine.org/learn/book/plugin-development/#main-branch-tracking)

An unambitious single-line text input widget for `bevy_ui`.

![animated screenshot of text input widget gaining focus and text typed and submitted](assets/screenshot.gif)

## Usage

> [!IMPORTANT]
> Code and examples in the main branch are under development and may not be compatible with the released version on crates.io. The [`latest`](https://github.com/rparrett/bevy_simple_text_input/tree/latest) branch will contain the code from the most recent release.

See [`examples/basic.rs`](https://github.com/rparrett/bevy_simple_text_input/blob/latest/examples/basic.rs).

## Features

I am not trying to build and maintain an enterprise-grade text input, just something that is good enough to be useful in small projects.

- [X] Scrolling
- [X] Keyboard cursor movement (char, word, start/end)
- [X] Disable / focus
- [X] Placeholders
- [X] Doesn't *completely* choke on unicode
- [X] Password masking
- [X] "Submit" events

### Maybe

- [ ] Input filtering
- [ ] Length limit
- [ ] Mouse cursor movement
- [ ] Proper unicode grapheme support

### Probably not

I *might* consider very high quality contributions in these areas, but probably won't be working on them myself. These would likely involve adding dependencies or adding lots of code that I don't want to commit to maintaining.

- [ ] Multi-line
- [ ] Copy/paste
- [ ] IME
- [ ] Selection
- [ ] Mobile

### Definitely not

- [ ] Rich text

## Compatibility

| `bevy_simple_text_input` | `bevy` |
| :--                      | :--    |
| `0.10`                   | `0.15` |
| `0.8`-`0.9`              | `0.14` |
| `0.4`-`0.7`              | `0.13` |
| `0.3`                    | `0.12` |
| `0.1`-`0.2`              | `0.11` |

## Contributing

Please feel free to open a PR!

The code should be simple enough for users to quickly understand and modify for their own purposes. Any new dependencies must not also depend on Bevy. Ideally, we should not add *any* new dependencies.

Please keep PRs small and scoped to a single feature or fix.

## Alternatives

If you need more features, check out [`bevy_cosmic_edit`](https://github.com/StaffEngineer/bevy_cosmic_edit) or [`bevy_egui`](https://github.com/mvlabat/bevy_egui).
