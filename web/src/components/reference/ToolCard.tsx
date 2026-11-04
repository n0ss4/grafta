import { Rich } from "../Rich";
import type { Tool } from "./types";

/** A tool as the server describes it, and its arguments, required first. */
export function ToolCard({ tool }: { tool: Tool }) {
  return (
    <article
      id={tool.name}
      className="tool rounded-2xl border border-line bg-surface p-5 [overflow-wrap:anywhere] sm:p-6"
    >
      <h3 className="mb-1.5 font-mono text-[17px] font-semibold">
        <a href={`#${tool.name}`} className="text-text no-underline hover:text-accent-text">
          {tool.name}
        </a>
      </h3>
      <p className="mb-4 text-[15px] text-muted">
        <Rich text={tool.description} />
      </p>
      {tool.arguments.length === 0 ? (
        <p className="text-[14px] text-muted">No arguments.</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[560px] table-fixed border-collapse text-[14px]">
            <thead>
              <tr className="border-b border-line text-left font-mono text-[12px] tracking-[0.05em] text-muted uppercase">
                <th scope="col" className="w-[30%] px-2.5 py-1.5 font-semibold">
                  Argument
                </th>
                <th scope="col" className="w-[24%] px-2.5 py-1.5 font-semibold">
                  Type
                </th>
                <th scope="col" className="px-2.5 py-1.5 font-semibold">
                  Description
                </th>
              </tr>
            </thead>
            <tbody>
              {tool.arguments.map((arg) => (
                <tr key={arg.name} className="border-b border-line last:border-0 align-top">
                  <td className="px-2.5 py-2 break-words">
                    <code>{arg.name}</code>
                    {arg.required && (
                      <span className="ml-1.5 font-mono text-[12px] font-semibold tracking-[0.05em] text-limit uppercase">
                        required
                      </span>
                    )}
                  </td>
                  <td className="px-2.5 py-2 font-mono text-[12px] break-words text-muted">{arg.type}</td>
                  <td className="px-2.5 py-2 break-words">
                    <Rich text={arg.description} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </article>
  );
}
