use tracing::info;

use anyhow::Result;

pub mod codecs;
pub mod config;
pub mod email;

#[tokio::main]
pub async fn main() -> Result<()> {
    // Load configuration variables
    let config = config::Config::from_env(&config::SystemEnvironment);
    // install global collector configured based on RUST_LOG env var.
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(format!("pregonero={}", config.log_level).parse().unwrap())
        .add_directive("html2text=info".parse().unwrap());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        // .with_max_level(config.log_level)
        .init();

    info!("Initializing config with {:?}", config);

    let res = email::fetch_inbox(
        &config.imap_host,
        &config.imap_username,
        &config.imap_password,
    )
    .await;

    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            info!("Error while fetching inbox: {:?}", e);
            Err(e)
        }
    }

    // if let Err(cause) = imap_fetch("imap.mail.me.com", config.imap_username, config.imap_password, config.imap_mailbox).await {
    // 	error!("Fatal error: {cause}");
    // } else {
    // 	info!("IMAP messages fetched successfully");
    // }
}

// async fn imap_fetch(
// 	server: &str,
// 	login: String,
// 	password: String,
// 	mailbox: String,
// ) -> Result<(), ImapError> {
// 	info!("Will connect to {server}");
// 	let (_, mut tls_client) = client::TlsClient::connect(server)
// 		.await
// 		.map_err(|e| ImapError::Connect { cause: e })?;

// 	let responses = tls_client
// 		.call(CommandBuilder::login(&login, &password))
// 		.try_collect::<Vec<_>>()
// 		.await
// 		.map_err(|e| ImapError::Login { cause: e })?;
// 	debug!("Login responses: {responses:?}");

// 	match responses[0].parsed() {
// 		Response::Capabilities(_) => {}
// 		Response::Done { information, .. } => {
// 			if let Some(info) = information {
// 				// Log the information and continue
// 				info!("Login detected: {info:?}");
// 			} else {
// 				// If there's no information, consider it as a login failure
// 				return Err(ImapError::Login {
// 					cause: io::Error::new(io::ErrorKind::Other, "login failed"),
// 				});
// 			}
// 		}
// 		_ => unimplemented!(),
// 	}

// 	let _ = tls_client
// 		.call(CommandBuilder::select(&mailbox))
// 		.try_collect::<Vec<_>>()
// 		.await
// 		.map_err(|e| ImapError::Select { cause: e })?;

// 	// Fetch emails
// 	let cmd = CommandBuilder::fetch()
// 		.range_from(1_u32..) // TODO - Fetch emails in a paginated way
// 		.attr(Attribute::Uid)
// 		.attr(Attribute::Rfc822Text)
// 		.attr(Attribute::Envelope);
// 	tls_client
// 		.call(cmd)
// 		.try_for_each(process_email)
// 		.await
// 		.map_err(|e| ImapError::UidFetch { cause: e })?;

// 	let _ = tls_client
// 		.call(CommandBuilder::close())
// 		.try_collect::<Vec<_>>()
// 		.await
// 		.map_err(|e| ImapError::Close { cause: e })?;

// 	info!("Finished fetching messages");
// 	Ok(())
// }

// async fn process_email(response_data: codec::ResponseData) -> Result<(), io::Error> {
// 	if let Response::Fetch(_, ref attr_vals) = *response_data.parsed() {
// 		// debug!("Message: {response_data:?} || {attr_vals:?}");
// 		debug!("EMAIL TO PARSE -------");
// 		for val in attr_vals {
// 			match val {
// 				AttributeValue::Uid(u) => {
// 					info!("Message UID: {u}");
// 				}
// 				AttributeValue::Envelope(src) => {
// 					// let envelope = str::from_utf8(src.subject).unwrap();
// 					// info!("Date: {:?}", src.date);
// 					// info!("From: {:?}", src.from);
// 					// info!("To: {:?}", src.to);
// 					// info!("Sender: {:?}", src.sender);
// 					// info!("Subject: {:?}", src.subject);
// 					// let parsed_mail: ParsedMail = parse_header(&Some(src.subject)).unwrap();

