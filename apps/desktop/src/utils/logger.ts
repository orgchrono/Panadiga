/**
 * @file logger.ts
 * @brief Structured Frontend Telemetry & Logging Facade powered by Pino + OpenTelemetry API.
 *
 * Replaces ad-hoc console.* calls and custom severity tables with industry-standard
 * Pino level filtering, OpenTelemetry tracer spans/events, and pluggable IPC/test sinks.
 */

import { SpanStatusCode, trace } from '@opentelemetry/api';
import pino, { type DestinationStream, type Logger as PinoLogger } from 'pino';
import type { RuntimeLogLevel } from '@/models';
import { isRecord } from './typeGuards';

export type LogSeverity = 'debug' | 'info' | 'warn' | 'error';
export type PinoFilterLevel = LogSeverity | 'silent' | 'trace';

export interface LogMessage {
  readonly timestamp: string;
  readonly severity: LogSeverity;
  readonly subsystem: string;
  readonly message: string;
  readonly details?: unknown;
}

export type LogSink = (entry: LogMessage) => void;

export interface StructuredLogger {
  readonly setMinLevel: (level: PinoFilterLevel) => void;
  readonly addSink: (sink: LogSink) => () => void;
  readonly debug: (subsystem: string, message: string, details?: unknown) => void;
  readonly info: (subsystem: string, message: string, details?: unknown) => void;
  readonly warn: (subsystem: string, message: string, details?: unknown) => void;
  readonly error: (subsystem: string, message: string, details?: unknown) => void;
}

const VALID_SEVERITIES: Readonly<Record<string, LogSeverity>> = Object.freeze({
  debug: 'debug',
  trace: 'debug',
  info: 'info',
  warn: 'warn',
  error: 'error',
});

/** Maps Rust `RuntimeLogLevel` (`off`..`trace`) cleanly to Pino filter levels. */
export function mapRuntimeLogLevelToPino(level: RuntimeLogLevel): PinoFilterLevel {
  return level === 'off' ? 'silent' : level;
}

function toSeverity(raw: unknown): LogSeverity {
  if (typeof raw === 'string' && raw in VALID_SEVERITIES) {
    return VALID_SEVERITIES[raw];
  }
  return 'info';
}

function readRecordField(obj: object, key: string): unknown {
  return Object.entries(obj).find(([k]) => k === key)?.[1];
}

function recordOtelEvent(entry: LogMessage): void {
  const tracer = trace.getTracer('panadiga-desktop');
  const span = tracer.startSpan(`ui.log.${entry.subsystem.toLowerCase()}`, {
    attributes: {
      'log.severity': entry.severity,
      'panadiga.subsystem': entry.subsystem,
      'log.message': entry.message,
    },
  });
  if (entry.severity === 'error') {
    span.setStatus({ code: SpanStatusCode.ERROR, message: entry.message });
    if (entry.details instanceof Error) {
      span.recordException(entry.details);
    }
  }
  span.end();
}

/**
 * Closed factory closure binding Pino + OpenTelemetry API behind the immutable StructuredLogger interface.
 */
export function createLogger(initialLevel: PinoFilterLevel = 'info'): StructuredLogger {
  let sinks: readonly LogSink[] = [];

  const emitToSinks = (rawPayload: object): void => {
    const rawTime = readRecordField(rawPayload, 'time');
    const eventDate = typeof rawTime === 'number' ? new Date(rawTime) : new Date();
    const timestamp = eventDate.toTimeString().split(' ')[0];

    const entry: LogMessage = Object.freeze({
      timestamp,
      severity: toSeverity(readRecordField(rawPayload, 'severity')),
      subsystem: String(readRecordField(rawPayload, 'subsystem') ?? 'APP'),
      message: String(readRecordField(rawPayload, 'msg') ?? ''),
      details: readRecordField(rawPayload, 'details'),
    });

    recordOtelEvent(entry);

    sinks.forEach((sink) => {
      try {
        sink(entry);
      } catch {
        // Telemetry sinks must never disrupt hardware or UI workflows
      }
    });
  };

  const nodeDestination: DestinationStream = {
    write: (rawLine: string) => {
      try {
        const parsed: unknown = JSON.parse(rawLine);
        if (isRecord(parsed)) {
          emitToSinks(parsed);
        }
      } catch {
        // Ignore non-JSON trailing fragments cleanly
      }
    },
  };

  const levelFormatter = (label: string): { readonly severity: string } => ({ severity: label });

  const buildPino = (level: PinoFilterLevel): PinoLogger =>
    pino(
      {
        level,
        formatters: { level: levelFormatter },
        browser: {
          asObject: true,
          serialize: true,
          formatters: { level: levelFormatter },
          write: emitToSinks,
        },
      },
      nodeDestination,
    );

  let pinoInstance: PinoLogger = buildPino(initialLevel);

  const dispatch = (
    severity: LogSeverity,
    subsystem: string,
    message: string,
    details?: unknown,
  ): void => {
    const payload = details === undefined ? { subsystem } : { subsystem, details };
    pinoInstance[severity](payload, message);
  };

  return Object.freeze({
    setMinLevel: (level: PinoFilterLevel) => {
      pinoInstance = buildPino(level);
    },
    addSink: (sink: LogSink) => {
      sinks = [...sinks, sink];
      return () => {
        sinks = sinks.filter((candidate) => candidate !== sink);
      };
    },
    debug: (subsystem: string, message: string, details?: unknown) =>
      dispatch('debug', subsystem, message, details),
    info: (subsystem: string, message: string, details?: unknown) =>
      dispatch('info', subsystem, message, details),
    warn: (subsystem: string, message: string, details?: unknown) =>
      dispatch('warn', subsystem, message, details),
    error: (subsystem: string, message: string, details?: unknown) =>
      dispatch('error', subsystem, message, details),
  });
}

export const logger = createLogger();
