import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

const remarkPlugins = [remarkGfm];

const mdComponents: React.ComponentProps<typeof ReactMarkdown>["components"] = {
  pre: ({ children, node }) => {
    const codeEl = node?.children?.[0];
    const className =
      codeEl?.type === "element"
        ? (codeEl.properties?.className as string[] | undefined)
        : undefined;
    const language = className
      ?.find((c: string) => c.startsWith("language-"))
      ?.replace("language-", "");

    return (
      <div className="overflow-hidden rounded-md border border-border/60 bg-muted/40 shadow-[0_2px_8px_rgba(0,0,0,0.2)]">
        {language && (
          <div className="border-b border-border/50 px-3 py-1.5 font-mono text-[11px] uppercase tracking-[0.08em] text-primary/60 bg-primary/3">
            {language}
          </div>
        )}
        <pre className="overflow-x-auto px-3 py-2">
          <code className="font-mono text-[12px] text-foreground">
            {codeEl?.type === "element" && codeEl.children?.[0]?.type === "text"
              ? codeEl.children[0].value
              : children}
          </code>
        </pre>
      </div>
    );
  },
  h1: ({ children }) => (
    <h1 className="text-[18px] font-semibold text-foreground">{children}</h1>
  ),
  h2: ({ children }) => (
    <h2 className="text-[16px] font-semibold text-foreground">{children}</h2>
  ),
  h3: ({ children }) => (
    <h3 className="text-[15px] font-semibold text-foreground">{children}</h3>
  ),
  p: ({ children }) => (
    <p className="whitespace-pre-wrap text-foreground/90">{children}</p>
  ),
  ul: ({ children }) => (
    <ul className="list-disc space-y-1 pl-5 text-foreground/90">{children}</ul>
  ),
  ol: ({ children }) => (
    <ol className="list-decimal space-y-1 pl-5 text-foreground/90">
      {children}
    </ol>
  ),
  li: ({ children }) => <li className="break-words">{children}</li>,
  blockquote: ({ children }) => (
    <blockquote className="border-l-2 border-primary/30 pl-3 text-muted-foreground">
      {children}
    </blockquote>
  ),
  hr: () => <hr className="border-border" />,
  a: ({ href, children }) => (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      className="text-codex underline decoration-codex/50 underline-offset-2 hover:text-codex/80"
    >
      {children}
    </a>
  ),
  code: ({ children }) => (
    <code className="rounded bg-muted/80 px-1 py-0.5 font-mono text-[12px] text-primary/90 border border-primary/10">
      {children}
    </code>
  ),
  table: ({ children }) => (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-left text-[12px]">
        {children}
      </table>
    </div>
  ),
  th: ({ children }) => (
    <th className="border border-border bg-muted/40 px-2 py-1 font-semibold">
      {children}
    </th>
  ),
  td: ({ children }) => (
    <td className="border border-border px-2 py-1 align-top">{children}</td>
  ),
};

export function MessageMarkdownRenderer({ content }: { content: string }) {
  return (
    <ReactMarkdown remarkPlugins={remarkPlugins} components={mdComponents}>
      {content}
    </ReactMarkdown>
  );
}