//     				if let Some(subject_bytes) = &src.subject {
// 						let encoded_subject = std::str::from_utf8(&subject_bytes).unwrap();
// 						if encoded_subject.starts_with("=?") && encoded_subject.ends_with("?=") {
// 							// Create a regex to match special characters.
// 							let re = Regex::new(r"=\w{2}").unwrap();
// 							// Remove special characters from the encoded_subject.
// 							let filtered_subject = re.replace_all(&encoded_subject, "");

// 							// // If the subject line is encoded according to RFC 2047,
// 							// // decode it using the appropriate function.
// 							// let encoded_lines: Vec<&str> = filtered_subject.split_whitespace().collect();
// 							// let mut decoded_subject = String::new();

// 							// for line in encoded_lines {
// 							// 	let result = rfc::decode_rfc2047(line);
// 							// 	match result {
// 							// 		Ok(decoded_line) => {
// 							// 			decoded_subject.push_str(&decoded_line);
// 							// 		},
// 							// 		Err(e) => {
// 							// 			error!("Unable to decode subject line: {}. Original subject line {}", e, line);
// 							// 		},
// 							// 	}
// 							// }
// 							// info!("Decoded subject: {}", decoded_subject);

// 							let decoded_subject = rfc::decode_rfc2047(&filtered_subject);
// 							match decoded_subject {
// 								Ok(decoded_subject) => {
// 									info!("Decoded subject: {}", decoded_subject);
// 								}
// 								Err(e) => {
// 									error!("Unable to decode subject: {}. Original subject {}", e, encoded_subject);
// 								}
// 							}
// 						} else {
// 							// If the subject line is not encoded, use it as is.
// 							info!("Subject: {}", encoded_subject);
// 						}
// 					} else {
// 						info!("No subject");
// 					}
// 				}
// 				AttributeValue::Rfc822Text(Some(src)) => {
// 					info!("Body length: {}", src.to_vec().len());
// 					let parsed_mail: ParsedMail = parse_mail(src).unwrap();
// 					// info!("Headers {:?}", parsed_mail.headers);
// 					// info!("Subject: {}", parsed_mail.headers.get_first_value("Subject").unwrap_or_default());
// 					info!("Body headers: {:?}", parsed_mail.headers);
// 					info!("Body: {}", parsed_mail.get_body().unwrap_or_default().split("\n").next().unwrap_or_default());
// 				}
// 				_ => (),
// 			}
// 		}
// 		debug!("EMAIL PARSED -------");
// 	}
// 	Ok(())
// }

// #[derive(Debug)]
// pub enum ImapError {
// 	Connect { cause: io::Error },
// 	Login { cause: io::Error },
// 	Select { cause: io::Error },
// 	UidFetch { cause: io::Error },
// 	Close { cause: io::Error },
// }

// impl Error for ImapError {
// 	fn description(&self) -> &'static str {
// 		""
// 	}

// 	fn cause(&self) -> Option<&dyn Error> {
// 		match *self {
// 			ImapError::Connect { ref cause }
// 			| ImapError::Login { ref cause }
// 			| ImapError::Select { ref cause }
// 			| ImapError::UidFetch { ref cause }
// 			| ImapError::Close { ref cause } => Some(cause),
// 		}
// 	}
// }

// impl Display for ImapError {
// 	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
// 		match *self {
// 			ImapError::Connect { ref cause } => write!(f, "Connect failed: {cause}"),
// 			ImapError::Login { ref cause } => write!(f, "Login failed: {cause}"),
// 			ImapError::Select { ref cause } => write!(f, "Mailbox selection failed: {cause}"),
// 			ImapError::UidFetch { ref cause } => write!(f, "Fetching messages failed: {cause}"),
// 			ImapError::Close { ref cause } => write!(f, "Closing failed: {cause}"),
// 		}
// 	}
// }
