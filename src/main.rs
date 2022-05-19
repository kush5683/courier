extern crate imap;
extern crate native_tls;
extern crate regex;
extern crate quoted_printable;
use std::{error::Error};
use dotenv::dotenv;
use quoted_printable::{decode, ParseMode};
fn main() -> Result<(), Box<dyn Error>> {
    //load the .env file to read in credentials
    dotenv().ok();
    //get the credentials from the .env file
    let user = std::env::var("USER")?;
    let password = std::env::var("PASS")?;
    //get latest email
    let msg_text = fetch_inbox_top(user, password,5).expect("Something went wrong").unwrap();
    println!("{}", msg_text);
    Ok(())
}

//Gets the content encoding type of the email to be decoded later
fn get_content_encoding(text: &str) -> &str {
    // println!("{}", text);
    let re = regex::Regex::new(r"Content-Transfer-Encoding: (.*)").unwrap();
    let caps = re.find(text);
    if !caps.is_none(){
        return caps.unwrap().as_str().split(':').collect::<Vec<&str>>()[1].trim();
    }
    return "custom";
    
}

fn get_html_section(text: &str, quoted_printable: bool) -> Result<String, Box<dyn Error>>{
    let re = regex::Regex::new(r"Content-Type: text/html[\s\S]*").unwrap();
    let caps = re.find(text).expect("No html section found");
    let mut html_text;
    if quoted_printable{
        html_text = caps.as_str().split("\n").collect::<Vec<&str>>()[3..].join("\n");
    }else{
        html_text = caps.as_str().split("\n").collect::<Vec<&str>>()[1..].join("\n");
    }
    
    // println!("html section\n{}", html_text);
    Ok(html_text.to_string())
}

fn get_base64_section(text: &str) -> Result<String, Box<dyn Error>>{
    let re = regex::Regex::new(r"\n\n[\s\S]*=\n").unwrap();
    let caps = re.find(text).expect("No base64 section found");
    let mut base64_text = String::from("");
    base64_text = caps.as_str().split("\n").collect::<Vec<&str>>()[1..].join("\n");
    // println!("base64 section\n{}", base64_text);
    Ok(base64_text.to_string())
}

//decodes quoted printable content encoding to plain text HTML
fn get_quoted_printable_text(text: &str) -> Result<String, Box<dyn Error>> {
    let html = String::from(get_html_section(text,true)?);
    let decoded = String::from_utf8(decode(html.as_bytes(), ParseMode::Robust).unwrap()).expect("Could not decode quoted printable");
    let mut content = String::from("<html>");
    content.push_str(&decoded);
    content.push_str("</html>");
    let re = regex::Regex::new(r"<html[\s\S]*</html>").unwrap();
    let caps = re.find(&content).expect("No match found for html tags");
    Ok(caps.as_str().to_string())
}

//gets the plain text section of the email and wraps it in html tags
fn get_custom_text(text: &str) -> Result<String, Box<dyn Error>> {
    let html = String::from(get_html_section(text,false)?);
    // println!("{}", html);
    let mut content = String::from("<html>");
    content.push_str(&html);
    content.push_str("</html>");
    let re = regex::Regex::new(r"<html[\s\S]*</html>").unwrap();
    let caps = re.find(&content).expect("No match found for html tags");
    Ok(caps.as_str().to_string())
}

//finds the base64 encoded image for placement in the html
fn get_base64_text(text: &str) -> Result<String, Box<dyn Error>> {
    let base64 = get_base64_section(text)?;
    Ok(base64.trim().to_string())
}

//gets the email text of the latest message in the main inbox of the given user
fn fetch_inbox_top(user: String, pass: String, offset: u32) -> imap::error::Result<Option<String>> {
    //imap server domain
    let domain = "outlook.office365.com";
    let tls = native_tls::TlsConnector::builder().build().unwrap();

    // we pass in the domain twice to check that the server's TLS
    // certificate is valid for the domain we're connecting to.
    let client = imap::connect((domain, 993), domain, &tls).unwrap();

    // the client we have here is unauthenticated.
    // to do anything useful with the e-mails, we need to log in
    let mut imap_session = client
        .login(user, pass)
        .map_err(|e| e.0)?;

    // we want to fetch the first email in the INBOX mailbox
    let mailbox = imap_session.select("INBOX")?;

    // fetch message number 1 in this mailbox, along with its RFC822 field.
    // RFC 822 dictates the format of the body of e-mails
    let messages = imap_session.fetch((mailbox.exists-offset).to_string(), "(RFC822.HEADER RFC822.TEXT)")?; 
    let message = if let Some(m) = messages.iter().next() {
        m
    } else {
        return Ok(None);
    };

    let text = message.text().expect("message did not have a text body!");
    let mut text = std::str::from_utf8(text)
        .expect("message was not valid utf-8")
        .to_string();

    let encoding = get_content_encoding(&text);
    println!("{:?}", encoding);
    // be nice to the server and log out
    imap_session.logout()?;
    
    if encoding == "quoted-printable" {
        text = get_quoted_printable_text(&text).expect("Could not decode quoted printable");
    } else if encoding == "custom"{
        text = get_custom_text(&text).expect("Could not get html section");
    }
    Ok(Some(text))
}