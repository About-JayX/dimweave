import { Search, X } from "lucide-react";

export function BackToBottomButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="z-10 rounded-full bg-transparent px-3 py-1.5 text-[11px] text-muted-foreground transition-colors hover:text-foreground"
    >
      ↓ Back to bottom
    </button>
  );
}

export function SearchRow({
  searchQuery,
  searchSummary,
  inputRef,
  onQueryChange,
  onClose,
}: {
  searchQuery: string;
  searchSummary: string | null;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onQueryChange: (query: string) => void;
  onClose: () => void;
}) {
  return (
    <div className="flex items-center gap-2 border-b border-border/35 px-4 py-1.5">
      <input
        ref={inputRef}
        aria-label="Search messages"
        type="search"
        value={searchQuery}
        onChange={(event) => onQueryChange(event.target.value)}
        placeholder="Search messages"
        className="flex-1 rounded-lg border border-border/45 bg-background/65 px-3 py-1.5 text-[13px] text-foreground outline-none transition-colors focus:border-primary/50"
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus
      />
      {searchSummary && (
        <span className="shrink-0 text-[11px] text-muted-foreground/70">
          {searchSummary}
        </span>
      )}
      <button
        type="button"
        onClick={onClose}
        className="shrink-0 rounded-md p-1 text-muted-foreground hover:text-foreground"
        aria-label="Close search"
      >
        <X className="size-4" />
      </button>
    </div>
  );
}

export function MessageSearchChrome({
  searchOpen,
  searchQuery,
  searchSummary,
  inputRef,
  onOpen,
  onQueryChange,
  onClose,
}: {
  searchOpen: boolean;
  searchQuery: string;
  searchSummary: string | null;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onOpen: () => void;
  onQueryChange: (query: string) => void;
  onClose: () => void;
}) {
  return (
    <>
      <div className="flex items-center border-b border-border/35 px-4 py-1.5">
        <button
          type="button"
          onClick={onOpen}
          className="rounded-md p-1 text-muted-foreground/50 hover:text-foreground transition-colors"
          aria-label="Search messages"
        >
          <Search className="size-4" />
        </button>
      </div>
      {searchOpen ? (
        <SearchRow
          searchQuery={searchQuery}
          searchSummary={searchSummary}
          inputRef={inputRef}
          onQueryChange={onQueryChange}
          onClose={onClose}
        />
      ) : null}
    </>
  );
}
