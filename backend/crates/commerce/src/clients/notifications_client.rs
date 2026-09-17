//! HTTP client for the notifications service's `/internal/*` API. Commerce
//! fires-and-forgets a notification request after an order event; a
//! delivery failure here is logged and swallowed (the order/settlement
//! itself already committed) — never surfaced as a checkout failure.

use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NotificationsClient {
    http: reqwest::Client,
    base_url: String,
    internal_token: String,
}

impl NotificationsClient {
    pub fn new(base_url: String, internal_token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            internal_token,
        }
    }

    async fn post(
        &self,
        account_id: Uuid,
        kind: &str,
        title: &str,
        body: &str,
        data: serde_json::Value,
    ) {
        let result = self
            .http
            .post(format!("{}/internal/notifications", self.base_url))
            .header("X-Internal-Token", &self.internal_token)
            .json(&json!({
                "account_id": account_id,
                "kind": kind,
                "title": title,
                "body": body,
                "data": data,
            }))
            .send()
            .await;
        if let Err(err) = result {
            tracing::warn!(error = ?err, kind, "notifications service call failed; continuing");
        }
    }

    pub async fn notify_order_placed(&self, warehouse_account_id: Uuid, order_id: Uuid) {
        self.post(
            warehouse_account_id,
            "order_placed",
            "New order received",
            "A store has placed a new order.",
            json!({ "order_id": order_id }),
        )
        .await;
    }

    pub async fn notify_order_status_changed(
        &self,
        store_account_id: Uuid,
        order_id: Uuid,
        status: &str,
    ) {
        self.post(
            store_account_id,
            "order_status_changed",
            "Order update",
            &format!("Your order is now {status}."),
            json!({ "order_id": order_id, "status": status }),
        )
        .await;
    }

    pub async fn notify_bulk_order_formed(&self, warehouse_account_id: Uuid, bulk_order_id: Uuid) {
        self.post(
            warehouse_account_id,
            "bulk_order_formed",
            "Bulk order formed",
            "Enough demand pooled to form a bulk order.",
            json!({ "bulk_order_id": bulk_order_id }),
        )
        .await;
    }
}
