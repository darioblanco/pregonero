use async_imap::{imap_proto::Envelope, types::Fetch};
use itertools::Itertools;
use std::fmt;
use tracing::{debug, error};

use super::codecs;

#[derive(Debug)]
pub struct Message {
    pub account: String,
    pub senders: Vec<Address>,
    pub subject: String,
    pub body: String,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(account: {}, senders: {}, subject: {}, body: {})",
            self.account,
            self.senders.iter().map(|s| format!("{}", s)).join(", "),
            self.subject,
            self.body
        )
    }
}

#[derive(Debug)]
pub struct Address {
    pub name: Option<String>,
    pub email: String,
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name {
            Some(ref name) => write!(f, "{} <{}>", name, self.email),
            None => write!(f, "{}", self.email),
        }
    }
}

pub fn parse_message(account: String, raw_message: &Fetch) -> Option<Message> {
    let mut message = Message {
        account,
        senders: Vec::<Address>::new(),
        subject: "".to_string(),
        body: "".to_string(),
    };

    match raw_message.text() {
        Some(text) => message.body = parse_text(text),
        None => {
            debug!("message did not have a text!");
            // Unable to parse any type of message, go to the next one
            return None;
        }
    }
    match raw_message.envelope() {
        Some(envelope) => {
            message.senders = parse_sender(envelope);
            message.subject = parse_subject(envelope);
        }
        None => {
            debug!("message did not have an envelope!");
            // Unable to parse any type of envelope, go to the next one
            return None;
        }
    }
    debug!("message parsed: {}", message);
    return Some(message);
}

fn parse_sender(envelope: &Envelope<'_>) -> Vec<Address> {
    // Parse sender
    let mut senders = Vec::<Address>::new();
    match &envelope.sender {
        Some(sender) => {
            // Parse sender
            for address in sender {
                let email = format!(
                    "{}@{}",
                    String::from_utf8(address.mailbox.as_ref().unwrap().to_vec()).unwrap(),
                    String::from_utf8(address.host.as_ref().unwrap().to_vec()).unwrap()
                );
                let raw_name = address.name.as_ref();
                match raw_name {
                    Some(name) => senders.push(Address {
                        name: Some(String::from_utf8(name.to_vec()).unwrap()),
                        email,
                    }),
                    None => senders.push(Address { name: None, email }),
                }
            }
        }
        None => {
            debug!("message did not have a sender!");
            // Unable to parse any type of envelope, go to the next one
        }
    }
    return senders;
}

fn parse_subject(envelope: &Envelope<'_>) -> String {
    if let Some(ascii_subject) = &envelope.subject {
        let subject = String::from_utf8(ascii_subject.to_vec()).unwrap();
        if subject.starts_with("=?") && subject.ends_with("?=") {
            // RFC2047 encoding detected
            let result = codecs::decode_rfc2047(&subject);
            match result {
                Ok(decoded_subject) => {
                    return decoded_subject;
                }
                Err(e) => {
                    error!(
                        "Unable to decode subject line: {}. Original subject line {}",
                        e, subject
                    );
                    return subject;
                }
            }
        } else {
            return subject;
        }
    } else {
        error!("unable to read subject from {:?}", envelope.subject);
        return "Not yet".to_string();
    }
}

fn parse_text(text: &[u8]) -> String {
    let parsed = mailparse::parse_mail(text);
    match parsed {
        Ok(parsed) => {
            let decoded_body = parsed.get_body();
            match decoded_body {
                Ok(decoded_body) => {
                    if decoded_body.contains("<!DOCTYPE html>") || decoded_body.contains("<html>") {
                        // Strip HTML (and do not wrap the lines)
                        let text = html2text::from_read(decoded_body.as_bytes(), usize::MAX);
                        return text;
                    } else {
                        return decoded_body;
                    }
                }
                Err(e) => {
                    error!("Unable to decoded body with mailparser: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Unable to parse email with mailparser: {}", e);
        }
    }

    // Try manual parsing if there is a mailparse error
    let text = std::str::from_utf8(text)
        .expect("message was not valid utf-8")
        .to_string();
    if text.contains("<!DOCTYPE html>") || text.contains("<html>") {
        // Strip HTML (and do not wrap the lines)
        let text = html2text::from_read(text.as_bytes(), usize::MAX);
        return text;
    } else {
        return text;
    }
}
