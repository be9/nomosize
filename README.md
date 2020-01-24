# nomosize

Calculate `node_modules` size for a Node app (recursively descending into dependencies' `node_modules`).

## Usage

```bash
cargo run /path/to/repo
```

* `-m` flag will group packages by version.
* `-t N` will report first `N` packages (10 by default)
* `-s versions` will sort the output by the number of different versions (descending).
