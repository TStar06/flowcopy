// CC-Speak Cloud-Proxy
//
// Hält den zentralen Groq-API-Key als Server-Secret und reicht die Anfragen
// der CC-Speak-Apps durch (Chat-Completions für Cleanup/Übersetzung,
// Audio-Transkription für Cloud-ASR). Zählt Anfragen anonym in SQLite und
// liefert ein passwortgeschütztes Dashboard.
//
// Bewusst ohne npm-Dependencies (nur Node-Builtins, Node >= 24 wegen
// node:sqlite) — nichts zu installieren, nichts zu auditieren.
//
// Datenschutz: Diktat-Texte und Audio werden NIE gespeichert, nur Zähler
// (Art, Status, Dauer) und anonyme Telemetrie-Events ohne Personenbezug.

import { createServer } from "node:https";
import { request as httpsRequest } from "node:https";
import { readFileSync } from "node:fs";
import { DatabaseSync } from "node:sqlite";
import { createHash, timingSafeEqual } from "node:crypto";

const PORT = Number(process.env.PORT || 9443);
const GROQ_BASE = "api.groq.com";
const GROQ_API_KEY = required("GROQ_API_KEY");
const CC_APP_TOKEN = required("CC_APP_TOKEN");
const DASHBOARD_PASSWORD = required("DASHBOARD_PASSWORD");
const CERT_PATH = process.env.CERT_PATH || "/certs/server-cert.pem";
const KEY_PATH = process.env.KEY_PATH || "/certs/server-key.pem";
const DB_PATH = process.env.DB_PATH || "/data/ccspeak.db";
const MAX_REQUESTS_PER_DAY = Number(process.env.MAX_REQUESTS_PER_DAY || 20000);
const MAX_REQUESTS_PER_IP_PER_DAY = Number(
  process.env.MAX_REQUESTS_PER_IP_PER_DAY || 3000,
);
const UPSTREAM_TIMEOUT_MS = 25000;

function required(name) {
  const value = process.env[name];
  if (!value) {
    console.error(`Fehlende Umgebungsvariable: ${name}`);
    process.exit(1);
  }
  return value;
}

// ---------------------------------------------------------------------------
// Storage
// ---------------------------------------------------------------------------

const db = new DatabaseSync(DB_PATH);
db.exec(`
  CREATE TABLE IF NOT EXISTS requests (
    ts INTEGER NOT NULL,
    kind TEXT NOT NULL,
    status INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    ip_hash TEXT NOT NULL
  );
  CREATE INDEX IF NOT EXISTS idx_requests_ts ON requests(ts);
  CREATE TABLE IF NOT EXISTS events (
    ts INTEGER NOT NULL,
    kind TEXT NOT NULL,
    target_lang TEXT,
    outcome TEXT NOT NULL,
    reject_reason TEXT,
    app_version TEXT
  );
  CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts);
`);

const insertRequest = db.prepare(
  "INSERT INTO requests (ts, kind, status, duration_ms, ip_hash) VALUES (?, ?, ?, ?, ?)",
);
const insertEvent = db.prepare(
  "INSERT INTO events (ts, kind, target_lang, outcome, reject_reason, app_version) VALUES (?, ?, ?, ?, ?, ?)",
);
const countToday = db.prepare(
  "SELECT COUNT(*) AS n FROM requests WHERE ts >= ?",
);
const countTodayByIp = db.prepare(
  "SELECT COUNT(*) AS n FROM requests WHERE ts >= ? AND ip_hash = ?",
);

function startOfTodayMs() {
  const now = new Date();
  return new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
}

// Anonymisierte IP-Kennung nur fürs Tageslimit (kein Personenbezug im Dashboard).
function ipHash(ip) {
  return createHash("sha256")
    .update(`${CC_APP_TOKEN}:${ip}`)
    .digest("hex")
    .slice(0, 12);
}

// ---------------------------------------------------------------------------
// Auth-Helfer
// ---------------------------------------------------------------------------

function safeEqual(a, b) {
  const ba = Buffer.from(String(a));
  const bb = Buffer.from(String(b));
  if (ba.length !== bb.length) return false;
  return timingSafeEqual(ba, bb);
}

function hasAppToken(req) {
  const auth = req.headers["authorization"] || "";
  return auth.startsWith("Bearer ") && safeEqual(auth.slice(7), CC_APP_TOKEN);
}

function hasDashboardAuth(req) {
  const auth = req.headers["authorization"] || "";
  if (!auth.startsWith("Basic ")) return false;
  const decoded = Buffer.from(auth.slice(6), "base64").toString("utf8");
  return safeEqual(decoded, `admin:${DASHBOARD_PASSWORD}`);
}

// ---------------------------------------------------------------------------
// Upstream-Weiterleitung
// ---------------------------------------------------------------------------

