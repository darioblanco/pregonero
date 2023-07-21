use std::env;
use tracing::Level;

pub trait Environment {
	fn get_var(&self, var: &str) -> Result<String, env::VarError>;
}

pub struct SystemEnvironment;

impl Environment for SystemEnvironment {
	fn get_var(&self, var: &str) -> Result<String, env::VarError> {
		env::var(var)
	}
}

#[derive(Clone, Debug)]
pub struct Config {
	pub imap_server: String,
	pub imap_username: String,
	pub imap_password: String,
	pub imap_mailbox: String,
	pub log_level: Level,
	pub redis_server: String,
	pub version: String,
}

impl Config {
	pub fn from_env<T: Environment>(env: &T) -> Config {
		let imap_host = env
			.get_var("IMAP_HOST")
			.unwrap_or_else(|_| "localhost".to_string());
		let imap_port: u16 = env
			.get_var("IMAP_PORT")
			.unwrap_or_else(|_| "993".to_string())
			.parse()
			.unwrap_or(993);
		let imap_username = env
			.get_var("IMAP_USERNAME")
			.unwrap_or_else(|_| "".to_string());
		let imap_password = env
			.get_var("IMAP_PASSWORD")
			.unwrap_or_else(|_| "".to_string());
		let imap_mailbox = env
			.get_var("IMAP_MAILBOX")
			.unwrap_or_else(|_| "inbox".to_string());
		let log_level = env
			.get_var("LOG_LEVEL")
			.unwrap_or_else(|_| "info".to_string());
		let redis_host = env
			.get_var("REDIS_HOST")
			.unwrap_or_else(|_| "localhost".to_string());
		let redis_port: u16 = env
			.get_var("REDIS_PORT")
			.unwrap_or_else(|_| "6379".to_string())
			.parse()
			.unwrap_or(6379);
		let version = env
			.get_var("VERSION")
			.unwrap_or_else(|_| "experimental".to_string());

		let imap_server = format!("{}:{}", imap_host, imap_port)
			.parse()
			.expect("Failed to parse IMAP_HOST and IMAP_PORT");
		let redis_server = format!("redis://{}:{}", redis_host, redis_port)
			.parse()
			.expect("Failed to parse REDIS_HOST and REDIS_PORT");

		let log_level = match log_level.to_lowercase().as_str() {
			"trace" => Level::TRACE,
			"debug" => Level::DEBUG,
			"info" => Level::INFO,
			"warn" => Level::WARN,
			"error" => Level::ERROR,
			_ => Level::INFO,
		};

		Config {
			imap_server,
			imap_username,
			imap_password,
			imap_mailbox,
			log_level,
			redis_server,
			version,
		}
	}

	pub fn from_params(version: String) -> Config {
		Config {
			imap_server: "127.0.0.1:993".to_string().parse().unwrap(),
			imap_username: "username".to_string(),
			imap_password: "password".to_string(),
			imap_mailbox: "inbox".to_string(),
			log_level: Level::INFO,
			redis_server: "redis://127.0.0.1:6359".to_string().parse().unwrap(),
			version,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct MockEnvironment {
		vars: std::collections::HashMap<String, String>,
	}

	impl Environment for MockEnvironment {
		fn get_var(&self, var: &str) -> Result<String, env::VarError> {
			match self.vars.get(var) {
				Some(val) => Ok(val.to_owned()),
				None => Err(env::VarError::NotPresent),
			}
		}
	}

	#[test]
	fn test_config_from_env() {
		let mut vars = std::collections::HashMap::new();
		vars.insert("IMAP_HOST".to_string(), "myimaphost".to_string());
		vars.insert("IMAP_PORT".to_string(), "143".to_string());
		vars.insert("IMAP_USERNAME".to_string(), "myuser".to_string());
		vars.insert("IMAP_PASSWORD".to_string(), "secret".to_string());
		vars.insert("IMAP_MAILBOX".to_string(), "mymailbox".to_string());
		vars.insert("LOG_LEVEL".to_string(), "warn".to_string());
		vars.insert("REDIS_HOST".to_string(), "myredishost".to_string());
		vars.insert("REDIS_PORT".to_string(), "6359".to_string());
		vars.insert("VERSION".to_string(), "myversion".to_string());
		let env = MockEnvironment { vars };
		let config = Config::from_env(&env);
		assert_eq!(config.imap_server, "myimaphost:143".to_string());
		assert_eq!(config.imap_username, "myuser");
		assert_eq!(config.imap_password, "secret");
		assert_eq!(config.imap_mailbox, "mymailbox");
		assert_eq!(config.log_level, Level::WARN);
		assert_eq!(
			config.redis_server.to_string(),
			"redis://myredishost:6359".to_string()
		);
		assert_eq!(config.version.to_string(), "myversion".to_string());
	}

	#[test]
	fn test_config_from_params() {
		let config = Config::from_params("test".to_string());
		assert_eq!(config.imap_server, "127.0.0.1:993".to_string());
		assert_eq!(config.imap_username, "username");
		assert_eq!(config.imap_password, "password");
		assert_eq!(config.imap_mailbox, "inbox");
		assert_eq!(config.log_level, Level::INFO);
		assert_eq!(
			config.redis_server.to_string(),
			"redis://127.0.0.1:6359".to_string()
		);
		assert_eq!(config.version.to_string(), "test".to_string());
	}
}
