// Verbatim excerpts from screamingfrog.co.uk's SEO Spider user guide.
// Used to populate per-field tooltips in the UI so it behaves like SF's
// own help system. Expand this dictionary as new fields are surfaced in
// later batches — do not paraphrase; keep the upstream wording.

export interface SfFieldDoc {
  title: string;
  body: string;
  href: string;
}

export const sfUserGuide: Record<string, SfFieldDoc> = {
  'crawl.seed_url': {
    title: 'Seed URL',
    body:
      'Enter the URL you want to crawl in the bar at the top of the application. By default the SEO Spider will crawl the subdomain you enter and treat all other subdomains it encounters as external links. You can adjust this in Configuration > Spider.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/#crawling-a-website-subdomain',
  },
  'crawl.start': {
    title: 'Start',
    body:
      'Click Start to begin the crawl. The SEO Spider will crawl the site in real-time and populate the results tabs as URLs are discovered. You can pause at any point.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'crawl.pause': {
    title: 'Pause',
    body:
      'Pauses the in-progress crawl. Results discovered so far remain available in the grid. Press Start again to resume from the queue position.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'crawl.stop': {
    title: 'Stop',
    body:
      'Stops the crawl and marks it as completed. The queue is cleared; results already discovered are kept. You cannot resume a stopped crawl, but you can start a new one.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'mode.spider': {
    title: 'Mode: Spider',
    body:
      'In Spider mode the SEO Spider will crawl a website in real-time by discovering URLs via internal links. Enter a URL in the top bar and click Start to begin.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/#mode',
  },
  'mode.list': {
    title: 'Mode: List',
    body:
      'In List mode you supply a fixed list of URLs to crawl — useful for auditing a specific set of pages such as a sitemap, a backlink list, or a set of redirects.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/#mode',
  },
  'mode.serp': {
    title: 'Mode: SERP',
    body:
      'SERP mode lets you upload a file of page titles and meta descriptions to review their pixel width and truncation without performing a crawl.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/#mode',
  },
  'menu.file': {
    title: 'File Menu',
    body:
      'The File menu contains options to start a new crawl, open or save crawls, access recent crawls, export results, and manage scheduled crawls.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'menu.configuration': {
    title: 'Configuration Menu',
    body:
      'The Configuration menu lets you control how the SEO Spider crawls a site — which URLs to include or exclude, rendering mode, speed, user-agent, custom extractions, API integrations, and more.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/',
  },
  'menu.bulk_export': {
    title: 'Bulk Export Menu',
    body:
      'The Bulk Export menu allows you to export bulk data across the crawl such as all inlinks, all outlinks, all images, response codes, canonicals, hreflang and more as a CSV or spreadsheet.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'menu.reports': {
    title: 'Reports Menu',
    body:
      'The Reports menu outputs a selection of summary reports based on the crawl data, such as crawl overview, redirect chains, canonical errors, insecure content and hreflang issues.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'menu.visualisations': {
    title: 'Visualisations Menu',
    body:
      'Force-directed diagrams and tree graphs that visualise how the crawler discovered the site. Includes crawl visualisations, directory trees and Inlink Anchor Text word clouds.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'menu.view': {
    title: 'View Menu',
    body:
      'The View menu toggles which panes and tabs are shown in the main window, and lets you reset the layout to defaults.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'menu.mode': {
    title: 'Mode Menu',
    body:
      'Switch between Spider, List, SERP and Compare modes. Spider crawls a site from a seed URL; List audits a fixed URL set; SERP reviews titles/descriptions; Compare diffs two crawls.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/#mode',
  },
  'detail.inlinks': {
    title: 'Inlinks',
    body:
      'The Inlinks tab shows every URL that links to the selected page. The source URL, anchor text and link type (hyperlink, canonical, redirect, etc.) are listed for each inlink.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/#inlinks',
  },
  'detail.outlinks': {
    title: 'Outlinks',
    body:
      'The Outlinks tab shows every URL that the selected page links to. Columns include the destination URL, anchor text and link type.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/#outlinks',
  },
  'detail.images': {
    title: 'Image Details',
    body:
      'Lists every image referenced from the selected page, with the image URL, response code, size, and — where available — the alt text.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/#image-details',
  },
  'detail.resources': {
    title: 'Resources',
    body:
      'Every CSS, JavaScript, image and font resource the page loaded. A useful inventory when auditing page weight or third-party dependencies.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'detail.serp': {
    title: 'SERP Snippet',
    body:
      'Simulates how the page would appear in Google search results based on its title, meta description and URL. Reports whether the title or description would be truncated.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'detail.headers': {
    title: 'HTTP Headers',
    body:
      'The full set of HTTP response headers returned by the server for this URL, in the order the server sent them.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'detail.cookies': {
    title: 'Cookies',
    body:
      'Every cookie the server set via Set-Cookie response headers on this URL — name, value, domain, path, expiry, Secure / HttpOnly / SameSite flags.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'detail.source': {
    title: 'View Source',
    body:
      'The raw HTML response body as captured by the crawler. For non-HTML responses (images, binary, redirects) this is empty.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'detail.duplicates': {
    title: 'Duplicate Details',
    body:
      'URLs with identical content to the selected page (exact-match duplicates by content hash). Near-duplicate detection is a separate analysis.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'detail.structured_data': {
    title: 'Structured Data',
    body:
      'Every JSON-LD, Microdata and RDFa block extracted from the page. Use this to verify schema markup is present and well-formed.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
  'detail.overview': {
    title: 'Overview',
    body:
      'Summary of the selected URL — HTTP status, content type, response time, indexability, title/meta/headings and any findings raised against it.',
    href: 'https://www.screamingfrog.co.uk/seo-spider/user-guide/general/',
  },
};
