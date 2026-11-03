import * as React from "react";
import { AlertTriangle, ChevronDown, ChevronRight, ExternalLink, Eye, GitFork, Search, Star } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useGithubSearch, useSourcePreview, useSourceSpec } from "@/lib/api/hooks";
import { cn, pluralize, relativeTime } from "@/lib/utils";
import { Badge, Button, EmptyState, Input, Label, Markdown, SectionTitle, Skeleton, openExternal } from "@/components/ui";
import { ErrorNote, InlineCode, TagChip } from "@/components/app/common";
import { CandidateList } from "./CandidateList";
import { ImportControls } from "./ImportControls";
import { ImportResultSummary } from "./ImportResultSummary";
import { useImportSession } from "./useImportSession";

const SOURCE_KIND_LABEL: Record<T.SourceKind, string> = {
  github: "GitHub repository",
  git: "Git repository",
  url: "Archive URL",
  local_path: "Local folder",
  agent_dir: "Agent folder",
  catalog: "Catalog entry",
  authored: "Authored",
};

const EXAMPLES = ["anthropics/skills", "obra/superpowers", "https://github.com/supabase/agent-skills/tree/main/skills", "~/dev/team-skills"];

/**
 * "Online": preview a repository / URL / path and import selected candidates, plus GitHub search.
 * The source being previewed lives in the URL (`?source=`) so the catalog and search can hand one over.
 */
export function OnlineTab({ source, onPreview }: { source: string | null; onPreview: (source: string) => void }) {
  const topRef = React.useRef<HTMLDivElement>(null);
  const previewAndScroll = (s: string) => {
    onPreview(s);
    topRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  };
  return (
    <div className="flex flex-col gap-8">
      <div ref={topRef}>
        <SourceSection key={source ?? ""} source={source} onPreview={previewAndScroll} />
      </div>
      <SearchSection onPreview={previewAndScroll} />
    </div>
  );
}

function SourceSection({ source, onPreview }: { source: string | null; onPreview: (source: string) => void }) {
  const [input, setInput] = React.useState(source ?? "");
  const [debounced, setDebounced] = React.useState(input);
  React.useEffect(() => {
    const t = window.setTimeout(() => setDebounced(input.trim()), 250);
    return () => window.clearTimeout(t);
  }, [input]);
  const spec = useSourceSpec(debounced);
  const preview = useSourcePreview(source);

  const submit = () => {
    const v = input.trim();
    if (!v) return;
    if (v === source) void preview.refetch();
    else onPreview(v);
  };

  return (
    <section className="flex flex-col gap-3">
      <div className="surface flex flex-col gap-2 p-3">
        <Label htmlFor="source-input">Source</Label>
        <div className="flex gap-2">
          <Input
            id="source-input"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") submit();
            }}
            placeholder="owner/repo, a GitHub tree or blob URL, a git URL, an archive URL or a local path"
            className="mono flex-1"
            spellCheck={false}
            autoComplete="off"
          />
          <Button variant="primary" onClick={submit} loading={preview.isFetching} disabled={!input.trim()}>
            <Eye /> Preview
          </Button>
        </div>
        <SpecFeedback spec={spec} input={debounced} onExample={(s) => setInput(s)} />
      </div>

      {preview.isError ? (
        <div className="flex flex-col gap-2">
          <ErrorNote error={preview.error} />
          <div>
            <Button size="sm" variant="secondary" onClick={() => void preview.refetch()}>
              Try again
            </Button>
          </div>
        </div>
      ) : preview.isFetching && !preview.data ? (
        <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(280px,360px)]" aria-busy="true" aria-label="Fetching source">
          <div className="surface flex flex-col gap-2 p-3">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} className="h-10 w-full" />
            ))}
          </div>
          <Skeleton className="h-48 w-full" />
        </div>
      ) : preview.data ? (
        <PreviewPanel key={source ?? ""} preview={preview.data} input={source!} />
      ) : null}
    </section>
  );
}

function SpecFeedback({ spec, input, onExample }: { spec: ReturnType<typeof useSourceSpec>; input: string; onExample: (s: string) => void }) {
  if (!input) {
    return (
      <p className="flex flex-wrap items-center gap-1 text-[11px] text-subtle">
        <span>Try</span>
        {EXAMPLES.map((e) => (
          <button key={e} type="button" onClick={() => onExample(e)} className="mono rounded bg-sunken px-1 py-0.5 hover:text-fg">
            {e}
          </button>
        ))}
      </p>
    );
  }
  if (spec.isError) {
    return (
      <p className="flex items-center gap-1.5 text-[11px] text-danger">
        <AlertTriangle className="size-3" /> {(spec.error as Error).message}
      </p>
    );
  }
  if (!spec.data) return <p className="text-[11px] text-subtle">Parsing…</p>;
  const s = spec.data;
  return (
    <p className="flex flex-wrap items-center gap-1.5 text-[11px] text-muted">
      <Badge tone="outline">{SOURCE_KIND_LABEL[s.kind]}</Badge>
      <InlineCode>{s.locator}</InlineCode>
      {s.subpath ? (
        <>
          <span>in</span>
          <InlineCode>{s.subpath}</InlineCode>
        </>
      ) : null}
      {s.reference ? (
        <>
          <span>at</span>
          <InlineCode>{s.reference}</InlineCode>
        </>
      ) : null}
    </p>
  );
}

