#[macro_use]
extern crate serde;

use std::fs::OpenOptions;
use std::io::Read;
use std::thread::sleep;
use std::time::Duration;
use funbox::Session;
use ini::configparser::ini::Ini;
use ovh::{DynHostAccount, OVHClient};
use util::is_ipv4;

use crate::log::Logger;

mod log;
mod funbox;
mod ovh;
mod util;

pub const ROUTER_IP: &str = "192.168.1.1";
pub const ROUTER_ADMIN_PASSWORD: &str = "password";

fn get_log_file_path() -> String
{
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

    log_path
}

fn create_logger() -> Result<Logger, String>
{
    println!("Opening 'log_path.txt'...");

    Logger::new(&get_log_file_path().as_str()).map_err(|e| e.to_string())
}

fn get_accounts() -> Option<Vec<DynHostAccount>>
{
    println!("Opening 'accounts.ini'...");

    let mut accounts = vec![];

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
                    accounts.push(DynHostAccount {
                        domain: fields.get(&"domain".to_string()).unwrap().clone().unwrap_or("".to_string()),
                        username: fields.get(&"username".to_string()).unwrap().clone().unwrap_or("".to_string()),
                        password: fields.get(&"password".to_string()).unwrap().clone().unwrap_or("".to_string()),
                    });
                }
            }
        }
        Err(err) => { println!("Could not open, exiting: {}", err); return None; }
    }

    println!("Done! Loaded {} accounts", accounts.len());

    Some(accounts)
}

fn main()
{
    let mut logger = match create_logger()
    {
        Ok(a) => a,
        Err(e) => {
            println!("Could not create logger, exiting: {}", e);
            return;
        }
    };

    let accounts = if let Some(accounts) = get_accounts()
    {
        accounts
    }
    else
    {
        return;
    };

    let mut router_session = match Session::connect(ROUTER_IP)
    {
        Ok(sess) => sess,
        Err(e) => {
            println!("Could not create FunBox session, exiting: {}", e);
            return;
        }
    };

    let dynhost_client = match OVHClient::new(accounts)
    {
        Ok(client) => client,
        Err(e) => {
            println!("Could not create DynHost client, exiting: {}", e);
            return;
        },
    };

    let mut last_ip = String::new();

    let mut retry_accounts: Vec<usize> = vec![];
    let mut retry_time: u64 = 0;

    loop 
    {
        match router_session.get_public_ip()
        {
            Ok(pub_ip) => 
            {

                if !is_ipv4(&pub_ip)
                {
                    logger.error(format!("Got wrong public IP '{}', retrying", pub_ip).as_str());
                    sleep(Duration::from_secs(15));
                }
                else if pub_ip != last_ip
                {
                    logger.info(format!("Detected that public IP changed to {}, updating...", &pub_ip).as_str());
                    match dynhost_client.update_ip(pub_ip.clone(), &mut logger)
                    {
                        Ok(_) => {},
                        Err(e) => { logger.error(e.as_str()); },
                    }

                    last_ip = pub_ip;
                }

                sleep(Duration::from_secs(1));

            },
            Err(e) => 
            {
                match e
                {
                    Ok(msg) => {
                        logger.error(format!("Could not get public IP from FunBox: {}; retrying in 30s", msg).as_str());
                        sleep(Duration::from_secs(30));
                    },
                    Err(_) => 
                    {
                        match router_session.login()
                        {
                            Ok(_) => todo!(),
                            Err(e) => {
                                match e {
                                    Ok(msg) => {
                                        logger.error(format!("Could not reconnect to router, exiting: {}", msg).as_str());
                                        return;
                                    },
                                    Err(_) => {
                                        logger.error("Could not reconnect to router - unexepected access unauthorized 401, exiting");
                                        return;
                                    },
                                }
                            },
                        }
                    },
                }
            },
        }
        /*match router_session.get_public_ip()
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
                                Ok(status) => {
                                    match status {
                                        Good => { logger.info(format!("Updated {} to {}", acc.domain, ip).as_str()); }
                                        NoChange => { logger.info(format!("{} is already mapped to {}", acc.domain, ip).as_str()); }
                                    }
                                }
                                Err(error_code) => {
                                    logger.error(format!("Encountered error while tried to update {}: {}; retrying in 1 minute", acc.domain, error_code).as_str());

                                    retry_accounts.push(idx);
                                }
                            }
                        }

                        if !retry_accounts.is_empty()
                        {
                            retry_time = now + 60;
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
                                            Ok(status) => {
                                                match status {
                                                    Good => { logger.info(format!("Updated {} to {}", acc.domain, ip).as_str()); }
                                                    NoChange => { logger.info(format!("{} is already mapped to {}", acc.domain, ip).as_str()); }
                                                }

                                                retry_accounts.remove(actuall_i as usize);
                                            }
                                            Err(error_code) => {
                                                logger.error(format!("Encountered error while tried to update {}: {}; retrying in 1 minute", acc.domain, error_code).as_str());
                                            }
                                        }
                                    }
                                }

                                i -= 1;
                            }

                            if !retry_accounts.is_empty()
                            {
                                retry_time = now + 60;
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
        }*/
    }
}