function forwardToGroq(req, res, groqPath, { kind, bufferedBody = null }) {
  const started = Date.now();
  const headers = {
    authorization: `Bearer ${GROQ_API_KEY}`,
  };
  if (req.headers["content-type"]) {
    headers["content-type"] = req.headers["content-type"];
  }
  if (bufferedBody !== null) {
    headers["content-length"] = Buffer.byteLength(bufferedBody);
  } else if (req.headers["content-length"]) {
    headers["content-length"] = req.headers["content-length"];
  }

  const upstream = httpsRequest(
    { host: GROQ_BASE, path: groqPath, method: req.method, headers },
    (groqRes) => {
      res.writeHead(groqRes.statusCode || 502, {
        "content-type": groqRes.headers["content-type"] || "application/json",
      });
      groqRes.pipe(res);
      groqRes.on("end", () => {
        insertRequest.run(
          Date.now(),
          kind,
          groqRes.statusCode || 0,
          Date.now() - started,
          ipHash(req.socket.remoteAddress || "?"),
        );
      });
    },
  );

  upstream.setTimeout(UPSTREAM_TIMEOUT_MS, () => {
    upstream.destroy(new Error("upstream timeout"));
  });
  upstream.on("error", (err) => {
    console.error(`Upstream-Fehler (${kind}): ${err.message}`);
    insertRequest.run(
      Date.now(),
      kind,
      -1,
      Date.now() - started,
      ipHash(req.socket.remoteAddress || "?"),
    );
    if (!res.headersSent) {
      res.writeHead(502, { "content-type": "application/json" });
      res.end(JSON.stringify({ error: "upstream unreachable" }));
    } else {
      res.destroy();
    }
  });

  if (bufferedBody !== null) {
    upstream.end(bufferedBody);
  } else {
    req.pipe(upstream);
  }
}

function readBody(req, limitBytes) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    req.on("data", (chunk) => {
      size += chunk.length;
      if (size > limitBytes) {
        reject(new Error("body too large"));
        req.destroy();
        return;
      }
      chunks.push(chunk);
    });
    req.on("end", () => resolve(Buffer.concat(chunks)));
    req.on("error", reject);
  });
}

function deny(res, status, message) {
  res.writeHead(status, { "content-type": "application/json" });
  res.end(JSON.stringify({ error: message }));
}

function overLimit(req, res) {
  const today = startOfTodayMs();
  if (countToday.get(today).n >= MAX_REQUESTS_PER_DAY) {
    deny(res, 429, "daily limit reached");
    return true;
  }
  const perIp = countTodayByIp.get(
    today,
    ipHash(req.socket.remoteAddress || "?"),
  );
  if (perIp.n >= MAX_REQUESTS_PER_IP_PER_DAY) {
    deny(res, 429, "per-source daily limit reached");
    return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

function dashboardStats() {
  const now = Date.now();
  const ranges = [
    ["Heute", startOfTodayMs()],
    ["7 Tage", now - 7 * 86400000],
    ["30 Tage", now - 30 * 86400000],
  ];

  const perRange = ranges.map(([label, since]) => {
    const kinds = db
      .prepare(
        "SELECT kind, COUNT(*) AS n FROM requests WHERE ts >= ? GROUP BY kind",
      )
      .all(since);
    const errors = db
      .prepare(
        "SELECT status, COUNT(*) AS n FROM requests WHERE ts >= ? AND (status < 200 OR status >= 300) GROUP BY status ORDER BY n DESC",
      )
      .all(since);
    const outcomes = db
      .prepare(
        "SELECT outcome, COALESCE(reject_reason,'') AS reason, COUNT(*) AS n FROM events WHERE ts >= ? AND outcome != 'ok' GROUP BY outcome, reason ORDER BY n DESC",
      )
      .all(since);
    const langs = db
      .prepare(
        "SELECT COALESCE(target_lang,'?') AS lang, COUNT(*) AS n FROM events WHERE ts >= ? AND kind = 'translate' GROUP BY lang ORDER BY n DESC",
      )
      .all(since);
    const eventTotals = db
      .prepare(
        "SELECT kind, COUNT(*) AS n FROM events WHERE ts >= ? GROUP BY kind",
      )
      .all(since);
    return { label, kinds, errors, outcomes, langs, eventTotals };
  });

  return perRange;
}

function esc(s) {
  return String(s).replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ],
  );
}

