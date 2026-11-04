// The site's words, typed. A missing
// field fails the build; a figure without its source does not compile.

export interface Product {
  name: string;
  extension: ".fig" | ".deck" | ".jam" | ".site";
  summary: string;
  /** The roadmap rows that built and proved it. */
  roadmap: string[];
}

export interface Prompt {
  job: string;
  text: string;
  tools: string[];
}

/** Text that may hold `code` in backticks and [links](href). */
export type Rich = string;

export interface FaqEntry {
  question: string;
  answer: Rich;
}

export interface Client {
  id: string;
  name: string;
  kind: "command" | "json" | "download";
  snippet: string;
  file?: string;
  note: string;
}

export interface Card {
  title: string;
  text: Rich;
}

export interface Link {
  label: string;
  href: string;
}

export interface FooterColumn {
  title: string;
  links: Link[];
}

export interface Announcement {
  text: string;
  href: string;
}
