import { useLocation } from "react-router-dom";

export function PlaceholderView() {
  const location = useLocation();
  return (
    <section>
      <h1 className="font-mono text-[22px] font-semibold">Placeholder</h1>
      <p className="mt-2 text-sm text-muted-foreground">
        {location.pathname} arrives in a later phase.
      </p>
    </section>
  );
}

export function TokensView() {
  const swatches = [
    "background",
    "foreground",
    "sidebar",
    "card",
    "muted",
    "primary",
    "success",
    "warning",
    "destructive",
    "info",
  ];
  return (
    <section>
      <h1 className="font-mono text-[22px] font-semibold">Tokens</h1>
      <ul className="mt-4 grid grid-cols-2 gap-2">
        {swatches.map((name) => (
          <li key={name} className="flex items-center gap-2 text-sm">
            <span
              className="inline-block h-6 w-6 rounded-sm border border-border"
              style={{ background: `var(--${name})` }}
            />
            {name}
          </li>
        ))}
      </ul>
    </section>
  );
}
