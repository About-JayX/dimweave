export function ConfigInput(props: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  placeholder?: string;
}) {
  return (
    <label className="block">
      <span className="text-[10px] text-muted-foreground">{props.label}</span>
      <input
        type={props.type ?? "text"}
        placeholder={props.placeholder}
        className="mt-0.5 w-full rounded-md border border-border/50 bg-background/40 px-2 py-1 text-[11px] text-foreground placeholder:text-muted-foreground/40 focus:border-primary/50 focus:outline-none"
        value={props.value}
        onChange={(e) => props.onChange(e.target.value)}
      />
    </label>
  );
}
