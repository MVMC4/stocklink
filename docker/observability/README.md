# StockLink — API & health monitoring

A self-contained **Prometheus + Grafana** stack for the Rust API. Prometheus
scrapes the API's `GET /metrics`; Grafana is provisioned with the datasource and
the **"StockLink — API & Health"** dashboard, so there is no manual setup.

| Component | Image | Notes |
| --- | --- | --- |
| Prometheus | `prom/prometheus:v3.14.0` | Prometheus 3 rejects scrapes without a valid `Content-Type`; the API sends `text/plain; version=0.0.4; charset=utf-8` (`backend/src/controllers/health.rs`) |
| Grafana | `grafana/grafana:13.2.1` | The provisioned dashboard uses only React panels (`stat`, `timeseries`, `gauge`), so Grafana 12's Angular removal does not affect it; the datasource has a fixed UID (`stocklink-prometheus`) |

Both containers' health checks probe `127.0.0.1`, not `localhost` (see
`docker/README.md` for why).

## What the API exposes

`GET /metrics` (Prometheus text format, unauthenticated — never publish it; the
gateways return 404 for it):

| Metric | Type | Meaning |
| --- | --- | --- |
| `stocklink_http_requests_total` | counter | all HTTP requests |
| `stocklink_http_requests_in_flight` | gauge | requests currently being handled |
| `stocklink_http_responses_total{class="2xx\|4xx\|5xx"}` | counter | responses by status class |
| `stocklink_rate_limited_requests_total` | counter | requests rejected by the rate limiter |
| `stocklink_up` | gauge | `1` whenever the process answers a scrape |
| `stocklink_dependency_up{dependency="database\|redis"}` | gauge | readiness ping result, refreshed each scrape |
| `stocklink_build_info{version="…"}` | gauge | build version (value always `1`) |

## Run it

```bash
docker compose -f docker/observability/docker-compose.observability.yml up -d
```

- Grafana: <http://localhost:3019> — `admin` / `admin` locally (override with
  `GF_SECURITY_ADMIN_PASSWORD`). The dashboard is under **Dashboards →
  StockLink**.
- Prometheus: <http://localhost:9095>

### Pointing at the API

Set `STOCKLINK_API_TARGET` (default `host.docker.internal:8080`):

| Where the API runs | `STOCKLINK_API_TARGET` |
| --- | --- |
| `cargo run` on the host | `host.docker.internal:8080` (default) |
| `docker/compose.dev.yml` (API published on `HF_DEV_API_PORT`, default 8280) | `host.docker.internal:8280` |
| Same Docker network as the API | `api:8080`, after `docker network connect stocklink-dev-app stocklink-prometheus` |

```bash
STOCKLINK_API_TARGET=host.docker.internal:8280 \
  docker compose -f docker/observability/docker-compose.observability.yml up -d
```

Staging and production bind the API's direct port to `127.0.0.1`, so run
Prometheus on the same host or network rather than scraping across machines.

## Linking from other tools

StockLink's admin console (WO-04, WO-10) is expected to link to this
dashboard through a non-secret `ADMIN_GRAFANA_URL` setting, and Grafana
alerts should go to an on-call channel once one exists. Neither the admin
console nor an operations runbook exists yet in this repository — this
section will point at real docs once WO-10 adds them, rather than at paths
that do not exist.

## Production notes

- Set a real `GF_SECURITY_ADMIN_PASSWORD`, terminate TLS at the proxy, and keep
  both `/metrics` and Grafana off the public internet (allowlist the scrape
  source; put Grafana behind SSO).
- `--storage.tsdb.retention.time=15d` — raise it or add remote-write for
  long-term storage.
- Alerting: add Prometheus alert rules (`stocklink_up == 0`,
  `stocklink_dependency_up == 0`, a high 5xx ratio) and wire Grafana contact
  points. Alert rules and any Kubernetes deployment manifests are WO-10 work;
  none exist in this repository yet.
- Upgrading: Prometheus 3 no longer adds default ports to scrape targets, so
  always include the port in `STOCKLINK_API_TARGET`. Check the Prometheus
  migration guide and Grafana release notes before future major upgrades.
