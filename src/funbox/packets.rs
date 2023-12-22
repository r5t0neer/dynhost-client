use serde::{Serialize, Deserialize};

#[derive(Serialize)]
pub struct LoginRequest
{
    pub method: String,
    pub parameters: LoginParameters,
    pub service: String,
}

#[derive(Serialize)]
pub struct LoginParameters
{
    pub applicationName: String,
    pub password: String,
    pub username: String,
}

#[derive(Deserialize)]
pub struct LoginResponse
{
    pub status: i32,
    pub data: LoginData
}

#[derive(Deserialize)]
pub struct LoginData
{
    pub contextID: String,
    pub username: String,
    pub groups: String,
}

impl LoginRequest
{
    pub fn create(username: String, password: String) -> LoginRequest
    {
        LoginRequest { 
            method: "createContext".to_string(), 
            parameters: LoginParameters { 
                applicationName: "webui".to_string(),
                password, 
                username 
            }, 
            service: "sah.Device.Information".to_string()
        }
    }
}

#[derive(Serialize)]
pub struct StateRequest
{
    pub method: String,
    pub parameters: NoParameters,
    pub service: String,
}

#[derive(Serialize)]
pub struct NoParameters;

#[derive(Deserialize)]
pub struct StateResponse
{
    pub status: String,
}

impl StateRequest
{
    pub fn create() -> StateRequest
    {
        StateRequest { method: "getState".to_string(), parameters: NoParameters{}, service: "UserInterface".to_string() }
    }
}

#[derive(Serialize)]
pub struct WANStatusRequest
{
    pub method: String,
    pub parameters: NoParameters,
    pub service: String,
}

#[derive(Deserialize)]
pub struct FTTH
{
    pub WanState: String,
    pub LinkType: String,
    pub LinkState: String,
    pub GponState: String,
    pub MACAddress: String,
    pub Protocol: String,
    pub ConnectionState: String,
    pub LastConnectionError: String,
    pub IPAddress: String,
    pub RemoteGateway: String,
    pub DNSServers: String,
    pub IPv6Address: String,
}

#[derive(Deserialize)]
pub struct WANStatusResponse
{
    pub status: bool,
    pub data: FTTH
}

impl WANStatusRequest
{
    pub fn create() -> WANStatusRequest
    {
        WANStatusRequest { method: "getWANStatus".to_string(), parameters: NoParameters{}, service: "NMC".to_string() }
    }
}