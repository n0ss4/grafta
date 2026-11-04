// grafta's layout snapshot: what a browser laid out, as JSON,
// for `import_layout` to write as design layers. grafta never runs a page; a
// browser runs this, and grafta reads what it returns.
//
// In any browser console, or a model's browser tool, on the page:
//
//   copy(JSON.stringify(graftaSnapshot('main')))     // or any CSS selector
//
// scripts/snapshot.sh runs it in headless Chrome on a local file.
//
// Each element gives the box it was laid out in, relative to the root, and its
// computed style; each text node its lines and its font. What import_layout
// does not map is marked under `unsupported`, so it is listed, not guessed.
function graftaSnapshot(selector, win) {
  win = win || window;
  const doc = win.document;
  const root = doc.querySelector(selector || 'body > *');
  if (!root) throw new Error('grafta: no element matches ' + selector);
  const origin = root.getBoundingClientRect();
  const px = (v) => parseFloat(v) || 0;
  const colour = (c) => (!c || c === 'transparent' || /rgba\([^)]*,\s*0\)$/.test(c)) ? null : c;
  const at = (r) => ({ x: r.left - origin.left, y: r.top - origin.top, width: r.width, height: r.height });
  let count = 0;

  function element(el) {
    const s = win.getComputedStyle(el);
    if (s.display === 'none' || s.visibility === 'hidden') return null;
    count += 1;
    const unsupported = [];
    if (s.backgroundImage !== 'none') unsupported.push('background-image: ' + s.backgroundImage.slice(0, 80));
    if (s.transform !== 'none') unsupported.push('transform');
    if (s.display.includes('grid')) unsupported.push('display: grid (its children are placed where the browser put them)');
    const widths = [s.borderTopWidth, s.borderRightWidth, s.borderBottomWidth, s.borderLeftWidth];
    const colours = [s.borderTopColor, s.borderRightColor, s.borderBottomColor, s.borderLeftColor];
    const uniform = widths.every((w) => w === widths[0]) && colours.every((c) => c === colours[0]);
    if (!uniform && widths.some((w) => px(w) > 0)) unsupported.push('borders that differ per side');
    for (const pseudo of ['::before', '::after']) {
      const p = win.getComputedStyle(el, pseudo);
      if (p.content && p.content !== 'none' && p.content !== 'normal') unsupported.push(pseudo + ' content');
    }
    const tag = el.tagName.toLowerCase();
    const node = {
      tag,
      name: (typeof el.className === 'string' && el.className.trim().split(/\s+/)[0]) || tag,
      box: at(el.getBoundingClientRect()),
      background: colour(s.backgroundColor),
      radius: [px(s.borderTopLeftRadius), px(s.borderTopRightRadius), px(s.borderBottomRightRadius), px(s.borderBottomLeftRadius)],
      border: uniform && px(widths[0]) > 0 && colour(colours[0]) ? { width: px(widths[0]), colour: colours[0] } : null,
      shadow: s.boxShadow !== 'none' ? s.boxShadow : null,
      opacity: px(s.opacity),
      position: s.position,
      layout: s.display.includes('flex') ? {
        direction: s.flexDirection, wrap: s.flexWrap !== 'nowrap',
        gap: px(s.rowGap), columnGap: px(s.columnGap),
        justify: s.justifyContent, align: s.alignItems,
        padding: [px(s.paddingTop), px(s.paddingRight), px(s.paddingBottom), px(s.paddingLeft)],
      } : null,
      children: [],
      unsupported,
    };
    if (tag === 'svg') {
      unsupported.push('inline svg');
      return node;
    }
    if (tag === 'img') node.image = el.currentSrc || el.src;
    for (const child of el.childNodes) {
      if (child.nodeType === 3 && child.textContent.trim()) {
        const range = doc.createRange();
        range.selectNodeContents(child);
        const lines = [...range.getClientRects()].map(at);
        if (!lines.length) continue;
        count += 1;
        node.children.push({
          tag: '#text',
          text: child.textContent.replace(/\s+/g, ' ').trim(),
          box: at(range.getBoundingClientRect()),
          lines,
          font: {
            family: s.fontFamily.split(',')[0].replace(/["']/g, '').trim(),
            size: px(s.fontSize), weight: parseInt(s.fontWeight, 10), italic: s.fontStyle !== 'normal',
            lineHeight: s.lineHeight === 'normal' ? null : px(s.lineHeight),
            letterSpacing: s.letterSpacing === 'normal' ? 0 : px(s.letterSpacing),
            align: s.textAlign, colour: s.color, decoration: s.textDecorationLine, transform: s.textTransform,
          },
        });
      } else if (child.nodeType === 1) {
        const e = element(child);
        if (e) node.children.push(e);
      }
    }
    return node;
  }

  const tree = element(root);
  return { grafta: 'layout-snapshot/1', url: win.location.href, nodes: count, root: tree };
}
