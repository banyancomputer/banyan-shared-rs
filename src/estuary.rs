use anyhow::{Error, Result};
use reqwest::{multipart, Body, Client};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use std::env::var;
use tokio_util::codec::{BytesCodec, FramedRead};
use std::fmt;

/// Content - What's returned from the Estuary API /content/stats endpoint
#[derive(Deserialize)]
#[allow(unused)]
pub struct Content {
    id: u32,
    #[serde(rename = "cid", deserialize_with = "des_cid_from_map")]
    cid_str: String,
    #[serde(rename = "dealId")]
    deal_id: u32,
    name: String,
    size: u32,
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.id, self.cid_str, self.deal_id)
    }
}

// Note (al) - The Estuary API returns a CID as a map with a "/" key
pub fn des_cid_from_map<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let map: Map<String, Value> = Map::deserialize(deserializer).unwrap();
    let cid_str = map.get("/").unwrap().as_str().unwrap();
    Ok(cid_str.to_string())
}

/// EstuaryClient - A struct for managing Requests to an Estuary API
pub struct EstuaryClient {
    /// The Estuary API Hostname
    pub estuary_api_hostname: String,
    /// The Estuary API Key
    pub estuary_api_key: Option<String>,
}

impl Default for EstuaryClient {
    /// Create a new EstuaryClient from the Environment
    /// ```no_run
    /// use banyan_shared::estuary::EstuaryClient;
    /// let estuary_client = EstuaryClient::default();
    /// ```
    /// # Panics
    /// This function will panic if the `ESTUARY_API_HOSTNAME` environment variable is not set.
    fn default() -> Self {
        Self {
            estuary_api_hostname: var("ESTUARY_API_HOSTNAME")
                .unwrap_or_else(|_| panic!("ESTUARY_API_HOSTNAME environment variable is not set")),
            estuary_api_key: var("ESTUARY_API_KEY").ok(),
        }
    }
}

// TODO: Should I be initializing a ReqWest Client here, or is ok to do it in each function?
impl EstuaryClient {
    /// Create a new EstuaryClient using custom values
    /// # Arguments
    /// * `estuary_api_hostname` - The Hostname of the Estuary API to use.
    /// * `estuary_api_key` - The (optional) API Key to use for the Estuary API.
    /// ```no_run
    /// use  banyan_shared::estuary::EstuaryClient;
    /// let estuary_client = EstuaryClient::new("http://localhost:3004".to_string(), None);
    /// ```
    /// # Panics
    /// This function should not panic.
    /// Misconfiguration will result in an error when making requests.
    pub fn new(estuary_api_hostname: String, estuary_api_key: Option<String>) -> Self {
        Self {
            estuary_api_hostname,
            estuary_api_key,
        }
    }

    /* Struct Methods */

    /// Get the Estuary API Hostname
    pub fn get_estuary_api_hostname(&self) -> String {
        self.estuary_api_hostname.clone()
    }

