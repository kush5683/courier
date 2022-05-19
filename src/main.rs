extern crate imap;
extern crate native_tls;
extern crate regex;
extern crate quoted_printable;
use std::error::Error;
use quoted_printable::{decode, ParseMode};
fn main() {
    let msg_text = fetch_inbox_top().expect("Something went wrong").unwrap();
    println!("{}", msg_text);
}

fn get_content_encoding(text: &str) -> &str {
    // println!("{}", text);
    let re = regex::Regex::new(r"Content-Transfer-Encoding: (.*)").unwrap();
    let caps = re.find(text).expect("No match found");
    return caps.as_str().split(':').collect::<Vec<&str>>()[1].trim();
    
}

fn get_quoted_printable_text(text: &str) -> Result<String, & 'static dyn Error> {
    let decoded = String::from_utf8(decode(text.as_bytes(), ParseMode::Robust).unwrap()).expect("Could not decode quoted printable");
    // println!("{}", decoded);
    let re = regex::Regex::new(r"<html[\s\S]*</html>").unwrap();
    let caps = re.find(&decoded).expect("No match found for html tags");
    Ok(caps.as_str().to_string())
    // .split("\r\n").collect::<Vec<&str>>()[1].trim();
}

fn fetch_inbox_top() -> imap::error::Result<Option<String>> {
    let domain = "outlook.office365.com";
    let tls = native_tls::TlsConnector::builder().build().unwrap();

    // we pass in the domain twice to check that the server's TLS
    // certificate is valid for the domain we're connecting to.
    let client = imap::connect((domain, 993), domain, &tls).unwrap();

    // the client we have here is unauthenticated.
    // to do anything useful with the e-mails, we need to log in
    let mut imap_session = client
        .login("kshah2@wpi.edu", "Bonjour2022!!")
        .map_err(|e| e.0)?;

    // we want to fetch the first email in the INBOX mailbox
    let mailbox = imap_session.select("INBOX")?;

    // fetch message number 1 in this mailbox, along with its RFC822 field.
    // RFC 822 dictates the format of the body of e-mails
    let messages = imap_session.fetch((mailbox.exists-3).to_string(), "(RFC822.HEADER RFC822.TEXT)")?;
    let message = if let Some(m) = messages.iter().next() {
        m
    } else {
        return Ok(None);
    };

    // extract the message's body
    // let body = message.body().expect("message did not have a body!");
    // let body = std::str::from_utf8(body)
    //     .expect("message was not valid utf-8")
    //     .to_string();
    


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
    } 
    Ok(Some(text))
}