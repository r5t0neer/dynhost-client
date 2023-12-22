use reqwest::blocking::Client;

use crate::log::Logger;

pub struct DynHostAccount
{
    pub domain: String,
    pub username: String,
    pub password: String,
}

pub struct OVHClient
{
    http_client: Client,
    accounts: Vec<DynHostAccount>
}

impl OVHClient
{
    pub fn new(accounts: Vec<DynHostAccount>) -> Result<OVHClient, String>
    {
        Ok(OVHClient { 
            http_client: Client::builder()
            .deflate(true)
            .gzip(true)
            .brotli(true)
            .use_native_tls()
            .build()
            .map_err(|e| e.to_string())?,
            accounts
        })
    }

    pub fn update_ip(&self, new_ip: String, logger: &mut Logger) -> Result<(), String>
    {
        for account in &self.accounts
        {
            let response = self.http_client
                .get(format!("https://www.ovh.com/nic/update?system=dyndns&hostname={}&myip={}", account.domain, &new_ip))
                .basic_auth(account.username.as_str(), Some(account.password.as_str()))
                .send()
                .map_err(|e| format!("Encountered error while tried to update {}: Could not create request: {}", account.domain, e.to_string()))?;

            if !response.status().is_success()
            {
                logger.error(format!("Encountered error while tried to update {}: HTTP status {}", account.domain, response.status().as_u16()).as_str());
            }
            else
            {
                match response.text()
                {
                    Ok(text) => {
                        if text.starts_with(format!("good {}", &new_ip).as_str()) 
                        {
                            logger.info(format!("Updated {} to {}", account.domain, &new_ip).as_str());
                        }
                        else if text.starts_with(format!("nochg {}", &new_ip).as_str()) 
                        {
                            logger.info(format!("{} is already mapped to {}", account.domain, &new_ip).as_str());
                        }
                        else
                        {
                            logger.error(format!("Encountered error while tried to update {}: {}", account.domain, text).as_str());
                        }
                    }
                    Err(e) => 
                    {
                        logger.error(format!("Encountered error while tried to update {}: Could not get body: {}", account.domain, e).as_str());
                    }
                }
            }
        }

        Ok(())
    }
}