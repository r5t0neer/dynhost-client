#[macro_use]
extern crate serde;

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Read;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ini::configparser::ini::Ini;
use serde::Deserialize;

use reqwest::blocking::{Body, Client};
use reqwest::Error;
use reqwest::header::{HeaderMap, HeaderValue};
use crate::log::Logger;

mod log;

struct Account
{
    domain: String,
    username: String,
    password: String,
}

fn update_ip(account: &Account, ip: &String, logger: &mut Logger) -> Result<(), u16>
{
    let client = Client::builder()
        .deflate(true)
        .gzip(true)
        .brotli(true)
        .use_native_tls()
        .build()
        .unwrap();

    let response = client
        .get(format!("https://www.ovh.com/nic/update?system=dyndns&hostname={}&myip={}", account.domain, ip))
        .basic_auth(account.username.as_str(), Some(account.password.as_str()))
        .send()
        .unwrap();

    if !response.status().is_success()
    {
        Err(response.status().as_u16())
    }
    else { Ok(()) }
}

fn get_public_ip() -> Result<String, String>
{
    let mut headers = HeaderMap::new();
    headers.append("Content-Type", HeaderValue::from_static("application/json"));

    let mut client = Client::builder()
        .deflate(true)
        .gzip(true)
        .brotli(true)
        .use_native_tls()
        .default_headers(headers)
        .http1_only()
        .build()
        .unwrap();

    let response = client
        .post("http://192.168.1.1/sysbus/NMC:getWANStatus")
        .body(Body::from("{\"parameters\":{}}"))
        .send()
        .unwrap();

    #[derive(Deserialize)]
    struct FTTH
    {
        WanState: String,
        LinkType: String,
        LinkState: String,
        GponState: String,
        MACAddress: String,
        Protocol: String,
        ConnectionState: String,
        LastConnectionError: String,
        IPAddress: String,
        RemoteGateway: String,
        DNSServers: String,
        IPv6Address: String,
    }

    #[derive(Deserialize)]
    struct JSONRoot
    {
        result: JSONStatus
    }

    #[derive(Deserialize)]
    struct JSONStatus
    {
        status: bool,
        data: FTTH
    }

    let json: Result<JSONRoot, reqwest::Error> = response.json();

    match json {
        Ok(json) => { Ok(json.result.data.IPAddress) }
        Err(e) => { Err(e.to_string()) }
    }
}

fn is_ipv4(s: &String) -> bool
{
    if s.len() < 7 || s.len() > 15
    {
        return false;
    }

    fn check_octet(octet: &String) -> bool
    {
        match octet.len()
        {
            3 => {
                let mut chrs = octet.chars();
                let ch1 = chrs.next().unwrap();

                if !(ch1 == '1' || ch1 == '2') { return false; }
                if ch1 == '2'
                {
                    let ch2 = chrs.next().unwrap();
                    if !(ch2 == '0' || ch2 == '1' || ch2 == '2' || ch2 == '3' || ch2 == '4')
                    {
                        if ch2 == '5'
                        {
                            let ch3 = chrs.next().unwrap();
                            if !(ch3 == '0' || ch3 == '1' || ch3 == '2' || ch3 == '3' || ch3 == '4' || ch3 == '5')
                            {
                                return false;
                            }
                        }
                        else
                        {
                            return false;
                        }
                    }
                }
            },
            2 => {},
            1 => {},
            _ => return false
        }

        true
    }

    let mut octet = String::new();
    for ch in s.chars()
    {
        if ch == '.'
        {
            if !check_octet(&octet) { return false; }

            octet = String::new();
        }
        else
        {
            octet.push(ch);
        }
    }

    if !check_octet(&octet) { return false; }

    true
}

