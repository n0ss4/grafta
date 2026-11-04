import { Fragment } from "react";

/** Text with `code` in backticks, **bold**, and [links](href), as the content modules write it. */
export function Rich({ text }: { text: string }) {
  const parts = text.split(/(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\))/g);
  return (
    <>
      {parts.map((part, i) => {
        if (part.startsWith("`")) return <code key={i}>{part.slice(1, -1)}</code>;
        if (part.startsWith("**")) return <b key={i}>{part.slice(2, -2)}</b>;
        const link = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(part);
        if (link)
          return (
            <a key={i} href={link[2]}>
              {link[1]}
            </a>
          );
        return <Fragment key={i}>{part}</Fragment>;
      })}
    </>
  );
}
