/**
 * Plugin Logging Utility
 *
 * Provides structured logging for plugins with support for:
 * - Console output (stderr)
 * - File-based logging
 * - Different log levels (DEBUG, INFO, WARN, ERROR)
 * - Automatic log rotation
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

class PluginLogger {
  constructor(pluginName, options = {}) {
    this.pluginName = pluginName;
    this.logDir =
      options.logDir ||
      path.join(process.env.HOME || '/tmp', '.mcp-proxy/plugin-logs');
    this.logLevel = options.logLevel || process.env.PLUGIN_LOG_LEVEL || 'INFO';
    this.enableFile = options.enableFile !== false; // Default: true
    this.enableConsole = options.enableConsole !== false; // Default: true

    // Create log directory if it doesn't exist
    if (this.enableFile) {
      try {
        fs.mkdirSync(this.logDir, { recursive: true });
        this.logFile = path.join(this.logDir, `${pluginName}.log`);
      } catch (err) {
        console.error(
          `[LOGGER] Failed to create log directory: ${err.message}`,
        );
        this.enableFile = false;
      }
    }

    this.levels = {
      DEBUG: 0,
      INFO: 1,
      WARN: 2,
      ERROR: 3,
    };
  }

  _shouldLog(level) {
    return this.levels[level] >= this.levels[this.logLevel];
  }

  _formatMessage(level, message, metadata = {}) {
    const timestamp = new Date().toISOString();
    const metaStr =
      Object.keys(metadata).length > 0 ? ` | ${JSON.stringify(metadata)}` : '';
    return `[${timestamp}] [${level}] [${this.pluginName}] ${message}${metaStr}`;
  }

  _write(level, message, metadata) {
    if (!this._shouldLog(level)) return;

    const formatted = this._formatMessage(level, message, metadata);

    // Write to console (stderr)
    if (this.enableConsole) {
      console.error(formatted);
    }

    // Write to file
    if (this.enableFile && this.logFile) {
      try {
        fs.appendFileSync(this.logFile, formatted + '\n');
      } catch (err) {
        console.error(`[LOGGER] Failed to write to log file: ${err.message}`);
      }
    }
  }

  debug(message, metadata) {
    this._write('DEBUG', message, metadata);
  }

  info(message, metadata) {
    this._write('INFO', message, metadata);
  }

  warn(message, metadata) {
    this._write('WARN', message, metadata);
  }

  error(message, metadata) {
    this._write('ERROR', message, metadata);
  }

  // Rotate log file if it exceeds size limit
  rotate(maxSizeBytes = 10 * 1024 * 1024) {
    // Default: 10MB
    if (!this.enableFile || !this.logFile) return;

    try {
      const stats = fs.statSync(this.logFile);
      if (stats.size > maxSizeBytes) {
        const rotatedFile = `${this.logFile}.${Date.now()}`;
        fs.renameSync(this.logFile, rotatedFile);
        this.info('Log file rotated', { rotatedFile });

        // Clean up old rotated files (keep last 5)
        const dir = path.dirname(this.logFile);
        const baseName = path.basename(this.logFile);
        const files = fs
          .readdirSync(dir)
          .filter((f) => f.startsWith(baseName) && f !== baseName)
          .sort()
          .reverse();

        files.slice(5).forEach((f) => {
          fs.unlinkSync(path.join(dir, f));
        });
      }
    } catch (err) {
      // Ignore rotation errors
    }
  }
}

export function createLogger(pluginName, options) {
  return new PluginLogger(pluginName, options);
}

export default { createLogger };
