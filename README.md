# ICIE ![](https://img.shields.io/travis/pustaczek/icie.svg) ![](https://img.shields.io/visual-studio-marketplace/d/pustaczek.icie.svg) ![](https://img.shields.io/github/license/pustaczek/icie.svg) ![](https://img.shields.io/visual-studio-marketplace/v/pustaczek.icie.svg)

ICIE is intended to be a VS Code plugin which turns it into an IDE focused on competitive programming. It aims to cover every aspect of participating in programming competitions, from setting up template code, through building solutions and running the example tests to submitting the solution. Both efficiency and convenience are priorities, with automated behavior and keyboard shortcuts making coding hassle-free and achieving otherwise impossible time penalties. More advanced aspects of competitions such as output-only, library and interactive tasks, as well as profiling solutions or using certain technical tricks will also be added in the future.

## Quick start

- Start Linux, launch [Visual Studio Code](https://code.visualstudio.com/), go to Extensions, search for ICIE and click Install.
- Open a Codeforces task in your browser(e.g. [560A Remainder](https://codeforces.com/contest/1165/problem/A))
- Press <kbd>Alt</kbd><kbd>F11</kbd> in VS Code and paste the task URL into the input box
- Solve the problem :)
- Press <kbd>Alt</kbd><kbd>F12</kbd> to automatically build, run example tests and submit if tests passed
- If they did not pass, click one of the rewind icons on output to launch the test in a debugger(gdb or [rr](https://rr-project.org/))

### More features

- <kbd>Alt</kbd><kbd>;</kbd> to manually compile a file
- <kbd>Alt</kbd><kbd>-</kbd> to add a new test
- <kbd>Alt</kbd><kbd>=</kbd> to create a new file from a template
- <kbd>Alt</kbd><kbd>t</kbd> to launch a terminal
- <kbd>Alt</kbd><kbd>0</kbd> to run tests without submitting
- <kbd>Alt</kbd><kbd>9</kbd> to find small tests using a test generator and a slow solution
- <kbd>Alt</kbd><kbd>i</kbd> to generate a simple struct with input operator>>
- ~~<kbd>Alt</kbd><kbd>[</kbd> to copy-paste commonly used algorithms~~ Soon!
- To customize ICIE behaviour, click <kbd>Ctrl</kbd><kbd>,</kbd> and go to Extensions > ICIE

## Development & Building from source

The instructions can be found in [CONTRIBUTING.md](https://github.com/pustaczek/icie/blob/master/CONTRIBUTING.md). The project is still in development, the Rust language does not have an official VS Code API, and there is a custom build system, but nevertheless I have tried to make it as streamlined as possible.
