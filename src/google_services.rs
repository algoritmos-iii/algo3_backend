use anyhow::bail;
use reqwest::Response;
use serde::{de, Deserialize, Serialize};
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

#[derive(Serialize, Debug)]
pub struct Events {
    events: Vec<Event>,
}

impl Events {
    pub fn first(&self) -> Option<&Event> {
        self.events.first()
    }
}

impl<'de> Deserialize<'de> for Events {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let request = serde_json::Value::deserialize(deserializer)?;

        Ok(Self {
            events: serde_json::from_value(request["items"].clone()).map_err(de::Error::custom)?,
        })
    }
}
#[derive(Serialize, Debug, Clone)]
pub struct Event {
    summary: String,
    description: String,
    start_date_time: String,
    end_date_time: String,
    time_zone: String,
}

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let request = serde_json::Value::deserialize(deserializer)?;

        Ok(Self {
            summary: serde_json::from_value(request["summary"].clone())
                .map_err(de::Error::custom)?,
            description: serde_json::from_value(request["description"].clone())
                .map_err(de::Error::custom)?,
            start_date_time: serde_json::from_value(request["start"]["dateTime"].clone())
                .map_err(de::Error::custom)?,
            end_date_time: serde_json::from_value(request["end"]["dateTime"].clone())
                .map_err(de::Error::custom)?,
            time_zone: serde_json::from_value(request["end"]["timeZone"].clone())
                .map_err(de::Error::custom)?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDate {
    pub date_time: String,
    pub time_zone: String,
}

pub struct GoogleService {
    client: reqwest::Client,
    access_token: String,
    spreadsheet_url: String,
    calendar_url: String,
}

impl GoogleService {
    fn new_service(client: reqwest::Client, access_token: String) -> Self {
        Self {
            client,
            access_token,
            spreadsheet_url: "https://sheets.googleapis.com/v4".to_string(),
            calendar_url: "https://www.googleapis.com/calendar/v3".to_string(),
        }
    }

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

    pub async fn new_reading_service_account_key<P>(
        path: P,
        scopes: &[&str],
    ) -> Result<Arc<Self>, anyhow::Error>
    where
        P: AsRef<Path> + Send,
    {
        let auth_token = match Self::auth_token(path, scopes).await {
            Ok(token) => token,
            Err(e) => bail!("{}", e),
        };

        Ok(Arc::new(Self::new_service(
            reqwest::Client::new(),
            auth_token.as_str().to_string(),
        )))
    }

    pub async fn spreadsheets(&self, spreadsheet_id: &str) -> Result<Spreadsheet, anyhow::Error> {
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
                "{}/spreadsheets/{spreadsheet_id}",
                self.spreadsheet_url
            ))
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

    pub async fn append_row(
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
            .post(format!("{}/spreadsheets/{spreadsheet_id}/values/{sheet_id}:append?insertDataOption=INSERT_ROWS&valueInputOption=USER_ENTERED", self.spreadsheet_url))
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

    pub async fn get_values(
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
                self.spreadsheet_url
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

    pub async fn events(&self, calendar_id: String) -> Result<Events, anyhow::Error> {
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
                "{}/calendars/{calendar_id}/events",
                self.calendar_url
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

        let events = match response.json::<Events>().await {
            Ok(events) => events,
            Err(e) => bail!("{e}"),
        };

        Ok(events)
    }
}

impl Clone for GoogleService {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            access_token: self.access_token.clone(),
            spreadsheet_url: self.spreadsheet_url.clone(),
            calendar_url: self.calendar_url.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::google_services::GoogleService;

    #[tokio::test]
    #[ignore = "Hasta conseguir credenciales de prueba o en su defecto averiguar como usar las reales"]
    async fn test01_spreadsheet_append_row() {
        let spreadsheet_id = "1Ss7FMebxZxxGi15mREvQLYuBJ1sWVWbD".to_string();

        let service = GoogleService::new_reading_service_account_key(
            "./clientsecret.json",
            &[
                "https://www.googleapis.com/auth/spreadsheets",
                "https://www.googleapis.com/auth/calendar.events.readonly",
            ],
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
