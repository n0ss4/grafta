import type { Config } from "@react-router/dev/config";

// Every page is drawn to HTML at build time, so crawlers, social cards and
// readers with scripts off get the content; GitHub Pages serves the files.
export const pages = ["/", "/open", "/tools", "/changelog"];

/** Drawn too, at an address no link names: GitHub Pages serves it as 404.html. */
const notFound = "/404";

export default {
  appDirectory: "src",
  basename: "/grafta/",
  ssr: false,
  prerender: [...pages, notFound],
} satisfies Config;
