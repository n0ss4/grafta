// The site's address and the metadata every page carries: canonical, Open
// Graph and Twitter, with the social card grafta draws (docs/showcase/card.py).
export const SITE = "https://n0ss4.github.io/grafta";
export const CARD = `${SITE}/assets/card.png`;

export interface PageMeta {
  title: string;
  description: string;
  path: string;
}

type Descriptor =
  | { title: string }
  | { name: string; content: string }
  | { property: string; content: string }
  | { tagName: "link"; rel: string; href: string };

export function meta({ title, description, path }: PageMeta): Descriptor[] {
  const url = `${SITE}${path}`;
  return [
    { title },
    { name: "description", content: description },
    { tagName: "link", rel: "canonical", href: url },
    { property: "og:type", content: "website" },
    { property: "og:site_name", content: "grafta" },
    { property: "og:url", content: url },
    { property: "og:title", content: title },
    { property: "og:description", content: description },
    { property: "og:image", content: CARD },
    { property: "og:image:width", content: "1200" },
    { property: "og:image:height", content: "630" },
    {
      property: "og:image:alt",
      content:
        "grafta: your agent inside real Figma files. A local .fig with named, editable layers, drawn by grafta.",
    },
    { name: "twitter:card", content: "summary_large_image" },
    { name: "twitter:title", content: title },
    { name: "twitter:description", content: description },
    { name: "twitter:image", content: CARD },
  ];
}

/** A path inside the site, under the base it is served from. */
export function href(path: string): string {
  return `${import.meta.env.BASE_URL}${path.replace(/^\//, "")}`;
}
