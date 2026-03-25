import {
  mkdirSync,
  symlinkSync,
  existsSync,
  writeFileSync,
  rmSync,
  readdirSync,
} from "node:fs";
import { join } from "node:path";
import { homedir, tmpdir } from "node:os";
import { appendFileSync } from "node:fs";
import type { RoleId } from "./role-config";
import { buildStarlarkRules } from "./role-config";

const LOG_FILE = "/tmp/agentbridge.log";
const SESSION_PREFIX = "agentbridge-";

export interface SessionConfig {
  sessionId: string;
  roleId: RoleId;
  projectDir?: string;
  bridgePath?: string;
  controlPort?: number;
}

export interface SessionPaths {
  codexHome: string;
  authJson: string;
  rulesDir: string;
  rulesFile: string;
}

/**
 * Manages CODEX_HOME temporary directories for isolated Codex sessions.
 *
 * Lifecycle:
 *   createSession() → mkdirSync + symlink auth.json + write Starlark rules
 *   destroySession() → rm -rf temp dir
 *   cleanupAll() → destroy all sessions + scan /tmp for stale dirs
 */
export class SessionManager {
  private sessions = new Map<string, SessionPaths>();

  constructor() {
    // Clean up stale sessions from previous daemon runs on startup
    this.cleanupStale();
  }

  /**
   * Create an isolated CODEX_HOME for a session.
   * Structure:
   *   /tmp/agentbridge-<sessionId>/codex/
   *   ├── auth.json → symlink → ~/.codex/auth.json
   *   └── rules/
   *       └── role.rules
   */
  createSession(config: SessionConfig): SessionPaths {
    const base = join(tmpdir(), `${SESSION_PREFIX}${config.sessionId}`);
    const codexHome = join(base, "codex");
    const rulesDir = join(codexHome, "rules");

    mkdirSync(rulesDir, { recursive: true, mode: 0o700 });

    // Symlink auth.json (read-only reference, no copy)
    const originalAuth = join(homedir(), ".codex", "auth.json");
    const authJson = join(codexHome, "auth.json");
    if (existsSync(originalAuth) && !existsSync(authJson)) {
      try {
        symlinkSync(originalAuth, authJson);
        this.log(`Symlinked auth.json for session ${config.sessionId}`);
      } catch (err: any) {
        this.log(`WARN: Failed to symlink auth.json: ${err.message}`);
      }
    }

    // Write Starlark rules for role enforcement
    const rulesFile = join(rulesDir, "role.rules");
    const rules = buildStarlarkRules(config.roleId);
    if (rules) {
      writeFileSync(rulesFile, rules, "utf-8");
      this.log(
        `Wrote Starlark rules for role ${config.roleId} in session ${config.sessionId}`,
      );
    }

    // Write MCP config for agentbridge communication
    if (config.bridgePath) {
      const mcpJson = join(codexHome, "mcp.json");
      writeFileSync(
        mcpJson,
        JSON.stringify(
          {
            mcpServers: {
              agentbridge: {
                command: "bun",
                args: ["run", config.bridgePath],
                env: {
                  AGENTBRIDGE_CONTROL_PORT: String(config.controlPort ?? 4502),
                  AGENTBRIDGE_AGENT: "codex",
                },
              },
            },
          },
          null,
          2,
        ),
        "utf-8",
      );
      this.log(`Wrote mcp.json for session ${config.sessionId}`);
    }

    const paths: SessionPaths = { codexHome, authJson, rulesDir, rulesFile };
    this.sessions.set(config.sessionId, paths);
    this.log(`Created session ${config.sessionId} at ${codexHome}`);
    return paths;
  }

  /**
   * Destroy a single session's temp directory.
   */
  destroySession(sessionId: string): void {
    const paths = this.sessions.get(sessionId);
    if (!paths) return;
    this.sessions.delete(sessionId);

    try {
      // Go up one level to remove the agentbridge-<sessionId> dir
      const base = join(paths.codexHome, "..");
      rmSync(base, { recursive: true, force: true });
      this.log(`Destroyed session ${sessionId}`);
    } catch (err: any) {
      this.log(`WARN: Failed to destroy session ${sessionId}: ${err.message}`);
    }
  }

  /**
   * Destroy all active sessions. Called on daemon shutdown.
   */
  cleanupAll(): void {
    for (const sessionId of this.sessions.keys()) {
      this.destroySession(sessionId);
    }
    this.cleanupStale();
  }

  /**
   * Scan /tmp for leftover agentbridge-* dirs from crashed previous runs.
   */
  private cleanupStale(): void {
    try {
      const tmp = tmpdir();
      const entries = readdirSync(tmp);
      for (const entry of entries) {
        if (
          entry.startsWith(SESSION_PREFIX) &&
          !this.sessions.has(entry.slice(SESSION_PREFIX.length))
        ) {
          const fullPath = join(tmp, entry);
          try {
            rmSync(fullPath, { recursive: true, force: true });
            this.log(`Cleaned up stale session dir: ${entry}`);
          } catch {}
        }
      }
    } catch {}
  }

  getSession(sessionId: string): SessionPaths | undefined {
    return this.sessions.get(sessionId);
  }

  get activeSessions(): string[] {
    return Array.from(this.sessions.keys());
  }

  private log(msg: string) {
    const line = `[${new Date().toISOString()}] [SessionManager] ${msg}\n`;
    process.stderr.write(line);
    try {
      appendFileSync(LOG_FILE, line);
    } catch {}
  }
}
