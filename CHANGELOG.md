## 1.0.0

No major changes since 0.7.26, but I don't anticipate any major changes in the future so might as well call it stable.

- Fixed some issues with sio2 backend, though it's still kind of broken depending on the sio2 fork/version.
- Fixed some Codeforces/AtCoder issues, maybe. Or at least I can't reproduce them now.

## 0.7.26

- Fixed bug where CSS/JS in test view and stress view sometimes wouldn't load, working around some VS Code resource loading bugs.

## 0.7.25

- Fixed WCNBA errors when submitting CodeChef optimization problems.

## 0.7.24

- Removed all mentions and links to the Discord channel.
- Removed displaying newsletter on first launch.
- Removed all telemetry and usage statistics.
- Removed automatic error reports.
- Removed manual WCNBA error reports.

## 0.7.23

- Fixed submitting to a Codeforces contest you are not registered for causing a 'website could not be analyzed' error.
- Optimized code size, which should speed up startup times.

## 0.7.22

- Added scanning ongoing CodeChef contests. They are added to the bottom of the list, because they are really long and they would clutter the view for people who don't use CodeChef.
- Added falling back to VS Code integrated terminal, if an external one can't be found or has returned an error. This also fixed some crashes when x-terminal-emulator did not exist, like on Windows.

## 0.7.21

- Added automatically hiding compilation errors after a next compilation is successful.
- Added automatically selecting code template and filename to suggestions displayed when source file is not found in stress testing.
- Added configuration option to disable all tutorial messages.
- Added message in test view reminding how to add new tests.
- Added hiding the tutorial for adding new tests after 6 successful uses.
- Added listing supported platforms for some features in error messages.

## 0.7.20

- Fixed broken user interface in test view and stress view on Windows.
- Fixed not including chrono header in default Windows code template.

## 0.7.19

- Fixed low contrast of stderr text.

## 0.7.18

- **Added modern, more clear test view user interface.**
- Added help text when adding new tests.
- Fixed configuration option descriptions being split into random lines.

## 0.7.17

- Added message about the Discord to the default code template and newsletter.
- Added documentation about resetting passwords to README. Just press <kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>P</kbd> and type "ICIE Password reset".
- Fixed AtCoder latest "website could not be analyzed" errors when submitting.

## 0.7.16

- Added official Discord channel.
- Fixed displaying dates in UTC instead of local time.
- Fixed not detecting access denied on some sio2 sites.
- Fixed not treating no languages found as a WCNBA error.

## 0.7.15

- Added support for Alacritty terminal emulator.
- Fixed Codeforces 'website could not be analyzed' error fetching contest title, which has not yet started.

## 0.7.14

- Fixed handling IE (Internal Error) verdict at AtCoder.
- Fixed panics when trying to launch quickpasting.
- Fixed handling of weird characters in contest/task titles, which could result in EINVAL error, especially on Windows. Now, all non-alphanumeric (Unicode aware) characters are stripped from titles.

## 0.7.13

