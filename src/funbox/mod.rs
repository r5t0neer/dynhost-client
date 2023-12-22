use std::collections::HashMap;

use reqwest::{blocking::Client, header::{HeaderMap, HeaderValue}};
use serde::de::DeserializeOwned;

use crate::ROUTER_ADMIN_PASSWORD;

use self::packets::{LoginResponse, LoginRequest, StateRequest, StateResponse, WANStatusRequest, WANStatusResponse};

mod packets;

struct SahClient
{
    http_client: Client
}

impl SahClient
{
    pub fn new() -> Result<SahClient, String>
    {
        let mut def_headers = HeaderMap::new();
        def_headers.insert("Content-Type", HeaderValue::from_static("application/x-sah-ws-4-call+json"));

        let mut client = Client::builder()
            .deflate(true)
            .gzip(true)
            .brotli(true)
            .use_native_tls()
            .default_headers(def_headers)
            .http1_only()
            .build()
            .map_err(|e| e.to_string())?;

        Ok(SahClient { http_client: client })
    }
}

pub struct Session
{
    client: SahClient,
    ip: String,
    context_id: String,
    cookie: String,
}

impl Session
{
    pub fn connect(ip: &str) -> Result<Session, String>
    {
        let mut sess = Session {
            client: SahClient::new()?,
            ip: ip.to_string(),
            context_id: String::new(),
            cookie: String::new(),
        };

        sess.login().map_err(|e| format!("Could not login: {}", match e {
            Ok(msg) => msg,
            Err(_) => "Session timed out (HTTP status 401)".to_string(),
        }))?;

        Ok(sess)
    }

    pub fn get_public_ip(&self) -> Result<String, Result<String, ()>>
    {
        let resp = self.init_authorized_post()
        .body(serde_json::to_string(&WANStatusRequest::create()).map_err(|e| e.to_string()).map_err(|e| Ok(e))?)
        .send()
        .map_err(|e| e.to_string()).map_err(|e| Ok(e))?;

        let resp: WANStatusResponse = self.parse_response(resp)?;

        Ok(resp.data.IPAddress)
    }

    fn is_internet(&self) -> Result<bool, Result<String, ()>>
    {
        let resp = self.init_authorized_post()
        .body(serde_json::to_string(&StateRequest::create()).map_err(|e| e.to_string()).map_err(|e| Ok(e))?)
        .send()
        .map_err(|e| e.to_string()).map_err(|e| Ok(e))?;

        let resp: StateResponse = self.parse_response(resp)?;
        
        Ok(resp.status.as_str() == "connected")
    }

    pub fn login(&mut self) -> Result<(), Result<String, ()>>
    {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("X-Sah-Login"));

        let resp = self.client.http_client
        .post(format!("http://{}/ws", self.ip.clone()))
        .headers(headers)
        .body(serde_json::to_string(&LoginRequest::create("admin".to_string(), ROUTER_ADMIN_PASSWORD.to_string())).map_err(|e| Ok(e.to_string()))?)
        .send()
        .map_err(|e| Ok(e.to_string()))?;

        resp.headers().get_all("set-cookie").iter().for_each(|elm| 
            {
                let value = unsafe{ String::from_utf8_unchecked(elm.as_bytes().to_vec()) };
                if value.contains("HttpOnly")
                {
                    if let Some(cookie_value) = value.split(";").next()
                    {
                        self.cookie = cookie_value.to_string();
                    }
                }
            }
        );

        let resp: LoginResponse = self.parse_response(resp)?;
        let ctx_id = resp.data.contextID;

        if ctx_id.is_empty()
        {
            Err(Ok("Response context ID is empty".to_string()))
        }
        else
        {
            self.context_id = ctx_id;
            Ok(())
        }
    }

    fn init_authorized_post(&self) -> reqwest::blocking::RequestBuilder
    {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(format!("X-Sah {}", self.context_id).as_str()).unwrap());
        headers.insert("Cookie", HeaderValue::from_bytes(format!("{}; sah/contextId={}", 
            self.cookie.as_str(), 
            urlencoding::encode(self.context_id.as_str())
        ).as_bytes()).unwrap());

        self.client.http_client.post(format!("http://{}/ws", self.ip.clone()))
        .headers(headers)
    }

    /// Returns Ok(None) if Unauthorized Access
    fn parse_response<T>(&self, resp: reqwest::blocking::Response) -> Result<T, Result<String, ()>>
        where T: DeserializeOwned
    {
        // check for errors

        if resp.status().as_u16() == 401
        {
            return Err(Err(()));
        }

        let bytes = resp.bytes().map_err(|e| Ok(e.to_string()))?;
        let body = unsafe{ String::from_utf8_unchecked(bytes.to_vec()) };

        if body.contains("errors")
        {
            Err(Ok(format!("Got some errors in response: {}", body)))
        }
        else
        {
            serde_json::from_slice(body.as_bytes()).map_err(|e| Ok(format!("Could not parse response: {}", e.to_string())))
        }
    }
}