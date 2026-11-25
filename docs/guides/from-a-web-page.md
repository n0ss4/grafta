# From a web page

`import_layout` turns HTML into an editable design: flexbox becomes auto-layout,
sized fill, hug or fixed as the browser measured it. grafta runs no browser; a
browser takes a snapshot of what it laid out, and grafta reads that.

```bash
curl -fsSLO https://n0ss4.github.io/grafta/snapshot.sh && curl -fsSLO https://n0ss4.github.io/grafta/snapshot.js
sh snapshot.sh page.html page.json 'main'     # headless Chrome; a CSS selector picks the part
```

Or, in any browser console or a model's browser tool, paste
[`snapshot.js`](../../scripts/snapshot.js) and run `copy(JSON.stringify(graftaSnapshot('main')))`.
Then ask the model to import `page.json`. What does not map (a gradient, a
transform, an inline SVG, a remote image) is listed, not guessed.
