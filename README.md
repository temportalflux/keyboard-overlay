# Layered Key Map Display

Application for displaying keymap layout and input key strokes.


SVG Editing tools:
- General editor: https://yqnn.github.io/svg-path-editor/
- Path Inversion: https://codepen.io/enxaneta/pen/WWPYqQ (clockwise paths always show, but anti-clockwise paths will subtract from other layers)

loosely inspired by other screencasting applications
https://gitlab.com/screenkey/screenkey

using rdev to listen for global input
https://www.reddit.com/r/rust/comments/wskkia/comment/il0as5p/?utm_source=share&utm_medium=web2x&context=3
https://www.reddit.com/r/rust/comments/16x2y9q/tauri_listen_to_global_keyboard_and_mouse_events/

Keymap Editor GUI: https://nickcoutsos.github.io/keymap-editor/

layer swapping; will need a little bit of custom firmware code to dispatch an HID or virtual serial message to host computer
- https://www.reddit.com/r/olkb/comments/90jg71/how_to_display_when_layer_is_active/
- https://www.reddit.com/r/ErgoDoxEZ/comments/dkmohn/display_active_layer_on_computer/
- https://www.reddit.com/r/ergodox/comments/b5r55m/ez_layout_display_application/?utm_source=share&utm_medium=web2x
