# Roadmap

What grafta does today is in the [README](../README.md) and the
[changelog](../CHANGELOG.md). This is what comes next, in no promised order.
Tell us what you need in the [issues](https://github.com/n0ss4/grafta/issues).

## Drawing

- `LINEAR_BURN` and `LINEAR_DODGE` blend modes as stored. Today they are drawn as
  multiply and screen, the nearest the renderer has, and the picture's notes say so.
- Noise and grain effects, which are not drawn yet.
- An instance laid out again when an overridden text changes its width.

## PDF

- Gradients inside a layer with opacity, a mask or a blend, which Apple's PDF
  renderer misplaces, Preview included. They read right in Adobe Reader and
  others, and the page's notes say so.
- Soft masks in Preview, which may come out wrong at some zooms.

## Reach

- Serving over HTTP, for clients that only connect to remote servers.


