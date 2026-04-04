import { Suspense, lazy, useMemo } from "react";
import { stripEscapes } from "@/lib/strip-escapes";

const MARKDOWN_SYNTAX_PATTERN =
  /(^|\n)\s{0,3}(#{1,6}\s|[-*+]\s|>\s|\d+\.\s|```)|`[^`]+`|\[[^\]]+\]\([^)]+\)|\*\*[^*\n]+?\*\*|__[^_\n]+?__|~~[^~\n]+?~~|^\|.+\|$/m;
const LazyMessageMarkdownRenderer = lazy(async () => {
  const module = await import("./MessageMarkdownRenderer");
  return { default: module.MessageMarkdownRenderer };
});

interface MessageMarkdownProps {
  content: string;
}

export function prepareMessageContent(content: string): {
  cleaned: string;
  renderMode: "plain" | "markdown";
} {
  const cleaned = stripEscapes(content);
  return {
    cleaned,
    renderMode: MARKDOWN_SYNTAX_PATTERN.test(cleaned) ? "markdown" : "plain",
  };
}

export function MessageMarkdown({ content }: MessageMarkdownProps) {
  const prepared = useMemo(() => prepareMessageContent(content), [content]);

  if (prepared.renderMode === "plain") {
    return (
      <div className="space-y-3 break-words text-[13px] leading-relaxed">
        <div className="whitespace-pre-wrap text-foreground/90">
          {prepared.cleaned}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-3 break-words text-[13px] leading-relaxed">
      <Suspense
        fallback={
          <div className="whitespace-pre-wrap text-foreground/90">
            {prepared.cleaned}
          </div>
        }
      >
        <LazyMessageMarkdownRenderer content={prepared.cleaned} />
      </Suspense>
    </div>
  );
}
