use std::str;

use anyhow::{anyhow, Result};
use chrono::Utc;
use hyper::{body, client::HttpConnector, Body, Request, StatusCode};
use hyper_tls::HttpsConnector;
use log::debug;
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Private},
    sign::Signer,
};
use serde::Deserialize;

pub struct Client {
    customer_id: CustomerId,
    key: PKey<Private>,
    url: String,
    client: hyper::Client<HttpsConnector<HttpConnector>, Body>,
}

impl Client {
    pub fn new(config: ClientConfig) -> Result<Self> {
        let (customer_id, shared_key) = config.into_parts();

        let url = format!(
            "https://{}.ods.opinsights.azure.com/api/logs?api-version=2016-04-01",
            customer_id
        );

        let key = base64::decode(shared_key)?;
        let key = PKey::hmac(&key)?;

        let client = hyper::Client::builder().build(HttpsConnector::new());

        Ok(Self {
            customer_id,
            key,
            url,
            client,
        })
    }

    pub async fn send(&self, log_name: &str, data: Vec<u8>) -> Result<()> {
        let date = Utc::now().format("%a, %d %b %Y %T GMT").to_string();
        let signature = self.build_signature(&date, &data)?;

        debug!("sending data to {} log", log_name);

        let req = Request::builder()
            .method(hyper::Method::POST)
            .uri(&self.url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Log-Type", log_name)
            .header("Authorization", signature)
            .header("x-ms-date", date)
            .header("time-generated-field", "")
            .body(hyper::Body::from(data))?;

        let res = self.client.request(req).await?;

        if res.status() != StatusCode::OK {
            let mut err = format!("Response: {:?}. ", res);

            let bytes = body::to_bytes(res.into_body()).await?;
            let content = str::from_utf8(bytes.as_ref())?;
            err.push_str(&format!("Content: {}", content));

            return Err(anyhow!(err));
        }

        Ok(())
    }

    fn build_signature(&self, date: &str, payload: &[u8]) -> Result<String> {
        let secret = format!(
            "POST\n{}\napplication/json\nx-ms-date:{}\n/api/logs",
            payload.len(),
            date
        );
        let hash = self.sign(&secret)?;
        let signature = format!("SharedKey {}:{}", self.customer_id, hash);

        Ok(signature)
    }

    fn sign(&self, secret: &str) -> Result<String> {
        let mut signer = Signer::new(MessageDigest::sha256(), &self.key)?;
        signer.update(secret.as_bytes().as_ref())?;
        let signature = signer.sign_to_vec()?;

        Ok(base64::encode(signature))
    }
}

#[derive(Debug, Deserialize)]
pub struct ClientConfig {
    customer_id: CustomerId,
    shared_key: SharedKey,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct CustomerId(String);

impl std::fmt::Display for CustomerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for CustomerId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct SharedKey(String);

impl AsRef<[u8]> for SharedKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl ClientConfig {
    pub fn new(customer_id: impl Into<String>, shared_key: impl Into<String>) -> Self {
        Self {
            customer_id: CustomerId(customer_id.into()),
            shared_key: SharedKey(shared_key.into()),
        }
    }

    pub fn into_parts(self) -> (CustomerId, SharedKey) {
        (self.customer_id, self.shared_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_converts_date() {
        use chrono::offset::TimeZone;
        let date = chrono::Utc.ymd(2019, 1, 2).and_hms(3, 4, 5);
        let converted = date.format("%a, %d %b %Y %T GMT").to_string();
        assert_eq!(converted, "Wed, 02 Jan 2019 03:04:05 GMT");
    }

    #[tokio::test]
    async fn it_sends_data() {
        let config = ClientConfig::new("", "");
        let client = Client::new(config).unwrap();

        let data = r#"[{"DemoField1":"DemoValue1","DemoField2":"DemoValue2"},{"DemoField3":"DemoValue3","DemoField4":"DemoValue4"}]"#;
        let res = client.send("TestData", data.as_bytes().to_vec()).await;

        assert!(matches!(res, Ok(_)));
    }
}
