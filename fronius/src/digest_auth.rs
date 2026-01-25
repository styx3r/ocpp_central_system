use std::{error::Error, time::Duration};

use reqwest::{
    StatusCode,
    blocking::{Client, Response},
    header::{self, HeaderValue},
};

use serde::Serialize;
use sha2::{Digest, Sha256};

use log::{debug, error};

use chrono::offset::Utc;

//-------------------------------------------------------------------------------------------------

static GET_REQUEST_METHOD: &str = "GET";
static POST_REQUEST_METHOD: &str = "POST";

//-------------------------------------------------------------------------------------------------

pub struct DigestAuth {
    username: String,
    password: String,

    algorithm: Option<String>,
    nonce: Option<String>,
    realm: Option<String>,
    qop: Option<String>,
    nc: usize,
    cnonce: Option<String>,
    response: Option<String>,
}

impl DigestAuth {
    pub fn new(username: &String, password: &String) -> Self {
        Self {
            username: username.clone(),
            password: password.clone(),
            algorithm: None,
            nonce: None,
            realm: None,
            qop: None,
            nc: 1,
            cnonce: None,
            response: None,
        }
    }

    fn format_authentication_header(&self, uri: &String) -> String {
        let padded_nc = format!("{:0>8}", self.nc);
        let auth_header = format!(
            "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", response=\"{}\", qop={}, nc={}, cnonce=\"{}\"",
            self.username,
            self.realm.clone().unwrap(),
            self.nonce.clone().unwrap(),
            uri,
            self.response.clone().unwrap(),
            self.qop.clone().unwrap(),
            padded_nc,
            self.cnonce.clone().unwrap()
        );

        debug!("Authorization header: {}", auth_header);
        auth_header
    }

    fn populate_digest_auth_fields(
        &mut self,
        response: &Response,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.response.is_some() && response.status() == StatusCode::OK {
            return Ok(());
        }

        if response.status() != StatusCode::UNAUTHORIZED {
            error!("First request needs to return {}", StatusCode::UNAUTHORIZED);
            return Err(
                format!("First request needs to return {}", StatusCode::UNAUTHORIZED).into(),
            );
        }

        let authentication_string = match response.headers().get("x-www-authenticate") {
            Some(authentication_header) => authentication_header
                .to_str()?
                .split(",")
                .map(|v| v.replace("Digest ", ""))
                .collect::<Vec<_>>(),
            _ => {
                vec![]
            }
        };

        for element in &authentication_string {
            let trimmed = element.trim();
            if trimmed.starts_with("algorithm=") {
                self.algorithm = Some(element.replace("algorithm=", "").replace("\"", ""));
            } else if trimmed.starts_with("nonce=") {
                self.nonce = Some(
                    element
                        .replace("nonce=", "")
                        .replace("\"", "")
                        .trim()
                        .to_owned(),
                );
            } else if trimmed.starts_with("realm=") {
                self.realm = Some(
                    element
                        .replace("realm=", "")
                        .replace("\"", "")
                        .trim()
                        .to_owned(),
                );
            } else if trimmed.starts_with("qop=") {
                self.qop = Some(
                    element
                        .replace("qop=", "")
                        .replace("\"", "")
                        .trim()
                        .to_owned(),
                );
            }
        }

        Ok(())
    }

    fn calculate_authentication_header(
        &mut self,
        uri: &String,
        http_method: &str,
    ) -> Result<HeaderValue, Box<dyn Error>> {
        let offset = Duration::from_secs(self.nc as u64);
        self.cnonce = Some(
            format!("{:x}", md5::compute((Utc::now() + offset).to_rfc2822()))
                .chars()
                .take(16)
                .collect::<String>(),
        );

        if let Some(algorithm) = &self.algorithm
            && let Some(nonce) = &self.nonce
            && let Some(realm) = &self.realm
            && let Some(qop) = &self.qop
            && let Some(cnonce) = &self.cnonce
        {
            debug!("Algorithm: {} None: {} Realm: {}", algorithm, nonce, realm);

            // NOTE: https://www.rfc-editor.org/rfc/rfc7616.html#section-3.4.4 states that HA1 SHALL BE
            //       SHA256(username, realm, password) BUT Fronius uses MD5(username, realm, password).
            //       Aaaaand I don't know why :)
            let md5_hashed_pw =
                md5::compute(format!("{}:{}:{}", &self.username, &realm, &self.password));

            let mut ha2 = Sha256::new();
            ha2.update(format!("{}:{}", http_method, uri));

            let padded_nc = format!("{:0>8}", self.nc);
            // Final hash is calculated with SHA256(MD5_HASHED_PW:NONCE:NC:CNONCE:AUTH:HA2)
            let mut hash = Sha256::new();
            hash.update(format!(
                "{:x}:{}:{}:{}:{}:{:x}",
                md5_hashed_pw,
                nonce,
                padded_nc, // NC     (counter which starts by 1)
                cnonce,    // CNONCE (needs to be calculated)
                qop,       // QOS    (given by the first server response)
                ha2.finalize()
            ));

            self.response = Some(format!("{:x}", hash.finalize()));
            let auth_header = self.format_authentication_header(&uri);

            self.nc += 1;

            return Ok(header::HeaderValue::from_str(auth_header.as_str())?);
        }

        Err("Authorization failed".into())
    }

    pub fn get(
        &mut self,
        hostname: &String,
        uri: &String,
        request_params: &String,
    ) -> Result<String, Box<dyn Error>> {
        let request_url = format!("{}{}?{}", hostname, uri, request_params);
        let response: Response = Client::new().get(&request_url).send()?;

        self.populate_digest_auth_fields(&response)?;

        let response: Response = Client::new()
            .get(&request_url)
            .header(
                header::AUTHORIZATION,
                self.calculate_authentication_header(uri, GET_REQUEST_METHOD)?,
            )
            .send()?;

        if response.status() != StatusCode::OK {
            return Err(format!("Request returned {}", response.status()).into());
        }

        Ok(response.text()?)
    }

    pub fn post_json<T: Serialize>(
        &mut self,
        hostname: &String,
        uri: &String,
        request_params: &T,
    ) -> Result<String, Box<dyn Error>> {
        if self.response.is_none() {
            return Err("Authorization handshake missing!".into());
        }

        let request_url = format!("{}{}", hostname, uri);
        debug!(
            "Sending POST request to {} {}",
            request_url,
            serde_json::to_string(request_params)?
        );
        let response: Response = Client::new()
            .post(&request_url)
            .json(request_params)
            .header(
                header::AUTHORIZATION,
                self.calculate_authentication_header(uri, POST_REQUEST_METHOD)?,
            )
            .send()?;

        if response.status() != StatusCode::OK {
            return Err(format!("Request returned {}", response.status()).into());
        }

        Ok(response.text()?)
    }
}
