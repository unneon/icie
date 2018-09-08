# Icie

Icie is intended to be a VS Code plugin which changes it into a fully-functional IDE focused on competitive programming. It aims to cover every step from the typical workflow, including downloading example test cases, building code, automatically testing it, and submitting. Every action should be available under keyboard shortcuts, in order to shave off important seconds from your time penalty. Aside from time-saving-IDE aspect, I eventually plan to introduce convenient integration with automated searching for test cases, profiling, debugging, working with multiple solutions and more.

Most of the functionality is achieved using [ci](https://github.com/matcegla/ci). In constrast to it, this plugin does store state and aims to be a complete IDE that does everything for you, rather than a flexible set of commands line utilities.

## Usage

- ICIE currently only works on Linux. Windows support will be added immediately after just a few more basic features are implemented.
- If Ci is not installed or outdated, ICIE will display a notification with a Install/Update button. Press it and installation/update will happen automatically.
- Press <kbd>Alt</kbd><kbd>F11</kbd> to create a new project from task description URL. This will create a new randomly-named project in `~` directory, download example test cases, download task description, set up an ICIE project and open it in current window. Supported task URLs are:
    - `https://codeforces.com/contest/1036/problem/A`. Task descriptions will not be downloaded, but this will be implemented in the future.
    - `https://sio2.staszic.waw.pl/c/wwi-2018-grupa-4/p/uni/`. Only `sio2.staszic.waw.pl` is supported, other OIOIOI-based sites are not due to our usage of nonstandard endpoints.
- Opened project will contain a template file and place the cursor inside the main function. The configuration file and template contents can be edited at `~/.config/icie`.
- Press <kbd>Alt</kbd><kbd>F12</kbd> to build, test and submit your code. The code will be submitted only if the code compiles and passes all saved tests.
- If the project does not compile, an error message will be shown. It does not yet include details of the compilation error. Either rely on VS Code C++ plugin's error checking, or compile the code from the terminal with `~/local/share/icie/ci build main.cpp`(assuming you are in project directory).
- If the project does not pass tests, a new tab will be opened showing inputs, outputs and desired outputs of all tests.
- To add new tests to the project, run <kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>P</kbd> and type "ICIE Run" to show the test running tab. At the top, write the input, the expected output and press "Run And Save".

## Features

- [x] Set up a project from task description URL
- [x] Build solutions written in C++
- [x] Test solutions against provided example test cases, as well as your own tests
- [x] Quickly submit solutions to programming contest sites
- [x] Run solutions and automatically save entered tests
- [x] Check the status of your submissions
- [ ] Nice configuration UI
- [x] Provide customizable solution templates
- [ ] Select first/smallest failing test out of already saved ones and show its output/debug it
- [ ] Find first/smallest failing test using a test generator program
- [ ] An automated snippet inclusion system
- [ ] Allow using third-party header-only libraries in submissions
- [ ] Browse task descriptions and other contest info inside of the editor
- [ ] Provide warnings related to competitive programming
- [ ] Set up a project from existing code
- [ ] Allow working with multiple solutions
- [ ] Add profiler integration
- [ ] Add debugger integration
- [ ] Support popular competitive programming sites
    - [x] Codeforces
    - [x] OIOIOI-based sites
    - [ ] Kattis-based sites
    - [ ] Sphere Online Judge
