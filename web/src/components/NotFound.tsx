import { href } from "~/lib/seo";
import { Page } from "./chrome/Page";

interface Props {
  title?: string;
  detail?: string;
}

/** The page for an address the site does not have; also what an error shows. */
export function NotFound({
  title = "No node at this path.",
  detail = "grafta would say what it left out. This page is not here.",
}: Props) {
  return (
    <Page>
      <title>Not found: grafta</title>
      <meta name="robots" content="noindex" />
      <section className="grid min-h-[70vh] grid-cols-4 content-center px-4 py-28 lg:grid-cols-8 lg:px-0">
        <div className="col-span-4 lg:col-span-4 lg:col-start-2">
          <h1 className="text-[40px] leading-[1.05] font-normal tracking-[-0.03em] sm:text-[64px]">
            {title}
          </h1>
        </div>
        <p className="col-span-4 mt-8 max-w-[480px] text-[17px] text-muted lg:col-start-2">{detail}</p>
        <p className="col-span-4 mt-8 flex gap-6 text-[15px] font-medium lg:col-start-2">
          <a href={href("")} className="text-text underline underline-offset-4">
            Home
          </a>
          <a href={href("tools/")} className="text-text underline underline-offset-4">
            Tool reference
          </a>
        </p>
      </section>
    </Page>
  );
}
