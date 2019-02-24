# ICIE

ICIE is intended to be a VS Code plugin which turns it into an IDE focused on competitive programming. It aims to cover every aspect of participating in programming competitions, from setting up template code, through building solutions and running the example tests to submitting the solution. Both convenience and speed are priorities, with automated behavior and keyboard shortcuts making coding hassle-free and achieving otherwise impossible time penalties. More advanced aspects of competitions such as output-only, library and interactive tasks, as well as profiling solutions or using certain technical tricks will also be added in the future.

## Quick start

- <kbd>Alt</kbd><kbd>F11</kbd> to set up a project from task URL
- <kbd>Alt</kbd><kbd>F12</kbd> to build, run example tests and submit solution if tests pass
- <kbd>Alt</kbd><kbd>+</kbd> to create a new code file from template
- <kbd>Alt</kbd><kbd>0</kbd> to view tests
- <kbd>Alt</kbd><kbd>-</kbd> to start and finish adding new test (test view must be opened already)
- Linux only, but only until I bother figuring out the build system
- Config is in `~/.config/icie`, default one will be created if one does not exist

## Features

- [x] Set up a project from a task description URL
	- [x] Download example tests
	- [ ] Download task description
	- [x] Create code from a template
	- [x] Save task URL for submitting
- [x] Build solutions
	- [x] Use latest compiler debugging flags and sanitizers
	- [ ] Move cursor to the location of first compiler error
- [x] Test solutions
	- [x] Run and check output on all example tests
	- [x] View tests and solution outputs
	- [x] Add own tests
	- [ ] Treat a different output as correct
	- [x] Support custom output checkers
- [x] Submit solutions
	- [x] Submit using HTTP requests, faster than any human
	- [x] Track submission status refreshing each 0.5s
- [ ] Debug solutions
	- [ ] Launch a selected test in gdb
	- [x] Record a selected test in rr and replay it
	- [ ] Find small tests using a test generator
- [ ] Profile solutions
	- [ ] Find tests of appropriate size
	- [ ] Run test in callgrind and display results in kcachegrind
	- [ ] Integrate callgrind output with VS Code
- [ ] Automate common tasks
	- [x] Create code files from templates
- [ ] Lint code
	- [ ] Make sure C/C++ extension is installed and configured
	- [ ] Show warnings useful in comptetitive programming
- [ ] Manage and compare multiple solutions
	- [ ] Support output-only tasks
	- [ ] Benchmark solutions
- [ ] Use competitive programming libraries automatically
- [ ] Allow to submit code using third-party header-only libraries
- [ ] Emulate environment used on judging systems
	- [ ] OIOIOI
- [ ] Manage C++ compilers and compiler options
	- [ ] Manage clang installation
	- [ ] Manage gdb installation
	- [ ] Manage rr installation
	- [ ] Manage valgrind&kcachegrind installation
- [ ] Support for many competitive programming sites
	- [x] [Codeforces](https://codeforces.com)
	- [x] OIOIOI
	- [ ] [Sphere Online Judge](https://spoj.com)
- [ ] OS support
	- [x] Linux
	- [ ] Windows
