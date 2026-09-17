//! Admin console "link out" configuration (contract 09): Grafana, Prometheus,
//! Jira and status-page URLs, plus a docs link. All non-secret and all
//! optional — an absent var means the console shows no tile for it, never a
//! placeholder or a fabricated URL.

use stocklink_shared::config::non_empty_env;

#[derive(Debug, Clone, Default)]
pub struct AdminLinksConfig {
    pub grafana_url: Option<String>,
    pub prometheus_url: Option<String>,
    pub jira_url: Option<String>,
    pub status_url: Option<String>,
    pub docs_url: Option<String>,
}

/// Loads the admin console deep links from environment variables. Every field
/// is optional and unvalidated beyond "non-empty" — these are link-outs, not
/// endpoints this process calls, so there is nothing to fail startup over.
pub fn load_admin_links_config() -> AdminLinksConfig {
    AdminLinksConfig {
        grafana_url: non_empty_env("ADMIN_GRAFANA_URL"),
        prometheus_url: non_empty_env("ADMIN_PROMETHEUS_URL"),
        jira_url: non_empty_env("ADMIN_JIRA_URL"),
        status_url: non_empty_env("ADMIN_STATUS_URL"),
        docs_url: non_empty_env("ADMIN_DOCS_URL"),
    }
}

#[cfg(test)]
mod tests {
    use super::load_admin_links_config;

    #[test]
    fn absent_vars_yield_none_not_placeholders() {
        for key in [
            "ADMIN_GRAFANA_URL",
            "ADMIN_PROMETHEUS_URL",
            "ADMIN_JIRA_URL",
            "ADMIN_STATUS_URL",
            "ADMIN_DOCS_URL",
        ] {
            std::env::remove_var(key);
        }
        let cfg = load_admin_links_config();
        assert!(cfg.grafana_url.is_none());
        assert!(cfg.prometheus_url.is_none());
        assert!(cfg.jira_url.is_none());
        assert!(cfg.status_url.is_none());
        assert!(cfg.docs_url.is_none());
    }
}
