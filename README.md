# ICIE [![](https://img.shields.io/azure-devops/build/pustaczek/7b7eb991-b079-479b-8716-8248c968eaf8/1?logo=azure-pipelines)](https://dev.azure.com/pustaczek/ICIE/_build?definitionId=1) [![](https://img.shields.io/visual-studio-marketplace/i/pustaczek.icie.svg?logo=visual-studio-code)](https://marketplace.visualstudio.com/items?itemName=pustaczek.icie) [![](https://img.shields.io/visual-studio-marketplace/v/pustaczek.icie.svg)](https://marketplace.visualstudio.com/items?itemName=pustaczek.icie) [![](https://img.shields.io/github/license/pustaczek/icie.svg?logo=github)](https://github.com/pustaczek/icie/blob/master/LICENSE)

ICIE is intended to be a VS Code plugin which turns it into an IDE focused on competitive programming. It aims to cover every aspect of participating in programming competitions, from downloading statements and setting up template code, through building solutions and running the example tests to submitting the solution and tracking its status. Both efficiency and convenience are priorities, with automated behavior and keyboard shortcuts making coding hassle-free and achieving otherwise impossible time penalties. Currently, it works on Windows, Linux and macOS, with support for [Codeforces](https://codeforces.com), [AtCoder](https://atcoder.jp), ~~[CodeChef](https://www.codechef.com/)~~ and [SPOJ](https://www.spoj.com).

## Quick start

- Launch [Visual Studio Code](https://code.visualstudio.com/), go to Extensions, search for ICIE and click Install.
- **To participate in a contest**, press <kbd>Alt</kbd><kbd>F9</kbd> before it starts and select it from the list.
- Use <kbd>Alt</kbd><kbd>F12</kbd> to automatically build, run example tests and submit if tests pass.
- Use <kbd>Alt</kbd><kbd>Backspace</kbd> to quickly switch between tasks.
- **To open a single task or an old contest**, press <kbd>Alt</kbd><kbd>F11</kbd> and copy-paste its URL.
- Check out all the other features below!

### More features

- Hover over the test input/output and press <kbd>Ctrl</kbd><kbd>C</kbd> to copy it
- Click ✎ action to edit a test input or output
- Click ✓ action on a failing test to mark the output as correct
- Click ◀ action to launch the test in the gdb debugger
- Click ⏪ action to launch the test in the [rr](https://rr-project.org/) debugger
- <kbd>Alt</kbd><kbd>-</kbd> to add a new test
- <kbd>Alt</kbd><kbd>t</kbd> to launch a terminal
- <kbd>Alt</kbd><kbd>0</kbd> to run tests without submitting
- <kbd>Alt</kbd><kbd>9</kbd> to run stress tests
- <kbd>Alt</kbd><kbd>8</kbd> to reopen task statement
- <kbd>Alt</kbd><kbd>i</kbd> to generate a simple struct and an input operator>>
- <kbd>Alt</kbd><kbd>[</kbd> to automatically [copy-paste parts of your library](https://github.com/pustaczek/icie/blob/master/docs/QUICKPASTE.md)
- <kbd>Alt</kbd><kbd>=</kbd> to create a new file from a template
- <kbd>Alt</kbd><kbd>;</kbd> to manually compile a file
- <kbd>Alt</kbd><kbd>\\</kbd> and <kbd>Alt</kbd><kbd>0</kbd> to run tests on currently open file instead of the solution
- Use custom checker.cpp; see details in checker configuration entry
- <kbd>Ctrl</kbd><kbd>,</kbd> and select Extensions > ICIE to easily configure ICIE's behavior.
- To alter settings only for the current task, use the "Workspace" tab in the settings view.

### Supported sites
| | Contests | Statement | Examples | Submit | Track |
| - | - | - | - | - | - |
| [Codeforces](https://codeforces.com) | Yes | Yes | Yes | Yes | Yes |
| [AtCoder](https://atcoder.jp) | Yes | Yes | Yes | Yes | Yes |
| [CodeChef](https://www.codechef.com/) | ~~Yes~~ | Yes | | ~~Yes~~ | ~~Yes~~ |
| [SPOJ](https://spoj.com) | | Yes | | Yes | Yes |
| *sio2 sites* | | Yes | | Yes | Yes |

## Development & Building from source

The instructions can be found in [CONTRIBUTING.md](https://github.com/pustaczek/icie/blob/master/CONTRIBUTING.md). The project is still in development, the Rust language does not have an official VS Code API, there is a custom build system, it uses WebAssembly which is still in heavy development, and it also patches the compiler output with regexes to remove some type checks, but nevertheless I have tried to make it as streamlined as possible.