function renderDashboard() {
  const stats = dashboardStats();
  const kindLabel = {
    cleanup: "KI-Formatierung",
    translate: "Übersetzung (LLM)",
    asr: "Cloud-Spracherkennung",
    models: "Modell-Listen",
    dictation: "Diktate",
  };
  const statusLabel = (s) =>
    s === -1
      ? "Groq nicht erreichbar/Timeout"
      : s === 429
        ? "Rate-Limit (429)"
        : `HTTP ${s}`;

  const sections = stats
    .map(({ label, kinds, errors, outcomes, langs, eventTotals }) => {
      const row = (cells) =>
        `<tr>${cells.map((c) => `<td>${esc(c)}</td>`).join("")}</tr>`;
      const table = (headers, rows) =>
        rows.length
          ? `<table><tr>${headers.map((h) => `<th>${esc(h)}</th>`).join("")}</tr>${rows.join("")}</table>`
          : `<p class="empty">keine Daten</p>`;

      return `
      <section>
        <h2>${esc(label)}</h2>
        <h3>Server-Anfragen</h3>
        ${table(
          ["Art", "Anzahl"],
          kinds.map((k) => row([kindLabel[k.kind] || k.kind, k.n])),
        )}
        <h3>Diktate laut App (Telemetrie)</h3>
        ${table(
          ["Art", "Anzahl"],
          eventTotals.map((k) => row([kindLabel[k.kind] || k.kind, k.n])),
        )}
        <h3>Übersetzungen nach Zielsprache</h3>
        ${table(
          ["Sprache", "Anzahl"],
          langs.map((l) => row([l.lang, l.n])),
        )}
        <h3>Fehler (Server)</h3>
        ${table(
          ["Fehler", "Anzahl"],
          errors.map((e) => row([statusLabel(e.status), e.n])),
        )}
        <h3>Verworfene / fehlgeschlagene Verarbeitung (App)</h3>
        ${table(
          ["Ergebnis", "Grund", "Anzahl"],
          outcomes.map((o) => row([o.outcome, o.reason || "—", o.n])),
        )}
      </section>`;
    })
    .join("\n");

  return `<!doctype html>
<html lang="de"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CC Speak — Nutzung</title>
<style>
  body { font-family: system-ui, sans-serif; background: #111; color: #eee; margin: 2rem auto; max-width: 720px; padding: 0 1rem; }
  h1 { font-size: 1.4rem; } h2 { font-size: 1.15rem; margin-top: 2rem; border-bottom: 1px solid #333; padding-bottom: .3rem; }
  h3 { font-size: .95rem; color: #aaa; margin: 1rem 0 .3rem; }
  table { border-collapse: collapse; width: 100%; }
  td, th { text-align: left; padding: .25rem .6rem; border-bottom: 1px solid #2a2a2a; font-size: .9rem; }
  th { color: #888; font-weight: 600; }
  .empty { color: #555; font-size: .85rem; margin: .2rem 0; }
  footer { margin-top: 2.5rem; color: #555; font-size: .8rem; }
</style></head><body>
<h1>CC Speak — Nutzungsübersicht</h1>
${sections}
<footer>Anonyme Zähler ohne Personenbezug. Diktat-Inhalte und Audio werden nicht gespeichert.</footer>
</body></html>`;
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

const server = createServer(
  { cert: readFileSync(CERT_PATH), key: readFileSync(KEY_PATH) },
  async (req, res) => {
    try {
      const url = new URL(req.url || "/", "https://localhost");
      const path = url.pathname;

      if (path === "/dashboard" && req.method === "GET") {
        if (!hasDashboardAuth(req)) {
          res.writeHead(401, {
            "www-authenticate": 'Basic realm="CC Speak Dashboard"',
          });
          res.end("Auth required");
          return;
        }
        res.writeHead(200, { "content-type": "text/html; charset=utf-8" });
        res.end(renderDashboard());
        return;
      }

      if (path === "/healthz" && req.method === "GET") {
        res.writeHead(200, { "content-type": "text/plain" });
        res.end("ok");
        return;
      }

      if (!hasAppToken(req)) {
        deny(res, 401, "unauthorized");
        return;
      }

      if (path === "/v1/chat/completions" && req.method === "POST") {
        if (overLimit(req, res)) return;
        // Body wird gebuffert, um die Art (Cleanup vs. Übersetzung) am Modell
        // zu erkennen — Inhalte werden nicht gespeichert.
        const body = await readBody(req, 1024 * 1024);
        let kind = "cleanup";
        try {
          const model = String(JSON.parse(body.toString("utf8")).model || "");
          if (model.includes("70b")) kind = "translate";
        } catch {
          /* kind bleibt cleanup */
        }
        forwardToGroq(req, res, "/openai/v1/chat/completions", {
          kind,
          bufferedBody: body,
        });
        return;
      }

      if (path === "/v1/audio/transcriptions" && req.method === "POST") {
        if (overLimit(req, res)) return;
        forwardToGroq(req, res, "/openai/v1/audio/transcriptions", {
          kind: "asr",
        });
        return;
      }

      if (path === "/v1/models" && req.method === "GET") {
        forwardToGroq(req, res, "/openai/v1/models", { kind: "models" });
        return;
      }

      if (path === "/v1/cc/telemetry" && req.method === "POST") {
        const body = await readBody(req, 16 * 1024);
        try {
          const e = JSON.parse(body.toString("utf8"));
          insertEvent.run(
            Date.now(),
            String(e.kind || "?").slice(0, 24),
            e.target_lang ? String(e.target_lang).slice(0, 8) : null,
            String(e.outcome || "?").slice(0, 24),
            e.reject_reason ? String(e.reject_reason).slice(0, 64) : null,
            e.app_version ? String(e.app_version).slice(0, 16) : null,
          );
        } catch {
          deny(res, 400, "invalid event");
          return;
        }
        res.writeHead(204);
        res.end();
        return;
      }

      deny(res, 404, "not found");
    } catch (err) {
      console.error(`Request-Fehler: ${err.message}`);
      if (!res.headersSent) deny(res, 500, "internal error");
    }
  },
);

server.listen(PORT, () => {
  console.log(`CC-Speak-Proxy lauscht auf :${PORT}`);
});