    /// Stage a File on Estuary
    /// # Arguments
    /// * `file` - The handle to the file to stage
    /// * `deal_id` - The Deal ID to use for the file
    /// * `b3_hash` - The Blake3 Hash of the file
    /// ```no_run
    /// use anyhow::{Result, Error};
    /// use  banyan_shared::estuary::EstuaryClient;
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = EstuaryClient::default();
    ///     client.stage_file(
    ///         "path_to_file.txt".to_string(),
    ///         Some("0".to_string()),
    ///         Some("hash".to_string())
    ///     ).await?;
    ///    Ok(())
    /// }
    /// ```
    /// # Panics
    /// * If there is an error reading the file
    /// * If there is an error sending the request
    /// Stage a File on Estuary
    /// # Arguments
    /// * `file_path` - The path to the file to stage
    /// * `deal_id_str` - The (optional) Deal ID to use for the file, as a String
    /// * `b3_hash_str` - The (optional) Blake3 Hash of the file, as a Hex String
    /// # Returns
    /// * `Result<(), Error>` - Errors if there is an error staging the file
    pub async fn stage_file(
        &self,
        file_path: String,
        deal_id_str: Option<String>,
        b3_hash_str: Option<String>,
    ) -> Result<(), Error> {
        if self.estuary_api_key.is_none() {
            panic!("No Estuary API Key is set");
        }
        let estuary_api_key = self.estuary_api_key.clone().unwrap();
        // Initialize an HTTP Client
        let client = Client::new();
        // Read the File as a Tokio File
        let file = tokio::fs::File::open(&file_path).await?;
        // Read file body stream
        let file_body = Body::wrap_stream(FramedRead::new(file, BytesCodec::new()));
        // Define a Form Part for the File
        let some_file = multipart::Part::stream(file_body)
            .file_name(file_path)
            .mime_str("text/plain")?;
        // Create the multipart form
        let form = multipart::Form::new()
            .part("data", some_file); //add the file part
        // Add the Deal ID to the form, if it exists
        let form = if let Some(deal_id_str) = deal_id_str {
            form.text("dealId", deal_id_str)
        } else {
            form
        };
        // Add the Blake3 Hash to the form, if it exists
        let form = if let Some(b3_hash_str) = b3_hash_str {
            form.text("b3Hash", b3_hash_str)
        } else {
            form
        };
        let res = client
            // POST to the /content/add endpoint
            .post(format!("{}/content/add", self.estuary_api_hostname))
            // Set the Authorization Header
            .header("Authorization", format!("Bearer {}", estuary_api_key))
            // Add the Form
            .multipart(form)
            // Send the Request
            .send()
            // Await the Response
            .await?;
        // Check the Status Code
        if res.status().is_success() {
            // No Need to listen to the Response - We're good!
            Ok(())
        } else {
            Err(Error::msg(format!(
                "Error staging file: {}",
                res.status().as_str()
            )))
        }
    }

    /// Get the First 500 pieces of Content from Estuary
    /// ```no_run
    /// use anyhow::{Result, Error};
    /// use  banyan_shared::estuary::EstuaryClient;
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = EstuaryClient::default();
    ///     let content = client.get_content().await?;
    ///     Ok(())
    /// }
    /// ```
    /// # Panics
    /// * If there is an error sending the request
    /// * If there is an error parsing the response
    pub async fn get_content(&self) -> Result<Vec<Content>, Error> {
        if self.estuary_api_key.is_none() {
            panic!("No Estuary API Key is set");
        }
        let estuary_api_key = self.estuary_api_key.clone().unwrap();
        // Initialize an HTTP Client
        let client = Client::new();
        // Initialize the Request
        let res = client
            // GET to the /content endpoint
            .get(format!("{}/content/stats", self.estuary_api_hostname))
            // Set the Authorization Header
            .header("Authorization", format!("Bearer {}", estuary_api_key))
            // Send the Request
            .send()
            // Await the Response
            .await?;
        // Check the Status Code
        if res.status().is_success() {
            // Parse the Response
            let content: Vec<Content> = res.json().await?;
            Ok(content)
        } else {
            Err(Error::msg(format!(
                "Error getting content: {}",
                res.status().as_str()
            )))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    /// Test that we can create a BanyanClient from the Environment
    fn default_client() {
        let _client = EstuaryClient::default();
        return;
    }

    #[tokio::test]
    /// Try to stage a file on Estuary with a fake DealId
    async fn stage_file() {
        let client = EstuaryClient::default();
        let deal_id_str = "0".to_string();
        let b3_hash_str =
            "a291f28711c5238dc415f64a5525ff428f3fd6fd45fca181384a3f31091b5d81".to_string();
        client
            .stage_file("Cargo.toml".to_string(), Some(deal_id_str), Some(b3_hash_str))
            .await
            .unwrap();
        return;
    }

    #[tokio::test]
    /// Try to get content from Estuary
    async fn get_contents() {
        let client = EstuaryClient::default();
        let _: Vec<Content> = client.get_content().await.unwrap();
        return;
    }
}
