// All UI-rendered timestamps go through this util so we display a single
// timezone (America/Vancouver) regardless of the browser's locale. DST
// is handled by Intl automatically — PDT in summer, PST in winter.

const ZONE = 'America/Vancouver';

const dateTimeFormatter = new Intl.DateTimeFormat('en-CA', {
  timeZone: ZONE,
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hour12: false,
  timeZoneName: 'short',
});

const timeOnlyFormatter = new Intl.DateTimeFormat('en-CA', {
  timeZone: ZONE,
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hour12: false,
});

const tzShortFormatter = new Intl.DateTimeFormat('en-CA', {
  timeZone: ZONE,
  timeZoneName: 'short',
});

export function formatVictoria(input: string | number | Date | null | undefined): string {
  if (input == null) return '—';
  const d = input instanceof Date ? input : new Date(input);
  if (Number.isNaN(d.getTime())) return String(input);
  return dateTimeFormatter.format(d);
}

export function formatVictoriaTime(input: string | number | Date | null | undefined): string {
  if (input == null) return '—';
  const d = input instanceof Date ? input : new Date(input);
  if (Number.isNaN(d.getTime())) return String(input);
  return timeOnlyFormatter.format(d);
}

export function victoriaZoneAbbr(at: Date = new Date()): string {
  const parts = tzShortFormatter.formatToParts(at);
  return parts.find((p) => p.type === 'timeZoneName')?.value ?? '';
}

export function nowInVictoria(): string {
  return formatVictoria(new Date());
}
