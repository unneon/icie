## 0.5.7

- Added a "report issue?" link to error messages, which opens ICIE's GitHub issue tracker
- Improved backtraces from HTML scrappers to give a more precise error location
- Improved some common error messages
- Fixed HTML scrapping on main2.edu.pl
- Fixed non-alphabetical commands and configuration ordering in extension page
- Fixed not removing : characters from task titles
- Fixed HTML scrapping on Atcoder tasks with multiple pre elements

## 0.5.6

This release concludes the month-long rapid development following the 0.5 rewrite. The next planned feature is contest mode, which will take more than a few days - meanwhile, feel free to [ask for other features or bugfixes on GitHub](https://github.com/pustaczek/icie/issues)!

- **Added customizable directory names**, which support using task symbol/title, contest id, site name and random elements, as well as using custom cases like PascalCase or kebab-case. See icie.init.projectNameTemplate configuraiton entry for more details.
- **Added quickpasting**, which automatically copy-pastes data structures and algorithms to your code after pressing <kbd>Alt</kbd><kbd>[</kbd>. See its [setup docs](https://github.com/pustaczek/icie#quickpasting-setup) to start using it.
- Improved README to contain more information
- Fixed SPOJ login not working at all

## 0.5.5

- **Added [Atcoder](https://atcoder.jp/) support**
- **Added [Sphere Online Judge(SPOJ)](https://www.spoj.com/) support**
- Added session caching, which should speed up all network-related operations
- Added displaying execution times if they exceed 100ms or a configured value.
- Added an empty line and a welcome message to the default code template.
- Added stopping the submit if there are no tests; adding new tests with a keyboard shortcut will be suggested, unless the user decides to ignore it.
- Changed test actions to make them easier to discover by showing them at all times. The effect disappears when the user has used them more than s4 times.
- Changed the value of User-Agent header to contain ICIE's name, version and a link to the repository.
- Fixed handling of wrong password errors. ICIE will now recognized that it is not logged in, try to log in, and ask for password again if it happens to be invalid.

## 0.5.4

- **Added time limits**, violation of which results in a TLE verdict. The default limit is set to 1500 milliseconds and can be configured or disabled
- **Added copying test inputs/outputs with a keyboard shortcut** by hovering over the test cell and pressing <kbd>Ctrl</kbd><kbd>C</kbd>. The old copy action icon can be disabled in the configuration
- **Fixed long and wide tests display** in test view and made other improvements to its UI, such as parallel scroll in output and desired output cells
- Added individual custom compilation flags for Debug/Release/Profile profiles
- Added parsing linking errors(ones caused by e.g. a missing main function)
- Added resetting passwords with an "ICIE Password reset" command
- Improved some error messages and logs relating to misuse of Codeforces
- Fixed test view update closing the old test view and breaking custom layouts
- Fixed crashes caused by setting invalid values in the configuration

## 0.5.3

- **Fixed common launch crashes** which happened due to lack of recent OpenSSL or glibc on the system
- **Added an edit test action** shorthand, which opens the related .in/.out
- Improved error messages, making sure all handled errors are described properly
- Improved documentation on configuration entries
- Fixed an unclear error message describint a missing compiler
- Fixed a tracking error that happened due to hacked verdicts in submission history

## 0.5.2

- Nicer errors and richer logs when ICIE main process crashes
- Added instructions for building from source and development

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
