import inter from "@fontsource-variable/inter/files/inter-latin-wght-normal.woff2?url";
import type { ReactNode } from "react";
import { isRouteErrorResponse, Links, Meta, Outlet, Scripts, ScrollRestoration } from "react-router";
import type { Route } from "./+types/root";
import { NotFound } from "./components/NotFound";
import { href } from "./lib/seo";
import "./styles/app.css";

export const links: Route.LinksFunction = () => [
  { rel: "preload", href: inter, as: "font", type: "font/woff2", crossOrigin: "anonymous" },
  { rel: "icon", href: href("brand/mark.svg"), type: "image/svg+xml" },
  { rel: "apple-touch-icon", href: href("brand/apple-touch-icon.png") },
];

export function Layout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="theme-color" content="#E8E8E4" media="(prefers-color-scheme: light)" />
        <meta name="theme-color" content="#17191D" media="(prefers-color-scheme: dark)" />
        {/* Before the first paint: that scripts run (data-js, which the hero's panels
            wait on), and the visitor's choice of light or dark. */}
        <script
          dangerouslySetInnerHTML={{
            __html: `document.documentElement.dataset.js="";try{var t=sessionStorage.getItem("theme");if(t==="dark"||t==="light")document.documentElement.dataset.theme=t}catch(e){}`,
          }}
        />
        <Meta />
        <Links />
      </head>
      <body>
        <a
          href="#main"
          className="absolute -left-[999px] top-2 z-50 rounded-lg bg-text px-3.5 py-2 text-bg focus:left-2"
        >
          Skip to content
        </a>
        {children}
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export default function App() {
  return <Outlet />;
}

export function ErrorBoundary({ error }: Route.ErrorBoundaryProps) {
  if (isRouteErrorResponse(error) && error.status === 404) return <NotFound />;
  const detail = import.meta.env.DEV && error instanceof Error ? error.message : undefined;
  return <NotFound title="Something broke." detail={detail ?? "This page could not be drawn."} />;
}
