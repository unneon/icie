## 0.5.1

- **Added displaying stderr** in test view
- **Added marking alternative answers as correct** with the ✓ action on test outs with Wrong Answer verdict. This is useful for tasks in which there are multiple correct answers.
- **Simplified and documented manual builds**, now available under <kbd>Alt</kbd><kbd>;</kbd>.
- **Documented creating new files from templates** with <kbd>Alt</kbd><kbd>=</kbd>
- Added icie.build.additionalCppFlags config entry, which allows adding additional flags during compilation.
- Debugger terminal titles now contain the name of the test.
- Hovering over an action now displays its short description.
- Created this changelog :)

## 0.5

This version included a complete internal rewrite. There is a new event model, so the plugin now supports running multiple operations simultaneously. Also, the plugin is finally written entirely in [Rust](https://www.rust-lang.org/). Thanks to that, the build system was greatly simplified and a lot of boilerplate was eliminated, which will make adding features much faster. **Expect much more stable behaviour, fewer bugs, extended configuration options and quicker development.**