- Added manual error reports for 'website could not be analyzed' errors. If you are getting one, please press the report button and [make a bug report](https://github.com/pustaczek/icie/blob/master/docs/WEBSITE_COULD_NOT_BE_ANALYZED.md). It's really hard to debug these without any information!

## 0.7.12

- Fixed parsing example tests on Codeforces, where "Input" and "Output" headers were downloaded as test data instead of the actual test data.

## 0.7.11

- Fixed parsing some CodeChef problems failing with "missing field practice_submission_allowed".

## 0.7.10

- Added resetting credentials by selecting from a list, in addition to entering an URL.
- Fixed only detecting "C++14 (GCC 5.4.1)" and not "C++ (GCC 9.2.1)" at AtCoder.
- Fixed rare panics when executable exited too fast and closed stdin before we could write to it.

## 0.7.9

- Added links to quickpaste documentation on more kinds of quickpaste errors.
- Changed "Build" to "Compile", "Discover" to "Stress" and "Init" to "Open". This has the unfortunate effect of breaking all configuration relating to these operations, although the old values are still present in the VS Code extension .json config file.
- Removed detecting C++ template path from the pre-0.7.4 configuration entry. It's now replaced with an easier to edit entry, and can be set without going in the preferences. To do so, press <kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>P</kbd> and select "ICIE Template Configure".

## 0.7.8

- Added a suggestion to run normal tests with <kbd>Alt</kbd><kbd>0</kbd> if brut.cpp is not found in stress tests.
- Added support for Codeforces /contests/1260 links.
- Released under a more permissive license, Mozilla Public License 2.0.

## 0.7.7

- Fixed some 'website could not be analyzed' errors when submitting on CodeChef. The problems were caused by trying to submit to contests that ended, but now ICIE will try to determine whether to submit to practice or to a contest.

## 0.7.6

- Added an even simpler, interactive way to customize your C++ template file
- Added checking whether C++ compiler is installed before contests
- Added link to URL documentation when encountering malformed or unknown URLs
- Added retry button for failing to open a website in an external browser
- Added displaying suggested filenames during template instantation
- Added more elegant 'website could not be analyzed' errors
- Added config switch to enable logging, now off by default
- Fixed hardcoding problem id at SPOJ, which could've possibly caused submit failures
- Removed opening tasks in an existing directory

## 0.7.5

- Fixed logging too much

## 0.7.4

- Added a simpler way to customize your C++ template file
- Added builtin code templates for slow solutions, input generators and checkers
- Added many new action buttons for learning how to use ICIE
- Added better error messages

## 0.7.3

- **Added showing compilation output**
- Added more friendly error messages
- Added automated error reporting
- **Fixed broken CodeChef submitting**, which happened due to website updates
- Removed move-to-warning in favor of compilation output channel

## 0.7.2

- Fixed AtCoder 'unrecognized login outcome' errors, and possibly the same error in Codeforces
- Fixed AtCoder ongoing contest support, by fixing selecting ◉ dots as a part of AtCoder contest titles
- Removed way too customizable contest/task directory naming

## 0.7.1

- Added more logging to help diagnose a Codeforces login issue

## 0.7

- **Added Windows and macOS support**
- Added support for Codeforces training group tasks
- Added progress bar when scanning for contests
- Added link to submission details in tracking notification
- Added extension icon and customized background colors
- Added displaying package manager name in install suggestions
- Added support for installing packages with Arch's Pacman
- Fixed the newsletter to appear less often
- Fixed errors when dealing with some rare Codeforces verdicts
- Fixed SIO2 backend not checking if logged in when submitting
- Fixed telemetry to follow the intended usage patterns

## 0.6.4

- Added focusing test view when adding new tests with <kbd>Alt</kbd><kbd>-</kbd>
- Added button to add tests when trying to submit without any
- Added help when submitting without an open task
- Added apologies and 0.7 promise on Windows/MacOS
- Fixed treating every exit as crash internally

## 0.6.3

- Added telemetry

## 0.6.2

- **Added CodeChef support** with contests support under <kbd>Alt</kbd><kbd>F9</kbd>, but without example test support
- Added support for displaying PDF Codeforces statements
- Added support for opening Codeforces Gym contest with an URL
- Added commands "ICIE Web Contest" and "ICIE Web Task" for quickly opening a contest/task in a web browser
- Added nicer default and customizable contest directory names with a new contest.title variable
- Added nicer name for the first task in a contest, now it's task symbol and title like the other tasks
- Added handling compilation errors caused by invalid #include statements
- Changed submission tracking delays to 5 seconds
- Fixed race condition causing not displaying some PDF statements
- Fixed pluralization in contest countdown text
- Fixed non-monospace font for tests in Discover view

## 0.6.1

- Added reopening task statements with <kbd>Alt</kbd><kbd>8</kbd>.
- Added automatically generating new test output when brut exists
- Added support for configuring external terminals
- Optimized PDF statement loading
- Diagnostic improvement of missing language error
- Diagnostic improvement for many rare judge errors
- Fixed not focusing cursor on launch
- Fixed scraping ongoing Codeforces contests' titles
- Fixed opening duplicate editors
- Fixed TLE when output size exceeded pipe buffer size
- Fixed login on some sio2 sites
- Fixed capitalization of AtCoder name

## 0.6

- **Added contest mode** with <kbd>Alt</kbd><kbd>F9</kbd> - just press it and select the one you want to participate in. Or copy-paste its URL into <kbd>Alt</kbd><kbd>F11</kbd>, that works too!
- **Added downloading task statements** and displaying them from VS Code
- **Added switching tasks** with <kbd>Alt</kbd><kbd>Backspace</kbd>, which works with tasks created during contests and ones in the same directory
- **Added checker support**, which uses a custom checker.cpp program to check if answers are correct
- Added a newsletter message informing you of interesting updates
- Added forcing rebuilds during manual builds
- Added error messages on panics
- Added help on keyring errors when the user uses KWallet
- Added trying to retry operations in presence of network issues
- Added sorting library code pieces by title
- Changed submission tracking delay from 500ms to 2000ms
- Changed site names in path templates to nicer ones
- Fixed not replacing more special characters in task titles
- Fixed ignoring time limits support to ICIE Discover
- Fixed Codeforces problemset submit support
- Fixed not reporting some rare errors
- Fixed relying on which(1) to check whether programs are installed

## 0.5.7

- Added a "report issue?" link to error messages, which opens ICIE's GitHub issue tracker
- Improved backtraces from HTML scrappers to give a more precise error location
- Improved some common error messages
- Fixed HTML scrapping on main2.edu.pl
- Fixed non-alphabetical commands and configuration ordering in extension page
- Fixed not removing : characters from task titles
- Fixed HTML scrapping on AtCoder tasks with multiple pre elements

## 0.5.6

This release concludes the month-long rapid development following the 0.5 rewrite. The next planned feature is contest mode, which will take more than a few days - meanwhile, feel free to [ask for other features or bugfixes on GitHub](https://github.com/pustaczek/icie/issues)!

- **Added customizable directory names**, which support using task symbol/title, contest id, site name and random elements, as well as using custom cases like PascalCase or kebab-case. See icie.init.projectNameTemplate configuraiton entry for more details.
- **Added quickpasting**, which automatically copy-pastes data structures and algorithms to your code after pressing <kbd>Alt</kbd><kbd>[</kbd>. See its [setup docs](https://github.com/pustaczek/icie/blob/master/docs/QUICKPASTE.md) to start using it.
- Improved README to contain more information
- Fixed SPOJ login not working at all

## 0.5.5

- **Added [AtCoder](https://atcoder.jp/) support**
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
