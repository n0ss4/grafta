import type { Announcement, FooterColumn, Link } from "./types";

export const GITHUB = "https://github.com/n0ss4/grafta";
const blob = `${GITHUB}/blob/main`;

export const announcement: Announcement | undefined = {
  text: "0.1.0: the first release. Early, and bug reports are welcome",
  href: "changelog/",
};

export const nav: Link[] = [
  { label: "Products", href: "#products" },
  { label: "Examples", href: "#jobs" },
  { label: "Tools", href: "tools/" },
  { label: "Changelog", href: "changelog/" },
];

export const footer: FooterColumn[] = [
  {
    title: "Product",
    links: [
      { label: "Install", href: "#install" },
      { label: "Examples", href: "#jobs" },
      { label: "FAQ", href: "#faq" },
    ],
  },
  {
    title: "Docs",
    links: [
      { label: "Tool reference", href: "tools/" },
      { label: "Architecture", href: `${blob}/docs/ARCHITECTURE.md` },
      { label: "Contributing", href: `${blob}/docs/CONTRIBUTING.md` },
    ],
  },
  {
    title: "Project",
    links: [
      { label: "GitHub", href: GITHUB },
      { label: "Changelog", href: "changelog/" },
      { label: "Releases", href: `${GITHUB}/releases` },
      { label: "Roadmap", href: `${blob}/docs/ROADMAP.md` },
    ],
  },
  {
    title: "Legal",
    links: [
      { label: "MIT licence", href: `${blob}/LICENSE` },
      { label: "Security", href: `${blob}/SECURITY.md` },
    ],
  },
];
