# ICIE [![](https://img.shields.io/azure-devops/build/pustaczek/7b7eb991-b079-479b-8716-8248c968eaf8/1?logo=azure-pipelines)](https://dev.azure.com/pustaczek/ICIE/_build?definitionId=1) [![](https://img.shields.io/visual-studio-marketplace/i/pustaczek.icie.svg?logo=visual-studio-code)](https://marketplace.visualstudio.com/items?itemName=pustaczek.icie) [![](https://img.shields.io/visual-studio-marketplace/v/pustaczek.icie.svg?color=green)](https://marketplace.visualstudio.com/items?itemName=pustaczek.icie) [![](https://img.shields.io/github/languages/top/pustaczek/icie?color=success&logo=rust)](https://www.rust-lang.org/) [![](https://img.shields.io/github/license/pustaczek/icie.svg?logo=github&color=success)](https://github.com/pustaczek/icie/blob/master/LICENSE)

ICIE is intended to be a VS Code plugin which turns it into an IDE focused on competitive programming.
It aims to cover every aspect of participating in programming competitions, from downloading statements and setting up template code, through building solutions and running the example tests to submitting the solution and tracking its status.
Both efficiency and convenience are priorities, with automated behavior and keyboard shortcuts making coding hassle-free and achieving otherwise impossible time penalties.
Currently, it works on Windows, Linux and macOS, with support for [Codeforces], [AtCoder], [CodeChef] and [SPOJ].
If you have any questions, just create a [GitHub issue]!

## Quick start

- Launch [Visual Studio Code], go to Extensions, search for ICIE and click Install.
- **To participate in a contest**, press <kbd>Alt</kbd><kbd>F9</kbd> before it starts and select it from the list.
- Use <kbd>Alt</kbd><kbd>F12</kbd> to automatically build, run example tests and submit if tests pass.
- Use <kbd>Alt</kbd><kbd>Backspace</kbd> to quickly switch between tasks.
- **To open a single task or an old contest**, press <kbd>Alt</kbd><kbd>F11</kbd> and copy-paste its URL.
- Check out all the other features below!

### More features

- Hover over the test input or output and press <kbd>Ctrl</kbd><kbd>C</kbd> to copy it
- Click "Edit" icon on test input or output to edit it
- Click "Accept" icon on a failing test output to mark it as correct
- Click "Reverse" icon on a failing test output to launch it in [GDB debugger]
- Click "Reverse 2x" icon on a failing test output to launch it in [RR debugger]
- <kbd>Alt</kbd><kbd>-</kbd> to add a new test
- <kbd>Alt</kbd><kbd>t</kbd> to launch a terminal
- <kbd>Alt</kbd><kbd>0</kbd> to run tests without submitting
- <kbd>Alt</kbd><kbd>9</kbd> to run stress tests (test your solution on thousands of random tests)
- <kbd>Alt</kbd><kbd>8</kbd> to reopen task statement
- <kbd>Alt</kbd><kbd>i</kbd> to generate a simple struct and an input operator>>
- <kbd>Alt</kbd><kbd>[</kbd> to automatically [copy-paste parts of your library]
- <kbd>Alt</kbd><kbd>=</kbd> to create a new file from a template
- <kbd>Alt</kbd><kbd>;</kbd> to manually compile a file
- <kbd>Alt</kbd><kbd>\\</kbd> and <kbd>Alt</kbd><kbd>0</kbd> to run tests on currently open file instead of the solution
- <kbd>Alt</kbd><kbd>+</kbd> and select "C++ Checker" to use custom code for checking output correctness
- <kbd>Alt</kbd><kbd>+</kbd> and select something else to create more .cpp files from templates
- <kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>P</kbd> and select "ICIE Web" to open contest or task page in a web browser
- <kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>P</kbd> and select "ICIE Password reset" to log out or reset a password
- <kbd>Ctrl</kbd><kbd>,</kbd> and select Extensions > ICIE to easily configure ICIE's behavior.
- To alter settings only for the current task, use the "Workspace" tab in the settings view.

### Supported sites
| | Contests | Statement | Examples | Submit | Track |
| - | - | - | - | - | - |
| [Codeforces] | Yes | Yes | Yes | Yes | Yes |
| [AtCoder] | Yes | Yes | Yes | Yes | Yes |
| [CodeChef] | Yes | Yes | | Yes | Yes |
| [SPOJ] | | Yes | | Yes | Yes |
| *sio2 sites* | | Yes | | Yes | Yes |

## Development & Building from source

The instructions can be found in [CONTRIBUTING.md].
The project is still in development, the Rust language does not have an official VS Code API, there is a custom build system, it uses WebAssembly which is still in heavy development, and it also patches the compiler output with regexes to remove some type checks, but nevertheless I have tried to make it as streamlined as possible.
If you have any trouble, just create a [GitHub issue]!

[AtCoder]: https://atcoder.jp
[CodeChef]: https://www.codechef.com/
[Codeforces]: https://codeforces.com
[CONTRIBUTING.md]: https://github.com/pustaczek/icie/blob/master/CONTRIBUTING.md
[copy-paste parts of your library]: https://github.com/pustaczek/icie/blob/master/docs/QUICKPASTE.md
[GDB debugger]: https://medium.com/@amit.kulkarni/gdb-basics-bf3407593285
[GitHub issue]: https://github.com/pustaczek/icie/issues
[RR debugger]: https://rr-project.org/
[SPOJ]: https://www.spoj.com
[Visual Studio Code]: https://code.visualstudio.com/
