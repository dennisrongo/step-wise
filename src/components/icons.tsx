// Monochrome SVG glyphs — currentColor, no emoji.

export function Footprint({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <circle cx="13.1" cy="3.3" r="1.05" />
      <circle cx="15.5" cy="2.9" r="1" />
      <circle cx="17.6" cy="3.6" r="0.9" />
      <path d="M15 4.6c1.8 0 3.1 1.7 3.1 4.1 0 2.2-1.1 4.6-2.6 5.8-1 .8-2.2.9-3 .1-.9-.9-1.2-2.4-1-4.3.4-3.4 1.8-5.7 3.5-5.7z" />
      <path d="M9.4 13.9c1.4-.2 2.5 1.1 2.6 2.9.1 2-1 3.8-2.5 4-1.4.2-2.6-1.1-2.7-2.9-.1-2 1-3.8 2.6-4z" />
    </svg>
  );
}

export function LinkSlash({ size = 22 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M9.5 14.5l5-5" />
      <path d="M11.5 6.6l1-1a3.6 3.6 0 0 1 5.1 5.1l-1 1" />
      <path d="M12.5 17.4l-1 1a3.6 3.6 0 0 1-5.1-5.1l1-1" />
      <path d="M3.5 3.5l17 17" />
    </svg>
  );
}

export function ChevronUp() {
  return (
    <svg viewBox="0 0 8 8" width="7" height="7" fill="currentColor" aria-hidden="true">
      <path d="M4 1.4 7.2 6.4H.8z" />
    </svg>
  );
}

export function ChevronDown() {
  return (
    <svg viewBox="0 0 8 8" width="7" height="7" fill="currentColor" aria-hidden="true">
      <path d="M4 6.6 .8 1.6H7.2z" />
    </svg>
  );
}

export function RefreshIcon({ size = 14 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M20 11A8 8 0 1 0 18 16" />
      <path d="M20 5v6h-6" />
    </svg>
  );
}
