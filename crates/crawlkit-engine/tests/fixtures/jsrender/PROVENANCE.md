# Raw-versus-rendered capture

A pair of documents for `analyzers::js_render_parity`, captured from a real
browser rather than written by hand.

| file | what it is |
|---|---|
| `served.html` | the HTTP response body, exactly as a crawler that does not run JavaScript receives it |
| `dom.html` | the same page after scripts ran, from Chrome's `--dump-dom` |

The page is a deliberately minimal SPA: the response contains an empty
`<div id="root">`, and a script fills it with a heading, two paragraphs of copy,
two links and a JSON-LD block, then rewrites `document.title`.

Chrome 151.0.7922.34, `--headless --dump-dom --virtual-time-budget=3000`.

Hand-written fixtures cannot catch a broken comparison, because the author
already knows what the two documents differ by and encodes that assumption into
both halves. These two files were produced independently of the analyzer, so the
test below is a real check on it.
