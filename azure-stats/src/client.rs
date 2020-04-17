use anyhow::{anyhow, Result};
use log::debug;
use reqwest::Body;
use ring::hmac::{self, Key, HMAC_SHA256};

pub struct Client {
    customer_id: String,
    key: Key,
    url: String,
    name: String,
}

impl Client {
    pub fn from_config(config: ClientConfig) -> Result<Self> {
        let url = format!(
            "https://{}.ods.opinsights.azure.com/api/logs?api-version=2016-04-01",
            config.customer_id
        );

        let key = base64::decode(config.shared_key)?;
        let key = Key::new(HMAC_SHA256, &key);
        Ok(Self {
            customer_id: config.customer_id,
            name: config.name,
            key,
            url,
        })
    }

    pub async fn send(&self, data: Vec<u8>) -> Result<()> {
        let date = chrono::Utc::now().format("%a, %d %b %Y %T GMT").to_string();
        let signature = self.build_signature(&date, &data)?;

        debug!("sending data to {} log", self.name);
        let res = reqwest::Client::new()
            .post(&self.url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Log-Type", &self.name)
            .header("Authorization", signature)
            .header("x-ms-date", date)
            .header("time-generated-field", "")
            .body(Body::from(data))
            .send()
            .await?;

        if res.status() != reqwest::StatusCode::OK {
            let mut err = format!("Response: {:?}. ", res);
            err.push_str(&format!(
                "Content: {}",
                res.text().await.expect("read content")
            ));
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
        let tag = hmac::sign(&self.key, secret.as_bytes());
        let signature = base64::encode(tag.as_ref());
        Ok(signature)
    }
}

pub struct ClientConfig {
    customer_id: String,
    shared_key: String,
    name: String,
}

impl ClientConfig {
    pub fn new(
        customer_id: impl Into<String>,
        shared_key: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            customer_id: customer_id.into(),
            shared_key: shared_key.into(),
            name: name.into(),
        }
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
        let config = ClientConfig::new("", "", "TestData");
        let client = Client::from_config(config).unwrap();

        let data = r#"[{"DemoField1":"DemoValue1","DemoField2":"DemoValue2"},{"DemoField3":"DemoValue3","DemoField4":"DemoValue4"}]"#;
        let res = client.send(data.as_bytes().to_vec()).await;

        assert!(matches!(res, Ok(_)));
    }
}
