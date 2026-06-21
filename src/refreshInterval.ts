// How often the app re-pulls from Google Health while it's actually on screen.
// We only poll when the panel is focused or the hover glance is showing — never
// in the background — so an idle, hidden app uses no bandwidth. (Upstream is the
// real limit: the phone only syncs to Google's cloud every few minutes, so
// polling much faster than this just re-fetches identical numbers.)
export const REFRESH_MS = 15_000;
