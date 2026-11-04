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
| The guided image-to-Figma recording and result pictures               | `public/sessions/image.claude-code.json`, `public/showcase/`               | `docs/showcase/transcripts_site.py`, from recordings (spec 075)  |
| The changelog                                                         | `../CHANGELOG.md`                                                          | hand; rendered at build time                                     |
| Pictures, brand, install scripts, `snapshot.js`                       | `../docs/assets`, `../docs/brand`, `../install.*`, `../scripts/snapshot.*` | copied into `public/` by `scripts/sync-public.mjs` (git-ignored) |

The Markdown renderer runs in a route loader at build time only; it does not
reach the browser.

## Adding a section

Write its words in `src/content/` (`editorial.ts` for the home page's), its
component in `src/components/home/`, its Figma-inspired decoration in
`src/components/figma/`, and place it in `src/routes/home.tsx`. Add a sidecar
test when it has behaviour.
