# grafta's website

React 19 and TypeScript on Vite, every page pre-rendered to HTML by React
Router (`ssr: false`, `prerender`), served by GitHub Pages under `/grafta/`.

```bash
npm ci
npm run dev                  # http://localhost:5173/grafta/
npm run typecheck && npm run lint && npm test
npm run build && npm run preview   # the site as Pages serves it: http://localhost:4173/grafta/
npm run e2e                  # Playwright: routes, keyboard, scripts off, axe
npm run lighthouse           # budgets: ≥ 95 in each category, ≤ 150 KB of script
```

## Where each fact lives

| What                                                                  | Where                                                                      | Written by                                                       |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Products, prompts, FAQ, promises, limits, clients, stats, nav, footer | `src/content/*.ts`, typed by `src/content/types.ts`                        | hand; a stat without its source does not compile                 |
| Every tool and its arguments                                          | `src/generated/tools.json`                                                 | `scripts/tool-reference.py`, checked in CI                       |
| The showcase's sessions and pictures                                  | `src/generated/session-first.json`, `public/sessions/`, `public/showcase/` | `docs/showcase/transcripts_site.py`, from recordings             |
| The changelog                                                         | `../CHANGELOG.md`                                                          | hand; rendered at build time                                     |
| Pictures, brand, install scripts, `snapshot.js`                       | `../docs/assets`, `../docs/brand`, `../install.*`, `../scripts/snapshot.*` | copied into `public/` by `scripts/sync-public.mjs` (git-ignored) |

The highlighter (Shiki) and Markdown renderer run in route loaders, at build
time only; nothing of them reaches the browser.

## Adding a section

Write its words in `src/content/` (`editorial.ts` for the home page's), its component in `src/components/home/` (sections), `src/components/editorial/` (the grid, labels, selection box, wordmark, nav) or `src/components/live/` (the order card, its layers, the session)
(with a sidecar test when it has behaviour), and place it in
`src/routes/home.tsx` inside a `Section`, which draws the hairline, the corner
marks, the eyebrow and the title.