/** Keyed per source so the selection resets with every preview. */
function PreviewPanel({ preview, input }: { preview: T.SourcePreview; input: string }) {
  const session = useImportSession(preview.candidates);
  const [readmeOpen, setReadmeOpen] = React.useState(true);
  const importMut = useApiMutation(
    (chosen: T.Candidate[]) =>
      api.importSource(
        input,
        chosen.map((c) => c.path),
        session.tags,
        session.overwrite,
      ),
    [keys.itemsAll, keys.stats, keys.dashboard],
    (res, chosen) => session.finish(res, chosen),
  );

  return (
    <div className={cn("grid gap-4", preview.readme && "xl:grid-cols-[minmax(0,1fr)_minmax(280px,360px)]")}>
      <div className="flex min-w-0 flex-col gap-3">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted">
          <GitFork className="size-3.5" />
          <span className="font-medium text-fg">{preview.spec.locator}</span>
          {preview.spec.subpath ? <InlineCode>{preview.spec.subpath}</InlineCode> : null}
          {preview.spec.reference ? <span>@ {preview.spec.reference}</span> : null}
          {preview.revision ? (
            <span>
              · revision <InlineCode>{preview.revision.slice(0, 12)}</InlineCode>
            </span>
          ) : null}
          <span>· {pluralize(session.candidates.length, "candidate")}</span>
        </div>
        {preview.warnings.map((w, i) => (
          <div key={i} className="flex items-start gap-2 rounded-lg bg-warn-soft px-3 py-2 text-xs text-warn">
            <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
            <span>{w}</span>
          </div>
        ))}
        <CandidateList candidates={session.candidates} selected={session.selected} onSelectedChange={session.setSelected} />
        <ImportControls
          selectedCount={session.selectedCandidates.length}
          existingCount={session.existingCount}
          tags={session.tags}
          onTagsChange={session.setTags}
          overwrite={session.overwrite}
          onOverwriteChange={session.setOverwrite}
          onImport={() => importMut.mutate(session.selectedCandidates)}
          loading={importMut.isPending}
        />
        <ErrorNote error={importMut.error} />
        {session.result ? <ImportResultSummary result={session.result} onDismiss={session.clearResult} /> : null}
      </div>
      {preview.readme ? (
        <aside className="surface self-start overflow-hidden xl:sticky xl:top-0">
          <button
            type="button"
            onClick={() => setReadmeOpen((o) => !o)}
            aria-expanded={readmeOpen}
            className="flex w-full items-center gap-1.5 border-b bg-sunken/60 px-3 py-1.5 text-left text-xs font-medium hover:text-fg"
          >
            {readmeOpen ? <ChevronDown className="size-3.5 text-muted" /> : <ChevronRight className="size-3.5 text-muted" />}
            README
          </button>
          {readmeOpen ? (
            <div className="max-h-[560px] scroll-y p-3">
              <Markdown className="text-xs">{preview.readme}</Markdown>
            </div>
          ) : null}
        </aside>
      ) : null}
    </div>
  );
}

function SearchSection({ onPreview }: { onPreview: (source: string) => void }) {
  const [text, setText] = React.useState("");
  const [query, setQuery] = React.useState("");
  const search = useGithubSearch(query);
  return (
    <section className="flex flex-col gap-3">
      <SectionTitle title="Search GitHub" description="Repositories that publish agent skills. Preview one before importing anything." />
      <form
        onSubmit={(e) => {
          e.preventDefault();
          setQuery(text.trim());
        }}
        className="flex items-center gap-2"
      >
        <div className="relative w-full max-w-md">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle" />
          <Input value={text} onChange={(e) => setText(e.target.value)} placeholder="e.g. postgres, terraform, data analysis…" className="pl-8" aria-label="Search GitHub" />
        </div>
        <Button type="submit" variant="secondary" loading={search.isFetching} disabled={!text.trim()}>
          <Search /> Search
        </Button>
      </form>
      {search.isError ? (
        <ErrorNote error={search.error} />
      ) : search.isFetching && !search.data ? (
        <div className="surface flex flex-col gap-2 p-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full" />
          ))}
        </div>
      ) : search.data ? (
        search.data.length === 0 ? (
          <EmptyState compact icon={Search} title="No repositories found" description={`Nothing on GitHub matched "${query}". Try a broader term.`} />
        ) : (
          <div className="surface divide-y overflow-hidden">
            {search.data.map((r) => (
              <RepoRow key={r.fullName} repo={r} onPreview={() => onPreview(r.fullName)} />
            ))}
          </div>
        )
      ) : (
        <p className="text-xs text-subtle">Results show stars, topics and the last update. Searching counts against GitHub's rate limit; a token in Settings raises it.</p>
      )}
    </section>
  );
}

function RepoRow({ repo, onPreview }: { repo: T.RepoSearchResult; onPreview: () => void }) {
  const updated = repo.updatedAt ? Date.parse(repo.updatedAt) : NaN;
  return (
    <div className="flex items-center gap-3 px-3 py-2 text-xs">
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="mono text-[13px] font-medium">{repo.fullName}</span>
          <span className="num flex items-center gap-1 text-subtle">
            <Star className="size-3" /> {repo.stars.toLocaleString()}
          </span>
          {Number.isFinite(updated) ? <span className="text-subtle">updated {relativeTime(updated)}</span> : null}
        </div>
        <p className="line-clamp-1 text-muted">{repo.description || "No description"}</p>
        {repo.topics.length ? (
          <div className="mt-1 flex flex-wrap gap-1">
            {repo.topics.slice(0, 6).map((t) => (
              <TagChip key={t} tag={t} />
            ))}
          </div>
        ) : null}
      </div>
      {repo.url ? (
        <Button size="icon-sm" variant="ghost" onClick={() => void openExternal(repo.url)} aria-label={`Open ${repo.fullName} on GitHub`}>
          <ExternalLink />
        </Button>
      ) : null}
      <Button size="sm" variant="secondary" onClick={onPreview}>
        <Eye /> Preview
      </Button>
    </div>
  );
}
