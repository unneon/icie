# ICIE [![](https://travis-ci.org/pustaczek/icie.svg?branch=master)](https://travis-ci.org/pustaczek/icie) [![](https://img.shields.io/visual-studio-marketplace/d/pustaczek.icie.svg)](https://marketplace.visualstudio.com/items?itemName=pustaczek.icie) [![](https://img.shields.io/visual-studio-marketplace/v/pustaczek.icie.svg)](https://marketplace.visualstudio.com/items?itemName=pustaczek.icie) [![](https://img.shields.io/github/license/pustaczek/icie.svg)](https://github.com/pustaczek/icie/blob/master/LICENSE) [![](https://coveralls.io/repos/github/pustaczek/icie/badge.svg?branch=master)](https://coveralls.io/github/pustaczek/icie?branch=master)

ICIE is intended to be a VS Code plugin which turns it into an IDE focused on competitive programming. It aims to cover every aspect of participating in programming competitions, from setting up template code, through building solutions and running the example tests to submitting the solution. Both efficiency and convenience are priorities, with automated behavior and keyboard shortcuts making coding hassle-free and achieving otherwise impossible time penalties. More advanced aspects of competitions such as output-only, library and interactive tasks, as well as profiling solutions or using certain technical tricks will also be added in the future.

## Quick start

- Start Linux, launch [Visual Studio Code](https://code.visualstudio.com/), go to Extensions, search for ICIE and click Install.
- Open a Codeforces task in your browser(e.g. [560A Remainder](https://codeforces.com/contest/1165/problem/A))
- Press <kbd>Alt</kbd><kbd>F11</kbd> in VS Code and paste the task URL into the input box
- Solve the problem :)
- Press <kbd>Alt</kbd><kbd>F12</kbd> to automatically build, run example tests and submit if tests passed
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
- <kbd>Alt</kbd><kbd>i</kbd> to generate a simple struct and an input operator>>
- <kbd>Alt</kbd><kbd>[</kbd> to automatically copy-paste parts of your competitive programming library
- <kbd>Alt</kbd><kbd>=</kbd> to create a new file from a template
- <kbd>Alt</kbd><kbd>;</kbd> to manually compile a file
- <kbd>Alt</kbd><kbd>\\</kbd> and <kbd>Alt</kbd><kbd>0</kbd> to run tests on currently open file instead of the solution

### Configuration

Most of ICIE's behaviour is easily configurable - just press <kbd>Ctrl</kbd><kbd>,</kbd> and select Extensions > ICIE. To alter settings only for the current task, use the "Workspace" tab in settings view.

#### Quickpasting setup

This is the single feature that requires configuration - quickly pasting common data structures or algorithms into right places in code. This is meant for things that only appear once in code and are declared in the global scope; for others, you probably want to use [snippets](https://code.visualstudio.com/docs/editor/userdefinedsnippets). After you complete this setup, press <kbd>Alt</kbd><kbd>[</kbd> to copy-paste parts of your library.

First, open the normal configuration screen and find the "Paste Library Path" entry. Enter a path to the directory where you want to keep your code pieces, like `~/code-pieces`. Now, create this directory and a file `find-and-union.cpp`, then enter the following:
```cpp
/// Name: FU
/// Description: Find & Union
/// Detail: Disjoint sets in O(α n) proven by Tarjan(1975)
/// Guarantee: struct FU {
struct FU {
	FU(int n):link(n,-1),rank(n,0){}
	int find(int i) const { return link[i] == -1 ? i : (link[i] = find(link[i])); }
	bool tryUnion(int a, int b) {
		a = find(a), b = find(b);
		if (a == b) return false;
		if (rank[a] < rank[b]) swap(a, b);
		if (rank[a] == rank[b]) ++rank[a];
		link[b] = a;
		return true;
	}
	mutable vector<int> link;
	vector<int> rank;
};
```
Most lines are self-explanatory, except for the `/// Guarantee: struct FU {` one. This required field should contain something that ICIE can use to tell if a piece has been already copy-pasted(like `struct X {` for structs or `int f(` for functions). The Description and Detail headers are optional.

You can also specify the Dependencies header with a comma-separated list of things that need to be pasted before this piece(e.g. if your modular arithmetic implementation uses a quick exponentiation function from `qpow.cpp`, write `/// Dependencies: qpow` and it will be pasted automatically).

### Supported sites
| Site | Support | Example test support |
| - | - | - |
| [Codeforces](https://codeforces.com) | Yes | Yes |
| [Atcoder](https://atcoder.jp) | Yes | Yes |
| [SPOJ](https://spoj.com) | Yes | No |
| *sio2 sites* | Yes | No |

## Development & Building from source

The instructions can be found in [CONTRIBUTING.md](https://github.com/pustaczek/icie/blob/master/CONTRIBUTING.md). The project is still in development, the Rust language does not have an official VS Code API, and there is a custom build system, but nevertheless I have tried to make it as streamlined as possible.
