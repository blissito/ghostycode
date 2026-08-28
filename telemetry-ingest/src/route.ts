/**
 * The one route.
 *
 * This lives in its own module rather than next to the handler because the
 * Workers runtime treats **every named export of the entrypoint module** as an
 * entrypoint: exporting a plain string from `src/index.ts` fails the worker at
 * startup with "Incorrect type for map entry: the provided value is not of type
 * 'function or ExportedHandler'". `src/index.ts` therefore has exactly one
 * export, the default handler, and every shared value the tests need lives
 * somewhere else.
 */

/** The only path this service answers. Every other path is 404. */
export const INGEST_PATH = "/v1/telemetry";
