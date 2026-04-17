import { useCallback, useEffect, useState } from 'react';
import { ExternalLink } from 'lucide-react';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { InfoTip } from './InfoTip';
import type {
  CookiesPayload,
  DuplicatesPayload,
  HeadersPayload,
  ImageRow,
  InlinkRow,
  OutlinkRow,
  ResourcesPayload,
  SourcePayload,
  StructuredDataPayload,
  UrlDetail,
} from '@/api';
import {
  getCookies,
  getDuplicates,
  getHeaders,
  getImages,
  getInlinks,
  getOutlinks,
  getResources,
  getSource,
  getStructuredData,
  getUrlDetail,
} from '@/api';

interface Props {
  crawlId: string | null;
  urlId: string | null;
}

function fmt(n: number | null | undefined): string {
  if (n == null) return '—';
  return String(n);
}

function fmtBytes(n: number | null | undefined): string {
  if (n == null) return '—';
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(2)} MB`;
}

type TabKey =
  | 'overview'
  | 'inlinks'
  | 'outlinks'
  | 'images'
  | 'resources'
  | 'headers'
  | 'cookies'
  | 'source'
  | 'duplicates'
  | 'structured';

/**
 * Per-URL detail pane. Tabs lazy-load from the matching per-URL endpoint
 * so the payload stays small regardless of crawl size. Each tab holds its
 * own cache keyed by urlId; switching URLs wipes it.
 */
export function DetailPane({ crawlId, urlId }: Props) {
  const [tab, setTab] = useState<TabKey>('overview');
  const [overview, setOverview] = useState<UrlDetail | null>(null);
  const [inlinks, setInlinks] = useState<InlinkRow[] | null>(null);
  const [outlinks, setOutlinks] = useState<OutlinkRow[] | null>(null);
  const [images, setImages] = useState<ImageRow[] | null>(null);
  const [resources, setResources] = useState<ResourcesPayload | null>(null);
  const [headers, setHeaders] = useState<HeadersPayload | null>(null);
  const [cookies, setCookies] = useState<CookiesPayload | null>(null);
  const [source, setSource] = useState<SourcePayload | null>(null);
  const [duplicates, setDuplicates] = useState<DuplicatesPayload | null>(null);
  const [structured, setStructured] = useState<StructuredDataPayload | null>(null);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    setOverview(null);
    setInlinks(null);
    setOutlinks(null);
    setImages(null);
    setResources(null);
    setHeaders(null);
    setCookies(null);
    setSource(null);
    setDuplicates(null);
    setStructured(null);
    setErr(null);
    setTab('overview');
  }, [crawlId, urlId]);

  const fetchForTab = useCallback(
    async (which: TabKey) => {
      if (!crawlId || !urlId) return;
      setLoading(true);
      setErr(null);
      try {
        switch (which) {
          case 'overview':
            if (!overview) setOverview(await getUrlDetail(crawlId, urlId));
            break;
          case 'inlinks':
            if (!inlinks) setInlinks(await getInlinks(crawlId, urlId));
            break;
          case 'outlinks':
            if (!outlinks) setOutlinks(await getOutlinks(crawlId, urlId));
            break;
          case 'images':
            if (!images) setImages(await getImages(crawlId, urlId));
            break;
          case 'resources':
            if (!resources) setResources(await getResources(crawlId, urlId));
            break;
          case 'headers':
            if (!headers) setHeaders(await getHeaders(crawlId, urlId));
            break;
          case 'cookies':
            if (!cookies) setCookies(await getCookies(crawlId, urlId));
            break;
          case 'source':
            if (!source) setSource(await getSource(crawlId, urlId));
            break;
          case 'duplicates':
            if (!duplicates) setDuplicates(await getDuplicates(crawlId, urlId));
            break;
          case 'structured':
            if (!structured) setStructured(await getStructuredData(crawlId, urlId));
            break;
        }
      } catch (e) {
        setErr((e as Error).message);
      } finally {
        setLoading(false);
      }
    },
    [
      crawlId,
      urlId,
      overview,
      inlinks,
      outlinks,
      images,
      resources,
      headers,
      cookies,
      source,
      duplicates,
      structured,
    ],
  );

  useEffect(() => {
    if (!crawlId || !urlId) return;
    void fetchForTab(tab);
  }, [tab, crawlId, urlId, fetchForTab]);

  if (!urlId || !crawlId) {
    return (
      <aside className="flex h-full w-[420px] shrink-0 items-center justify-center border-l border-border bg-background p-4 text-xs text-muted-foreground">
        Select a row to see details.
      </aside>
    );
  }

  const header = overview
    ? overview.url
    : (inlinks && inlinks[0]?.source_url) || 'Loading…';

  return (
    <aside className="flex h-full w-[420px] shrink-0 flex-col border-l border-border bg-background">
      <div className="border-b border-border px-3 py-1.5">
        <div className="truncate text-[11px] text-muted-foreground">Detail</div>
        <div className="truncate font-mono text-[12px] font-medium text-primary" title={header}>
          {header}
        </div>
      </div>
      <Tabs
        value={tab}
        onValueChange={(v) => setTab(v as TabKey)}
        className="flex min-h-0 flex-1 flex-col"
      >
        <div className="sf-tabs-scroll shrink-0 overflow-x-auto overflow-y-hidden border-b border-border">
          <TabsList className="inline-flex w-max border-0">
            <TabsTrigger value="overview">
              Overview <InfoTip field="detail.overview" className="ml-1" />
            </TabsTrigger>
            <TabsTrigger value="inlinks">
              Inlinks <InfoTip field="detail.inlinks" className="ml-1" />
            </TabsTrigger>
            <TabsTrigger value="outlinks">
              Outlinks <InfoTip field="detail.outlinks" className="ml-1" />
            </TabsTrigger>
            <TabsTrigger value="images">
              Images <InfoTip field="detail.images" className="ml-1" />
            </TabsTrigger>
            <TabsTrigger value="resources">
              Resources <InfoTip field="detail.resources" className="ml-1" />
            </TabsTrigger>
            <TabsTrigger value="headers">
              Headers <InfoTip field="detail.headers" className="ml-1" />
            </TabsTrigger>
            <TabsTrigger value="cookies">
              Cookies <InfoTip field="detail.cookies" className="ml-1" />
            </TabsTrigger>
            <TabsTrigger value="source">
              Source <InfoTip field="detail.source" className="ml-1" />
            </TabsTrigger>
            <TabsTrigger value="duplicates">
              Duplicates <InfoTip field="detail.duplicates" className="ml-1" />
            </TabsTrigger>
            <TabsTrigger value="structured">
              Structured <InfoTip field="detail.structured_data" className="ml-1" />
            </TabsTrigger>
          </TabsList>
        </div>

        <div className="min-h-0 flex-1 overflow-auto">
          {err && (
            <div className="m-3 rounded border border-destructive/40 bg-destructive/10 p-2 text-xs text-destructive">
              {err}
            </div>
          )}
          {loading && (
            <div className="p-3 text-xs text-muted-foreground">Loading…</div>
          )}

          <TabsContent value="overview" className="mt-0 p-3">
            {overview && <OverviewView detail={overview} />}
          </TabsContent>
          <TabsContent value="inlinks" className="mt-0">
            {inlinks && <LinksTable rows={inlinks} kind="inlinks" />}
          </TabsContent>
          <TabsContent value="outlinks" className="mt-0">
            {outlinks && <LinksTable rows={outlinks} kind="outlinks" />}
          </TabsContent>
          <TabsContent value="images" className="mt-0">
            {images && <ImagesTable rows={images} />}
          </TabsContent>
          <TabsContent value="resources" className="mt-0">
            {resources && <ResourcesView data={resources} />}
          </TabsContent>
          <TabsContent value="headers" className="mt-0">
            {headers && <HeadersTable data={headers} />}
          </TabsContent>
          <TabsContent value="cookies" className="mt-0">
            {cookies && <CookiesTable data={cookies} />}
          </TabsContent>
          <TabsContent value="source" className="mt-0">
            {source && <SourceView data={source} />}
          </TabsContent>
          <TabsContent value="duplicates" className="mt-0">
            {duplicates && <DuplicatesView data={duplicates} />}
          </TabsContent>
          <TabsContent value="structured" className="mt-0">
            {structured && <StructuredView data={structured} />}
          </TabsContent>
        </div>
      </Tabs>
    </aside>
  );
}

function KV({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[110px_1fr] gap-2 py-0.5 text-xs">
      <div className="text-muted-foreground">{k}</div>
      <div className="break-words">{v}</div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="mb-3">
      <div className="mb-1 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">
        {title}
      </div>
      {children}
    </div>
  );
}

function OverviewView({ detail }: { detail: UrlDetail }) {
  return (
    <div>
      <Section title="Response">
        <KV k="Status" v={<span className="font-mono">{detail.status_code ?? '—'}</span>} />
        <KV k="Content-Type" v={detail.content_type ?? '—'} />
        <KV k="Internal" v={detail.is_internal ? 'yes' : 'no'} />
        <KV k="Depth" v={fmt(detail.depth)} />
        <KV k="Size" v={fmtBytes(detail.content_length)} />
        <KV k="Response time" v={`${fmt(detail.response_time_ms)} ms`} />
        <KV k="Crawled at" v={detail.crawled_at ?? '—'} />
        {detail.indexability && (
          <KV
            k="Indexability"
            v={
              <span>
                {detail.indexability}
                {detail.indexability_status && (
                  <span className="ml-2 text-muted-foreground">
                    ({detail.indexability_status})
                  </span>
                )}
              </span>
            }
          />
        )}
      </Section>

      <Section title="Title">
        <div className="text-xs">{detail.title ?? <i className="text-muted-foreground">—</i>}</div>
        <div className="mt-0.5 text-[10px] text-muted-foreground">
          {fmt(detail.title_length)} chars · {fmt(detail.title_pixel_width)} px
        </div>
      </Section>

      <Section title="Meta Description">
        <div className="text-xs">
          {detail.meta_description ?? <i className="text-muted-foreground">—</i>}
        </div>
        <div className="mt-0.5 text-[10px] text-muted-foreground">
          {fmt(detail.meta_description_length)} chars
        </div>
      </Section>

      <Section title="Headings">
        <KV k="H1" v={`${detail.h1_first ?? '—'} (${fmt(detail.h1_count)})`} />
        <KV k="H2" v={`${detail.h2_first ?? '—'} (${fmt(detail.h2_count)})`} />
      </Section>

      <Section title="Directives">
        <KV k="Canonical" v={detail.canonical_url ?? '—'} />
        <KV k="Robots" v={detail.meta_robots ?? '—'} />
        <KV k="Redirect" v={detail.redirect_url ?? '—'} />
      </Section>

      {detail.findings && detail.findings.length > 0 && (
        <Section title={`Findings (${detail.findings.length})`}>
          <div className="flex flex-wrap gap-1">
            {detail.findings.map((f) => (
              <Badge key={f} variant="outline" className="font-mono">
                {f}
              </Badge>
            ))}
          </div>
        </Section>
      )}
    </div>
  );
}

function LinksTable({
  rows,
  kind,
}: {
  rows: InlinkRow[] | OutlinkRow[];
  kind: 'inlinks' | 'outlinks';
}) {
  if (rows.length === 0) {
    return <div className="p-3 text-xs text-muted-foreground">No {kind}.</div>;
  }
  return (
    <table className="sf-grid">
      <thead>
        <tr>
          <th>{kind === 'inlinks' ? 'Source URL' : 'Target URL'}</th>
          <th>Anchor</th>
          <th>Type</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r, i) => {
          const href = 'source_url' in r ? r.source_url : r.target_url;
          return (
            <tr key={`${href}-${i}`}>
              <td>
                <span className="sf-url" title={href}>
                  {href}
                </span>
              </td>
              <td className="max-w-[180px] truncate" title={r.anchor_text ?? ''}>
                {r.anchor_text ?? '—'}
              </td>
              <td className="text-muted-foreground">{r.link_type ?? '—'}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function ImagesTable({ rows }: { rows: ImageRow[] }) {
  if (rows.length === 0) {
    return <div className="p-3 text-xs text-muted-foreground">No images.</div>;
  }
  return (
    <table className="sf-grid">
      <thead>
        <tr>
          <th>Image URL</th>
          <th>Code</th>
          <th>Size</th>
          <th>Alt</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r) => (
          <tr key={r.id}>
            <td>
              <span className="sf-url" title={r.url}>
                {r.url}
              </span>
            </td>
            <td className="tabular-nums">{r.status_code ?? '—'}</td>
            <td className="tabular-nums">{fmtBytes(r.size_bytes)}</td>
            <td className="max-w-[200px] truncate" title={r.alt_text ?? ''}>
              {r.alt_text ?? <span className="text-destructive">— missing</span>}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function ResourcesView({ data }: { data: ResourcesPayload }) {
  return (
    <div>
      <div className="px-3 py-2 text-[11px] text-muted-foreground">
        {data.count} resource{data.count === 1 ? '' : 's'} ·{' '}
        {Object.entries(data.counts_by_type)
          .map(([k, v]) => `${k}: ${v}`)
          .join(' · ')}
      </div>
      {data.resources.length === 0 ? (
        <div className="p-3 text-xs text-muted-foreground">No resources recorded.</div>
      ) : (
        <table className="sf-grid">
          <thead>
            <tr>
              <th>Resource</th>
              <th>Type</th>
            </tr>
          </thead>
          <tbody>
            {data.resources.map((r, i) => (
              <tr key={`${r.url}-${i}`}>
                <td>
                  <span className="sf-url" title={r.url}>
                    {r.url}
                  </span>
                </td>
                <td className="text-muted-foreground">{r.type}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

function HeadersTable({ data }: { data: HeadersPayload }) {
  return (
    <table className="sf-grid">
      <thead>
        <tr>
          <th>Header</th>
          <th>Value</th>
        </tr>
      </thead>
      <tbody>
        {data.headers.map((h, i) => (
          <tr key={`${h.name}-${i}`}>
            <td className="whitespace-nowrap font-mono text-[11px] font-medium">{h.name}</td>
            <td className="break-all font-mono text-[11px] text-muted-foreground">{h.value}</td>
          </tr>
        ))}
        {data.headers.length === 0 && (
          <tr>
            <td colSpan={2} className="py-4 text-center text-muted-foreground">
              No headers captured.
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}

function CookiesTable({ data }: { data: CookiesPayload }) {
  if (data.cookies.length === 0) {
    return <div className="p-3 text-xs text-muted-foreground">No cookies set.</div>;
  }
  return (
    <div className="space-y-2 p-3">
      {data.cookies.map((c, i) => (
        <div key={`${c.name}-${i}`} className="rounded border border-border p-2 text-[11px]">
          <div className="mb-1 flex items-center gap-2">
            <span className="font-mono font-semibold">{c.name}</span>
            {c.secure && <Badge variant="opportunity">Secure</Badge>}
            {c.http_only && <Badge variant="stat">HttpOnly</Badge>}
            {c.same_site && <Badge variant="outline">SameSite={c.same_site}</Badge>}
          </div>
          <div className="break-all font-mono text-muted-foreground">{c.value}</div>
          <div className="mt-1 grid grid-cols-2 gap-x-2 text-[10px] text-muted-foreground">
            <div>domain: {c.domain ?? '—'}</div>
            <div>path: {c.path ?? '—'}</div>
            <div>expires: {c.expires ?? '—'}</div>
            <div>max-age: {c.max_age ?? '—'}</div>
          </div>
        </div>
      ))}
    </div>
  );
}

function SourceView({ data }: { data: SourcePayload }) {
  return (
    <div className="p-3">
      <div className="mb-2 flex items-center gap-3 text-[10px] text-muted-foreground">
        <span>{fmtBytes(data.html_length)}</span>
        {data.content_hash && (
          <span className="font-mono" title={data.content_hash}>
            hash: {data.content_hash.slice(0, 12)}…
          </span>
        )}
      </div>
      {data.html ? (
        <pre className="max-h-[60vh] overflow-auto rounded border border-border bg-muted/40 p-2 text-[10px] leading-snug">
          {data.html}
        </pre>
      ) : (
        <div className="text-xs text-muted-foreground">No HTML body captured.</div>
      )}
    </div>
  );
}

function DuplicatesView({ data }: { data: DuplicatesPayload }) {
  return (
    <div>
      <div className="px-3 py-2 text-[11px] text-muted-foreground">
        Match type: {data.match_type} · {data.count} duplicate{data.count === 1 ? '' : 's'}
        {data.content_hash && (
          <span className="ml-2 font-mono">hash {data.content_hash.slice(0, 12)}…</span>
        )}
      </div>
      {data.duplicates.length === 0 ? (
        <div className="px-3 pb-3 text-xs text-muted-foreground">No duplicates.</div>
      ) : (
        <table className="sf-grid">
          <thead>
            <tr>
              <th>URL</th>
              <th>Similarity</th>
            </tr>
          </thead>
          <tbody>
            {data.duplicates.map((d) => (
              <tr key={d.url_id}>
                <td>
                  <span className="sf-url" title={d.url}>
                    {d.url}
                  </span>
                </td>
                <td className="tabular-nums">{(d.similarity * 100).toFixed(0)}%</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

function StructuredView({ data }: { data: StructuredDataPayload }) {
  if (!data.items || data.items.length === 0) {
    return (
      <div className="p-3 text-xs text-muted-foreground">
        No structured data extracted for this URL.
      </div>
    );
  }
  return (
    <div className="space-y-2 p-3">
      {data.items.map((it, i) => (
        <details
          key={i}
          className="rounded border border-border bg-muted/30 p-2 text-[11px]"
          open={i === 0}
        >
          <summary className="cursor-pointer font-mono font-medium">
            Block #{i + 1}
            {typeof it === 'object' && it && '@type' in (it as Record<string, unknown>) && (
              <span className="ml-2 text-muted-foreground">
                {(it as { '@type': string })['@type']}
              </span>
            )}
          </summary>
          <pre className="mt-1 overflow-auto text-[10px] leading-snug">
            {JSON.stringify(it, null, 2)}
          </pre>
        </details>
      ))}
      <div className="pt-1 text-[10px] text-muted-foreground">
        Verify at{' '}
        <a
          className="inline-flex items-center gap-0.5 text-primary hover:underline"
          target="_blank"
          rel="noreferrer"
          href="https://validator.schema.org/"
        >
          validator.schema.org <ExternalLink className="h-3 w-3" />
        </a>
      </div>
    </div>
  );
}