fn main()
{
    println!("Opening 'log_path.txt'...");

    let mut log_path: String = "".to_string();

    match OpenOptions::new().read(true).open("log_path.txt")
    {
        Ok(mut file) => {
            match file.read_to_string(&mut log_path)
            {
                Ok(_) => {
                    log_path = log_path.replace(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' || c == ',' || c == '/')    , "");
                }
                Err(err) => {
                    println!("Error while reading 'log_path.txt': {}", err);
                    println!("Defaulting to log path /var/log/dynhost.log");
                    log_path = "/var/log/dynhost.log".to_string();
                }
            }
        }
        Err(err) => {
            println!("Could not open: {}", err);
            println!("Defaulting to log path /var/log/dynhost.log");
            log_path = "/var/log/dynhost.log".to_string();
        }
    }

    println!("Opening 'accounts.ini'...");

    let mut accounts: Vec<Account> = vec![];

    let mut ini = Ini::new();
    match ini.load("accounts.ini")
    {
        Ok(map) => {

            // section -> domain, username, password

            for (section, fields) in map.iter()
            {
                if !fields.contains_key(&"domain".to_string()) ||
                    !fields.contains_key(&"username".to_string()) ||
                    !fields.contains_key(&"password".to_string())
                {
                    println!("[accounts.ini] Section '{}' is missing fields, ensure there are 'domain', 'username' and 'password'; skipping", section);
                }
                else
                {
                    accounts.push(Account {
                        domain: fields.get(&"domain".to_string()).unwrap().clone().unwrap_or("".to_string()),
                        username: fields.get(&"username".to_string()).unwrap().clone().unwrap_or("".to_string()),
                        password: fields.get(&"password".to_string()).unwrap().clone().unwrap_or("".to_string()),
                    });
                }
            }
        }
        Err(err) => { println!("Could not open, exiting: {}", err); return; }
    }

    println!("Done! Loaded {} accounts", accounts.len());

    let mut logger = Logger::new(log_path.as_str()).unwrap();

    let mut last_ip = String::new();

    let mut retry_accounts: Vec<usize> = vec![];
    let mut retry_time: u64 = 0;

    loop {

        match get_public_ip()
        {
            Ok(ip) => {

                if !is_ipv4(&ip)
                {
                    logger.error(format!("Got wrong public IP '{}', retrying", ip).as_str());
                }
                else
                {
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

                    if ip != last_ip
                    {
                        retry_accounts.clear();

                        last_ip = ip.clone();

                        for (idx, acc) in accounts.iter().enumerate()
                        {
                            match update_ip(acc, &ip, &mut logger)
                            {
                                Ok(_) => {
                                    logger.info(format!("Updated {} to {}", acc.domain, ip).as_str());
                                }
                                Err(error_code) => {
                                    logger.error(format!("Encountered error while tried to update {}: HTTP Error {}; retrying in 2 minutes", acc.domain, error_code).as_str());

                                    retry_accounts.push(idx);
                                }
                            }
                        }

                        if !retry_accounts.is_empty()
                        {
                            retry_time = now + 120;
                        }
                    }
                    else
                    {
                        if retry_time <= now
                        {
                            let mut i = retry_accounts.len() as i32;

                            while i > 0
                            {
								let actuall_i = i-1;
								
                                if let Some(idx) = retry_accounts.get(actuall_i as usize)
                                {
                                    let idx = *idx;

                                    if let Some(acc) = accounts.get(idx)
                                    {
                                        match update_ip(acc, &ip, &mut logger)
                                        {
                                            Ok(_) => {
                                                logger.info(format!("Updated {} to {}", acc.domain, ip).as_str());

                                                retry_accounts.remove(actuall_i as usize);
                                            }
                                            Err(error_code) => {
                                                logger.error(format!("Encountered error while tried to update {}: HTTP Error {}; retrying in 2 minutes", acc.domain, error_code).as_str());
                                            }
                                        }
                                    }
                                }

                                i -= 1;
                            }

                            if !retry_accounts.is_empty()
                            {
                                retry_time = now + 120;
                            }
                        }
                    }
                }

                sleep(Duration::from_secs(1));

            }
            Err(error) => {

                logger.error(format!("Could not get public IP from FunBox: {}; retrying", error).as_str());
                sleep(Duration::from_secs(5));
            }
        }
    }
}
