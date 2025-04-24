# Changelog

## v0.11.0

* Upgrade to Bevy 0.16 by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/97>

## v0.10.2

* Fix scrolling when scale factor is not 1 by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/90>
* Run CI on other operating systems by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/91>
* Use Bevy's built-in scrolling by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/88>
* Add a second text input to the focus example by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/92>

## v0.10.1

* Fix typo by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/81>
* Replace "return" with "continue" in scroll_with_cursor by @copygirl in <https://github.com/rparrett/bevy_simple_text_input/pull/80>
* Fix missing `Default` impl on `TextInput` by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/83>

## v0.10.0

* Upgrade to Bevy 0.15 by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/77>
* Fix missing type registrations by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/76>

## v0.9.2

* Fix insertion of a space between multibyte characters by @ilotterytea in <https://github.com/rparrett/bevy_simple_text_input/pull/72>

## v0.9.1

* Fix events piling up and causing duplicate key presses by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/70>

## v0.9.0

* scroll into view by @robtfm in <https://github.com/rparrett/bevy_simple_text_input/pull/59>
* Allow typing into multiple input fields at once by @davi4046 in <https://github.com/rparrett/bevy_simple_text_input/pull/58>
* Minor code style tweaks by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/60>
* add a few navigation shortcuts by @robtfm in <https://github.com/rparrett/bevy_simple_text_input/pull/61>
* Make `create` system an observer by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/64>
* Add names to created entities by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/65>
* Dress up new docs related to text navigation by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/68>

## v0.8.0

* Define `TextInputSystem` system set by @andrewhickman in <https://github.com/rparrett/bevy_simple_text_input/pull/56>
* Upgrade to Bevy 0.14 by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/55>

## v0.7.0

* feat: add placeholder text by @Xenira in <https://github.com/rparrett/bevy_simple_text_input/pull/51>
* feat: add character masking by @Xenira in <https://github.com/rparrett/bevy_simple_text_input/pull/50>
* Fix value example's top level comment by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/52>

## v0.6.1

* Fix cursor moving to the end when using the delete key by @tmacychen in <https://github.com/rparrett/bevy_simple_text_input/pull/47>

## v0.6.0

* Don't immediately panic on unicode input by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/42>

## v0.5.1

* Add note about dependencies by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/40>
* Add an example for the main page of the docs by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/38>
* Minor doc fixes by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/37>

## v0.5.0

* Initialize cursor position properly when spawning by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/35>
* Allow `&str` to be passed in `with_value` builder method by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/34>
* Add a setting to control behavior when enter is pressed by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/33>
* Make example colors consistent by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/30>
* Allow text input value to changed programmatically by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/26>
* Don't show cursor if input is spawned in an inactive state by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/29>
* Reflect all the things by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/24>
* Add cursor timer reset on input by @chompaa in <https://github.com/rparrett/bevy_simple_text_input/pull/20>
* Add additional contributing guideline to README by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/22>
* Fix names of private `blink_cursor` and `show_hide_cursor` systems being swapped by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/21>

## 0.4.0

* Upgrade to Bevy 0.13 and prepare for Release by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/19>
* Tweaks to TextInput by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/18>
* Make TextInput contain text information by @Leinnan in <https://github.com/rparrett/bevy_simple_text_input/pull/17>
* Hide cursor for inactive text inputs by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/15>
* Update text style when TextInputTextStyle changes by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/14>
* Refactor with new TextInputBundle by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/13>
* Refactor repeated logic into a `SystemParam` by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/12>

## 0.3.1

* Upgrade to Bevy 0.12 by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/7>

## 0.2.0

* Fix backspace and enter in web builds by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/3>
* Add support for delete key by @rparrett in <https://github.com/rparrett/bevy_simple_text_input/pull/4>

## 0.1.2

* Support linux backspace by @nicopap in <https://github.com/rparrett/bevy_simple_text_input/pull/1>
