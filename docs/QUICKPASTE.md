# ICIE Quickpasting guide

This is the single feature that requires configuration - quickly pasting common data structures or algorithms into right places in code. This is meant for things that only appear once in code and are declared in the global scope; for others, you probably want to use [snippets](https://code.visualstudio.com/docs/editor/userdefinedsnippets). After you complete this setup, press <kbd>Alt</kbd><kbd>[</kbd> to copy-paste parts of your library.

First, open the normal configuration screen and find the "Paste Library Path" entry. Enter a path to the directory where you want to keep your code pieces, like `~/code-pieces`. Now, create this directory and a file `find-and-union.cpp`, then enter the following:
```cpp
/// Name: FU
/// Description: Find & Union
/// Detail: Disjoint sets data structure in O(α n) proven by Tarjan(1975)
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

*Did you have any issues, or don't understand something? Please create an issue on the [issues page](https://github.com/pustaczek/icie/issues)!*

*See the [README](https://github.com/pustaczek/icie#icie----) to learn how to use ICIE.*
