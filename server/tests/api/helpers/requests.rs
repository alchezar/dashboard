use crate::helpers::TestApp;
use dashboard_server::web::types::Response;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub async fn get_response(app: &TestApp, endpoint: &str, bearer: &str) -> reqwest::Response {
    app.client
        .get(endpoint)
        .bearer_auth(bearer)
        .send()
        .await
        .unwrap()
}

pub async fn post_response(
    app: &TestApp,
    endpoint: &str,
    bearer: &str,
    payload: &Value,
) -> reqwest::Response {
    app.client
        .post(endpoint)
        .bearer_auth(bearer)
        .json(&payload)
        .send()
        .await
        .unwrap()
}

pub async fn delete_response(app: &TestApp, endpoint: &str, bearer: &str) -> reqwest::Response {
    app.client
        .delete(endpoint)
        .bearer_auth(bearer)
        .send()
        .await
        .unwrap()
}

pub async fn post_result<T>(app: &TestApp, endpoint: &str, payload: &Value) -> T
where
    T: DeserializeOwned,
{
    post_response(&app, endpoint, "", payload)
        .await
        .json::<Response<T>>()
        .await
        .unwrap()
        .result
}
