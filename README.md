# Icie

Icie is intended to be a VS Code plugin which changes it into a fully-functional IDE focused on competitive programming. It aims to cover every step from the typical workflow, including downloading example test cases, building code, automatically testing it, and submitting. Every action should be available under keyboard shortcuts, in order to shave off important seconds from your time penalty. Aside from time-saving-IDE aspect, I eventually plan to introduce convenient integration with automated searching for test cases, profiling, debugging, working with multiple solutions and more.

Most of the functionality is achieved using [ci](https://github.com/matcegla/ci). In constrast to it, this plugin does store state and aims to be a complete IDE that does everything for you, rather than a flexible set of commands line utilities.

## Features

- [x] Set up a project from task description URL
- [x] Build solutions written in C++
- [x] Test solutions against provided example test cases, as well as your own tests
- [x] Quickly submit solutions to programming contest sites
- [ ] Run solutions and automatically save entered tests
- [ ] Check the status of your submissions
- [ ] Nice configuration UI
- [ ] Provide customizable solution templates
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

## Requirements

- [ci](https://github.com/matcegla/ci) has to be installed and in PATH
- In order for setup/submit to work, you must first run them manually from ci in order to log in to sites, as the responsible interface is not ready yet
- POSIX conformance is required temporarily
