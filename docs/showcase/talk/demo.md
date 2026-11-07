# The first picture

`talk.html` makes `docs/assets/talk.png` and `web/public/showcase/talk.webp`. Everything in it happened, on `web/public/sample/order.fig`, the card `scripts/showcase.sh` draws:

- **What's in this file?** `summarize`, `read_node` and `list_variables`, answered in words.
- **Make it light, and check the contrast.** `set_properties` with `explicitVariableModes: { Theme: Light }`, then `check_contrast` (9 checked, 9 passed), `render_node` for `light.png`, and `save_as`.
- **Now turn it into code.** `read_node` at depth 6 with `geometry: true` and `list_variables`, and from them alone, by a model, `order.html`. Drawn in Chromium for `code.png`.

Against grafta's own picture of the card (`code-grafta.png`) the HTML differs by a mean of 3.84 of 255, with 3.28% of pixels off by more than 32, measured on 2026-11-07. That is one small file, compared with grafta's picture and not Figma's. How good code is depends on the model that writes it.
