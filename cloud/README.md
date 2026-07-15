# CC-Speak Cloud-Proxy

Hält den zentralen Groq-API-Key als Server-Secret, reicht App-Anfragen durch
(Chat-Completions, Audio-Transkription), zählt Nutzung anonym (SQLite) und
liefert ein passwortgeschütztes Dashboard unter `/dashboard`.

Keine npm-Dependencies — nur Node-Builtins (Node >= 24 wegen `node:sqlite`).

## Deployment (VPS, Docker)

Verzeichnisstruktur auf dem Server:

```
/opt/ccspeak/
├── app/server.mjs      # dieser Dienst
├── certs/              # Mini-CA + Server-Zertifikat (IP-SAN), siehe unten
├── data/               # SQLite (ccspeak.db)
└── .env                # Secrets — NIE ins Repo
```

`.env` (chmod 600):

```
GROQ_API_KEY=gsk_...
CC_APP_TOKEN=<zufälliger Token, gleicher Wert wie beim App-Release-Build>
DASHBOARD_PASSWORD=<Dashboard-Passwort>
```

Build + Start:

```bash
docker build -t ccspeak-proxy /opt/ccspeak/app
docker run -d --name ccspeak-proxy --restart unless-stopped \
  -p 9443:9443 \
  --env-file /opt/ccspeak/.env \
  -v /opt/ccspeak/certs:/certs:ro \
  -v /opt/ccspeak/data:/data \
  ccspeak-proxy
```

Zertifikate: eigene Mini-CA mit Server-Zertifikat auf IP-SAN (kein
Let's Encrypt nötig, die App bettet das CA-Zertifikat ein). Erzeugung siehe
Projekt-Handbuch; Dateien: `server-cert.pem`, `server-key.pem` (der CA-Key
`cc-ca-key.pem` wird nur zum Signieren gebraucht).

## Endpoints

| Pfad | Auth | Zweck |
|---|---|---|
| `POST /v1/chat/completions` | Bearer App-Token | LLM-Passthrough an Groq (Cleanup 8B / Übersetzung 70B) |
| `POST /v1/audio/transcriptions` | Bearer App-Token | Cloud-ASR-Passthrough (Audio wird gestreamt, nie gespeichert) |
| `GET /v1/models` | Bearer App-Token | Modell-Liste |
| `POST /v1/cc/telemetry` | Bearer App-Token | anonyme App-Events (kind, outcome, reject_reason, app_version) |
| `GET /dashboard` | Basic `admin:<DASHBOARD_PASSWORD>` | Nutzungsübersicht |
| `GET /healthz` | — | Liveness |

## Datenschutz

Es werden ausschließlich Zähler gespeichert (Art, HTTP-Status, Dauer,
anonymisierter Quell-Hash fürs Tageslimit) und anonyme App-Events. Diktat-
Inhalte und Audio werden zu keinem Zeitpunkt persistiert.

## Limits

`MAX_REQUESTS_PER_DAY` (Default 20000) gesamt, `MAX_REQUESTS_PER_IP_PER_DAY`
(Default 3000) pro Quelle — Schutz gegen Missbrauch bei Token-Leak. Token bei
Verdacht rotieren: neuen Wert in `.env` setzen, Container neu starten, App-
Release mit neuem eingebettetem Token bauen.
