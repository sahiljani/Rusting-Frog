// Shape of the Screaming Frog top-menu tree. Only the structure is
// modeled in K1 — actual actions are wired in later batches (K4
// Configuration, K7+ for Bulk Export / Reports). Items with action === null
// render as disabled stubs with a "Coming soon" badge.

export type MenuAction =
  | { kind: 'command'; id: string }
  | { kind: 'placeholder'; label?: string };

export interface MenuItem {
  label: string;
  action?: MenuAction;
  shortcut?: string;
  disabled?: boolean;
  separator?: boolean;
  children?: MenuItem[];
  docKey?: string;
}

export interface TopMenu {
  key: string;
  label: string;
  docKey?: string;
  items: MenuItem[];
}

export const SF_MENUS: TopMenu[] = [
  {
    key: 'file',
    label: 'File',
    docKey: 'menu.file',
    items: [
      { label: 'New Crawl', shortcut: 'Ctrl+N', action: { kind: 'command', id: 'file.new' } },
      { label: 'History…', shortcut: 'Ctrl+H', action: { kind: 'command', id: 'file.history' } },
      { separator: true, label: '' },
      { label: 'Save As…', disabled: true, action: { kind: 'placeholder' } },
      { separator: true, label: '' },
      { label: 'Clear Saved Token', action: { kind: 'command', id: 'file.clear_token' } },
      { label: 'Clear History', action: { kind: 'command', id: 'file.clear_history' } },
      { separator: true, label: '' },
      { label: 'Exit', disabled: true, action: { kind: 'placeholder' } },
    ],
  },
  {
    key: 'view',
    label: 'View',
    docKey: 'menu.view',
    items: [
      { label: 'Show Sidebar', disabled: true, action: { kind: 'placeholder' } },
      { label: 'Show Detail Pane', disabled: true, action: { kind: 'placeholder' } },
      { separator: true, label: '' },
      { label: 'Reset Layout', disabled: true, action: { kind: 'placeholder' } },
    ],
  },
  {
    key: 'mode',
    label: 'Mode',
    docKey: 'menu.mode',
    items: [
      { label: 'Spider', action: { kind: 'command', id: 'mode.spider' }, docKey: 'mode.spider' },
      { label: 'List', disabled: true, action: { kind: 'placeholder' }, docKey: 'mode.list' },
      { label: 'SERP', disabled: true, action: { kind: 'placeholder' }, docKey: 'mode.serp' },
      { label: 'Compare', disabled: true, action: { kind: 'placeholder' } },
    ],
  },
  {
    key: 'configuration',
    label: 'Configuration',
    docKey: 'menu.configuration',
    items: [
      { label: 'Spider…', action: { kind: 'command', id: 'config.open' } },
      { label: 'Robots.txt…', disabled: true, action: { kind: 'placeholder' } },
      { label: 'URL Rewriting…', disabled: true, action: { kind: 'placeholder' } },
      { label: 'Include / Exclude…', disabled: true, action: { kind: 'placeholder' } },
      { label: 'Speed…', disabled: true, action: { kind: 'placeholder' } },
      { label: 'User-Agent…', disabled: true, action: { kind: 'placeholder' } },
      { separator: true, label: '' },
      { label: 'API Access', disabled: true, action: { kind: 'placeholder' } },
    ],
  },
  {
    key: 'bulk_export',
    label: 'Bulk Export',
    docKey: 'menu.bulk_export',
    items: [
      { label: 'All Inlinks', disabled: true, action: { kind: 'placeholder' } },
      { label: 'All Outlinks', disabled: true, action: { kind: 'placeholder' } },
      { label: 'All Images', disabled: true, action: { kind: 'placeholder' } },
      { label: 'All Response Codes', disabled: true, action: { kind: 'placeholder' } },
      { label: 'All Canonicals', disabled: true, action: { kind: 'placeholder' } },
      { label: 'All hreflang', disabled: true, action: { kind: 'placeholder' } },
    ],
  },
  {
    key: 'reports',
    label: 'Reports',
    docKey: 'menu.reports',
    items: [
      { label: 'Crawl Overview', disabled: true, action: { kind: 'placeholder' } },
      { label: 'Redirect Chains', disabled: true, action: { kind: 'placeholder' } },
      { label: 'Canonical Errors', disabled: true, action: { kind: 'placeholder' } },
      { label: 'Insecure Content', disabled: true, action: { kind: 'placeholder' } },
      { label: 'hreflang', disabled: true, action: { kind: 'placeholder' } },
    ],
  },
  {
    key: 'visualisations',
    label: 'Visualisations',
    docKey: 'menu.visualisations',
    items: [
      { label: 'Crawl Visualisation', disabled: true, action: { kind: 'placeholder' } },
      { label: 'Directory Tree', disabled: true, action: { kind: 'placeholder' } },
      { label: 'Inlink Anchor Text Word Cloud', disabled: true, action: { kind: 'placeholder' } },
    ],
  },
];
