use anyhow::bail;
use async_trait::async_trait;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::Path, sync::Arc};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Spreadsheet {
    spreadsheet_id: String,
    properties: serde_json::Value, //SpreadsheetProperties,
    sheets: Vec<GeneralSheet>,
    // named_ranges: Vec<serde_json::Value>, // Vec<NamedRange>,
    spreadsheet_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GeneralSheet {
    properties: serde_json::Value,
    merges: Option<Vec<serde_json::Value>>,
    filter_views: Option<Vec<serde_json::Value>>,
    basic_filter: Option<serde_json::Value>,
    banded_ranges: Option<Vec<serde_json::Value>>,
    conditional_formats: Option<Vec<serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpreadsheetValue {
    range: String,
    major_dimension: String,
    pub values: Vec<Vec<String>>,
}

pub enum ServiceType {
    Spreadsheet,
    Calendar,
}

#[async_trait]
pub trait GoogleService {
    fn new_service(client: reqwest::Client, access_token: String, url: String) -> Self;

    async fn auth_token<P>(
        path: P,
        scopes: &[&str],
    ) -> Result<yup_oauth2::AccessToken, yup_oauth2::Error>
    where
        P: AsRef<Path> + Send,
    {
        let service_account_key = match yup_oauth2::read_service_account_key(path).await {
            Ok(key) => key,
            Err(e) => return Err(yup_oauth2::Error::LowLevelError(e)),
        };

        let auth = yup_oauth2::ServiceAccountAuthenticator::builder(service_account_key)
            .persist_tokens_to_disk("auth.json")
            .build()
            .await
            .unwrap();

        auth.token(scopes).await
    }

    async fn new_reading_service_account_key<P>(
        service_type: ServiceType,
        version: &str,
        path: P,
        scopes: &[&str],
    ) -> Result<Arc<SpreadsheetService>, anyhow::Error>
    where
        P: AsRef<Path> + Send,
    {
        let auth_token = match Self::auth_token(path, scopes).await {
            Ok(token) => token,
            Err(e) => bail!("{}", e),
        };

        let service = match service_type {
            ServiceType::Spreadsheet => Arc::new(SpreadsheetService::new_service(
                reqwest::Client::new(),
                auth_token.as_str().to_string(),
                format!("https://sheets.googleapis.com/{version}"),
            )),
            _ => bail!("Unknown API"),
        };

        Ok(service)
    }

    async fn spreadsheets(&self, _spreadsheet_id: &str) -> Result<Spreadsheet, anyhow::Error> {
        bail!("Not implemented")
    }

    async fn append_row(
        &self,
        _spreadsheet_id: &str,
        _sheet_id: &str,
        _values: Vec<&str>,
    ) -> Result<Response, anyhow::Error> {
        bail!("Not implemented")
    }

    async fn get_values(
        &self,
        _spreadsheet_id: &str,
        _sheet_id: &str,
        _range: &str,
    ) -> Result<SpreadsheetValue, anyhow::Error> {
        bail!("Not implemented")
    }
}

pub struct SpreadsheetService {
    client: reqwest::Client,
    access_token: String,
    url: String,
}

#[async_trait]
impl GoogleService for SpreadsheetService {
    fn new_service(client: reqwest::Client, access_token: String, url: String) -> Self {
        SpreadsheetService {
            client,
            access_token,
            url,
        }
    }

    async fn spreadsheets(&self, spreadsheet_id: &str) -> Result<Spreadsheet, anyhow::Error> {
        let mut header = reqwest::header::HeaderMap::new();
        header.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(
                format!("Bearer {}", self.access_token.as_str()).as_str(),
            )
            .unwrap(),
        );
        header.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let request = match self
            .client
            .get(format!("{}/spreadsheets/{spreadsheet_id}", self.url))
            .headers(header)
            .build()
        {
            Ok(request) => request,
            Err(e) => bail!("{e}"),
        };
        let response = match self.client.execute(request).await {
            Ok(response) => response,
            Err(e) => bail!("{e}"),
        };
        let spreadsheet = match response.json::<Spreadsheet>().await {
            Ok(spreadsheet) => spreadsheet,
            Err(e) => bail!("{e}"),
        };
        Ok(spreadsheet)
    }

    async fn append_row(
        &self,
        spreadsheet_id: &str,
        sheet_id: &str,
        values: Vec<&str>,
    ) -> Result<Response, anyhow::Error> {
        let mut header = reqwest::header::HeaderMap::new();
        header.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(
                format!("Bearer {}", self.access_token.as_str()).as_str(),
            )
            .unwrap(),
        );
        header.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let data = &json!({ "values": [values] });

        let request = match self
            .client
            .post(format!("{}/spreadsheets/{spreadsheet_id}/values/{sheet_id}:append?insertDataOption=INSERT_ROWS&valueInputOption=USER_ENTERED", self.url))
            .headers(header)
            .json(data)
            .build()
        {
            Ok(request) => request,
            Err(e) => bail!("{e}"),
        };

        let response = match self.client.execute(request).await {
            Ok(response) => response,
            Err(e) => bail!("{e}"),
        };
        Ok(response)
    }

    async fn get_values(
        &self,
        spreadsheet_id: &str,
        sheet_id: &str,
        range: &str,
    ) -> Result<SpreadsheetValue, anyhow::Error> {
        let mut header = reqwest::header::HeaderMap::new();
        header.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(
                format!("Bearer {}", self.access_token.as_str()).as_str(),
            )
            .unwrap(),
        );
        header.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        let request = match self
            .client
            .get(format!(
                "{}/spreadsheets/{spreadsheet_id}/values/{sheet_id}!{range}",
                self.url
            ))
            .headers(header)
            .build()
        {
            Ok(request) => request,
            Err(e) => bail!("Error building the request: {e}"),
        };

        let response = match self.client.execute(request).await {
            Ok(response) => response,
            Err(e) => bail!("Error executing the request: {e}"),
        };

        let spreadsheet_values = match response.json::<SpreadsheetValue>().await {
            Ok(spreadsheet_values) => spreadsheet_values,
            Err(e) => bail!("{e}"),
        };

        Ok(spreadsheet_values)
    }
}

impl Clone for SpreadsheetService {
    fn clone(&self) -> Self {
        SpreadsheetService {
            client: self.client.clone(),
            access_token: self.access_token.clone(),
            url: self.url.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::google_services::{GoogleService, ServiceType, SpreadsheetService};

    #[tokio::test]
    async fn test01_spreadsheet_append_row() {
        let spreadsheet_id = "1Ss7FMebxZxxGi15mREvQLYuBJ1sWVWbD".to_string();

        let service = SpreadsheetService::new_reading_service_account_key(
            ServiceType::Spreadsheet,
            "v4",
            "./clientsecret.json",
            &["https://www.googleapis.com/auth/spreadsheets"],
        )
        .await
        .unwrap();

        let expected_result = service
            .append_row(
                &spreadsheet_id,
                "Test",
                vec![
                    &chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    "0",
                    "Brindada",
                    "Yo",
                ],
            )
            .await;

        println!("{:?}", expected_result);

        assert!(expected_result.is_ok());
        assert!(expected_result.unwrap().status().is_success());
    }
}
