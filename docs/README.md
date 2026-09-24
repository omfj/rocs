# Rocs

Rocs serves a folder of Markdown documentation from a single binary.

## How to run

1. First download the binary.

1. From your repository, run:

   ```sh
   rocs ./docs
   # or point to any other path to your docs
   ```

1. Open <http://127.0.0.1:3000> to view this page.

To use another address or port:

```sh
rocs ./docs --host 0.0.0.0 --port 8080
```

To choose a syntax highlighting style for code blocks, pass its Syntect theme name:

```sh
rocs ./docs --highlight-style InspiredGitHub
```

## Index page

`rocs` uses `README.md` as the index page (think `index.html`) for each directory, so your
documentation is easy to browse both in `rocs` and on GitHub.

## Links

You can link like normally with `rocs`; just add a `[title](./path.md)`. `rocs` will fix the
link for you. For example, [you can navigate here](./foods.md).

## Code blocks

There is also support for syntax highlighting. Take look at this [awesome rust code](./rust.md).
